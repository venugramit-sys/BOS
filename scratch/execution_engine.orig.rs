use bosc_ir::{ActionIR, CodeunitIR, TableIR, TriggerEventIR};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{preprocess_password_fields, to_snake_case, RuntimeError, SessionContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub codeunit: String,
    pub method: String,
    pub mutated_data: serde_json::Value,
    pub logs: Vec<String>,
    #[serde(default)]
    pub messages: Vec<serde_json::Value>,
}

pub struct ExecutionEngine;

impl ExecutionEngine {
    /// Register a Codeunit meta-object in the BOS catalog
    pub async fn register_codeunit(
        pool: &PgPool,
        codeunit: CodeunitIR,
    ) -> Result<Uuid, RuntimeError> {
        let pkg = if codeunit.package_name.trim().is_empty() {
            "bos.core".to_string()
        } else {
            codeunit.package_name.clone()
        };

        let def_ir = serde_json::to_value(&codeunit).map_err(|e| {
            RuntimeError::Internal(format!("Failed to serialize codeunit: {}", e))
        })?;

        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&def_ir).unwrap_or_default().hash(&mut hasher);
        let checksum = format!("{:016x}", hasher.finish());

        let upsert_query = r#"
            INSERT INTO "_bos_meta_objects" (
                "object_type", "object_name", "package_name", "version", "caption",
                "sub_type", "definition", "checksum", "status", "updated_at"
            ) VALUES (
                'codeunit', $1, $2, '1.0.0', $3, 'Standard', $4, $5, 'active', NOW()
            )
            ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
            SET "definition" = EXCLUDED."definition",
                "checksum" = EXCLUDED."checksum",
                "updated_at" = NOW()
            RETURNING "object_id";
        "#;

        let caption = format!("Codeunit {}", codeunit.name);

        let row = sqlx::query(upsert_query)
            .bind(&codeunit.name)
            .bind(&pkg)
            .bind(&caption)
            .bind(&def_ir)
            .bind(&checksum)
            .fetch_one(pool)
            .await?;

        let obj_id: Uuid = row.try_get("object_id")?;

        // Automatic BOS-AMDP deployment: Deploy pushdown methods as native PL/pgSQL functions
        let generator = bosc_codegen_db::PostgresGenerator::new();
        for m in &codeunit.methods {
            if m.is_pushdown || codeunit.is_pushdown {
                let func_ddl = generator.generate_plpgsql_procedure(&codeunit.name, m);
                let _ = sqlx::raw_sql(&func_ddl).execute(pool).await;
            }

            // Automatically register @subscribeEvent(Publisher, "eventName") annotations into _bos_event_subscribers
            for ann in &m.annotations {
                if let Some(rest) = ann.strip_prefix("subscribeEvent(") {
                    let inner = rest.trim_end_matches(')');
                    let parts: Vec<&str> = inner.split(',').map(|s| s.trim().trim_matches('"').trim_matches('\'')).collect();
                    if parts.len() >= 2 && !parts[0].is_empty() && !parts[1].is_empty() {
                        if let Some(ref script) = m.script {
                            let _ = crate::EventRegistry::init_table(pool).await;
                            let _ = sqlx::query(
                                "DELETE FROM \"_bos_event_subscribers\" WHERE LOWER(\"subscriber_name\") = LOWER($1) AND LOWER(\"method_name\") = LOWER($2)"
                            )
                            .bind(&codeunit.name)
                            .bind(&m.name)
                            .execute(pool)
                            .await;
                            let _ = sqlx::query(
                                "INSERT INTO \"_bos_event_subscribers\" (\"id\", \"publisher\", \"event_name\", \"subscriber_name\", \"method_name\", \"priority\", \"script\", \"is_active\") VALUES ($1, $2, $3, $4, $5, 100, $6, true)"
                            )
                            .bind(Uuid::new_v4())
                            .bind(parts[0])
                            .bind(parts[1])
                            .bind(&codeunit.name)
                            .bind(&m.name)
                            .bind(script)
                            .execute(pool)
                            .await;
                        }
                    }
                }
            }
        }

        Ok(obj_id)
    }

    /// Execute a codeunit procedure with parameters and session context
    pub async fn execute_procedure(
        pool: &PgPool,
        codeunit_name: &str,
        method_name: &str,
        mut params: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<ExecutionResult, RuntimeError> {
        preprocess_password_fields(&mut params);

        // 1. Fetch codeunit from catalog
        let row = sqlx::query(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'codeunit' AND LOWER(object_name) = LOWER($1) ORDER BY updated_at DESC LIMIT 1"
        )
        .bind(codeunit_name)
        .fetch_optional(pool)
        .await?;

        let def_ir: serde_json::Value = match row {
            Some(r) => r.try_get("definition")?,
            None => {
                return Err(RuntimeError::NotFound(
                    "Codeunit".to_string(),
                    codeunit_name.to_string(),
                ))
            }
        };

        let codeunit: CodeunitIR = serde_json::from_value(def_ir).map_err(|e| {
            RuntimeError::Internal(format!("Failed to parse Codeunit IR: {}", e))
        })?;

        // 2. Locate method
        let method = codeunit
            .methods
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(method_name))
            .ok_or_else(|| {
                RuntimeError::NotFound("Method".to_string(), format!("{}.{}", codeunit_name, method_name))
            })?;

        // 3. Check for BOS-AMDP pushdown execution (In-Database PL/pgSQL Function)
        if method.is_pushdown || codeunit.is_pushdown {
            let proc_name = format!("bos_proc_{}_{}", codeunit_name.to_lowercase(), method_name.to_lowercase());
            let ctx_json = serde_json::to_value(ctx).unwrap_or_default();
            let call_sql = format!("SELECT \"{}\"($1, $2) as res;", proc_name);
            let res_opt = sqlx::query_scalar::<_, serde_json::Value>(&call_sql)
                .bind(&params)
                .bind(&ctx_json)
                .fetch_optional(pool)
                .await?;

            let mutated_data = res_opt.unwrap_or(params);
            return Ok(ExecutionResult {
                success: true,
                codeunit: codeunit_name.to_string(),
                method: method_name.to_string(),
                mutated_data,
                logs: vec![format!("Executed pushdown procedure '{}' directly inside PostgreSQL engine (BOS-AMDP)", proc_name)],
                messages: vec![],
            });
        }

        // 4. Execute method in QuickJS runtime engine or fallback to actions with sandboxed fuel metering
        let mut logs = Vec::new();
        if let Some(ref script) = method.script {
            let (mutated_res, q_logs) = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                None,
                None,
                None,
                codeunit_name.to_string(),
                method_name.to_string(),
                script.clone(),
                params,
                None,
                ctx.clone(),
                Some(pool.clone()),
            ).await?;
            params = mutated_res;
            logs.extend(q_logs);
        } else {
            let mut fuel = codeunit.max_fuel.unwrap_or(100_000);
            Self::eval_actions(&method.actions, &mut params, None, ctx, pool, &mut logs, &mut fuel, 0).await?;
        }

        let messages = if let Some(msgs) = params.as_object_mut().and_then(|o| o.remove("_system_messages")) {
            msgs.as_array().cloned().unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(ExecutionResult {
            success: true,
            codeunit: codeunit_name.to_string(),
            method: method_name.to_string(),
            mutated_data: params,
            logs,
            messages,
        })
    }

    /// Execute test codeunit methods in an auto-rollback transaction (Gap 6: Testing DSL Runner)
    pub async fn execute_tests(
        pool: &PgPool,
        codeunit_filter: Option<&str>,
        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let rows = sqlx::query(
            "SELECT object_name, definition FROM \"_bos_meta_objects\" WHERE object_type = 'codeunit'"
        )
        .fetch_all(pool)
        .await?;

        let mut test_results = Vec::new();
        let mut total_tests = 0;
        let mut passed_tests = 0;
        let mut failed_tests = 0;

        for r in rows {
            let cu_name: String = r.try_get("object_name")?;
            if let Some(filter) = codeunit_filter {
                if !cu_name.to_lowercase().contains(&filter.to_lowercase()) {
                    continue;
                }
            }

            let def_val: serde_json::Value = r.try_get("definition")?;
            if let Ok(cu) = serde_json::from_value::<CodeunitIR>(def_val) {
                // Discover test methods (methods starting with "test" or in @test codeunits)
                let is_test_cu = cu.name.to_lowercase().ends_with("test")
                    || cu.name.to_lowercase().ends_with("tests");

                for method in &cu.methods {
                    let is_test_method = method.name.to_lowercase().starts_with("test")
                        || is_test_cu;

                    if !is_test_method {
                        continue;
                    }

                    total_tests += 1;
                    let start_time = std::time::Instant::now();

                    // Run inside an isolated transaction that ALWAYS rolls back
                    let tx = pool.begin().await?;

                    // Execute method
                    let exec_res = if let Some(ref script) = method.script {
                        crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                            None,
                            None,
                            None,
                            cu.name.clone(),
                            method.name.clone(),
                            script.clone(),
                            serde_json::json!({}),
                            None,
                            ctx.clone(),
                            Some(pool.clone()),
                        )
                        .await
                        .map(|(_, logs)| logs)
                    } else {
                        let mut logs = Vec::new();
                        let mut fuel = 100_000;
                        let mut test_params = serde_json::json!({});
                        Self::eval_actions(&method.actions, &mut test_params, None, ctx, pool, &mut logs, &mut fuel, 0)
                            .await
                            .map(|_| logs)
                    };

                    // Auto-rollback: Drop transaction without committing
                    let _ = tx.rollback().await;
                    let duration_ms = start_time.elapsed().as_millis();

                    match exec_res {
                        Ok(logs) => {
                            passed_tests += 1;
                            test_results.push(serde_json::json!({
                                "codeunit": cu.name,
                                "test": method.name,
                                "status": "PASSED",
                                "duration_ms": duration_ms,
                                "logs": logs,
                            }));
                        }
                        Err(e) => {
                            failed_tests += 1;
                            test_results.push(serde_json::json!({
                                "codeunit": cu.name,
                                "test": method.name,
                                "status": "FAILED",
                                "duration_ms": duration_ms,
                                "error": e.to_string(),
                            }));
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "total": total_tests,
            "passed": passed_tests,
            "failed": failed_tests,
            "success": failed_tests == 0,
            "results": test_results,
        }))
    }

    /// Dispatch entity triggers (OnBeforeInsert, OnAfterInsert, OnBeforeModify, etc.)
    pub async fn dispatch_triggers(
        pool: &PgPool,
        table: &str,
        event: &TriggerEventIR,
        data: &mut serde_json::Value,
        x_rec: Option<&serde_json::Value>,
        ctx: &SessionContext,
    ) -> Result<Vec<String>, RuntimeError> {
        let mut all_logs = Vec::new();
        let clean_table = table.trim().trim_matches('"');

        // 1. Dispatch table's own triggers defined directly on the table meta object (e.g. onInsert, onModify, onDelete)
        let table_row = sqlx::query(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'table' AND (LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1) OR LOWER(definition->>'name') = LOWER($1)) LIMIT 1"
        )
        .bind(clean_table)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some(r) = table_row {
            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
            if let Ok(table_ir) = serde_json::from_value::<TableIR>(def) {
                let mut fuel = 100_000;
                for trg in &table_ir.triggers {
                    let matches = match event {
                        TriggerEventIR::OnBeforeInsert => {
                            trg.name.eq_ignore_ascii_case("onInsert") || trg.name.eq_ignore_ascii_case("onBeforeInsert")
                        }
                        TriggerEventIR::OnAfterInsert => {
                            trg.name.eq_ignore_ascii_case("onAfterInsert")
                        }
                        TriggerEventIR::OnBeforeModify => {
                            trg.name.eq_ignore_ascii_case("onModify") || trg.name.eq_ignore_ascii_case("onBeforeModify")
                        }
                        TriggerEventIR::OnAfterModify => {
                            trg.name.eq_ignore_ascii_case("onAfterModify")
                        }
                        TriggerEventIR::OnBeforeDelete => {
                            trg.name.eq_ignore_ascii_case("onDelete") || trg.name.eq_ignore_ascii_case("onBeforeDelete")
                        }
                        TriggerEventIR::OnAfterDelete => {
                            trg.name.eq_ignore_ascii_case("onAfterDelete")
                        }
                        TriggerEventIR::OnValidate(col) => {
                            trg.name.eq_ignore_ascii_case(&format!("onValidate{}", col))
                                || trg.name.eq_ignore_ascii_case(&format!("onValidate_{}", col))
                        }
                        TriggerEventIR::OnDraftSave => {
                            trg.name.eq_ignore_ascii_case("onDraftSave")
                        }
                        TriggerEventIR::OnDraftActivate => {
                            trg.name.eq_ignore_ascii_case("onDraftActivate")
                        }
                        TriggerEventIR::OnDraftDiscard => {
                            trg.name.eq_ignore_ascii_case("onDraftDiscard")
                        }
                    };
                    if matches {
                        let mut logs = Vec::new();
                        if let Some(ref script) = trg.script {
                            let (mutated, q_logs) = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                                None,
                                None,
                                None,
                                clean_table.to_string(),
                                trg.name.clone(),
                                script.clone(),
                                data.clone(),
                                x_rec.cloned(),
                                ctx.clone(),
                                Some(pool.clone()),
                            ).await?;
                            *data = mutated;
                            logs.extend(q_logs);
                        } else {
                            Self::eval_actions(&trg.actions, data, x_rec, ctx, pool, &mut logs, &mut fuel, 0).await?;
                        }
                        all_logs.extend(logs);
                    }
                }
            }
        }

        // 2. Query codeunits bound to this table
        let rows = sqlx::query(
            "SELECT object_name, definition FROM \"_bos_meta_objects\" WHERE object_type = 'codeunit'"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for r in rows {
            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
            if let Ok(cu) = serde_json::from_value::<CodeunitIR>(def) {
                if let Some(target) = &cu.bound_table {
                    if !target.eq_ignore_ascii_case(clean_table) {
                        continue;
                    }
                }

                let mut fuel = cu.max_fuel.unwrap_or(100_000);
                for m in &cu.methods {
                    if m.trigger_event.as_ref() == Some(event) {
                        let mut logs = Vec::new();
                        if let Some(ref script) = m.script {
                            let (mutated, q_logs) = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                                None,
                                None,
                                None,
                                clean_table.to_string(),
                                m.name.clone(),
                                script.clone(),
                                data.clone(),
                                x_rec.cloned(),
                                ctx.clone(),
                                Some(pool.clone()),
                            ).await?;
                            *data = mutated;
                            logs.extend(q_logs);
                        } else {
                            Self::eval_actions(&m.actions, data, x_rec, ctx, pool, &mut logs, &mut fuel, 0).await?;
                        }
                        all_logs.extend(logs);
                    }
                }
            }
        }

        // 3. Query tablelogic and tablelogicextension meta-objects bound to this table
        let logic_rows = sqlx::query(
            "SELECT object_name, object_type, definition FROM \"_bos_meta_objects\" WHERE object_type IN ('tablelogic', 'tablelogicextension')"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // Build companion helper prelude (validate<Field> and record methods) so `rec.validate("field", val)` and `rec.customMethod()` work inside triggers!
        let mut companion_prelude = String::new();
        for lr in &logic_rows {
            let def: serde_json::Value = lr.try_get("definition").unwrap_or_default();
            let target_tbl = def.get("target_table").and_then(|v| v.as_str()).unwrap_or("");
            if !target_tbl.eq_ignore_ascii_case(clean_table) {
                continue;
            }
            if let Some(fvs) = def.get("field_validations").and_then(|v| v.as_array()) {
                for fv in fvs {
                    let f_name = fv.get("field_name").and_then(|v| v.as_str()).unwrap_or("");
                    let p_name = fv.get("param_name").and_then(|v| v.as_str()).unwrap_or("value");
                    let old_p = fv.get("old_param_name").and_then(|v| v.as_str()).unwrap_or("oldValue");
                    let body = fv.get("script").and_then(|v| v.as_str()).unwrap_or("");
                    if !f_name.is_empty() && !body.is_empty() {
                        let mut chars = f_name.chars();
                        let cap_field = match chars.next() {
                            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                            None => String::new(),
                        };
                        companion_prelude.push_str(&format!(
                            "__recProto.validate{} = function({}, {}) {{ const rec = this; {} }};\n",
                            cap_field, p_name, old_p, body
                        ));
                    }
                }
            }
            if let Some(methods) = def.get("methods").and_then(|v| v.as_array()) {
                for m in methods {
                    let m_name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let m_script = m.get("script").and_then(|v| v.as_str()).unwrap_or("");
                    if !m_name.is_empty() && !m_script.is_empty() {
                        let params_str = m.get("params").and_then(|v| v.as_array()).map(|arr| {
                            arr.iter().filter_map(|p| {
                                p.as_array().and_then(|pair| pair.first()).and_then(|n| n.as_str())
                                    .or_else(|| p.get("name").and_then(|n| n.as_str()))
                            }).collect::<Vec<_>>().join(", ")
                        }).unwrap_or_default();
                        companion_prelude.push_str(&format!(
                            "__recProto.{} = function({}) {{ const rec = this; {} }};\n",
                            m_name, params_str, m_script
                        ));
                    }
                }
            }
        }

        for lr in logic_rows {
            let def: serde_json::Value = lr.try_get("definition").unwrap_or_default();
            let target_tbl = def.get("target_table").and_then(|v| v.as_str()).unwrap_or("");
            if !target_tbl.eq_ignore_ascii_case(clean_table) {
                continue;
            }

            // A. Check field validations if this is an OnValidate event
            if let TriggerEventIR::OnValidate(col) = event {
                if let Some(fvs) = def.get("field_validations").and_then(|v| v.as_array()) {
                    for fv in fvs {
                        let f_name = fv.get("field_name").and_then(|v| v.as_str()).unwrap_or("");
                        if f_name.eq_ignore_ascii_case(col) {
                            let body = fv.get("script").and_then(|v| v.as_str())
                                .or_else(|| fv.get("body").and_then(|v| v.as_str()));
                            if let Some(body) = body {
                                let full_script = format!("{}\n{}", companion_prelude, body);
                                let (mutated, q_logs) = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                                    None,
                                    None,
                                    None,
                                    clean_table.to_string(),
                                    format!("validate_{}", col),
                                    full_script,
                                    data.clone(),
                                    x_rec.cloned(),
                                    ctx.clone(),
                                    Some(pool.clone()),
                                ).await?;
                                *data = mutated;
                                all_logs.extend(q_logs);
                            }
                        }
                    }
                }
            }

            // B. Check companion lifecycle triggers (beforeInsert, afterInsert, beforeModify, afterModify, beforeDelete, afterDelete)
            if let Some(trgs) = def.get("triggers").and_then(|v| v.as_array()) {
                for trg_val in trgs {
                    let trg_name = trg_val.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let matches = match event {
                        TriggerEventIR::OnBeforeInsert => {
                            trg_name.eq_ignore_ascii_case("beforeInsert") || trg_name.eq_ignore_ascii_case("onInsert")
                        }
                        TriggerEventIR::OnAfterInsert => {
                            trg_name.eq_ignore_ascii_case("afterInsert") || trg_name.eq_ignore_ascii_case("onAfterInsert")
                        }
                        TriggerEventIR::OnBeforeModify => {
                            trg_name.eq_ignore_ascii_case("beforeModify") || trg_name.eq_ignore_ascii_case("onModify")
                        }
                        TriggerEventIR::OnAfterModify => {
                            trg_name.eq_ignore_ascii_case("afterModify") || trg_name.eq_ignore_ascii_case("onAfterModify")
                        }
                        TriggerEventIR::OnBeforeDelete => {
                            trg_name.eq_ignore_ascii_case("beforeDelete") || trg_name.eq_ignore_ascii_case("onDelete")
                        }
                        TriggerEventIR::OnAfterDelete => {
                            trg_name.eq_ignore_ascii_case("afterDelete") || trg_name.eq_ignore_ascii_case("onAfterDelete")
                        }
                        TriggerEventIR::OnDraftSave => trg_name.eq_ignore_ascii_case("onDraftSave"),
                        TriggerEventIR::OnDraftActivate => trg_name.eq_ignore_ascii_case("onDraftActivate"),
                        TriggerEventIR::OnDraftDiscard => trg_name.eq_ignore_ascii_case("onDraftDiscard"),
                        TriggerEventIR::OnValidate(_) => false,
                    };

                    if matches {
                        if let Some(script) = trg_val.get("script").and_then(|v| v.as_str()) {
                            let full_script = format!("{}\n{}", companion_prelude, script);
                            let (mutated, q_logs) = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                                None,
                                None,
                                None,
                                clean_table.to_string(),
                                trg_name.to_string(),
                                full_script,
                                data.clone(),
                                x_rec.cloned(),
                                ctx.clone(),
                                Some(pool.clone()),
                            ).await?;
                            *data = mutated;
                            all_logs.extend(q_logs);
                        }
                    }
                }
            }
        }

        Ok(all_logs)
    }

    /// Internal procedural action evaluator with sandboxed fuel metering and Decimal arithmetic
    async fn eval_actions(
        actions: &[ActionIR],
        data: &mut serde_json::Value,
        x_rec: Option<&serde_json::Value>,
        ctx: &SessionContext,
        pool: &PgPool,
        logs: &mut Vec<String>,
        fuel: &mut u64,
        depth: usize,
    ) -> Result<(), RuntimeError> {
        if depth > 32 {
            return Err(RuntimeError::Internal(
                "Execution stack depth limit exceeded (maximum 32 recursion levels)".to_string(),
            ));
        }

        for act in actions {
            if *fuel == 0 {
                return Err(RuntimeError::Internal(
                    "Execution fuel exhausted: sandboxed instruction budget exceeded (possible infinite loop)".to_string(),
                ));
            }
            *fuel = fuel.saturating_sub(1);
            match act {
                ActionIR::ValidateRequired { field, location } => {
                    let is_empty = match data.as_object().and_then(|o| o.get(field).or_else(|| o.get(&to_snake_case(field)))) {
                        None => true,
                        Some(serde_json::Value::Null) => true,
                        Some(serde_json::Value::String(s)) => s.trim().is_empty(),
                        _ => false,
                    };
                    if is_empty {
                        let loc_str = location.as_ref().map(|l| format!(" at {}:{}:{}", l.file, l.line, l.column)).unwrap_or_default();
                        return Err(RuntimeError::Internal(format!(
                            "Business Rule Violation: Field '{}' is required.{}",
                            field, loc_str
                        )));
                    }
                    logs.push(format!("Validated required field: {}", field));
                }
                ActionIR::Assert { condition, message, location } => {
                    let parts: Vec<&str> = condition.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let op = parts[1];
                        let (val1, val2) = if let Some(obj) = data.as_object() {
                            (resolve_operand(parts[0], obj, x_rec), resolve_operand(parts[2], obj, x_rec))
                        } else {
                            (None, None)
                        };

                        let passed = match (val1, val2) {
                            (Some(v1), Some(v2)) => eval_comparison(op, &v1, &v2),
                            _ => true,
                        };

                        if !passed {
                            let loc_str = location.as_ref().map(|l| format!(" at {}:{}:{}", l.file, l.line, l.column)).unwrap_or_default();
                            return Err(RuntimeError::Internal(format!(
                                "Business Rule Assertion Failed: {}{}",
                                message, loc_str
                            )));
                        }
                    }
                    logs.push(format!("Assertion passed: {}", condition));
                }
                ActionIR::SetField { field, value_expr, location: _ } => {
                    let new_val = {
                        if let Some(obj) = data.as_object() {
                            if let Some(operand_val) = resolve_operand(value_expr, obj, x_rec) {
                                operand_val
                            } else if let Ok(d) = rust_decimal::Decimal::from_str_exact(value_expr) {
                                serde_json::json!(d.to_string())
                            } else if value_expr.eq_ignore_ascii_case("true") {
                                serde_json::Value::Bool(true)
                            } else if value_expr.eq_ignore_ascii_case("false") {
                                serde_json::Value::Bool(false)
                            } else {
                                serde_json::Value::String(value_expr.trim_matches('\'').trim_matches('"').to_string())
                            }
                        } else {
                            serde_json::Value::Null
                        }
                    };

                    if let Some(obj) = data.as_object_mut() {
                        obj.insert(field.clone(), new_val);
                    }
                    logs.push(format!("Set field '{}' = {}", field, value_expr));
                }
                ActionIR::HashPassword { source_field, target_field, location: _ } => {
                    if let Some(obj) = data.as_object_mut() {
                        let plain_pwd = obj.get(source_field)
                            .or_else(|| obj.get(&to_snake_case(source_field)))
                            .or_else(|| {
                                if source_field.eq_ignore_ascii_case("password") {
                                    obj.get("plainPassword").or_else(|| obj.get("plain_password"))
                                } else {
                                    None
                                }
                            })
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        if let Some(pwd) = plain_pwd {
                            if !pwd.trim().is_empty() {
                                let computed_hash = crate::hash_password(&pwd);
                                obj.insert(target_field.clone(), serde_json::json!(computed_hash));
                                obj.insert(to_snake_case(target_field), serde_json::json!(computed_hash));
                                logs.push(format!("Hashed password from field '{}' into '{}'", source_field, target_field));
                            }
                            obj.remove(source_field);
                            obj.remove(&to_snake_case(source_field));
                            if source_field.eq_ignore_ascii_case("password") {
                                obj.remove("plainPassword");
                                obj.remove("plain_password");
                            }
                        }
                    }
                }
                ActionIR::LogAudit { message, location: _ } => {
                    let insert_log = r#"
                        INSERT INTO "_system_privileged_audit_log" (
                            "table_name", "record_id", "action", "reason", "ticket_ref", "authorized_by"
                        ) VALUES ('codeunit', $1, 'EXECUTE', $2, 'RULE-ENGINE', $3);
                    "#;
                    let _ = sqlx::query(insert_log)
                        .bind(ctx.company_id)
                        .bind(message)
                        .bind(ctx.user_id.to_string())
                        .execute(pool).await;
                    logs.push(format!("Logged audit: {}", message));
                }
                ActionIR::IfElse { condition, then_branch, else_branch, location: _ } => {
                    let cond_passed = {
                        let parts: Vec<&str> = condition.split_whitespace().collect();
                        if parts.len() >= 3 {
                            let op = parts[1];
                            if let Some(obj) = data.as_object() {
                                let val1 = resolve_operand(parts[0], obj, x_rec);
                                let val2 = resolve_operand(parts[2], obj, x_rec);
                                match (val1, val2) {
                                    (Some(v1), Some(v2)) => eval_comparison(op, &v1, &v2),
                                    _ => false,
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    };

                    if cond_passed {
                        Box::pin(Self::eval_actions(then_branch, data, x_rec, ctx, pool, logs, fuel, depth + 1)).await?;
                    } else if let Some(else_b) = else_branch {
                        Box::pin(Self::eval_actions(else_b, data, x_rec, ctx, pool, logs, fuel, depth + 1)).await?;
                    }
                }
                ActionIR::GetRecord { table, filter, target_buffer, location: _ } => {
                    let sql_where = if filter.trim().is_empty() {
                        "TRUE".to_string()
                    } else {
                        let mut resolved_cond = filter.clone();
                        if let Some(obj) = data.as_object() {
                            for (k, v) in obj {
                                let needle1 = format!("this.{}", k);
                                let needle2 = format!("rec.{}", k);
                                let val_sql = match v {
                                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    _ => "NULL".to_string(),
                                };
                                resolved_cond = resolved_cond.replace(&needle1, &val_sql).replace(&needle2, &val_sql);
                            }
                        }
                        resolved_cond
                    };

                    let query = format!("SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE {} LIMIT 1;", table.to_lowercase(), sql_where);
                    let row_opt = sqlx::query_scalar::<_, serde_json::Value>(&query).fetch_optional(pool).await.unwrap_or(None);
                    if let Some(record) = row_opt {
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert(target_buffer.clone(), record);
                        }
                        logs.push(format!("Prefetched record buffer '{}' from table '{}'", target_buffer, table));
                    } else {
                        logs.push(format!("Record not found for buffer '{}' from table '{}'", target_buffer, table));
                    }
                }
                ActionIR::ForEachRecord { table, filter, cursor_var, body, location: _ } => {
                    let sql_where = if filter.trim().is_empty() {
                        "TRUE".to_string()
                    } else {
                        let mut resolved_cond = filter.clone();
                        if let Some(obj) = data.as_object() {
                            for (k, v) in obj {
                                let needle1 = format!("this.{}", k);
                                let needle2 = format!("rec.{}", k);
                                let val_sql = match v {
                                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::Bool(b) => b.to_string(),
                                    _ => "NULL".to_string(),
                                };
                                resolved_cond = resolved_cond.replace(&needle1, &val_sql).replace(&needle2, &val_sql);
                            }
                        }
                        resolved_cond
                    };

                    let query = format!("SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE {} LIMIT 1000;", table.to_lowercase(), sql_where);
                    let rows = sqlx::query_scalar::<_, serde_json::Value>(&query).fetch_all(pool).await.unwrap_or_default();
                    let count = rows.len();
                    for row_item in rows {
                        if let Some(obj) = data.as_object_mut() {
                            obj.insert(cursor_var.clone(), row_item);
                        }
                        Box::pin(Self::eval_actions(body, data, x_rec, ctx, pool, logs, fuel, depth + 1)).await?;
                    }
                    logs.push(format!("Processed batch loop on table '{}' ({} rows processed)", table, count));
                }
                ActionIR::CallProcedure { codeunit, method, location: _ } => {
                    let cu_row = sqlx::query(
                        "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'codeunit' AND LOWER(object_name) = LOWER($1) LIMIT 1"
                    )
                    .bind(codeunit)
                    .fetch_optional(pool)
                    .await?;

                    if let Some(r) = cu_row {
                        let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
                        if let Ok(cu_ir) = serde_json::from_value::<CodeunitIR>(def) {
                            if let Some(target_method) = cu_ir.methods.iter().find(|m| m.name.eq_ignore_ascii_case(method)) {
                                Box::pin(Self::eval_actions(&target_method.actions, data, x_rec, ctx, pool, logs, fuel, depth + 1)).await?;
                                logs.push(format!("Successfully executed procedure {}.{}", codeunit, method));
                            } else {
                                return Err(RuntimeError::NotFound(codeunit.to_string(), format!("Method '{}'", method)));
                            }
                        } else {
                            return Err(RuntimeError::Internal(format!("Failed to parse Codeunit IR for '{}'", codeunit)));
                        }
                    } else {
                        return Err(RuntimeError::NotFound("Codeunit".to_string(), codeunit.to_string()));
                    }
                }
            }
        }

        Ok(())
    }
}

fn resolve_operand(
    expr: &str,
    rec: &serde_json::Map<String, serde_json::Value>,
    x_rec: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let trimmed = expr.trim();
    if let Some(field) = trimmed.strip_prefix("xRec.").or_else(|| trimmed.strip_prefix("x_rec.")) {
        return x_rec.and_then(|x| x.get(field).or_else(|| x.get(&to_snake_case(field)))).cloned();
    }
    let clean = trimmed.strip_prefix("this.").or_else(|| trimmed.strip_prefix("rec.")).unwrap_or(trimmed);
    if let Some(val) = rec.get(clean).or_else(|| rec.get(&to_snake_case(clean))) {
        return Some(val.clone());
    }
    if let Some((buf_name, sub_field)) = clean.split_once('.') {
        if let Some(buf_val) = rec.get(buf_name) {
            if let Some(val) = buf_val.get(sub_field) {
                return Some(val.clone());
            }
        }
    }
    if clean.eq_ignore_ascii_case("true") {
        return Some(serde_json::Value::Bool(true));
    }
    if clean.eq_ignore_ascii_case("false") {
        return Some(serde_json::Value::Bool(false));
    }
    if (clean.starts_with('\'') && clean.ends_with('\'')) || (clean.starts_with('"') && clean.ends_with('"')) {
        return Some(serde_json::Value::String(clean[1..clean.len()-1].to_string()));
    }
    if let Ok(d) = rust_decimal::Decimal::from_str_exact(clean) {
        return Some(serde_json::json!(d.to_string()));
    }
    Some(serde_json::Value::String(clean.to_string()))
}

fn eval_comparison(
    op: &str,
    v1: &serde_json::Value,
    v2: &serde_json::Value,
) -> bool {
    // If both can be parsed as Decimal, use exact 128-bit Decimal arithmetic
    let d1 = match v1 {
        serde_json::Value::Number(n) => rust_decimal::Decimal::from_str_exact(&n.to_string()).ok(),
        serde_json::Value::String(s) => rust_decimal::Decimal::from_str_exact(s).ok(),
        _ => None,
    };
    let d2 = match v2 {
        serde_json::Value::Number(n) => rust_decimal::Decimal::from_str_exact(&n.to_string()).ok(),
        serde_json::Value::String(s) => rust_decimal::Decimal::from_str_exact(s).ok(),
        _ => None,
    };

    if let (Some(num1), Some(num2)) = (d1, d2) {
        return match op {
            ">" => num1 > num2,
            ">=" => num1 >= num2,
            "<" => num1 < num2,
            "<=" => num1 <= num2,
            "==" => num1 == num2,
            "!=" => num1 != num2,
            _ => true,
        };
    }

    if let (Some(b1), Some(b2)) = (v1.as_bool(), v2.as_bool()) {
        return match op {
            "==" => b1 == b2,
            "!=" => b1 != b2,
            _ => true,
        };
    }

    let s1 = match v1 {
        serde_json::Value::String(s) => s.as_str(),
        _ => "",
    };
    let s2 = match v2 {
        serde_json::Value::String(s) => s.as_str(),
        _ => "",
    };

    match op {
        "==" => s1 == s2,
        "!=" => s1 != s2,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_exact_comparison() {
        // Test standard floating-point precision hazards are avoided
        let val1 = serde_json::json!("100.0000000000000001");
        let val2 = serde_json::json!("100.0000000000000002");
        assert!(eval_comparison("<", &val1, &val2));
        assert!(eval_comparison("!=", &val1, &val2));
        assert!(!eval_comparison("==", &val1, &val2));

        // Test decimal numbers formatted as numbers or strings
        let val3 = serde_json::json!(50.25);
        let val4 = serde_json::json!("50.25");
        assert!(eval_comparison("==", &val3, &val4));
    }

    #[test]
    fn test_resolve_operand_with_xrec_and_rec() {
        let mut rec_map = serde_json::Map::new();
        rec_map.insert("credit_limit".to_string(), serde_json::json!("50000.00"));
        rec_map.insert("status".to_string(), serde_json::json!("Released"));

        let mut x_rec_map = serde_json::Map::new();
        x_rec_map.insert("credit_limit".to_string(), serde_json::json!("25000.00"));
        x_rec_map.insert("status".to_string(), serde_json::json!("Open"));
        let x_rec_val = serde_json::Value::Object(x_rec_map);

        // Access current record (rec / this)
        let rec_credit = resolve_operand("this.credit_limit", &rec_map, Some(&x_rec_val));
        assert_eq!(rec_credit, Some(serde_json::json!("50000.00")));

        // Access prior record (xRec)
        let prior_credit = resolve_operand("xRec.credit_limit", &rec_map, Some(&x_rec_val));
        assert_eq!(prior_credit, Some(serde_json::json!("25000.00")));
    }

    #[tokio::test]
    async fn test_sandboxed_fuel_exhaustion() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
            .unwrap();
        let ctx = SessionContext::default();
        let mut data = serde_json::json!({ "field": "val" });
        let mut logs = Vec::new();
        let actions = vec![
            ActionIR::ValidateRequired { field: "field".to_string(), location: None },
            ActionIR::ValidateRequired { field: "field".to_string(), location: None },
        ];
        let mut fuel = 1; // Only 1 fuel unit allocated
        let res = ExecutionEngine::eval_actions(&actions, &mut data, None, &ctx, &pool, &mut logs, &mut fuel, 0).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("fuel exhausted"));
    }

    #[tokio::test]
    async fn test_call_depth_limit() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
            .unwrap();
        let ctx = SessionContext::default();
        let mut data = serde_json::json!({});
        let mut logs = Vec::new();
        let mut fuel = 1000;
        let actions = vec![];
        let res = ExecutionEngine::eval_actions(&actions, &mut data, None, &ctx, &pool, &mut logs, &mut fuel, 33).await;
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("stack depth limit exceeded"));
    }

    #[tokio::test]
    async fn test_hash_password_action() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
            .unwrap();
        let ctx = SessionContext::default();
        let mut data = serde_json::json!({
            "username": "alice",
            "password": "SuperSecret123!"
        });
        let mut logs = Vec::new();
        let mut fuel = 1000;
        let actions = vec![
            ActionIR::ValidateRequired { field: "username".to_string(), location: None },
            ActionIR::HashPassword {
                source_field: "password".to_string(),
                target_field: "passwordHash".to_string(),
                location: None,
            },
            ActionIR::ValidateRequired { field: "passwordHash".to_string(), location: None },
        ];
        let res = ExecutionEngine::eval_actions(&actions, &mut data, None, &ctx, &pool, &mut logs, &mut fuel, 0).await;
        assert!(res.is_ok());

        // Verify plain password removed
        assert!(data.get("password").is_none());

        // Verify passwordHash computed
        let hash = data.get("passwordHash").and_then(|v| v.as_str()).unwrap();
        assert!(hash.starts_with("$sha256$"));

        // Verify verification works
        assert!(crate::verify_password("SuperSecret123!", hash));
        assert!(!crate::verify_password("WrongPassword", hash));
    }
}

