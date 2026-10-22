use bosc_syntax::ast::*;
use bosc_ir::*;
use thiserror::Error;

pub mod script_opt;
pub mod ts_oxc;
pub mod ts_stripper;
pub mod bytecode_compiler;
pub mod page_merger;
pub mod page_expr;
pub mod page_resolver;
pub mod page_source;
pub mod page_print;
mod page_check;
mod page_lower;
#[cfg(test)]
mod page_tests;
pub use ts_stripper::strip_typescript;
pub use bytecode_compiler::BytecodeCompiler;
pub use page_merger::{merge_page_detailed, merge_page_with_extensions, sort_extensions_for_merge, MergeOrphan, MergeOutcome};
pub use page_resolver::{compile_page, prettify_caption, ModelCatalog, PageCatalog, PageCompileResult};

#[derive(Debug, Error)]
pub enum AnalyzerError {
    #[error("Unknown type: {0}")]
    UnknownType(String),
    #[error("Target table not found: {0}")]
    TableNotFound(String),
    #[error("Invalid state transition in {0}: from {1} to {2}")]
    InvalidTransition(String, String, String),
    #[error("Invalid property '{property}' on {owner}: {reason}")]
    InvalidProperty { owner: String, property: String, reason: String },
}

fn property_text(value: &Expr) -> Option<String> {
    match value {
        Expr::StringLit(s) | Expr::Ident(s) => Some(s.clone()),
        _ => None,
    }
}

/// Lower a field expression to a SQL fragment. A string is kept as authored SQL.
/// Identifiers become quoted snake_case column names.
fn expr_to_sql_fragment(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::StringLit(s) => Ok(s.clone()),
        Expr::Ident(name) => Ok(format!("\"{}\"", to_snake_case(name))),
        Expr::IntLit(n) => Ok(n.to_string()),
        Expr::FloatLit(n) => Ok(n.to_string()),
        Expr::BoolLit(b) => Ok(if *b { "TRUE" } else { "FALSE" }.to_string()),
        Expr::NullLit => Ok("NULL".to_string()),
        Expr::Unary { op, operand } => {
            let inner = expr_to_sql_fragment(operand)?;
            let sql_op = match op {
                UnaryOp::Neg => "-",
                UnaryOp::Not => "NOT ",
            };
            Ok(format!("({sql_op}{inner})"))
        }
        Expr::Binary { left, op, right } => {
            let sql_op = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::Eq => "=",
                BinaryOp::NotEq => "<>",
                BinaryOp::Lt => "<",
                BinaryOp::Lte => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Gte => ">=",
                BinaryOp::And => "AND",
                BinaryOp::Or => "OR",
                BinaryOp::In => "IN",
            };
            Ok(format!(
                "({} {} {})",
                expr_to_sql_fragment(left)?,
                sql_op,
                expr_to_sql_fragment(right)?
            ))
        }
        Expr::Call { callee, args } => {
            let Expr::Ident(name) = callee.as_ref() else {
                return Err("formula functions must be a bare name such as round(...)".to_string());
            };
            let allowed = matches!(name.to_ascii_lowercase().as_str(), "round" | "abs" | "coalesce" | "greatest" | "least");
            if !allowed {
                return Err(format!("function '{name}' is not allowed in a stored formula"));
            }
            let mut parts = Vec::new();
            for arg in args {
                parts.push(expr_to_sql_fragment(arg)?);
            }
            Ok(format!("{}({})", name.to_ascii_lowercase(), parts.join(", ")))
        }
        Expr::ArrayLit(_) | Expr::MemberAccess { .. } | Expr::Range { .. } => {
            Err("this expression cannot be stored as a SQL formula".to_string())
        }
    }
}

struct CopyBinding {
    source_table: String,
    source_field: String,
    target_field: String,
}

fn copy_bindings(expr: &Expr) -> Result<Vec<CopyBinding>, String> {
    match expr {
        Expr::ArrayLit(items) => {
            let mut out = Vec::new();
            for item in items {
                out.extend(copy_bindings(item)?);
            }
            Ok(out)
        }
        Expr::MemberAccess { object, member } => {
            let Expr::Ident(table) = object.as_ref() else {
                return Err("copyFrom expects Customer.field or a list of those".to_string());
            };
            Ok(vec![CopyBinding {
                source_table: table.clone(),
                source_field: member.clone(),
                target_field: member.clone(),
            }])
        }
        _ => Err("copyFrom expects Customer.field or [Customer.name, Customer.currencyCode]".to_string()),
    }
}

fn decimal_field_names(tables: &[TableIR], extensions: &[TableExtensionIR]) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    let consider = |names: &mut std::collections::HashSet<String>, column: &ColumnIR| {
        let ty = column.sql_type.to_ascii_uppercase();
        if ty.starts_with("NUMERIC") || ty.starts_with("DECIMAL") {
            names.insert(column.name.clone());
        }
    };
    for table in tables {
        for column in &table.columns {
            consider(&mut names, column);
        }
    }
    for extension in extensions {
        for column in &extension.added_columns {
            consider(&mut names, column);
        }
    }
    names
}

/// A method runs in PostgreSQL only when every statement translates.
/// `@pushdown` on a body that does not translate is an error, and QuickJS keeps the body.
fn assign_pushdown(codeunits: &mut [CodeunitIR]) -> Vec<CompilerDiagnosticIR> {
    let mut diags = Vec::new();
    for codeunit in codeunits {
        let unit_marked = codeunit.is_pushdown;
        codeunit.is_pushdown = false;
        codeunit.execution_mode = Some("BytecodeInterpreter".to_string());
        for method in &mut codeunit.methods {
            let marked = unit_marked
                || method.is_pushdown
                || method.annotations.iter().any(|a| a == "pushdown" || a == "amdp");
            match bosc_codegen_db::pushdown::body(method) {
                Ok(_) => method.is_pushdown = true,
                Err(reason) => {
                    method.is_pushdown = false;
                    if marked {
                        diags.push(CompilerDiagnosticIR {
                            severity: "error".to_string(),
                            code: "BOS3301".to_string(),
                            message: format!(
                                "Method '{}' cannot run entirely inside PostgreSQL: {reason}",
                                method.name
                            ),
                            line: 1,
                            column: 1,
                            end_line: 1,
                            end_column: 1,
                            hint: Some("Remove @pushdown, or keep the body to assignments, comparisons, testField, and ctx.audit.log.".to_string()),
                        });
                    }
                }
            }
        }
    }
    diags
}

fn host_member_diagnostics(codeunits: &[CodeunitIR]) -> Vec<CompilerDiagnosticIR> {
    let mut diags = Vec::new();
    for codeunit in codeunits {
        for method in &codeunit.methods {
            if let Some(script) = &method.script {
                scan_host_members(script, &mut diags);
            }
        }
    }
    diags
}

fn scan_host_members(script: &str, diags: &mut Vec<CompilerDiagnosticIR>) {
    let bytes = script.as_bytes();
    let mut i = 0;
    let mut in_string: Option<u8> = None;
    while i + 4 < bytes.len() {
        let b = bytes[i];
        if let Some(quote) = in_string {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == quote {
                in_string = None;
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' || b == b'`' {
            in_string = Some(b);
            i += 1;
            continue;
        }
        if bytes[i..].starts_with(b"ctx.") {
            let start = i + 4;
            let (first, next) = read_ident(script, start);
            if first.is_empty() {
                i += 4;
                continue;
            }
            let second_at = start + first.len();
            let second = if second_at < script.len() && script.as_bytes()[second_at] == b'.' {
                read_ident(script, second_at + 1).0
            } else {
                String::new()
            };
            if !host_member_allowed(&first, &second) {
                let shown = if second.is_empty() { format!("ctx.{first}") } else { format!("ctx.{first}.{second}") };
                diags.push(CompilerDiagnosticIR {
                    severity: "error".to_string(),
                    code: "BOS1002".to_string(),
                    message: format!("{shown} is not a host member"),
                    line: 1,
                    column: 1,
                    end_line: 1,
                    end_column: 1,
                    hint: Some("Use the members declared in ambient.d.ts.".to_string()),
                });
            }
            let _ = next;
            i = start + first.len();
            continue;
        }
        i += 1;
    }
}

fn read_ident(script: &str, start: usize) -> (String, usize) {
    let mut end = start;
    for (offset, ch) in script[start..].char_indices() {
        if ch == '_' || ch.is_ascii_alphanumeric() {
            end = start + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    (script[start..end].to_string(), end)
}

fn host_member_allowed(first: &str, second: &str) -> bool {
    const NESTED: &[(&str, &[&str])] = &[
        (
            "crypto",
            &[
                "sha256",
                "randomBytes",
                "argon2Hash",
                "argon2Verify",
                "hmacSha256",
                "uuid",
            ],
        ),
        (
            "user",
            &[
                "id",
                "userId",
                "tenantId",
                "companyId",
                "roles",
                "workDate",
                "email",
                "username",
            ],
        ),
        (
            "calendar",
            &[
                "addDays",
                "addMonths",
                "diffDays",
                "today",
                "now",
                "isWeekend",
                "startOfMonth",
                "endOfMonth",
            ],
        ),
        (
            "workflow",
            &[
                "submitForApproval",
                "approve",
                "reject",
                "transition",
                "canTransition",
                "currentState",
            ],
        ),
        ("stateMachine", &["transition", "canTransition", "currentState"]),
        ("jobQueue", &["enqueue", "schedule"]),
        ("numSeries", &["next", "peek"]),
        ("dimension", &["getSetId", "resolve", "validate"]),
        (
            "decimal",
            &[
                "from", "add", "sub", "mul", "div", "round", "cmp", "eq", "gt", "gte", "lt", "lte",
            ],
        ),
        ("audit", &["log"]),
        ("events", &["publish", "emit"]),
        ("security", &["hasRole", "requireRole", "checkPermission"]),
        ("http", &["get", "post"]),
        ("wasm", &["invoke"]),
        ("sidecar", &["call"]),
        ("ui", &[
            "toast", "message", "success", "warning", "error", "info", "confirm", "openModal",
            "navigate", "setFieldVisible", "setFieldEditable", "getMessages", "getDirectives", "clearMessages",
        ]),
    ];
    const TOP: &[&str] = &[
        "tenantId", "companyId", "userId", "roles", "now", "log", "call", "decimal", "crypto",
        "audit", "events", "security", "http", "wasm", "sidecar", "ui", "message", "toast", "db",
        "user", "calendar", "workflow", "numSeries", "dimension", "stateMachine", "jobQueue",
        "session", "env", "postLedger", "checkPermission", "dispatchInterface", "error",
    ];
    if !TOP.contains(&first) {
        return false;
    }
    if second.is_empty() || first == "db" {
        return true;
    }
    NESTED
        .iter()
        .find(|(name, _)| *name == first)
        .map(|(_, members)| members.contains(&second))
        .unwrap_or(true)
}

fn sift_advice(tables: &[TableIR]) -> Vec<CompilerDiagnosticIR> {
    let mut diags = Vec::new();
    for table in tables {
        for field in &table.flow_fields {
            let measured = matches!(
                field.func,
                FlowFieldFuncIR::Sum | FlowFieldFuncIR::Average | FlowFieldFuncIR::Min | FlowFieldFuncIR::Max
            );
            if measured && field.index_columns.is_empty() {
                diags.push(CompilerDiagnosticIR {
                    severity: "warning".to_string(),
                    code: "BOS3101".to_string(),
                    message: format!(
                        "FlowField '{}' has no equality filter, so its SIFT index is only _tenant_id.",
                        field.name
                    ),
                    line: 1,
                    column: 1,
                    end_line: 1,
                    end_column: 1,
                    hint: Some("Filter on the child key, for example customerNo = rec.no.".to_string()),
                });
            }
        }
    }
    diags
}

fn rewrite_script(script: &mut String, fields: &std::collections::HashSet<String>) {
    *script = crate::script_opt::rewrite_decimal(script, fields);
}

fn rewrite_decimal_scripts(
    fields: &std::collections::HashSet<String>,
    table_logics: &mut [TableLogicIR],
    table_logic_extensions: &mut [TableLogicExtensionIR],
    codeunits: &mut [CodeunitIR],
) {
    if fields.is_empty() {
        return;
    }
    for logic in table_logics {
        for validation in &mut logic.field_validations {
            rewrite_script(&mut validation.script, fields);
        }
        for trigger in &mut logic.triggers {
            if let Some(script) = trigger.script.as_mut() {
                rewrite_script(script, fields);
            }
        }
        for method in &mut logic.methods {
            if let Some(script) = method.script.as_mut() {
                rewrite_script(script, fields);
            }
        }
    }
    for logic in table_logic_extensions {
        for validation in &mut logic.field_validations {
            rewrite_script(&mut validation.script, fields);
        }
        for trigger in &mut logic.triggers {
            if let Some(script) = trigger.script.as_mut() {
                rewrite_script(script, fields);
            }
        }
        for method in &mut logic.methods {
            if let Some(script) = method.script.as_mut() {
                rewrite_script(script, fields);
            }
        }
    }
    for codeunit in codeunits {
        for method in &mut codeunit.methods {
            if let Some(script) = method.script.as_mut() {
                rewrite_script(script, fields);
            }
        }
    }
}

fn field_script_triggers(fields: &[FieldDef]) -> Result<Vec<TriggerIR>, AnalyzerError> {
    let mut triggers = Vec::new();
    for field in fields {
        for prop in &field.properties {
            if prop.name == "series" || prop.name == "numberSeries" {
                let code = property_text(&prop.value).ok_or_else(|| AnalyzerError::InvalidProperty {
                    owner: format!("field '{}'", field.name),
                    property: "series".to_string(),
                    reason: "expected the number series code, for example series = \"SO\"".to_string(),
                })?;
                let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");
                let name = &field.name;
                triggers.push(TriggerIR {
                    name: "beforeInsert".to_string(),
                    params: vec!["rec".to_string(), "ctx".to_string()],
                    actions: Vec::new(),
                    script: Some(format!(
                        "if (rec.{name} === undefined || rec.{name} === null || rec.{name} === \"\") {{ rec.{name} = ctx.call(\"NumberSeriesManagement\", \"getNext\", \"{escaped}\"); }}"
                    )),
                });
            }
            if prop.name == "copyFrom" || prop.name == "copy_from" {
                let bindings = copy_bindings(&prop.value).map_err(|reason| AnalyzerError::InvalidProperty {
                    owner: format!("field '{}'", field.name),
                    property: "copyFrom".to_string(),
                    reason,
                })?;
                if bindings.is_empty() {
                    continue;
                }
                let source_table = &bindings[0].source_table;
                if bindings.iter().any(|b| b.source_table != *source_table) {
                    return Err(AnalyzerError::InvalidProperty {
                        owner: format!("field '{}'", field.name),
                        property: "copyFrom".to_string(),
                        reason: "every copied field must come from the same table".to_string(),
                    });
                }
                let fk = &field.name;
                let mut script = format!(
                    "const __bosSrc = {source_table}.get(rec.{fk});\nif (!__bosSrc) throw new Error(\"{source_table} '\" + rec.{fk} + \"' does not exist.\");\n"
                );
                for binding in &bindings {
                    script.push_str(&format!(
                        "rec.{} = __bosSrc.{};\n",
                        binding.target_field, binding.source_field
                    ));
                }
                triggers.push(TriggerIR {
                    name: format!("onValidate{}", field.name),
                    params: vec!["rec".to_string()],
                    actions: Vec::new(),
                    script: Some(script),
                });
            }
        }
    }
    Ok(triggers)
}

pub struct SemanticAnalyzer {
    package_name: String,
}

impl SemanticAnalyzer {
    pub fn new(package_name: &str) -> Self {
        Self {
            package_name: package_name.to_string(),
        }
    }

    pub fn analyze_and_lower(&self, program: &Program) -> Result<ModelIR, AnalyzerError> {
        let mut tables = Vec::new();
        let mut state_machines = Vec::new();
        let mut state_machine_extensions = Vec::new();
        let mut codeunits = Vec::new();
        let mut interfaces = Vec::new();
        let mut enums = Vec::new();
        let mut enum_extensions = Vec::new();
        let mut permission_sets = Vec::new();
        let mut views = Vec::new();
        let mut queries = Vec::new();
        let mut pages = Vec::new();
        let mut page_extensions = Vec::new();
        let mut reports = Vec::new();
        let mut custom_metadata = Vec::new();
        let mut menus = Vec::new();
        let mut table_logics = Vec::new();
        let mut table_logic_extensions = Vec::new();

        for item in &program.items {
            match item {
                Item::Table(t) => tables.push(self.lower_table(t)?),
                Item::TableLogic(tl) => table_logics.push(self.lower_table_logic(tl)?),
                Item::TableLogicExtension(tle) => table_logic_extensions.push(self.lower_table_logic_extension(tle)?),
                Item::StateMachine(sm) => state_machines.push(self.lower_state_machine(sm)?),
                Item::StateMachineExtension(sme) => state_machine_extensions.push(self.lower_state_machine_extension(sme)?),
                Item::Codeunit(cu) => codeunits.push(self.lower_codeunit(cu)?),
                Item::Interface(iface) => interfaces.push(self.lower_interface(iface)?),
                Item::Enum(e) => enums.push(self.lower_enum(e)?),
                Item::EnumExtension(ee) => enum_extensions.push(self.lower_enum_extension(ee)?),
                Item::PermissionSet(ps) => permission_sets.push(self.lower_permission_set(ps)?),
                Item::View(v) => views.push(self.lower_view(v)?),
                Item::Query(q) => queries.push(self.lower_query(q)?),
                Item::Page(p) => pages.push(self.lower_page(p)?),
                Item::PageExtension(pe) => page_extensions.push(self.lower_page_extension(pe)?),
                Item::Report(r) => reports.push(self.lower_report(r)?),
                Item::CustomMetadata(cm) => custom_metadata.push(self.lower_custom_metadata(cm)?),
                Item::Menu(m) => menus.push(self.lower_menu(m)),
                _ => {}
            }
        }

        // Merge state_machine_extensions into in-memory state_machines if target is in same compilation unit
        for sme in &state_machine_extensions {
            if let Some(target_sm) = state_machines.iter_mut().find(|sm| sm.name.eq_ignore_ascii_case(&sme.target_state_machine)) {
                for st in &sme.states {
                    if !target_sm.states.iter().any(|existing| existing.name.eq_ignore_ascii_case(&st.name)) {
                        target_sm.states.push(st.clone());
                    }
                }
                for tr in &sme.transitions {
                    target_sm.transitions.push(tr.clone());
                }
            }
        }

        // Merge enum_extensions into in-memory enums if target is in same compilation unit
        for ee in &enum_extensions {
            if let Some(target_en) = enums.iter_mut().find(|en| en.name.eq_ignore_ascii_case(&ee.target_enum)) {
                for mem in &ee.members {
                    if !target_en.members.iter().any(|existing| existing.name.eq_ignore_ascii_case(&mem.name)) {
                        target_en.members.push(mem.clone());
                    }
                }
            }
        }

        let mut table_extensions = Vec::new();

        // Apply table extensions
        for item in &program.items {
            if let Item::TableExtension(ext) = item {
                let ext_ir = self.lower_table_extension(ext)?;
                table_extensions.push(ext_ir);

                // If target table is in the same compilation unit, also augment it in-memory
                if let Some(target) = tables.iter_mut().find(|t| t.name.eq_ignore_ascii_case(&ext.target_table)) {
                    for f in &ext.fields {
                        target.columns.push(self.lower_field(f)?);
                    }
                    for k in &ext.keys {
                        target.indexes.push(IndexIR {
                            name: format!("idx_{}_{}", target.sql_table_name, k.fields.join("_")),
                            columns: k.fields.clone(),
                            include_columns: Vec::new(),
                            is_unique: k.unique,
                        });
                    }
                }
            }
        }

        // Also merge TableLogic validations/triggers into in-memory tables if present in compilation unit
        for tl in &table_logics {
            if let Some(target) = tables.iter_mut().find(|t| t.name.eq_ignore_ascii_case(&tl.target_table)) {
                for trg in &tl.triggers {
                    target.triggers.push(trg.clone());
                }
                for fv in &tl.field_validations {
                    let trg_name = format!("onValidate_{}", to_snake_case(&fv.field_name));
                    target.triggers.push(TriggerIR {
                        name: trg_name,
                        params: vec![fv.param_name.clone()],
                        actions: Vec::new(),
                        script: Some(strip_typescript(&fv.script)),
                    });
                }
            }
        }

        for tle in &table_logic_extensions {
            if let Some(target) = tables.iter_mut().find(|t| t.name.eq_ignore_ascii_case(&tle.target_table)) {
                for trg in &tle.triggers {
                    target.triggers.push(trg.clone());
                }
                for fv in &tle.field_validations {
                    let trg_name = format!("onValidate_{}", to_snake_case(&fv.field_name));
                    target.triggers.push(TriggerIR {
                        name: trg_name,
                        params: vec![fv.param_name.clone()],
                        actions: Vec::new(),
                        script: Some(strip_typescript(&fv.script)),
                    });
                }
            }
        }

        let decimal_fields = decimal_field_names(&tables, &table_extensions);
        rewrite_decimal_scripts(&decimal_fields, &mut table_logics, &mut table_logic_extensions, &mut codeunits);
        for table in &mut tables {
            for trigger in &mut table.triggers {
                if let Some(script) = trigger.script.as_mut() {
                    *script = crate::script_opt::rewrite_decimal(script, &decimal_fields);
                }
            }
        }

        let mut diagnostics = self.collect_diagnostics(program);
        diagnostics.extend(assign_pushdown(&mut codeunits));
        diagnostics.extend(sift_advice(&tables));
        diagnostics.extend(host_member_diagnostics(&codeunits));

        let mut model = ModelIR {
            package_name: self.package_name.clone(),
            tables,
            table_logics,
            table_extensions,
            table_logic_extensions,
            state_machines,
            state_machine_extensions,
            codeunits,
            interfaces,
            enums,
            enum_extensions,
            permission_sets,
            views,
            queries,
            pages,
            page_extensions,
            reports,
            custom_metadata,
            menus,
            diagnostics,
        };
        let page_diagnostics = page_check::check_pages(program, &model);
        model.diagnostics.extend(page_diagnostics);
        Ok(model)
    }

    pub fn lower_table_extension(&self, ext: &TableExtDef) -> Result<TableExtensionIR, AnalyzerError> {
        let mut added_columns = Vec::new();
        for f in &ext.fields {
            added_columns.push(self.lower_field(f)?);
        }

        let mut added_indexes = Vec::new();
        for k in &ext.keys {
            added_indexes.push(IndexIR {
                name: format!("idx_{}", k.fields.join("_")),
                columns: k.fields.clone(),
                include_columns: Vec::new(),
                is_unique: k.unique,
            });
        }

        Ok(TableExtensionIR {
            name: ext.name.clone(),
            package_name: self.package_name.clone(),
            target_table: ext.target_table.clone(),
            added_columns,
            added_indexes,
        })
    }

    pub fn lower_table_logic(&self, tl: &TableLogicDef) -> Result<TableLogicIR, AnalyzerError> {
        let sql_table_name = to_snake_case(&tl.target_table);

        let mut field_validations = Vec::new();
        for fv in &tl.field_validations {
            let script = fv.body.raw_source.as_ref().map(|s| strip_typescript(s)).unwrap_or_default();
            field_validations.push(FieldValidationIR {
                field_name: fv.field_name.clone(),
                param_name: fv.param_name.clone(),
                old_param_name: fv.old_param_name.clone(),
                script,
            });
        }

        let mut triggers = Vec::new();
        for trg in &tl.triggers {
            triggers.push(TriggerIR {
                name: trg.name.clone(),
                params: trg.params.clone(),
                actions: lower_statements(&trg.body.statements),
                script: trg.body.raw_source.as_ref().map(|s| strip_typescript(s)),
            });
        }

        let mut methods = Vec::new();
        for m in &tl.methods {
            let params = m.params.iter().map(|p| (p.name.clone(), p.param_type.clone())).collect();
            let actions = lower_statements(&m.body.statements);
            let script = m.body.raw_source.as_ref().map(|s| strip_typescript(s));
            methods.push(MethodIR {
                name: m.name.clone(),
                is_public: m.is_public,
                is_static: m.is_static,
                params,
                return_type: m.return_type.clone(),
                trigger_event: None,
                annotations: m.annotations.clone(),
                actions,
                script,
                is_pushdown: false,
            });
        }

        Ok(TableLogicIR {
            name: tl.name.clone(),
            package_name: self.package_name.clone(),
            target_table: tl.target_table.clone(),
            sql_table_name,
            field_validations,
            triggers,
            methods,
        })
    }

    pub fn lower_table_logic_extension(&self, tle: &TableLogicExtDef) -> Result<TableLogicExtensionIR, AnalyzerError> {
        let sql_table_name = to_snake_case(&tle.target_table);

        let mut field_validations = Vec::new();
        for fv in &tle.field_validations {
            let script = fv.body.raw_source.as_ref().map(|s| strip_typescript(s)).unwrap_or_default();
            field_validations.push(FieldValidationIR {
                field_name: fv.field_name.clone(),
                param_name: fv.param_name.clone(),
                old_param_name: fv.old_param_name.clone(),
                script,
            });
        }

        let mut triggers = Vec::new();
        for trg in &tle.triggers {
            triggers.push(TriggerIR {
                name: trg.name.clone(),
                params: trg.params.clone(),
                actions: lower_statements(&trg.body.statements),
                script: trg.body.raw_source.as_ref().map(|s| strip_typescript(s)),
            });
        }

        let mut methods = Vec::new();
        for m in &tle.methods {
            let params = m.params.iter().map(|p| (p.name.clone(), p.param_type.clone())).collect();
            let actions = lower_statements(&m.body.statements);
            let script = m.body.raw_source.as_ref().map(|s| strip_typescript(s));
            methods.push(MethodIR {
                name: m.name.clone(),
                is_public: m.is_public,
                is_static: m.is_static,
                params,
                return_type: m.return_type.clone(),
                trigger_event: None,
                annotations: m.annotations.clone(),
                actions,
                script,
                is_pushdown: false,
            });
        }

        Ok(TableLogicExtensionIR {
            name: tle.name.clone(),
            package_name: self.package_name.clone(),
            target_table: tle.target_table.clone(),
            sql_table_name,
            field_validations,
            triggers,
            methods,
        })
    }

    fn lower_table(&self, t: &TableDef) -> Result<TableIR, AnalyzerError> {
        let mut caption = t.name.clone();
        let mut table_type = TableTypeIR::Normal;
        let mut data_per_company = true;
        let mut on_delete = None;
        let mut renamed_from = None;

        for prop in &t.properties {
            match prop.name.as_str() {
                "caption" => {
                    if let Expr::StringLit(s) = &prop.value {
                        caption = s.clone();
                    }
                }
                "tableType" => {
                    if let Expr::Ident(tt) = &prop.value {
                        table_type = match tt.as_str() {
                            "Normal" => TableTypeIR::Normal,
                            "Document" => TableTypeIR::Document,
                            "Ledger" => TableTypeIR::Ledger,
                            "Temporal" => TableTypeIR::Temporal,
                            "Temporary" => TableTypeIR::Temporary,
                            "Virtual" => TableTypeIR::Virtual,
                            _ => TableTypeIR::Normal,
                        };
                    }
                }
                "dataPerCompany" => {
                    if let Expr::BoolLit(b) = &prop.value {
                        data_per_company = *b;
                    }
                }
                "onDelete" => {
                    if let Expr::Ident(od) = &prop.value {
                        on_delete = Some(od.clone());
                    } else if let Expr::StringLit(s) = &prop.value {
                        on_delete = Some(s.clone());
                    }
                }
                "renamedFrom" => {
                    let previous = property_text(&prop.value).ok_or_else(|| AnalyzerError::InvalidProperty {
                        owner: format!("table '{}'", t.name),
                        property: "renamedFrom".to_string(),
                        reason: "expected the previous table name as a string".to_string(),
                    })?;
                    renamed_from = Some(to_snake_case(&previous));
                }
                _ => {}
            }
        }

        let sql_table_name = to_snake_case(&t.name);
        if renamed_from.as_deref() == Some(sql_table_name.as_str()) {
            renamed_from = None;
        }
        let mut columns = Vec::new();
        let mut flow_fields = Vec::new();

        for f in &t.fields {
            if let Some(computed) = &f.computed {
                let func = match computed.func {
                    AggregationFunc::Sum => FlowFieldFuncIR::Sum,
                    AggregationFunc::Average => FlowFieldFuncIR::Average,
                    AggregationFunc::Count => FlowFieldFuncIR::Count,
                    AggregationFunc::Min => FlowFieldFuncIR::Min,
                    AggregationFunc::Max => FlowFieldFuncIR::Max,
                    AggregationFunc::Lookup => FlowFieldFuncIR::Lookup,
                    AggregationFunc::Exist => FlowFieldFuncIR::Exist,
                };

                let target_table = to_snake_case(&computed.target_table);
                let target_col = computed.target_field.as_ref().map(|f| to_snake_case(f));

                let where_sql = lower_filter_to_sql(&computed.filter, &target_table, &sql_table_name);
                let index_columns = sift_index_columns(&computed.filter);
                let col_name = target_col.clone().unwrap_or_else(|| "amount".to_string());

                let subquery = match func {
                    FlowFieldFuncIR::Sum => format!(
                        "(SELECT COALESCE(SUM(\"{}\"), 0) FROM \"{}\" WHERE {})",
                        col_name, target_table, where_sql
                    ),
                    FlowFieldFuncIR::Average => format!(
                        "(SELECT COALESCE(AVG(\"{}\"), 0) FROM \"{}\" WHERE {})",
                        col_name, target_table, where_sql
                    ),
                    FlowFieldFuncIR::Count => format!(
                        "(SELECT COUNT(*) FROM \"{}\" WHERE {})",
                        target_table, where_sql
                    ),
                    FlowFieldFuncIR::Min => format!(
                        "(SELECT MIN(\"{}\") FROM \"{}\" WHERE {})",
                        col_name, target_table, where_sql
                    ),
                    FlowFieldFuncIR::Max => format!(
                        "(SELECT MAX(\"{}\") FROM \"{}\" WHERE {})",
                        col_name, target_table, where_sql
                    ),
                    FlowFieldFuncIR::Lookup => format!(
                        "(SELECT \"{}\" FROM \"{}\" WHERE {} LIMIT 1)",
                        col_name, target_table, where_sql
                    ),
                    FlowFieldFuncIR::Exist => format!(
                        "(SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE {}))",
                        target_table, where_sql
                    ),
                };

                flow_fields.push(FlowFieldIR {
                    name: f.name.clone(),
                    func,
                    target_table: computed.target_table.clone(),
                    target_column: computed.target_field.clone(),
                    sql_subquery: subquery,
                    drill_down_page: None,
                    index_columns,
                });
            } else {
                columns.push(self.lower_field(f)?);
            }
        }

        let mut indexes = Vec::new();
        for k in &t.keys {
            // Support both `key(idx_name, col1, col2)` and `key(col1, col2)`
            let (idx_name, col_fields): (String, Vec<String>) = if k.fields.len() >= 2
                && !columns.iter().any(|c| c.name.eq_ignore_ascii_case(&k.fields[0]) || c.sql_name.eq_ignore_ascii_case(&k.fields[0]))
            {
                (
                    to_snake_case(&k.fields[0]),
                    k.fields[1..].iter().map(|f| to_snake_case(f)).collect(),
                )
            } else {
                (
                    format!("idx_{}_{}", sql_table_name, k.fields.join("_")),
                    k.fields.iter().map(|f| to_snake_case(f)).collect(),
                )
            };
            indexes.push(IndexIR {
                name: idx_name,
                columns: col_fields,
                is_unique: k.unique,
                include_columns: Vec::new(),
            });
        }

        let pk_cols: Vec<String> = t.primary_key.iter().map(|k| to_snake_case(k)).collect();
        for col in &mut columns {
            if pk_cols.iter().any(|pk| pk.eq_ignore_ascii_case(&col.sql_name) || pk.eq_ignore_ascii_case(&col.name)) {
                col.is_nullable = false;
                if pk_cols.len() == 1 {
                    col.is_unique = true;
                }
            }
        }

        let mut triggers = Vec::new();
        for trg in &t.triggers {
            triggers.push(TriggerIR {
                name: trg.name.clone(),
                params: trg.params.clone(),
                actions: lower_statements(&trg.body.statements),
                script: trg.body.raw_source.as_ref().map(|s| strip_typescript(s)),
            });
        }

        // Gap 7: Synthesize field-level validation triggers from validate_block
        for f in &t.fields {
            if let Some(ref vb) = f.validate_block {
                let trg_name = format!("onValidate_{}", to_snake_case(&f.name));
                triggers.push(TriggerIR {
                    name: trg_name,
                    params: vec!["value".to_string()],
                    actions: lower_statements(&vb.statements),
                    script: vb.raw_source.as_ref().map(|s| strip_typescript(s)),
                });
            }
        }
        triggers.extend(field_script_triggers(&t.fields)?);

        let field_groups = t
            .field_groups
            .iter()
            .map(|fg| FieldGroupIR {
                name: fg.name.clone(),
                fields: fg.fields.iter().map(|f| to_snake_case(f)).collect(),
            })
            .collect();

        Ok(TableIR {
            name: t.name.clone(),
            sql_table_name,
            caption,
            table_type,
            data_per_company,
            columns,
            primary_key: t.primary_key.iter().map(|k| to_snake_case(k)).collect(),
            indexes,
            flow_fields,
            field_groups,
            triggers,
            check_constraints: Vec::new(),
            on_delete,
            renamed_from,
        })
    }

    fn lower_field(&self, f: &FieldDef) -> Result<ColumnIR, AnalyzerError> {
        let sql_name = to_snake_case(&f.name);
        let mut is_nullable = true;
        let mut is_unique = false;
        let mut caption = f.name.clone();
        let mut default_val = None;
        let mut foreign_key = None;
        let mut check_constraint = None;
        let mut formula = None;
        let mut on_delete_raw: Option<String> = None;
        let mut renamed_from = None;
        let owner = format!("field '{}'", f.name);

        for prop in &f.properties {
            match prop.name.as_str() {
                "notNull" => {
                    if let Expr::BoolLit(b) = &prop.value {
                        is_nullable = !b;
                    }
                }
                "unique" => {
                    if let Expr::BoolLit(b) = &prop.value {
                        is_unique = *b;
                    }
                }
                "caption" => {
                    if let Expr::StringLit(s) = &prop.value {
                        caption = s.clone();
                    }
                }
                "formula" => {
                    formula = Some(expr_to_sql_fragment(&prop.value).map_err(|reason| {
                        AnalyzerError::InvalidProperty {
                            owner: owner.clone(),
                            property: "formula".to_string(),
                            reason,
                        }
                    })?);
                }
                "default" => {
                    default_val = Some(expr_to_sql_default(&prop.value));
                }
                "check" => {
                    check_constraint = Some(expr_to_sql_fragment(&prop.value).map_err(|reason| {
                        AnalyzerError::InvalidProperty {
                            owner: owner.clone(),
                            property: "check".to_string(),
                            reason,
                        }
                    })?);
                }
                "relation" => {
                    if let Expr::MemberAccess { object, member } = &prop.value
                        && let Expr::Ident(tbl) = &**object
                    {
                        foreign_key = Some(ForeignKeyIR {
                            target_table: to_snake_case(tbl),
                            target_column: to_snake_case(member),
                            on_delete: "RESTRICT".to_string(),
                        });
                    } else {
                        return Err(AnalyzerError::InvalidProperty {
                            owner,
                            property: "relation".to_string(),
                            reason: "expected `Table.field`, for example `relation: Customer.no`".to_string(),
                        });
                    }
                }
                "onDelete" => {
                    on_delete_raw = Some(property_text(&prop.value).ok_or_else(|| AnalyzerError::InvalidProperty {
                        owner: owner.clone(),
                        property: "onDelete".to_string(),
                        reason: "expected one of restrict, cascade, setNull, noAction".to_string(),
                    })?);
                }
                "renamedFrom" => {
                    let previous = property_text(&prop.value).ok_or_else(|| AnalyzerError::InvalidProperty {
                        owner: owner.clone(),
                        property: "renamedFrom".to_string(),
                        reason: "expected the previous field name as a string".to_string(),
                    })?;
                    renamed_from = Some(to_snake_case(&previous));
                }
                _ => {}
            }
        }

        if let Some(raw) = on_delete_raw {
            let Some(fk) = foreign_key.as_mut() else {
                return Err(AnalyzerError::InvalidProperty {
                    owner,
                    property: "onDelete".to_string(),
                    reason: "only valid together with a `relation`".to_string(),
                });
            };
            let action = ForeignKeyIR::normalize_on_delete(&raw).ok_or_else(|| AnalyzerError::InvalidProperty {
                owner: owner.clone(),
                property: "onDelete".to_string(),
                reason: format!("'{raw}' is not one of restrict, cascade, setNull, noAction"),
            })?;
            if action == "SET NULL" && !is_nullable {
                return Err(AnalyzerError::InvalidProperty {
                    owner,
                    property: "onDelete".to_string(),
                    reason: "setNull cannot be used on a notNull field".to_string(),
                });
            }
            fk.on_delete = action.to_string();
        }
        if renamed_from.as_deref() == Some(sql_name.as_str()) {
            renamed_from = None;
        }

        let sql_type = match &f.field_type {
            FieldType::UUID => "UUID".to_string(),
            FieldType::String(len) => match len {
                Some(l) => format!("VARCHAR({})", l),
                None => "TEXT".to_string(),
            },
            FieldType::Integer => "INTEGER".to_string(),
            FieldType::BigInt => "BIGINT".to_string(),
            FieldType::Decimal(p, s) => format!("NUMERIC({}, {})", p, s),
            FieldType::Boolean => "BOOLEAN".to_string(),
            FieldType::Date => "DATE".to_string(),
            FieldType::DateTime => "TIMESTAMPTZ".to_string(),
            FieldType::Enum(_) => "VARCHAR(50)".to_string(),
            FieldType::Json => "JSONB".to_string(),
            FieldType::Blob => "BYTEA".to_string(),
            FieldType::FlowFilter(_) => "VARCHAR(100)".to_string(),
            FieldType::Custom(c) => c.clone(),
        };

        let data_type = Some(format!("{}", f.field_type));

        Ok(ColumnIR {
            name: f.name.clone(),
            sql_name,
            sql_type,
            data_type,
            is_nullable,
            is_unique,
            default_val,
            caption,
            foreign_key,
            check_constraint,
            formula,
            renamed_from,
        })
    }

    fn lower_state(&self, s: &StateDef, initial_out: &mut Option<String>) -> StateIR {
        let mut caption = s.name.clone();
        let mut badge = "neutral".to_string();
        let mut timeout_hours = None;
        let mut escalate_to = None;
        let mut allow_modify = true;
        let mut allow_delete = true;

        for p in &s.properties {
            match p.name.as_str() {
                "caption" => {
                    if let Expr::StringLit(val) = &p.value {
                        caption = val.clone();
                    }
                }
                "badge" => {
                    if let Expr::StringLit(val) = &p.value {
                        badge = val.clone();
                    }
                }
                "initial" => {
                    if let Expr::BoolLit(true) = &p.value {
                        *initial_out = Some(s.name.clone());
                    }
                }
                "allowModify" | "allow_modify" => {
                    if let Expr::BoolLit(b) = &p.value {
                        allow_modify = *b;
                    }
                }
                "allowDelete" | "allow_delete" => {
                    if let Expr::BoolLit(b) = &p.value {
                        allow_delete = *b;
                    }
                }
                _ => {}
            }
        }

        if let Some(to) = &s.timeout {
            let parsed_hours = match &to.duration_expr {
                Expr::IntLit(n) => Some(*n as u32),
                Expr::StringLit(st) => {
                    let lower = st.to_lowercase();
                    if let Some(num_str) = lower.strip_suffix('h') {
                        num_str.trim().parse::<u32>().ok()
                    } else if let Some(num_str) = lower.strip_suffix('d') {
                        num_str.trim().parse::<u32>().ok().map(|d| d * 24)
                    } else if let Some(num_str) = lower.strip_suffix('m') {
                        num_str.trim().parse::<u32>().ok().map(|m| m.div_ceil(60))
                    } else {
                        st.trim().parse::<u32>().ok()
                    }
                }
                _ => None,
            };
            timeout_hours = parsed_hours.or(Some(48));
            escalate_to = to.escalate_to.clone();
        }

        StateIR {
            name: s.name.clone(),
            caption,
            badge,
            timeout_hours,
            escalate_to,
            allow_modify,
            allow_delete,
        }
    }

    fn lower_transition(&self, tr: &TransitionDef) -> Result<TransitionIR, AnalyzerError> {
        let mut caption = tr.name.clone();
        let mut icon = "Activity".to_string();
        let mut required_roles = Vec::new();

        for p in &tr.properties {
            if p.name == "caption" {
                if let Expr::StringLit(val) = &p.value {
                    caption = val.clone();
                }
            } else if p.name == "icon" {
                if let Expr::StringLit(val) = &p.value {
                    icon = val.clone();
                }
            } else if p.name == "roles"
                && let Expr::ArrayLit(arr) = &p.value
            {
                for role_expr in arr {
                    if let Expr::StringLit(r) = role_expr {
                        required_roles.push(r.clone());
                    } else if let Expr::Ident(r) = role_expr {
                        required_roles.push(r.clone());
                    }
                }
            }
        }

        let mut params = Vec::new();
        for p in &tr.params {
            params.push(self.lower_field(p)?);
        }

        let guard_script = tr.guard.as_ref().and_then(|g| g.raw_source.as_ref().map(|s| strip_typescript(s)));
        let on_transition_script = tr.on_transition.as_ref().and_then(|ot| ot.raw_source.as_ref().map(|s| strip_typescript(s)));
        let mut approval_tiers = Vec::new();
        if let Some(matrix) = &tr.approval_matrix {
            for tier in matrix {
                approval_tiers.push(ApprovalTierIR {
                    name: tier.name.clone(),
                    condition: format_expr(&tier.condition),
                    roles: tier.roles.clone(),
                });
            }
        }

        Ok(TransitionIR {
            name: tr.name.clone(),
            caption,
            icon,
            from_states: tr.from_states.clone(),
            to_state: tr.to_state.clone(),
            required_roles,
            params,
            guard_script,
            on_transition_script,
            approval_tiers,
        })
    }

    fn lower_state_machine(&self, sm: &StateMachineDef) -> Result<StateMachineIR, AnalyzerError> {
        let mut initial_state = "Draft".to_string();
        for prop in &sm.properties {
            if (prop.name == "initial" || prop.name == "initialState")
                && let Expr::StringLit(s) = &prop.value
            {
                initial_state = s.clone();
            } else if (prop.name == "initial" || prop.name == "initialState")
                && let Expr::Ident(s) = &prop.value
            {
                initial_state = s.clone();
            }
        }

        let mut states = Vec::new();
        let mut detected_init = None;
        for s in &sm.states {
            states.push(self.lower_state(s, &mut detected_init));
        }
        if let Some(init_st) = detected_init {
            initial_state = init_st;
        }

        let mut transitions = Vec::new();
        for tr in &sm.transitions {
            transitions.push(self.lower_transition(tr)?);
        }

        Ok(StateMachineIR {
            name: sm.name.clone(),
            target_table: to_snake_case(&sm.target_table),
            status_field: to_snake_case(&sm.status_field),
            initial_state,
            states,
            transitions,
        })
    }

    pub fn lower_state_machine_extension(&self, sme: &StateMachineExtDef) -> Result<StateMachineExtensionIR, AnalyzerError> {
        let mut states = Vec::new();
        let mut unused_init = None;
        for s in &sme.states {
            states.push(self.lower_state(s, &mut unused_init));
        }
        let mut transitions = Vec::new();
        for tr in &sme.transitions {
            transitions.push(self.lower_transition(tr)?);
        }
        Ok(StateMachineExtensionIR {
            name: sme.name.clone(),
            package_name: self.package_name.clone(),
            target_state_machine: sme.target_state_machine.clone(),
            states,
            transitions,
        })
    }

    fn lower_codeunit(&self, cu: &CodeunitDef) -> Result<CodeunitIR, AnalyzerError> {
        let is_exposed = cu.annotations.iter().any(|a| a == "exposed");
        let is_pushdown = cu.annotations.iter().any(|a| a == "pushdown" || a == "amdp");
        let is_transactional = true;
        let mut methods = Vec::new();

        for m in &cu.methods {
            let params = m.params.iter().map(|p| (p.name.clone(), p.param_type.clone())).collect();
            let actions = lower_statements(&m.body.statements);
            let script = m.body.raw_source.as_ref().map(|s| strip_typescript(s));

            let is_method_pushdown = is_pushdown || m.annotations.iter().any(|a| a == "pushdown" || a == "amdp");

            methods.push(MethodIR {
                name: m.name.clone(),
                is_public: m.is_public,
                is_static: m.is_static,
                params,
                return_type: m.return_type.clone(),
                trigger_event: None,
                annotations: m.annotations.clone(),
                actions,
                script,
                is_pushdown: is_method_pushdown,
            });
        }

        Ok(CodeunitIR {
            package_name: self.package_name.clone(),
            name: cu.name.clone(),
            bound_table: cu.bound_table.clone(),
            is_exposed,
            is_transactional,
            is_pushdown,
            execution_mode: if is_pushdown { Some("AmdpPushdown".to_string()) } else { Some("BytecodeInterpreter".to_string()) },
            max_fuel: Some(100_000),
            methods,
        })
    }

    fn lower_interface(&self, iface: &InterfaceDef) -> Result<InterfaceIR, AnalyzerError> {
        let mut methods = Vec::new();
        for m in &iface.methods {
            let params = m.params.iter().map(|p| (p.name.clone(), p.param_type.clone())).collect();
            methods.push(MethodIR {
                name: m.name.clone(),
                is_public: true,
                is_static: false,
                params,
                return_type: m.return_type.clone(),
                trigger_event: None,
                annotations: Vec::new(),
                actions: Vec::new(),
                script: None,
                is_pushdown: false,
            });
        }
        Ok(InterfaceIR {
            name: iface.name.clone(),
            methods,
        })
    }

    fn lower_enum(&self, e: &EnumDef) -> Result<EnumIR, AnalyzerError> {
        let mut members = Vec::new();
        for m in &e.members {
            members.push(EnumMemberIR {
                name: m.name.clone(),
                bound_codeunit: m.bound_codeunit.clone(),
            });
        }
        Ok(EnumIR {
            name: e.name.clone(),
            implements_interface: e.implements_interface.clone(),
            members,
        })
    }

    pub fn lower_enum_extension(&self, ee: &EnumExtDef) -> Result<EnumExtensionIR, AnalyzerError> {
        let mut members = Vec::new();
        for m in &ee.members {
            members.push(EnumMemberIR {
                name: m.name.clone(),
                bound_codeunit: m.bound_codeunit.clone(),
            });
        }
        Ok(EnumExtensionIR {
            name: ee.name.clone(),
            package_name: self.package_name.clone(),
            target_enum: ee.target_enum.clone(),
            members,
        })
    }

    pub fn lower_permission_set(&self, ps: &PermissionSetDef) -> Result<PermissionSetIR, AnalyzerError> {
        let mut caption = None;
        for prop in &ps.properties {
            if prop.name.eq_ignore_ascii_case("caption") {
                caption = Some(format_expr(&prop.value));
            }
        }

        let mut permissions = Vec::new();
        for perm in &ps.permissions {
            let sql_table_name = to_snake_case(&perm.table_name);
            let mut can_read = false;
            let mut can_insert = false;
            let mut can_modify = false;
            let mut can_delete = false;

            for action in &perm.actions {
                match action {
                    PermissionAction::Read => can_read = true,
                    PermissionAction::Insert => can_insert = true,
                    PermissionAction::Modify => can_modify = true,
                    PermissionAction::Delete => can_delete = true,
                }
            }

            let row_filter = perm.row_filter.as_ref().map(|f| lower_expr_to_sql_filter(f, &sql_table_name));

            permissions.push(TablePermissionIR {
                table_name: perm.table_name.clone(),
                sql_table_name,
                can_read,
                can_insert,
                can_modify,
                can_delete,
                row_filter,
            });
        }

        Ok(PermissionSetIR {
            name: ps.name.clone(),
            caption,
            permissions,
            execute: ps.execute.clone(),
        })
    }

    pub fn lower_view(&self, v: &ViewDef) -> Result<ViewIR, AnalyzerError> {
        let sql_view_name = to_snake_case(&v.name);
        let mut is_materialized = false;
        let mut refresh_interval = None;

        for prop in &v.properties {
            if prop.name.eq_ignore_ascii_case("materialized") {
                if let Expr::BoolLit(b) = prop.value {
                    is_materialized = b;
                }
            } else if prop.name.eq_ignore_ascii_case("refreshInterval") {
                refresh_interval = Some(format_expr(&prop.value));
            }
        }

        let mut select_fields = Vec::new();
        for sf in &v.select_fields {
            select_fields.push(ViewSelectFieldIR {
                expr: format_expr(&sf.field_expr),
                alias: sf.alias.clone(),
            });
        }

        let mut joins = Vec::new();
        for j in &v.joins {
            joins.push(ViewJoinIR {
                join_type: match j.join_type {
                    JoinType::Left => "LEFT".to_string(),
                    JoinType::Inner => "INNER".to_string(),
                },
                target_table: to_snake_case(&j.target_table),
                alias: j.alias.clone(),
                condition: format_expr(&j.condition),
            });
        }

        let where_clause = v.where_filter.as_ref().map(|f| format_expr(f));

        Ok(ViewIR {
            name: v.name.clone(),
            sql_view_name,
            is_materialized,
            refresh_interval,
            select_fields,
            source_table: to_snake_case(&v.source_table),
            joins,
            where_clause,
        })
    }

    pub fn lower_query(&self, q: &QueryDef) -> Result<QueryIR, AnalyzerError> {
        let mut caption = None;
        for prop in &q.properties {
            if prop.name.eq_ignore_ascii_case("caption") {
                caption = Some(format_expr(&prop.value));
            }
        }

        let mut elements = Vec::new();
        for elem in &q.elements {
            elements.push(self.lower_dataitem(elem)?);
        }

        let mut order_by = Vec::new();
        for ord in &q.order_by {
            order_by.push((ord.column.clone(), ord.ascending));
        }

        let having = q.having.as_ref().map(|h| format_expr(h));

        // Synthesize full SQL query string
        let sql_query = generate_query_sql(&elements, &q.group_by, having.as_deref(), &order_by);

        Ok(QueryIR {
            name: q.name.clone(),
            caption,
            elements,
            group_by: q.group_by.clone(),
            having,
            order_by,
            sql_query,
        })
    }

    fn lower_dataitem(&self, di: &DataItemDef) -> Result<DataItemIR, AnalyzerError> {
        let mut columns = Vec::new();
        for col in &di.columns {
            columns.push(QueryColumnIR {
                name: col.name.clone(),
                source_field: to_snake_case(&col.source_field),
                aggregation: col.aggregation.as_ref().map(|a| match a {
                    AggregationFunc::Sum => "SUM".to_string(),
                    AggregationFunc::Average => "AVG".to_string(),
                    AggregationFunc::Count => "COUNT".to_string(),
                    AggregationFunc::Min => "MIN".to_string(),
                    AggregationFunc::Max => "MAX".to_string(),
                    _ => "SUM".to_string(),
                }),
            });
        }

        let mut child_dataitems = Vec::new();
        for child in &di.data_items {
            child_dataitems.push(self.lower_dataitem(child)?);
        }

        Ok(DataItemIR {
            name: di.name.clone(),
            table_name: to_snake_case(&di.table_name),
            columns,
            filter: di.filter.as_ref().map(|f| format_expr(f)),
            join_type: di.join_type.map(|j| match j {
                JoinType::Left => "LEFT JOIN".to_string(),
                JoinType::Inner => "INNER JOIN".to_string(),
            }),
            join_condition: di.join_condition.as_ref().map(|c| format_expr(c)),
            data_items: child_dataitems,
        })
    }

    pub fn lower_page(&self, p: &PageDef) -> Result<PageIR, AnalyzerError> {
        Ok(page_lower::lower_page(p))
    }

    pub fn lower_page_extension(&self, pe: &PageExtDef) -> Result<PageExtensionIR, AnalyzerError> {
        Ok(page_lower::lower_page_extension(pe))
    }
    pub fn lower_menu(&self, m: &MenuDef) -> MenuIR {
        let mut label = None;
        let mut role = None;
        for prop in &m.properties {
            match prop.name.as_str() {
                "caption" | "label" => label = property_text(&prop.value),
                "role" => role = property_text(&prop.value),
                _ => {}
            }
        }
        MenuIR {
            name: m.name.clone(),
            label,
            role,
            items: m.items.iter().map(|it| self.lower_menu_item(it)).collect(),
        }
    }

    fn lower_menu_item(&self, item: &MenuItemDef) -> MenuItemIR {
        let mut label = None;
        let mut icon = None;
        let mut page = None;
        let mut shortcut = None;
        let mut badge_field = None;
        for prop in &item.properties {
            match prop.name.as_str() {
                "caption" | "label" => label = property_text(&prop.value),
                "icon" => icon = property_text(&prop.value),
                "page" => page = property_text(&prop.value),
                "shortcut" | "shortcutKey" => shortcut = property_text(&prop.value),
                "badge" | "badgeField" => badge_field = property_text(&prop.value),
                _ => {}
            }
        }
        MenuItemIR {
            name: item.name.clone(),
            label,
            icon,
            page,
            shortcut,
            badge_field,
            items: item.items.iter().map(|ch| self.lower_menu_item(ch)).collect(),
        }
    }

    pub fn lower_report(&self, r: &ReportDef) -> Result<ReportIR, AnalyzerError> {
        let mut caption = None;
        for prop in &r.properties {
            if prop.name == "caption" {
                if let Expr::StringLit(ref s) = prop.value {
                    caption = Some(s.clone());
                }
            }
        }
        let mut elements = Vec::new();
        for di in &r.elements {
            elements.push(self.lower_dataitem(di)?);
        }
        let mut triggers = Vec::new();
        for trg in &r.triggers {
            let script = trg.body.raw_source.as_ref().map(|s| strip_typescript(s));
            triggers.push(TriggerIR {
                name: trg.name.clone(),
                params: trg.params.clone(),
                actions: lower_statements(&trg.body.statements),
                script,
            });
        }
        Ok(ReportIR {
            name: r.name.clone(),
            caption,
            source_table: r.source_table.clone(),
            elements,
            triggers,
        })
    }

    pub fn lower_custom_metadata(&self, cm: &CustomMetadataDef) -> Result<CustomMetadataIR, AnalyzerError> {
        let mut caption = None;
        for prop in &cm.properties {
            if prop.name == "caption" {
                if let Expr::StringLit(ref s) = prop.value {
                    caption = Some(s.clone());
                }
            }
        }
        let mut columns = Vec::new();
        for f in &cm.fields {
            if f.computed.is_none() {
                columns.push(self.lower_field(f)?);
            }
        }
        let mut records = Vec::new();
        for rec in &cm.records {
            let mut map = serde_json::Map::new();
            for prop in rec {
                let val = match &prop.value {
                    Expr::StringLit(s) => serde_json::Value::String(s.clone()),
                    Expr::IntLit(n) => serde_json::json!(n),
                    Expr::FloatLit(f) => serde_json::json!(f),
                    Expr::BoolLit(b) => serde_json::Value::Bool(*b),
                    _ => serde_json::Value::String(format_expr(&prop.value)),
                };
                map.insert(prop.name.clone(), val);
            }
            records.push(serde_json::Value::Object(map));
        }
        Ok(CustomMetadataIR {
            name: cm.name.clone(),
            caption,
            columns,
            records,
        })
    }

    pub fn collect_diagnostics(&self, program: &Program) -> Vec<CompilerDiagnosticIR> {
        let mut diags = Vec::new();

        // Collect interfaces and codeunits for cross-object validation (e.g. G-05 interface-bound enums)
        let interfaces: std::collections::HashMap<String, &InterfaceDef> = program
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Interface(iface) => Some((iface.name.to_ascii_lowercase(), iface)),
                _ => None,
            })
            .collect();
        let codeunits: std::collections::HashMap<String, &CodeunitDef> = program
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Codeunit(cu) => Some((cu.name.to_ascii_lowercase(), cu)),
                _ => None,
            })
            .collect();
        let enums_in_prog: std::collections::HashMap<String, &EnumDef> = program
            .items
            .iter()
            .filter_map(|it| match it {
                Item::Enum(en) => Some((en.name.to_ascii_lowercase(), en)),
                _ => None,
            })
            .collect();

        for item in &program.items {
            match item {
                Item::Table(t) => {
                    let mut seen_fields = std::collections::HashSet::new();
                    for f in &t.fields {
                        let lower = f.name.to_ascii_lowercase();
                        if !seen_fields.insert(lower) {
                            diags.push(CompilerDiagnosticIR {
                                severity: "error".to_string(),
                                code: "BOS2001".to_string(),
                                message: format!("Duplicate field '{}' in table '{}'", f.name, t.name),
                                line: f.span.line,
                                column: f.span.column,
                                end_line: f.span.line,
                                end_column: f.span.column + f.name.len(),
                                hint: Some(format!("Rename or remove the duplicate '{}' field declaration.", f.name)),
                            });
                        }
                        if let Some(ref block) = f.validate_block {
                            Self::check_embedded_block(block, &format!("validate({})", f.name), &mut diags);
                        }
                    }

                    if !t.fields.is_empty() && t.primary_key.is_empty() {
                        diags.push(CompilerDiagnosticIR {
                            severity: "warning".to_string(),
                            code: "BOS2002".to_string(),
                            message: format!("Table '{}' does not declare an explicit primaryKey(...)", t.name),
                            line: t.span.line,
                            column: t.span.column,
                            end_line: t.span.line,
                            end_column: t.span.column + t.name.len() + 6,
                            hint: Some("Add `primaryKey(id);` inside the table body.".to_string()),
                        });
                    }

                    for pk in &t.primary_key {
                        if !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case(pk)) {
                            diags.push(CompilerDiagnosticIR {
                                severity: "error".to_string(),
                                code: "BOS2003".to_string(),
                                message: format!("Primary key column '{}' is not defined as a field on table '{}'", pk, t.name),
                                line: t.span.line,
                                column: t.span.column,
                                end_line: t.span.line,
                                end_column: t.span.column + t.name.len() + 6,
                                hint: Some(format!("Declare `{}: UUID = auto();` or update `primaryKey(...)`.", pk)),
                            });
                        }
                    }

                    for k in &t.keys {
                        let start_idx = if k.fields.len() > 1 && !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case(&k.fields[0])) {
                            1
                        } else {
                            0
                        };
                        for col in &k.fields[start_idx..] {
                            if !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case(col)) {
                                diags.push(CompilerDiagnosticIR {
                                    severity: "error".to_string(),
                                    code: "BOS2004".to_string(),
                                    message: format!("Secondary key column '{}' does not exist on table '{}'", col, t.name),
                                    line: k.span.line,
                                    column: k.span.column,
                                    end_line: k.span.line,
                                    end_column: k.span.column + col.len() + 5,
                                    hint: Some(format!("Ensure '{}' matches one of the declared field names.", col)),
                                });
                            }
                        }
                    }

                    for fg in &t.field_groups {
                        for col in &fg.fields {
                            if !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case(col)) {
                                diags.push(CompilerDiagnosticIR {
                                    severity: "error".to_string(),
                                    code: "BOS2008".to_string(),
                                    message: format!(
                                        "FieldGroup '{}' references unknown field '{}' on table '{}'",
                                        fg.name, col, t.name
                                    ),
                                    line: fg.span.line,
                                    column: fg.span.column,
                                    end_line: fg.span.line,
                                    end_column: fg.span.column + col.len() + 10,
                                    hint: Some(format!("Ensure '{}' is declared as a field on table '{}'.", col, t.name)),
                                });
                            }
                        }
                    }

                    let is_document = t.properties.iter().any(|p| {
                        p.name == "tableType" && matches!(&p.value, Expr::Ident(s) | Expr::StringLit(s) if s.eq_ignore_ascii_case("Document"))
                    });
                    if is_document && !t.fields.iter().any(|f| f.name.eq_ignore_ascii_case("status")) {
                        diags.push(CompilerDiagnosticIR {
                            severity: "warning".to_string(),
                            code: "BOS2005".to_string(),
                            message: format!("Document table '{}' should include a 'status' field for lifecycle state tracking", t.name),
                            line: t.span.line,
                            column: t.span.column,
                            end_line: t.span.line,
                            end_column: t.span.column + t.name.len() + 6,
                            hint: Some("Add `status: String(20) = \"Draft\";` to track document state.".to_string()),
                        });
                    }

                    for trg in &t.triggers {
                        Self::check_embedded_block(&trg.body, &trg.name, &mut diags);
                    }
                }
                Item::TableLogic(tl) => {
                    for fv in &tl.field_validations {
                        Self::check_embedded_block(&fv.body, &format!("validate {}", fv.field_name), &mut diags);
                    }
                    for trg in &tl.triggers {
                        Self::check_embedded_block(&trg.body, &trg.name, &mut diags);
                    }
                    for m in &tl.methods {
                        Self::check_embedded_block(&m.body, &m.name, &mut diags);
                    }
                }
                Item::TableLogicExtension(tle) => {
                    for fv in &tle.field_validations {
                        Self::check_embedded_block(&fv.body, &format!("validate {}", fv.field_name), &mut diags);
                    }
                    for trg in &tle.triggers {
                        Self::check_embedded_block(&trg.body, &trg.name, &mut diags);
                    }
                    for m in &tle.methods {
                        Self::check_embedded_block(&m.body, &m.name, &mut diags);
                    }
                }
                Item::Codeunit(cu) => {
                    let mut seen_methods = std::collections::HashSet::new();
                    for m in &cu.methods {
                        if !seen_methods.insert(m.name.to_ascii_lowercase()) {
                            diags.push(CompilerDiagnosticIR {
                                severity: "error".to_string(),
                                code: "BOS2007".to_string(),
                                message: format!("Duplicate procedure '{}' in codeunit '{}'", m.name, cu.name),
                                line: m.span.line,
                                column: m.span.column,
                                end_line: m.span.line,
                                end_column: m.span.column + m.name.len(),
                                hint: Some("Each procedure in a codeunit must have a unique name.".to_string()),
                            });
                        }
                        Self::check_embedded_block(&m.body, &m.name, &mut diags);
                    }
                    let implemented_iface = cu
                        .annotations
                        .iter()
                        .find_map(|a| a.strip_prefix("implements:").map(|s| s.to_string()))
                        .or_else(|| {
                            cu.properties
                                .iter()
                                .find(|p| p.name == "implements")
                                .and_then(|p| property_text(&p.value))
                        });
                    if let Some(ref iface_name) = implemented_iface {
                        if let Some(iface) = interfaces.get(&iface_name.to_ascii_lowercase()) {
                            for sig in &iface.methods {
                                if !cu.methods.iter().any(|m| m.name.eq_ignore_ascii_case(&sig.name)) {
                                    diags.push(CompilerDiagnosticIR {
                                        severity: "error".to_string(),
                                        code: "BOS2014".to_string(),
                                        message: format!(
                                            "Codeunit '{}' implements interface '{}' but is missing method '{}'",
                                            cu.name, iface.name, sig.name
                                        ),
                                        line: cu.span.line,
                                        column: cu.span.column,
                                        end_line: cu.span.line,
                                        end_column: cu.span.column + cu.name.len() + 9,
                                        hint: Some(format!("Implement `procedure {}(...)` on codeunit '{}'.", sig.name, cu.name)),
                                    });
                                }
                            }
                        }
                    }
                }
                Item::Enum(en) => {
                    if let Some(ref iface_name) = en.implements_interface {
                        if let Some(iface) = interfaces.get(&iface_name.to_ascii_lowercase()) {
                            for mem in &en.members {
                                if let Some(ref cu_name) = mem.bound_codeunit {
                                    if let Some(cu) = codeunits.get(&cu_name.to_ascii_lowercase()) {
                                        for sig in &iface.methods {
                                            if !cu.methods.iter().any(|m| m.name.eq_ignore_ascii_case(&sig.name)) {
                                                diags.push(CompilerDiagnosticIR {
                                                    severity: "error".to_string(),
                                                    code: "BOS2015".to_string(),
                                                    message: format!(
                                                        "Enum '{}' member '{}' binds codeunit '{}' which does not implement method '{}' of interface '{}'",
                                                        en.name, mem.name, cu.name, sig.name, iface.name
                                                    ),
                                                    line: mem.span.line,
                                                    column: mem.span.column,
                                                    end_line: mem.span.line,
                                                    end_column: mem.span.column + mem.name.len(),
                                                    hint: Some(format!(
                                                        "Add `procedure {}(...)` to codeunit '{}' to satisfy interface '{}'.",
                                                        sig.name, cu.name, iface.name
                                                    )),
                                                });
                                            }
                                        }
                                    } else if !codeunits.is_empty() {
                                        diags.push(CompilerDiagnosticIR {
                                            severity: "error".to_string(),
                                            code: "BOS2016".to_string(),
                                            message: format!(
                                                "Enum '{}' member '{}' binds unknown codeunit '{}' for interface '{}'",
                                                en.name, mem.name, cu_name, iface.name
                                            ),
                                            line: mem.span.line,
                                            column: mem.span.column,
                                            end_line: mem.span.line,
                                            end_column: mem.span.column + mem.name.len(),
                                            hint: Some(format!(
                                                "Declare `codeunit {} implements {} {{ ... }}`.",
                                                cu_name, iface.name
                                            )),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                Item::EnumExtension(ee) => {
                    if let Some(base_en) = enums_in_prog.get(&ee.target_enum.to_ascii_lowercase()) {
                        if let Some(ref iface_name) = base_en.implements_interface {
                            if let Some(iface) = interfaces.get(&iface_name.to_ascii_lowercase()) {
                                for mem in &ee.members {
                                    if let Some(ref cu_name) = mem.bound_codeunit {
                                        if let Some(cu) = codeunits.get(&cu_name.to_ascii_lowercase()) {
                                            for sig in &iface.methods {
                                                if !cu.methods.iter().any(|m| m.name.eq_ignore_ascii_case(&sig.name)) {
                                                    diags.push(CompilerDiagnosticIR {
                                                        severity: "error".to_string(),
                                                        code: "BOS2015".to_string(),
                                                        message: format!(
                                                            "EnumExtension '{}' member '{}' binds codeunit '{}' which does not implement method '{}' of interface '{}'",
                                                            ee.name, mem.name, cu.name, sig.name, iface.name
                                                        ),
                                                        line: mem.span.line,
                                                        column: mem.span.column,
                                                        end_line: mem.span.line,
                                                        end_column: mem.span.column + mem.name.len(),
                                                        hint: Some(format!(
                                                            "Add `procedure {}(...)` to codeunit '{}' to satisfy interface '{}'.",
                                                            sig.name, cu.name, iface.name
                                                        )),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Item::StateMachine(sm) => {
                    let state_names: std::collections::HashSet<String> =
                        sm.states.iter().map(|s| s.name.to_ascii_lowercase()).collect();
                    if !state_names.is_empty() {
                        for tr in &sm.transitions {
                            for from_st in &tr.from_states {
                                if !from_st.is_empty() && !state_names.contains(&from_st.to_ascii_lowercase()) {
                                    diags.push(CompilerDiagnosticIR {
                                        severity: "error".to_string(),
                                        code: "BOS2006".to_string(),
                                        message: format!(
                                            "Transition '{}' references undeclared source state '{}' in state machine '{}'",
                                            tr.name, from_st, sm.name
                                        ),
                                        line: tr.span.line,
                                        column: tr.span.column,
                                        end_line: tr.span.line,
                                        end_column: tr.span.column + tr.name.len() + 10,
                                        hint: Some(format!("Declare state '{}' in `states` before referencing it in transitions.", from_st)),
                                    });
                                }
                            }
                            if !tr.to_state.is_empty() && !state_names.contains(&tr.to_state.to_ascii_lowercase()) {
                                diags.push(CompilerDiagnosticIR {
                                    severity: "error".to_string(),
                                    code: "BOS2006".to_string(),
                                    message: format!(
                                        "Transition '{}' references undeclared target state '{}' in state machine '{}'",
                                        tr.name, tr.to_state, sm.name
                                    ),
                                    line: tr.span.line,
                                    column: tr.span.column,
                                    end_line: tr.span.line,
                                    end_column: tr.span.column + tr.name.len() + 10,
                                    hint: Some(format!("Declare state '{}' in `states` before referencing it in transitions.", tr.to_state)),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        diags
    }

    fn check_embedded_block(block: &Block, context_name: &str, diags: &mut Vec<CompilerDiagnosticIR>) {
        let Some(ref raw) = block.raw_source else { return };
        let _ = context_name;
        diags.extend(crate::script_opt::diagnose(raw, block.span.line));
        diags.extend(crate::ts_stripper::syntax_diagnostics(raw, block.span.line));
        let mut stack: Vec<(char, usize, usize)> = Vec::new();
        let mut in_string: Option<char> = None;
        let mut escaped = false;
        let mut line = block.span.line;
        let mut col = block.span.column;

        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\n' {
                line += 1;
                col = 1;
                if in_string == Some('"') || in_string == Some('\'') {
                    in_string = None;
                }
                continue;
            }
            col += 1;

            if let Some(q) = in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == q {
                    in_string = None;
                }
                continue;
            }

            if ch == '/' && chars.peek() == Some(&'/') {
                for next_c in chars.by_ref() {
                    if next_c == '\n' {
                        line += 1;
                        col = 1;
                        break;
                    }
                }
                continue;
            }

            if ch == '"' || ch == '\'' || ch == '`' {
                in_string = Some(ch);
                continue;
            }

            match ch {
                '(' | '[' | '{' => stack.push((ch, line, col)),
                ')' | ']' | '}' => {
                    let expected_open = match ch {
                        ')' => '(',
                        ']' => '[',
                        '}' => '{',
                        _ => continue,
                    };
                    if let Some((open_ch, open_line, open_col)) = stack.pop() {
                        if open_ch != expected_open {
                            diags.push(CompilerDiagnosticIR {
                                severity: "error".to_string(),
                                code: "BOS2010".to_string(),
                                message: format!(
                                    "Mismatched delimiter '{}' in '{}' (opened '{}' at line {}, col {})",
                                    ch, context_name, open_ch, open_line, open_col
                                ),
                                line,
                                column: col,
                                end_line: line,
                                end_column: col + 1,
                                hint: Some(format!("Close '{}' before '{}'.", open_ch, ch)),
                            });
                        }
                    } else {
                        diags.push(CompilerDiagnosticIR {
                            severity: "error".to_string(),
                            code: "BOS2010".to_string(),
                            message: format!("Unexpected closing '{}' in '{}'", ch, context_name),
                            line,
                            column: col,
                            end_line: line,
                            end_column: col + 1,
                            hint: Some(format!("Remove stray '{}' or add a matching opening delimiter.", ch)),
                        });
                    }
                }
                _ => {}
            }
        }

        if let Some((open_ch, open_line, open_col)) = stack.pop() {
            diags.push(CompilerDiagnosticIR {
                severity: "error".to_string(),
                code: "BOS2011".to_string(),
                message: format!("Unclosed '{}' in '{}'", open_ch, context_name),
                line: open_line,
                column: open_col,
                end_line: open_line,
                end_column: open_col + 1,
                hint: Some(format!("Add a matching closing delimiter for '{}' in '{}'.", open_ch, context_name)),
            });
        }

        // Security & Anti-Destructive Guard: disallow dynamic code injection (`eval`, `new Function`)
        // and destructive DDL inside embedded procedural blocks.
        let forbidden_patterns: &[(&str, &str, &str)] = &[
            (
                "eval(",
                "BOS2015",
                "Dynamic code execution via `eval(...)` is forbidden in BOS-Script sandbox for security.",
            ),
            (
                "new Function(",
                "BOS2015",
                "Dynamic code construction via `new Function(...)` is forbidden in BOS-Script sandbox for security.",
            ),
            (
                "DROP TABLE ",
                "BOS2016",
                "Destructive DDL (`DROP TABLE`) is forbidden inside procedural BOS-Script triggers and codeunits.",
            ),
            (
                "DROP DATABASE ",
                "BOS2016",
                "Destructive DDL (`DROP DATABASE`) is forbidden inside procedural BOS-Script triggers and codeunits.",
            ),
            (
                "TRUNCATE TABLE ",
                "BOS2016",
                "Destructive DDL (`TRUNCATE TABLE`) is forbidden inside procedural BOS-Script triggers and codeunits.",
            ),
        ];

        for (offset, raw_line) in raw.lines().enumerate() {
            let trimmed_line = raw_line.trim();
            if trimmed_line.starts_with("//") {
                continue;
            }
            for (pat, code, msg) in forbidden_patterns {
                if let Some(col_idx) = raw_line.find(pat) {
                    // For `eval(`, verify it isn't part of another identifier like `myeval(`
                    if *pat == "eval(" && col_idx > 0 {
                        let prev = raw_line[..col_idx].chars().last().unwrap_or(' ');
                        if prev.is_alphanumeric() || prev == '_' || prev == '.' {
                            continue;
                        }
                    }
                    let err_line = block.span.line + offset;
                    let err_col = col_idx + 1;
                    diags.push(CompilerDiagnosticIR {
                        severity: "error".to_string(),
                        code: (*code).to_string(),
                        message: (*msg).to_string(),
                        line: err_line,
                        column: err_col,
                        end_line: err_line,
                        end_column: err_col + pat.len(),
                        hint: Some("Use strongly typed `models.<Table>` repository methods or `ctx` APIs instead.".to_string()),
                    });
                }
            }
        }
    }
}

fn lower_statements(stmts: &[Stmt]) -> Vec<ActionIR> {
    let mut actions = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Expr(Expr::Call { callee, args }) => {
                match &**callee {
                    Expr::Ident(func_name) => {
                        if func_name.eq_ignore_ascii_case("validateRequired") || func_name.eq_ignore_ascii_case("validate") {
                            if let Some(first_arg) = args.first() {
                                let field = match first_arg {
                                    Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(first_arg),
                                };
                                actions.push(ActionIR::ValidateRequired { field, location: None });
                            }
                        } else if func_name.eq_ignore_ascii_case("assert") {
                            if let Some(cond_arg) = args.first() {
                                let condition = format_expr(cond_arg);
                                let message = args.get(1).map(|a| match a {
                                    Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(a),
                                }).unwrap_or_else(|| format!("Assertion '{}' failed", condition));
                                actions.push(ActionIR::Assert { condition, message, location: None });
                            }
                        } else if func_name.eq_ignore_ascii_case("hashPassword") || func_name.eq_ignore_ascii_case("hashField") {
                            let source_field = args.first().map(|a| match a {
                                Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                _ => format_expr(a),
                            }).unwrap_or_else(|| "password".to_string());
                            let target_field = args.get(1).map(|a| match a {
                                Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                _ => format_expr(a),
                            }).unwrap_or_else(|| "passwordHash".to_string());
                            actions.push(ActionIR::HashPassword {
                                source_field,
                                target_field,
                                location: None,
                            });
                        } else if func_name.eq_ignore_ascii_case("logAudit") || func_name.eq_ignore_ascii_case("audit") {
                            if let Some(msg_arg) = args.first() {
                                let message = match msg_arg {
                                    Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(msg_arg),
                                };
                                actions.push(ActionIR::LogAudit { message, location: None });
                            }
                        } else if func_name.eq_ignore_ascii_case("getRecord") {
                            if args.len() >= 3 {
                                let table = match &args[0] {
                                    Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(&args[0]),
                                };
                                let filter = match &args[1] {
                                    Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(&args[1]),
                                };
                                let target_buffer = match &args[2] {
                                    Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                    _ => format_expr(&args[2]),
                                };
                                actions.push(ActionIR::GetRecord { table, filter, target_buffer, location: None });
                            }
                        } else if func_name.eq_ignore_ascii_case("callProcedure")
                            && args.len() >= 2
                        {
                            let cu = match &args[0] {
                                Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                _ => format_expr(&args[0]),
                            };
                            let meth = match &args[1] {
                                Expr::Ident(s) | Expr::StringLit(s) => s.clone(),
                                _ => format_expr(&args[1]),
                            };
                            actions.push(ActionIR::CallProcedure { codeunit: cu, method: meth, location: None });
                        } else {
                            actions.push(ActionIR::NotLowered { kind: "call".to_string() });
                        }
                    }
                    Expr::MemberAccess { object, member } => {
                        if let Expr::Ident(cu_name) = &**object {
                            actions.push(ActionIR::CallProcedure {
                                codeunit: cu_name.clone(),
                                method: member.clone(),
                                location: None,
                            });
                        } else {
                            actions.push(ActionIR::NotLowered { kind: "call".to_string() });
                        }
                    }
                    _ => {
                        actions.push(ActionIR::NotLowered { kind: "call".to_string() });
                    }
                }
            }
            Stmt::Assign { target, value } => {
                let field = match target {
                    Expr::Ident(s) => s.clone(),
                    Expr::MemberAccess { object, member } => {
                        if let Expr::Ident(obj_name) = &**object {
                            if obj_name == "this" {
                                member.clone()
                            } else {
                                format!("{}.{}", obj_name, member)
                            }
                        } else {
                            member.clone()
                        }
                    }
                    _ => format_expr(target),
                };
                let value_expr = format_expr(value);
                actions.push(ActionIR::SetField { field, value_expr, location: None });
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let cond_str = format_expr(condition);
                let then_actions = lower_statements(&then_branch.statements);
                let else_actions = else_branch.as_ref().map(|eb| lower_statements(&eb.statements));
                actions.push(ActionIR::IfElse {
                    condition: cond_str,
                    then_branch: then_actions,
                    else_branch: else_actions,
                    location: None,
                });
            }
            Stmt::Expr(_) => {
                actions.push(ActionIR::NotLowered { kind: "expression".to_string() });
            }
            Stmt::Return(_) => {
                actions.push(ActionIR::NotLowered { kind: "return".to_string() });
            }
            Stmt::VarDecl { .. } => {
                actions.push(ActionIR::NotLowered { kind: "declaration".to_string() });
            }
        }
    }
    actions
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

fn expr_to_sql_default(expr: &Expr) -> String {
    match expr {
        Expr::StringLit(s) => format!("'{}'", s),
        Expr::IntLit(n) => n.to_string(),
        Expr::FloatLit(f) => f.to_string(),
        Expr::BoolLit(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        Expr::Call { callee, .. } => {
            if let Expr::Ident(name) = &**callee {
                if name == "auto" || name == "gen_random_uuid" {
                    return "gen_random_uuid()".to_string();
                } else if name == "autoNow" || name == "now" {
                    return "NOW()".to_string();
                } else if name == "autoDate" || name == "today" {
                    return "CURRENT_DATE".to_string();
                }
            }
            "NULL".to_string()
        }
        _ => "NULL".to_string(),
    }
}

pub fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Ident(s) => s.clone(),
        Expr::StringLit(s) => s.clone(),
        Expr::IntLit(n) => n.to_string(),
        Expr::FloatLit(f) => f.to_string(),
        Expr::BoolLit(b) => b.to_string(),
        Expr::NullLit => "null".to_string(),
        Expr::Binary { left, op, right } => {
            let op_str = match op {
                BinaryOp::Eq => "==",
                BinaryOp::NotEq => "!=",
                BinaryOp::Lt => "<",
                BinaryOp::Lte => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Gte => ">=",
                BinaryOp::And => "&&",
                BinaryOp::Or => "||",
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
                BinaryOp::In => "in",
            };
            format!("{} {} {}", format_expr(left), op_str, format_expr(right))
        }
        Expr::Unary { op, operand } => {
            let op_str = match op {
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            };
            format!("{}{}", op_str, format_expr(operand))
        }
        Expr::MemberAccess { object, member } => format!("{}.{}", format_expr(object), member),
        Expr::Call { callee, args } => {
            let args_str: Vec<String> = args.iter().map(format_expr).collect();
            format!("{}({})", format_expr(callee), args_str.join(", "))
        }
        _ => String::new(),
    }
}

fn sift_index_columns(filter: &Expr) -> Vec<String> {
    match filter {
        Expr::Binary { op: BinaryOp::And, left, right } => {
            let mut keys = sift_index_columns(left);
            for key in sift_index_columns(right) {
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
            keys
        }
        Expr::Binary { op: BinaryOp::Eq, left, .. } => match left.as_ref() {
            Expr::Ident(name) => vec![to_snake_case(name)],
            Expr::MemberAccess { member, .. } => vec![to_snake_case(member)],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn lower_filter_to_sql(filter: &Expr, target_table: &str, parent_table: &str) -> String {
    match filter {
        Expr::Binary { op: BinaryOp::And, left, right } => format!(
            "({} AND {})",
            lower_filter_to_sql(left, target_table, parent_table),
            lower_filter_to_sql(right, target_table, parent_table)
        ),
        Expr::Binary { op: BinaryOp::Or, left, right } => format!(
            "({} OR {})",
            lower_filter_to_sql(left, target_table, parent_table),
            lower_filter_to_sql(right, target_table, parent_table)
        ),
        Expr::Binary { left, op, right } => {
            let left_str = match &**left {
                Expr::Ident(s) => format!("\"{}\".\"{}\"", target_table, to_snake_case(s)),
                Expr::MemberAccess { object, member } => format!("\"{}\".\"{}\"", format_expr(object), to_snake_case(member)),
                _ => format_expr(left),
            };
            let right_str = match &**right {
                Expr::Ident(s) => format!("\"{}\".\"{}\"", parent_table, to_snake_case(s)),
                Expr::MemberAccess { object, member } => {
                    let obj_name = format_expr(object);
                    if obj_name == "this" || obj_name == "rec" {
                        format!("\"{}\".\"{}\"", parent_table, to_snake_case(member))
                    } else {
                        format!("\"{}\".\"{}\"", obj_name, to_snake_case(member))
                    }
                }
                _ => expr_to_sql_default(right),
            };
            let op_str = match op {
                bosc_syntax::BinaryOp::Eq => "=",
                bosc_syntax::BinaryOp::NotEq => "<>",
                bosc_syntax::BinaryOp::Lt => "<",
                bosc_syntax::BinaryOp::Lte => "<=",
                bosc_syntax::BinaryOp::Gt => ">",
                bosc_syntax::BinaryOp::Gte => ">=",
                bosc_syntax::BinaryOp::And => "AND",
                bosc_syntax::BinaryOp::Or => "OR",
                _ => "=",
            };
            format!("{} {} {}", left_str, op_str, right_str)
        }
        _ => "TRUE".to_string(),
    }
}

fn lower_expr_to_sql_filter(expr: &Expr, table_name: &str) -> String {
    match expr {
        Expr::Binary { left, op, right } => {
            let left_str = match &**left {
                Expr::Ident(s) => format!("\"{}\".\"{}\"", table_name, to_snake_case(s)),
                Expr::MemberAccess { object, member } => format!("\"{}\".\"{}\"", format_expr(object), to_snake_case(member)),
                _ => format_expr(left),
            };
            let right_str = match &**right {
                Expr::Ident(s) => {
                    if s == "currentUserId" || s == "user_id" {
                        "NULLIF(current_setting('bos.current_user_id', true), '')::uuid".to_string()
                    } else if s == "currentTenantId" || s == "tenant_id" {
                        "NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid".to_string()
                    } else {
                        format!("\"{}\"", s)
                    }
                }
                Expr::StringLit(s) => format!("'{}'", s),
                Expr::IntLit(n) => n.to_string(),
                Expr::FloatLit(f) => f.to_string(),
                Expr::BoolLit(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
                _ => format_expr(right),
            };
            let op_str = match op {
                BinaryOp::Eq => "=",
                BinaryOp::NotEq => "<>",
                BinaryOp::Lt => "<",
                BinaryOp::Lte => "<=",
                BinaryOp::Gt => ">",
                BinaryOp::Gte => ">=",
                BinaryOp::And => "AND",
                BinaryOp::Or => "OR",
                _ => "=",
            };
            format!("{} {} {}", left_str, op_str, right_str)
        }
        _ => format_expr(expr),
    }
}

fn generate_query_sql(
    elements: &[DataItemIR],
    group_by: &[String],
    having: Option<&str>,
    order_by: &[(String, bool)],
) -> String {
    let mut select_clauses = Vec::new();
    let mut from_clause = String::new();
    let mut join_clauses = Vec::new();
    let mut where_clauses = Vec::new();

    for (idx, elem) in elements.iter().enumerate() {
        if idx == 0 {
            from_clause = format!("\"{}\" AS \"{}\"", elem.table_name, elem.name);
        } else {
            let jt = elem.join_type.as_deref().unwrap_or("LEFT JOIN");
            let jc = elem.join_condition.as_deref().unwrap_or("TRUE");
            join_clauses.push(format!("{} \"{}\" AS \"{}\" ON {}", jt, elem.table_name, elem.name, jc));
        }

        for col in &elem.columns {
            let col_expr = if let Some(agg) = &col.aggregation {
                format!("{}(\"{}\".\"{}\") AS \"{}\"", agg, elem.name, col.source_field, col.name)
            } else {
                format!("\"{}\".\"{}\" AS \"{}\"", elem.name, col.source_field, col.name)
            };
            select_clauses.push(col_expr);
        }

        if let Some(filt) = &elem.filter {
            where_clauses.push(filt.clone());
        }

        // Recursively handle nested child dataitems
        for child in &elem.data_items {
            let c_jt = child.join_type.as_deref().unwrap_or("LEFT JOIN");
            let c_jc = child.join_condition.as_deref().unwrap_or("TRUE");
            join_clauses.push(format!("{} \"{}\" AS \"{}\" ON {}", c_jt, child.table_name, child.name, c_jc));

            for col in &child.columns {
                let col_expr = if let Some(agg) = &col.aggregation {
                    format!("{}(\"{}\".\"{}\") AS \"{}\"", agg, child.name, col.source_field, col.name)
                } else {
                    format!("\"{}\".\"{}\" AS \"{}\"", child.name, col.source_field, col.name)
                };
                select_clauses.push(col_expr);
            }

            if let Some(filt) = &child.filter {
                where_clauses.push(filt.clone());
            }
        }
    }

    let mut sql = format!("SELECT {}\nFROM {}", select_clauses.join(", "), from_clause);
    if !join_clauses.is_empty() {
        sql.push_str(&format!("\n{}", join_clauses.join("\n")));
    }
    if !where_clauses.is_empty() {
        sql.push_str(&format!("\nWHERE {}", where_clauses.join(" AND ")));
    }
    if !group_by.is_empty() {
        sql.push_str(&format!("\nGROUP BY {}", group_by.join(", ")));
    }
    if let Some(h) = having {
        sql.push_str(&format!("\nHAVING {}", h));
    }
    if !order_by.is_empty() {
        let ord_strs: Vec<String> = order_by
            .iter()
            .map(|(col, asc)| format!("\"{}\" {}", col, if *asc { "ASC" } else { "DESC" }))
            .collect();
        sql.push_str(&format!("\nORDER BY {}", ord_strs.join(", ")));
    }

    sql
}

#[cfg(test)]
mod tests {
    use super::*;
    use bosc_syntax::{Lexer, Parser};

    #[test]
    fn test_lower_codeunit_with_if_and_calls() {
        let source = r#"
            codeunit OrderProcessor {
                Process(orderId: String, amount: Decimal) {
                    validateRequired(orderId);
                    if (amount > 1000) {
                        assert(amount < 1000000, "Limit exceeded");
                        logAudit("Large order approved");
                        status = "Approved";
                    } else {
                        status = "AutoApproved";
                    }
                    AuditLogger.RecordActivity();
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("test-pkg");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        assert_eq!(model.codeunits.len(), 1);
        let cu = &model.codeunits[0];
        assert_eq!(cu.name, "OrderProcessor");
        assert_eq!(cu.methods.len(), 1);

        let m = &cu.methods[0];
        assert_eq!(m.name, "Process");
        assert_eq!(m.actions.len(), 3);

        // Action 1: ValidateRequired
        match &m.actions[0] {
            ActionIR::ValidateRequired { field, .. } => assert_eq!(field, "orderId"),
            _ => panic!("Expected ValidateRequired"),
        }

        // Action 2: IfElse
        match &m.actions[1] {
            ActionIR::IfElse { condition, then_branch, else_branch, .. } => {
                assert!(condition.contains("amount > 1000"));
                assert_eq!(then_branch.len(), 3);
                assert!(else_branch.is_some());
                assert_eq!(else_branch.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected IfElse"),
        }

        // Action 3: CallProcedure
        match &m.actions[2] {
            ActionIR::CallProcedure { codeunit, method, .. } => {
                assert_eq!(codeunit, "AuditLogger");
                assert_eq!(method, "RecordActivity");
            }
            _ => panic!("Expected CallProcedure"),
        }
    }

    #[test]
    fn test_lower_table_with_hash_password_trigger() {
        let source = r#"
            table SecUser {
                username: String(50) { notNull = true; };
                passwordHash: String(255) { notNull = true; };

                onInsert() {
                    validateRequired(username);
                    hashPassword(password, passwordHash);
                    validateRequired(passwordHash);
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("bos.auth");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        assert_eq!(model.tables.len(), 1);
        let tbl = &model.tables[0];
        assert_eq!(tbl.triggers.len(), 1);
        let trg = &tbl.triggers[0];
        assert_eq!(trg.name, "onInsert");
        assert_eq!(trg.actions.len(), 3);

        match &trg.actions[1] {
            ActionIR::HashPassword { source_field, target_field, .. } => {
                assert_eq!(source_field, "password");
                assert_eq!(target_field, "passwordHash");
            }
            _ => panic!("Expected HashPassword action"),
        }
    }

    #[test]
    fn test_lower_table_with_typescript_raw_source() {
        let source = r#"
            table SecUser {
                username: String(50) { notNull = true; };
                passwordHash: String(255) { notNull = true; };

                onInsert(rec, ctx) {
                    if (rec.password) {
                        const salt = ctx.crypto.randomBytes(8).toHex();
                        rec.passwordHash = `$sha256$${salt}$${ctx.crypto.sha256(salt + rec.password)}`;
                        delete rec.password;
                    }
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("bos.auth");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        assert_eq!(model.tables.len(), 1);
        let tbl = &model.tables[0];
        assert_eq!(tbl.triggers.len(), 1);
        let trg = &tbl.triggers[0];
        assert_eq!(trg.name, "onInsert");
        assert!(trg.script.is_some());
        let script = trg.script.as_ref().unwrap();
        assert!(script.contains("ctx.crypto.randomBytes"));
        assert!(script.contains("delete rec.password"));
    }

    #[test]
    fn test_lower_permissionset_view_and_query() {
        let source = r#"
            permissionset SalesRole {
                caption = "Sales Representative";
                permissions = {
                    Customer = RIM;
                    SalesOrder = R where { status == "Open" };
                };
                execute = [SalesPostingCU];
            }

            view OpenSalesOrderView {
                caption = "Open Sales Orders";
                materialized = false;
                select {
                    id,
                    orderNo,
                    amount
                }
                from SalesOrder;
            }

            query SalesOrderStatsQuery {
                caption = "Sales Order Statistics";
                elements {
                    dataitem(so; SalesOrder) {
                        column(soCustomer; customerId) {}
                        column(totalSales; amount; sum) {}
                    }
                }
                groupBy(so.customerId);
                orderBy(totalSales desc);
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("bos.sales");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        assert_eq!(model.permission_sets.len(), 1);
        let ps = &model.permission_sets[0];
        assert_eq!(ps.name, "SalesRole");
        assert_eq!(ps.permissions.len(), 2);
        assert!(ps.permissions[0].can_read);
        assert!(ps.permissions[0].can_insert);
        assert!(ps.permissions[0].can_modify);
        assert!(!ps.permissions[0].can_delete);
        assert_eq!(ps.execute, vec!["SalesPostingCU"]);

        assert_eq!(model.views.len(), 1);
        let v = &model.views[0];
        assert_eq!(v.name, "OpenSalesOrderView");
        assert_eq!(v.sql_view_name, "open_sales_order_view");
        assert_eq!(v.select_fields.len(), 3);

        assert_eq!(model.queries.len(), 1);
        let q = &model.queries[0];
        assert_eq!(q.name, "SalesOrderStatsQuery");
        assert!(q.sql_query.contains("SELECT"));
        assert!(q.sql_query.contains("SUM(\"so\".\"amount\") AS \"totalSales\""));
        assert!(q.sql_query.contains("GROUP BY so.customerId"));
        assert!(q.sql_query.contains("ORDER BY \"totalSales\" DESC"));
    }

    #[test]
    fn test_diagnostics_and_all_meta_object_lowering() {
        let source = r#"
            table InvoiceHeader {
                tableType: Document;
                id: UUID notNull;
                code: String(20);
                code: String(20);
                primaryKey(missing_col);
            }

            statemachine InvoiceWorkflow for InvoiceHeader.status {
                initial state Draft {}
                transition release from Draft to UndeclaredState {}
            }

            codeunit InvoiceService {
                procedure post(rec: any) {
                    const x = 1;
                }
                procedure post(rec: any) {
                    const y = 2;
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("bos.sales");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        let codes: Vec<&str> = model.diagnostics.iter().map(|d| d.code.as_str()).collect();
        // Duplicate field `code` -> BOS2001
        assert!(codes.contains(&"BOS2001"), "Expected BOS2001 in {:?}", codes);
        // Primary key references non-existent field `missing_col` -> BOS2003
        assert!(codes.contains(&"BOS2003"), "Expected BOS2003 in {:?}", codes);
        // Document table missing `status` field -> BOS2005
        assert!(codes.contains(&"BOS2005"), "Expected BOS2005 in {:?}", codes);
        // StateMachine transition to `UndeclaredState` -> BOS2006
        assert!(codes.contains(&"BOS2006"), "Expected BOS2006 in {:?}", codes);
        // Duplicate procedure `post` -> BOS2007
        assert!(codes.contains(&"BOS2007"), "Expected BOS2007 in {:?}", codes);
    }

    #[test]
    fn test_all_gap_fixes_extensions_types_and_state_machine_policies() {
        let source = r#"
            enum OrderStatus {
                value(Draft);
                value(Released);
            }

            enumextension OrderStatusExt extends OrderStatus {
                value(Archived);
            }

            table Note {
                code: Code(30) check("LENGTH(code) > 0");
                content: Text;
                status: Enum(OrderStatus);
                primaryKey(code);
                key(idx_status, status);
            }

            tablelogicextension NoteLogicExt for Note {
                afterInsert(rec: any) {
                    ctx.ui.confirm("Confirm Note?");
                }
            }

            statemachine NoteWorkflow for Note.status {
                initial state Draft {
                    allowModify: true;
                    allowDelete: true;
                }
                state Released {
                    allowModify: false;
                    allowDelete: false;
                }
                transition release {
                    from: Draft;
                    to: Released;
                    guard(rec) {
                        return rec.code !== "";
                    }
                    onTransition(rec) {
                        rec.status = "Released";
                    }
                }
            }

            statemachineextension NoteWorkflowExt extends NoteWorkflow {
                state Archived {
                    allowModify: false;
                    allowDelete: false;
                }
                transition archive from Released to Archived {}
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        let analyzer = SemanticAnalyzer::new("bos.core");
        let model = analyzer.analyze_and_lower(&program).expect("Lowering failed");

        assert!(model.diagnostics.is_empty(), "Expected 0 diagnostics, got: {:?}", model.diagnostics);
        assert_eq!(model.enum_extensions.len(), 1);
        assert_eq!(model.enums[0].members.len(), 3, "Enum extension should merge into base enum");
        assert_eq!(model.state_machine_extensions.len(), 1);
        assert_eq!(model.state_machines[0].states.len(), 3, "StateMachine extension should merge into base state machine");
        assert_eq!(model.state_machines[0].states[1].allow_modify, false);
        assert_eq!(model.state_machines[0].states[1].allow_delete, false);
        assert!(model.state_machines[0].transitions[0].guard_script.is_some());
        assert!(model.state_machines[0].transitions[0].on_transition_script.is_some());
        assert_eq!(model.tables[0].columns[0].is_nullable, false, "primaryKey(code) should mark code as NOT NULL");
        assert_eq!(model.tables[0].columns[0].is_unique, true, "primaryKey(code) should mark code as UNIQUE");
        assert_eq!(model.tables[0].indexes[0].columns, vec!["status".to_string()], "Named key(idx_status, status) should only index status column");
    }

    fn lower(source: &str) -> Result<ModelIR, AnalyzerError> {
        let tokens = Lexer::new(source).tokenize().expect("Lexing failed");
        let program = Parser::with_source(tokens, source.to_string()).parse_program().expect("Parsing failed");
        SemanticAnalyzer::new("bos.test").analyze_and_lower(&program)
    }

    #[test]
    fn relations_on_delete_and_renames_are_lowered() {
        let model = lower(
            r#"
            table SalesOrder {
                renamedFrom = "Order";
                no: String(20) { notNull = true; };
                customerNo: String(20) { relation = Customer.no; onDelete = setNull; };
                itemNo: String(20) { relation: Item.no; };
                shipTo: String(100) { renamedFrom = "address"; };
                primaryKey(no);
            }
            "#,
        )
        .expect("lowering succeeds");
        let table = &model.tables[0];
        assert_eq!(table.renamed_from.as_deref(), Some("order"));
        let customer = table.columns.iter().find(|c| c.sql_name == "customer_no").expect("customer_no");
        let fk = customer.foreign_key.as_ref().expect("relation");
        assert_eq!((fk.target_table.as_str(), fk.target_column.as_str(), fk.on_delete.as_str()), ("customer", "no", "SET NULL"));
        let item = table.columns.iter().find(|c| c.sql_name == "item_no").expect("item_no");
        assert_eq!(item.foreign_key.as_ref().map(|f| f.on_delete.as_str()), Some("RESTRICT"));
        let ship_to = table.columns.iter().find(|c| c.sql_name == "ship_to").expect("ship_to");
        assert_eq!(ship_to.renamed_from.as_deref(), Some("address"));
    }

    #[test]
    fn invalid_relation_options_are_rejected() {
        let unknown = lower("table T { a: String(10) { relation = Customer.no; onDelete = explode; }; }");
        assert!(matches!(unknown, Err(AnalyzerError::InvalidProperty { .. })), "{unknown:?}");

        let without_relation = lower("table T { a: String(10) { onDelete = cascade; }; }");
        assert!(matches!(without_relation, Err(AnalyzerError::InvalidProperty { .. })), "{without_relation:?}");

        let set_null_required = lower("table T { a: String(10) { notNull = true; relation = Customer.no; onDelete = setNull; }; }");
        assert!(matches!(set_null_required, Err(AnalyzerError::InvalidProperty { .. })), "{set_null_required:?}");
    }

    #[test]
    fn pushdown_is_automatic_only_when_the_body_translates() {
        let model = lower(
            r#"
            codeunit Poster {
                mark(rec) {
                    rec.status = "Posted";
                }
                @pushdown
                load(rec) {
                    const item = Item.get(rec.itemNo);
                }
            }
            "#,
        )
        .expect("lowering succeeds");
        let unit = &model.codeunits[0];
        assert!(unit.methods[0].is_pushdown, "a literal assignment runs in PostgreSQL");
        assert!(!unit.methods[1].is_pushdown, "a get stays on QuickJS");
        assert!(model.diagnostics.iter().any(|d| d.code == "BOS3301"));
    }

    #[test]
    fn sift_index_uses_the_equality_filter() {
        let model = lower(
            r#"
            table Customer {
                no: String(20);
                balance: Decimal flowField(sum(CustLedgerEntry.amount where (customerNo == rec.no)));
                loose: Decimal flowField(sum(CustLedgerEntry.amount where (true)));
            }
            "#,
        )
        .expect("lowering succeeds");
        let table = &model.tables[0];
        let balance = table.flow_fields.iter().find(|f| f.name == "balance").expect("balance");
        assert_eq!(balance.index_columns, vec!["customer_no".to_string()]);
        assert!(balance.sql_subquery.contains("\"customer_no\""));
        let loose = table.flow_fields.iter().find(|f| f.name == "loose").expect("loose");
        assert!(loose.index_columns.is_empty());
        assert!(model.diagnostics.iter().any(|d| d.code == "BOS3101"));
    }

    #[test]
    fn formula_check_copy_series_and_when_lower_to_sql_and_scripts() {
        let model = lower(
            r#"
            table SalesLine {
                no: String(20) { series = "SO"; };
                customerNo: String(20) { copyFrom = [Customer.name, Customer.paymentTermsCode]; };
                quantity: Decimal { check = quantity >= 0; };
                unitPrice: Decimal;
                lineAmount: Decimal { formula = quantity * unitPrice; };

                onInsert(rec, ctx) {
                    rec.lineAmount = rec.unitPrice * rec.quantity;
                }

                onModify(rec, xRec) when status {
                    rec.changed = true;
                }
            }
            "#,
        )
        .expect("lowering succeeds");

        let table = &model.tables[0];
        let quantity = table.columns.iter().find(|c| c.name == "quantity").expect("quantity");
        assert_eq!(quantity.check_constraint.as_deref(), Some("(\"quantity\" >= 0)"));
        let line_amount = table.columns.iter().find(|c| c.name == "lineAmount").expect("lineAmount");
        assert_eq!(
            line_amount.formula.as_deref(),
            Some("(\"quantity\" * \"unit_price\")")
        );

        let script = |name: &str| {
            table
                .triggers
                .iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("missing trigger {name}"))
                .script
                .clone()
                .unwrap_or_default()
        };
        let series = script("beforeInsert");
        assert!(series.contains("ctx.call(\"NumberSeriesManagement\", \"getNext\", \"SO\")"), "{series}");
        let copied = script("onValidatecustomerNo");
        assert!(copied.contains("Customer.get(rec.customerNo)"), "{copied}");
        assert!(copied.contains("rec.paymentTermsCode = __bosSrc.paymentTermsCode"), "{copied}");
        let insert = script("onInsert");
        assert!(
            insert.contains("ctx.decimal(rec.unitPrice).mul(rec.quantity).toString()"),
            "{insert}"
        );
        let modify = script("onModify");
        assert!(modify.contains("String(rec.status)"), "{modify}");
        assert!(modify.contains("rec.changed = true"), "{modify}");
    }

    #[test]
    fn interface_bound_enum_and_fieldgroup_diagnostics_and_decimal_cmp() {
        let model = lower(
            r#"
            interface ITaxCalculator {
                calculate(amount: Decimal): Decimal;
            }
            codeunit Incomplete implements ITaxCalculator {
                helper() {}
            }
            codeunit CompleteTax implements ITaxCalculator {
                calculate(amount: Decimal): Decimal {
                    return amount;
                }
            }
            enum TaxStrategy implements ITaxCalculator {
                Standard = CompleteTax;
                Broken = Incomplete;
                Missing = NonExistentCu;
            }
            table Item {
                no: String(20);
                name: String(100);
                unitPrice: Decimal;
                primaryKey(no);
                fieldgroup DropDown(no, name);
                fieldgroup Brick(no, unknownCol);
                onInsert(rec, ctx) {
                    if (rec.unitPrice > 100) {
                        rec.name = "Premium";
                    }
                }
            }
            "#,
        )
        .expect("lowering succeeds");

        assert!(model.diagnostics.iter().any(|d| d.code == "BOS2008"), "expected BOS2008 for unknown fieldgroup column");
        assert!(model.diagnostics.iter().any(|d| d.code == "BOS2014"), "expected BOS2014 for incomplete codeunit");
        assert!(model.diagnostics.iter().any(|d| d.code == "BOS2015"), "expected BOS2015 for enum binding incomplete codeunit");
        assert!(model.diagnostics.iter().any(|d| d.code == "BOS2016"), "expected BOS2016 for enum binding missing codeunit");
        assert_eq!(model.tables[0].field_groups.len(), 2);
        let insert_script = model.tables[0].triggers[0].script.as_deref().unwrap_or("");
        assert!(
            insert_script.contains("ctx.decimal(rec.unitPrice).gt(100)"),
            "expected decimal comparison rewrite, got: {insert_script}"
        );
    }

    #[test]
    fn compact_page_and_menu_syntax_lowering() {
        let model = lower(
            r#"
            page CustomerList {
                type = List;
                source = Customer;
                caption = "Customers";
                editable = false;
                cardPageId = "CustomerCard";
                views {
                    view Active {
                        caption = "Active Customers";
                        where = "blocked == false";
                        orderBy = "name asc";
                        default = true;
                    }
                }
                layout {
                    repeater(Rows) {
                        no;
                        name { importance = Promoted; }
                        balance { drillDown = "CustLedgerEntries"; }
                    }
                }
                actions {
                    action Post {
                        caption = "Post Order";
                        primary = true;
                        shortcut = "F9";
                        confirm = "Post this order?";
                        showWhen = status == "Released";
                        run = "SalesPost.run";
                    }
                }
            }
            menu SalesNavigation {
                label = "Sales";
                role = "SalesManager";
                group Orders {
                    label = "Order Processing";
                    item Customers {
                        label = "Customers";
                        page = "CustomerList";
                        icon = "users";
                        badge = "openCount";
                    }
                }
            }
            "#,
        )
        .expect("compact page and menu lowering succeeds");

        assert_eq!(model.pages.len(), 1);
        let page = &model.pages[0];
        assert_eq!(page.source_table.as_deref(), Some("Customer"));
        assert_eq!(page.editable, Some(false));
        assert_eq!(page.views.len(), 1);
        assert_eq!(page.views[0].name, "Active");
        assert!(page.views[0].is_default);
        assert_eq!(page.actions.len(), 1);
        assert!(page.actions[0].primary);
        assert_eq!(page.actions[0].shortcut.as_deref(), Some("F9"));
        assert_eq!(page.actions[0].run_codeunit.as_deref(), Some("SalesPost.run"));

        assert_eq!(model.menus.len(), 1);
        let menu = &model.menus[0];
        assert_eq!(menu.name, "SalesNavigation");
        assert_eq!(menu.role.as_deref(), Some("SalesManager"));
        assert_eq!(menu.items.len(), 1);
        assert_eq!(menu.items[0].items.len(), 1);
        assert_eq!(menu.items[0].items[0].page.as_deref(), Some("CustomerList"));
    }
}


