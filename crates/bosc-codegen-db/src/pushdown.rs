//! Total translation of a codeunit method into PL/pgSQL.
//!
//! A body is either translated completely or refused. Partial SQL is not emitted:
//! a skipped line would drop business logic.

use bosc_ir::ActionIR;
use bosc_ir::MethodIR;

pub fn body(method: &MethodIR) -> Result<String, String> {
    if let Some(script) = method.script.as_deref().filter(|s| !s.trim().is_empty()) {
        return translate_script(script);
    }
    if method.actions.is_empty() {
        return Err("the method has no body to run inside PostgreSQL".to_string());
    }
    translate_actions(&method.actions)
}

fn translate_actions(actions: &[ActionIR]) -> Result<String, String> {
    let mut sql = String::new();
    for action in actions {
        sql.push_str(&translate_action(action)?);
    }
    Ok(sql)
}

fn translate_action(action: &ActionIR) -> Result<String, String> {
    match action {
        ActionIR::ValidateRequired { field, .. } => Ok(format!(
            "    IF (p_params->>'{field}') IS NULL OR TRIM(p_params->>'{field}') = '' THEN\n        RAISE EXCEPTION 'Business Rule Violation: Field ''%'' is required', '{field}';\n    END IF;\n"
        )),
        ActionIR::Assert { condition, message, .. } => {
            let cond = sql_comparison(condition).ok_or_else(|| {
                format!("assertion '{condition}' is not a single comparison")
            })?;
            Ok(format!(
                "    IF NOT ({cond}) THEN\n        RAISE EXCEPTION 'Business Rule Assertion Failed: {}';\n    END IF;\n",
                message.replace('\'', "''")
            ))
        }
        ActionIR::SetField { field, value_expr, .. } => {
            let expr = literal_jsonb(value_expr)?;
            Ok(format!("    p_params := jsonb_set(p_params, ARRAY['{field}'], {expr});\n"))
        }
        ActionIR::LogAudit { message, .. } => Ok(audit_insert(message)),
        ActionIR::GetRecord { table, filter, target_buffer, .. } => {
            if filter.contains(';') || filter.contains("--") {
                return Err("getRecord filter is not a single predicate".to_string());
            }
            Ok(format!(
                "    SELECT to_jsonb(t.*) INTO v_rec FROM \"{}\" t WHERE {filter} LIMIT 1;\n    IF v_rec IS NOT NULL THEN\n        p_params := jsonb_set(p_params, ARRAY['{target_buffer}'], v_rec);\n    END IF;\n",
                table.to_lowercase()
            ))
        }
        ActionIR::CallProcedure { codeunit, method, .. } => {
            let target = format!("bos_proc_{}_{}", codeunit.to_lowercase(), method.to_lowercase());
            Ok(format!("    p_params := \"{target}\"(p_params, p_ctx);\n"))
        }
        ActionIR::IfElse { condition, then_branch, else_branch, .. } => {
            let cond = sql_comparison(condition).ok_or_else(|| {
                format!("if condition '{condition}' is not a single comparison")
            })?;
            let mut sql = format!("    IF {cond} THEN\n");
            sql.push_str(&translate_actions(then_branch)?);
            if let Some(else_branch) = else_branch {
                sql.push_str("    ELSE\n");
                sql.push_str(&translate_actions(else_branch)?);
            }
            sql.push_str("    END IF;\n");
            Ok(sql)
        }
        ActionIR::HashPassword { .. } => {
            Err("hashPassword stays in the QuickJS host".to_string())
        }
        ActionIR::ForEachRecord { .. } => {
            Err("a record loop stays in QuickJS; PostgreSQL pushdown has no partial for-each".to_string())
        }
        ActionIR::NotLowered { kind } => {
            Err(format!("{kind} stays in QuickJS; it is not a PostgreSQL action"))
        }
    }
}

fn translate_script(script: &str) -> Result<String, String> {
    let mut sql = String::new();
    for raw_line in script.lines() {
        let line = raw_line.trim().trim_end_matches(';').trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if line == "}" {
            sql.push_str("    END IF;\n");
            continue;
        }
        if line == "} else {" {
            sql.push_str("    ELSE\n");
            continue;
        }
        sql.push_str(&translate_script_line(line)?);
    }
    Ok(sql)
}

fn translate_script_line(line: &str) -> Result<String, String> {
    if let Some(cond) = line.strip_prefix("if (").and_then(|s| s.strip_suffix(") {")) {
        let cond = sql_comparison(cond.trim()).ok_or_else(|| {
            format!("if condition '{}' is not a single comparison", cond.trim())
        })?;
        return Ok(format!("    IF {cond} THEN\n"));
    }
    if let Some(rest) = line.strip_prefix("throw new Error(").or_else(|| line.strip_prefix("error(")) {
        let msg = rest.trim_end_matches(')').trim().trim_matches('"').trim_matches('\'');
        return Ok(format!("    RAISE EXCEPTION '{}';\n", msg.replace('\'', "''")));
    }
    if let Some(rest) = line.strip_prefix("rec.testField(") {
        let inner = rest.trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_matches('"').trim_matches('\'')).collect();
        if parts.len() == 1 && !parts[0].is_empty() {
            let field = parts[0];
            return Ok(format!(
                "    IF (p_params->>'{field}') IS NULL OR TRIM(p_params->>'{field}') = '' THEN\n        RAISE EXCEPTION 'Business Rule Violation: Field ''%'' is required', '{field}';\n    END IF;\n"
            ));
        }
        if parts.len() >= 2 {
            return Ok(format!(
                "    IF COALESCE(p_params->>'{}', '') <> '{}' THEN\n        RAISE EXCEPTION 'Field ''{}'' assertion failed: expected ''{}''';\n    END IF;\n",
                parts[0],
                parts[1].replace('\'', "''"),
                parts[0],
                parts[1].replace('\'', "''")
            ));
        }
        return Err("testField needs a field name".to_string());
    }
    if let Some((field, value)) = line.strip_prefix("rec.").and_then(|r| r.split_once('=')) {
        let field = field.trim();
        if !field.is_empty() && !field.contains('(') && !field.contains('.') {
            let expr = literal_jsonb(value)?;
            return Ok(format!("    p_params := jsonb_set(p_params, ARRAY['{field}'], {expr});\n"));
        }
    }
    if let Some(rest) = line.strip_prefix("ctx.audit.log(") {
        let msg = rest.trim_end_matches(')').trim().trim_matches('"').trim_matches('\'');
        return Ok(audit_insert(msg));
    }
    Err(format!("'{line}' stays on QuickJS"))
}

fn audit_insert(message: &str) -> String {
    format!(
        "    INSERT INTO \"_system_privileged_audit_log\" (\"table_name\", \"record_id\", \"action\", \"reason\", \"ticket_ref\", \"authorized_by\") VALUES ('codeunit', COALESCE(p_ctx->>'company_id', 'SYSTEM'), 'EXECUTE', '{}', 'AMDP-ENGINE', COALESCE(p_ctx->>'user_id', 'SYSTEM'));\n",
        message.replace('\'', "''")
    )
}

fn literal_jsonb(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.parse::<f64>().is_ok() || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("false") {
        return Ok(format!("to_jsonb({value}::text)"));
    }
    let quoted = (value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2)
        || (value.starts_with('"') && value.ends_with('"') && value.len() >= 2);
    if quoted {
        let inner = &value[1..value.len() - 1];
        return Ok(format!("to_jsonb('{}'::text)", inner.replace('\'', "''")));
    }
    Err(format!("value '{value}' is not a literal"))
}

/// One comparison: `field === "Open"`, `amount >= 10`. Longer operators are tried first.
#[cfg(test)]
mod tests {
    use super::*;
    use bosc_ir::ActionIR;

    #[test]
    fn refuses_a_query_inside_the_body() {
        let method = MethodIR {
            name: "Load".into(),
            is_public: true,
            is_static: false,
            params: Vec::new(),
            return_type: None,
            trigger_event: None,
            annotations: Vec::new(),
            actions: Vec::new(),
            script: Some("const item = Item.get(rec.itemNo);".into()),
            is_pushdown: true,
        };
        assert!(body(&method).is_err());
    }

    #[test]
    fn translates_an_assignment_and_a_comparison() {
        let method = MethodIR {
            name: "Mark".into(),
            is_public: true,
            is_static: false,
            params: Vec::new(),
            return_type: None,
            trigger_event: None,
            annotations: Vec::new(),
            actions: vec![ActionIR::SetField {
                field: "status".into(),
                value_expr: "\"Posted\"".into(),
                location: None,
            }],
            script: Some("if (rec.amount >= 10) {\n  rec.status = \"Posted\";\n}\n".into()),
            is_pushdown: false,
        };
        let sql = body(&method).expect("script translates");
        assert!(sql.contains(">= 10"), "{sql}");
        assert!(sql.contains("jsonb_set"), "{sql}");
        assert!(sql.contains("END IF"), "{sql}");
    }
}

fn sql_comparison(cond: &str) -> Option<String> {
    let cond = cond.trim();
    if cond.contains("&&") || cond.contains("||") {
        return None;
    }
    for op in ["===", "!==", "==", "!=", ">=", "<=", ">", "<"] {
        let Some((lhs, rhs)) = cond.split_once(op) else { continue };
        let field = lhs.trim().trim_start_matches("rec.").trim_start_matches("this.").trim();
        if field.is_empty() || field.contains(' ') || field.contains('.') || field.contains('(') {
            return None;
        }
        let rhs = rhs.trim();
        let sql_op = match op {
            "===" | "==" => "=",
            "!==" | "!=" => "<>",
            other => other,
        };
        if let Ok(number) = rhs.parse::<f64>() {
            return Some(format!("COALESCE((p_params->>'{field}')::numeric, 0) {sql_op} {number}"));
        }
        let quoted = (rhs.starts_with('"') && rhs.ends_with('"') && rhs.len() >= 2)
            || (rhs.starts_with('\'') && rhs.ends_with('\'') && rhs.len() >= 2);
        if quoted {
            let inner = &rhs[1..rhs.len() - 1];
            return Some(format!(
                "COALESCE(p_params->>'{field}', '') {sql_op} '{}'",
                inner.replace('\'', "''")
            ));
        }
        return None;
    }
    None
}
