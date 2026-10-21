//! Schema diff planner: compares the live shape of a physical table with its declared [`TableIR`]
//! and produces ordered ALTER steps. Each step is classified as safe (no data can be lost) or
//! destructive (drops data or may truncate it); callers refuse destructive steps unless the
//! deployment explicitly allows them.

use bosc_ir::{ColumnIR, TableIR};
use serde::{Deserialize, Serialize};

/// A physical column as PostgreSQL reports it (`format_type`, `pg_get_expr`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveColumn {
    pub name: String,
    pub sql_type: String,
    pub is_nullable: bool,
    pub default_expr: Option<String>,
    pub generated_expr: Option<String>,
    /// A single-column unique index or constraint exists on this column.
    pub is_unique: bool,
    /// Name of the single-column UNIQUE constraint on this column, if any.
    pub unique_constraint: Option<String>,
}

/// One physical copy of a table (a company schema or a `company$table` copy share one definition).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveTable {
    pub schema: String,
    pub name: String,
    pub is_partitioned: bool,
    pub columns: Vec<LiveColumn>,
}

impl LiveTable {
    pub fn qualified(&self) -> String {
        format!("{}.{}", quote_ident(&self.schema), quote_ident(&self.name))
    }

    pub fn column(&self, name: &str) -> Option<&LiveColumn> {
        self.columns.iter().find(|c| c.name == name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepRisk {
    Safe,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Schema-qualified physical table the step changes.
    pub table: String,
    pub action: String,
    pub description: String,
    pub sql: String,
    pub risk: StepRisk,
}

impl MigrationStep {
    fn new(live: &LiveTable, action: &str, description: String, sql: String, risk: StepRisk) -> Self {
        Self {
            table: format!("{}.{}", live.schema, live.name),
            action: action.to_string(),
            description,
            sql,
            risk,
        }
    }
}

pub fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

/// True for runtime-owned columns (`_id`, `_tenant_id`, `_system_version`, ...) the planner never touches.
pub fn is_system_column(name: &str) -> bool {
    name.starts_with('_')
}

/// Canonical spelling of a SQL type so declared types (`VARCHAR(50)`, `DECIMAL(18, 2)`, `TIMESTAMPTZ`)
/// compare equal to what `format_type` reports (`character varying(50)`, `numeric(18,2)`, ...).
pub fn normalize_sql_type(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    let collapsed = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    let compact = collapsed.replace(" (", "(").replace("( ", "(").replace(" )", ")").replace(", ", ",").replace(" ,", ",");
    let (base, args) = match compact.find('(') {
        Some(pos) if compact.ends_with(')') => (compact[..pos].trim().to_string(), Some(compact[pos..].to_string())),
        _ => (compact.clone(), None),
    };
    let canonical = match base.as_str() {
        "varchar" | "character varying" => "character varying",
        "char" | "character" | "bpchar" => "character",
        "int" | "int4" | "integer" | "serial" | "serial4" => "integer",
        "int8" | "bigint" | "bigserial" | "serial8" => "bigint",
        "int2" | "smallint" | "smallserial" | "serial2" => "smallint",
        "decimal" | "numeric" => "numeric",
        "bool" | "boolean" => "boolean",
        "timestamptz" | "timestamp with time zone" => "timestamp with time zone",
        "timestamp" | "timestamp without time zone" => "timestamp without time zone",
        "timetz" | "time with time zone" => "time with time zone",
        "time" | "time without time zone" => "time without time zone",
        "float8" | "double precision" | "float" => "double precision",
        "float4" | "real" => "real",
        other => other,
    };
    match args {
        Some(a) => format!("{}{}", canonical, a),
        None => canonical.to_string(),
    }
}

fn type_args(normalized: &str) -> (&str, Vec<u32>) {
    match normalized.find('(') {
        Some(pos) => {
            let base = &normalized[..pos];
            let args = normalized[pos + 1..normalized.len().saturating_sub(1)]
                .split(',')
                .filter_map(|a| a.trim().parse::<u32>().ok())
                .collect();
            (base, args)
        }
        None => (normalized, Vec::new()),
    }
}

/// Whether every value of `from` converts to `to` without loss (so the change is safe).
pub fn is_lossless_widening(from: &str, to: &str) -> bool {
    let (from_base, from_args) = type_args(from);
    let (to_base, to_args) = type_args(to);
    let int_digits = |base: &str| match base {
        "smallint" => Some(5u32),
        "integer" => Some(10),
        "bigint" => Some(19),
        _ => None,
    };
    match (from_base, to_base) {
        ("character varying", "text") | ("text", "character varying") if to_args.is_empty() => true,
        ("character varying", "text") => true,
        ("character varying", "character varying") => match (from_args.first(), to_args.first()) {
            (_, None) => true,
            (Some(f), Some(t)) => t >= f,
            (None, Some(_)) => false,
        },
        ("character", "character varying") | ("character", "text") => match (from_args.first(), to_args.first()) {
            (_, None) => true,
            (Some(f), Some(t)) => t >= f,
            (None, Some(t)) => *t >= 1,
        },
        ("smallint", "integer") | ("smallint", "bigint") | ("integer", "bigint") => true,
        ("real", "double precision") => true,
        (f, "numeric") if int_digits(f).is_some() => match to_args.as_slice() {
            [] => true,
            [p] => *p >= int_digits(f).unwrap_or(u32::MAX),
            [p, s, ..] => p.saturating_sub(*s) >= int_digits(f).unwrap_or(u32::MAX),
        },
        ("numeric", "numeric") => match (from_args.as_slice(), to_args.as_slice()) {
            (_, []) => true,
            ([], _) => false,
            ([fp], [tp]) => tp >= fp,
            ([fp], [tp, ts, ..]) => tp.saturating_sub(*ts) >= *fp,
            ([fp, fs, ..], [tp]) => *fs == 0 && tp >= fp,
            ([fp, fs, ..], [tp, ts, ..]) => ts >= fs && tp.saturating_sub(*ts) >= fp.saturating_sub(*fs),
        },
        _ => false,
    }
}

/// Strip casts, quoting, parentheses and whitespace so PostgreSQL's rendering of a default or
/// generation expression compares equal to the declared one (`'Open'::character varying` ~ `'Open'`).
pub fn normalize_expr(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.trim().chars().peekable();
    let mut in_literal = false;
    while let Some(c) = chars.next() {
        if in_literal {
            out.push(c);
            if c == '\'' {
                if chars.peek() == Some(&'\'') {
                    out.push('\'');
                    chars.next();
                } else {
                    in_literal = false;
                }
            }
            continue;
        }
        match c {
            '\'' => {
                in_literal = true;
                out.push(c);
            }
            ':' if chars.peek() == Some(&':') => {
                chars.next();
                while let Some(&n) = chars.peek() {
                    if n.is_ascii_alphanumeric() || matches!(n, '_' | ' ' | '"' | '[' | ']') {
                        chars.next();
                    } else {
                        break;
                    }
                }
            }
            '"' | '(' | ')' => {}
            c if c.is_whitespace() => {}
            c => out.push(c.to_ascii_lowercase()),
        }
    }
    out
}

fn defaults_match(declared: Option<&str>, live: Option<&str>) -> bool {
    match (declared, live) {
        (None, None) => true,
        (Some(d), Some(l)) => normalize_expr(d) == normalize_expr(l),
        _ => false,
    }
}

/// Column definition used for `ADD COLUMN`.
pub fn column_definition(col: &ColumnIR) -> String {
    let mut def = format!("{} {}", quote_ident(&col.sql_name), col.sql_type);
    if let Some(expr) = &col.formula {
        def.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", expr));
        return def;
    }
    if let Some(d) = &col.default_val {
        def.push_str(&format!(" DEFAULT {}", d));
    }
    if !col.is_nullable {
        def.push_str(" NOT NULL");
    }
    def
}

/// Plan every change needed to bring `live` to the declared `columns` of `table`.
///
/// `columns` is the full declared column set (table fields plus table-extension fields); live
/// columns outside it and outside the runtime-owned `_` columns are dropped (destructive).
pub fn plan_table_changes(live: &LiveTable, table: &TableIR, columns: &[ColumnIR]) -> Vec<MigrationStep> {
    let target = live.qualified();
    let mut steps = Vec::new();
    let mut present: Vec<String> = live.columns.iter().map(|c| c.name.clone()).collect();

    for col in columns {
        let Some(old) = &col.renamed_from else { continue };
        if live.column(&col.sql_name).is_none() && present.iter().any(|p| p == old) {
            steps.push(MigrationStep::new(
                live,
                "rename_column",
                format!("Rename field '{}' to '{}' (data kept)", old, col.sql_name),
                format!("ALTER TABLE {} RENAME COLUMN {} TO {};", target, quote_ident(old), quote_ident(&col.sql_name)),
                StepRisk::Safe,
            ));
            present.retain(|p| p != old);
            present.push(col.sql_name.clone());
        }
    }

    let live_column = |col: &ColumnIR| -> Option<&LiveColumn> {
        live.column(&col.sql_name).or_else(|| {
            col.renamed_from
                .as_deref()
                .filter(|old| live.column(&col.sql_name).is_none() && live.column(old).is_some())
                .and_then(|old| live.column(old))
        })
    };

    let pk_columns: Vec<&str> = table.primary_key.iter().map(String::as_str).collect();

    for col in columns {
        if is_system_column(&col.sql_name) {
            continue;
        }
        let ident = quote_ident(&col.sql_name);
        let Some(existing) = live_column(col) else {
            steps.push(MigrationStep::new(
                live,
                "add_column",
                format!("Add field '{}' ({})", col.sql_name, col.sql_type),
                format!("ALTER TABLE {} ADD COLUMN {};", target, column_definition(col)),
                StepRisk::Safe,
            ));
            continue;
        };

        match (&col.formula, &existing.generated_expr) {
            (Some(expr), Some(live_expr)) => {
                if normalize_expr(expr) != normalize_expr(live_expr)
                    || normalize_sql_type(&col.sql_type) != normalize_sql_type(&existing.sql_type)
                {
                    steps.push(MigrationStep::new(
                        live,
                        "replace_formula",
                        format!("Recompute formula field '{}' as {}", col.sql_name, expr),
                        format!(
                            "ALTER TABLE {t} DROP COLUMN {c};\nALTER TABLE {t} ADD COLUMN {def};",
                            t = target,
                            c = ident,
                            def = column_definition(col)
                        ),
                        StepRisk::Safe,
                    ));
                }
                continue;
            }
            (Some(expr), None) => {
                steps.push(MigrationStep::new(
                    live,
                    "convert_to_formula",
                    format!("Replace stored values of '{}' with formula {}", col.sql_name, expr),
                    format!(
                        "ALTER TABLE {t} DROP COLUMN {c};\nALTER TABLE {t} ADD COLUMN {def};",
                        t = target,
                        c = ident,
                        def = column_definition(col)
                    ),
                    StepRisk::Destructive,
                ));
                continue;
            }
            (None, Some(_)) => {
                steps.push(MigrationStep::new(
                    live,
                    "drop_formula",
                    format!("Keep current values of '{}' and stop computing it", col.sql_name),
                    format!("ALTER TABLE {} ALTER COLUMN {} DROP EXPRESSION;", target, ident),
                    StepRisk::Safe,
                ));
            }
            (None, None) => {}
        }

        let declared_type = normalize_sql_type(&col.sql_type);
        let live_type = normalize_sql_type(&existing.sql_type);
        if declared_type != live_type {
            let risk = if is_lossless_widening(&live_type, &declared_type) {
                StepRisk::Safe
            } else {
                StepRisk::Destructive
            };
            steps.push(MigrationStep::new(
                live,
                "change_type",
                format!("Change type of '{}' from {} to {}", col.sql_name, existing.sql_type, col.sql_type),
                format!(
                    "ALTER TABLE {t} ALTER COLUMN {c} TYPE {ty} USING {c}::{ty};",
                    t = target,
                    c = ident,
                    ty = col.sql_type
                ),
                risk,
            ));
        }

        if !defaults_match(col.default_val.as_deref(), existing.default_expr.as_deref()) {
            let sql = match &col.default_val {
                Some(d) => format!("ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};", target, ident, d),
                None => format!("ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;", target, ident),
            };
            steps.push(MigrationStep::new(
                live,
                "change_default",
                format!("Set default of '{}' to {}", col.sql_name, col.default_val.as_deref().unwrap_or("none")),
                sql,
                StepRisk::Safe,
            ));
        }

        if col.is_nullable && !existing.is_nullable && !pk_columns.contains(&col.sql_name.as_str()) {
            steps.push(MigrationStep::new(
                live,
                "drop_not_null",
                format!("Allow '{}' to be empty", col.sql_name),
                format!("ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;", target, ident),
                StepRisk::Safe,
            ));
        } else if !col.is_nullable && existing.is_nullable {
            let mut sql = String::new();
            if col.default_val.is_some() {
                sql.push_str(&format!("UPDATE {t} SET {c} = DEFAULT WHERE {c} IS NULL;\n", t = target, c = ident));
            }
            sql.push_str(&format!("ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;", target, ident));
            steps.push(MigrationStep::new(
                live,
                "set_not_null",
                format!(
                    "Require '{}'{}",
                    col.sql_name,
                    if col.default_val.is_some() { " (empty rows get the default)" } else { "" }
                ),
                sql,
                StepRisk::Safe,
            ));
        }

        if col.is_unique && !existing.is_unique && !live.is_partitioned {
            let name = constraint_name(&format!("{}_{}_key", live.name, col.sql_name));
            steps.push(MigrationStep::new(
                live,
                "add_unique",
                format!("Make '{}' unique", col.sql_name),
                format!("ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});", target, quote_ident(&name), ident),
                StepRisk::Safe,
            ));
        } else if !col.is_unique && !pk_columns.contains(&col.sql_name.as_str()) {
            let default_name = constraint_name(&format!("{}_{}_key", live.name, existing.name));
            if existing.unique_constraint.as_deref() == Some(default_name.as_str()) {
                steps.push(MigrationStep::new(
                    live,
                    "drop_unique",
                    format!("Allow duplicate values in '{}'", col.sql_name),
                    format!("ALTER TABLE {} DROP CONSTRAINT {};", target, quote_ident(&default_name)),
                    StepRisk::Safe,
                ));
            }
        }
    }

    for name in &present {
        if is_system_column(name) || columns.iter().any(|c| &c.sql_name == name) {
            continue;
        }
        steps.push(MigrationStep::new(
            live,
            "drop_column",
            format!("Delete field '{}' and all of its data", name),
            format!("ALTER TABLE {} DROP COLUMN {};", target, quote_ident(name)),
            StepRisk::Destructive,
        ));
    }

    steps
}

fn constraint_name(full: &str) -> String {
    let mut end = full.len().min(63);
    while !full.is_char_boundary(end) {
        end -= 1;
    }
    full[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bosc_ir::{ForeignKeyIR, TableTypeIR};

    fn col(name: &str, ty: &str, nullable: bool) -> ColumnIR {
        ColumnIR {
            name: name.to_string(),
            sql_name: name.to_string(),
            sql_type: ty.to_string(),
            data_type: None,
            is_nullable: nullable,
            is_unique: false,
            default_val: None,
            caption: name.to_string(),
            foreign_key: None,
            check_constraint: None,
            formula: None,
            renamed_from: None,
        }
    }

    fn live_col(name: &str, ty: &str, nullable: bool) -> LiveColumn {
        LiveColumn {
            name: name.to_string(),
            sql_type: ty.to_string(),
            is_nullable: nullable,
            default_expr: None,
            generated_expr: None,
            is_unique: false,
            unique_constraint: None,
        }
    }

    fn table(columns: Vec<ColumnIR>) -> TableIR {
        TableIR {
            name: "Customer".to_string(),
            sql_table_name: "customer".to_string(),
            caption: "Customer".to_string(),
            table_type: TableTypeIR::Normal,
            data_per_company: true,
            columns,
            primary_key: vec!["no".to_string()],
            indexes: vec![],
            flow_fields: vec![],
            triggers: vec![],
            check_constraints: vec![],
            on_delete: None,
            renamed_from: None,
            field_groups: vec![],
        }
    }

    fn live(columns: Vec<LiveColumn>) -> LiveTable {
        let mut all = vec![live_col("_id", "uuid", false), live_col("_tenant_id", "uuid", false)];
        all.extend(columns);
        LiveTable { schema: "public".to_string(), name: "customer".to_string(), is_partitioned: false, columns: all }
    }

    #[test]
    fn type_normalization_matches_format_type() {
        assert_eq!(normalize_sql_type("VARCHAR(50)"), "character varying(50)");
        assert_eq!(normalize_sql_type("DECIMAL(18, 2)"), "numeric(18,2)");
        assert_eq!(normalize_sql_type("numeric(18,2)"), "numeric(18,2)");
        assert_eq!(normalize_sql_type("TIMESTAMPTZ"), "timestamp with time zone");
        assert_eq!(normalize_sql_type("INTEGER"), "integer");
        assert_eq!(normalize_sql_type("JSONB"), "jsonb");
    }

    #[test]
    fn widening_classification() {
        assert!(is_lossless_widening("character varying(20)", "character varying(50)"));
        assert!(!is_lossless_widening("character varying(50)", "character varying(20)"));
        assert!(is_lossless_widening("character varying(50)", "text"));
        assert!(!is_lossless_widening("text", "character varying(50)"));
        assert!(is_lossless_widening("integer", "bigint"));
        assert!(!is_lossless_widening("bigint", "integer"));
        assert!(is_lossless_widening("numeric(18,2)", "numeric(20,4)"));
        assert!(!is_lossless_widening("numeric(18,4)", "numeric(18,2)"));
        assert!(is_lossless_widening("integer", "numeric(18,2)"));
        assert!(!is_lossless_widening("integer", "numeric(8,2)"));
        assert!(!is_lossless_widening("text", "integer"));
    }

    #[test]
    fn expression_normalization_ignores_casts_and_quoting() {
        assert_eq!(normalize_expr("'Open'::character varying"), normalize_expr("'Open'"));
        assert_eq!(normalize_expr("(quantity * unit_price)"), normalize_expr("\"quantity\" * \"unit_price\""));
        assert_ne!(normalize_expr("'Open'"), normalize_expr("'Closed'"));
        assert_eq!(normalize_expr("now()"), normalize_expr("NOW()"));
    }

    #[test]
    fn unchanged_table_produces_no_steps() {
        let mut name = col("name", "VARCHAR(100)", true);
        name.default_val = Some("'New'".to_string());
        let mut live_name = live_col("name", "character varying(100)", true);
        live_name.default_expr = Some("'New'::character varying".to_string());
        let t = table(vec![col("no", "VARCHAR(20)", false), name]);
        let l = live(vec![live_col("no", "character varying(20)", false), live_name]);
        assert!(plan_table_changes(&l, &t, &t.columns).is_empty());
    }

    #[test]
    fn add_widen_and_drop_are_classified() {
        let t = table(vec![col("no", "VARCHAR(20)", false), col("name", "VARCHAR(200)", true), col("email", "TEXT", true)]);
        let l = live(vec![
            live_col("no", "character varying(20)", false),
            live_col("name", "character varying(100)", true),
            live_col("fax", "text", true),
        ]);
        let steps = plan_table_changes(&l, &t, &t.columns);
        let find = |action: &str| steps.iter().find(|s| s.action == action).cloned();
        let widen = find("change_type").expect("type change planned");
        assert_eq!(widen.risk, StepRisk::Safe);
        assert!(widen.sql.contains("TYPE VARCHAR(200)"));
        let add = find("add_column").expect("add planned");
        assert_eq!(add.risk, StepRisk::Safe);
        assert!(add.sql.contains("ADD COLUMN \"email\" TEXT"));
        let drop = find("drop_column").expect("drop planned");
        assert_eq!(drop.risk, StepRisk::Destructive);
        assert!(drop.sql.contains("DROP COLUMN \"fax\""));
    }

    #[test]
    fn narrowing_is_destructive() {
        let t = table(vec![col("no", "VARCHAR(10)", false)]);
        let l = live(vec![live_col("no", "character varying(20)", false)]);
        let steps = plan_table_changes(&l, &t, &t.columns);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].risk, StepRisk::Destructive);
    }

    #[test]
    fn rename_keeps_data_instead_of_drop_and_add() {
        let mut renamed = col("customer_name", "VARCHAR(100)", true);
        renamed.renamed_from = Some("name".to_string());
        let t = table(vec![col("no", "VARCHAR(20)", false), renamed]);
        let l = live(vec![live_col("no", "character varying(20)", false), live_col("name", "character varying(100)", true)]);
        let steps = plan_table_changes(&l, &t, &t.columns);
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert_eq!(steps[0].action, "rename_column");
        assert_eq!(steps[0].risk, StepRisk::Safe);
        assert!(steps[0].sql.contains("RENAME COLUMN \"name\" TO \"customer_name\""));
    }

    #[test]
    fn not_null_with_default_backfills_first() {
        let mut status = col("status", "VARCHAR(20)", false);
        status.default_val = Some("'Open'".to_string());
        let t = table(vec![col("no", "VARCHAR(20)", false), status]);
        let mut live_status = live_col("status", "character varying(20)", true);
        live_status.default_expr = Some("'Open'::character varying".to_string());
        let l = live(vec![live_col("no", "character varying(20)", false), live_status]);
        let steps = plan_table_changes(&l, &t, &t.columns);
        assert_eq!(steps.len(), 1, "{steps:?}");
        assert!(steps[0].sql.starts_with("UPDATE \"public\".\"customer\" SET \"status\" = DEFAULT WHERE \"status\" IS NULL;"));
        assert!(steps[0].sql.ends_with("SET NOT NULL;"));
    }

    #[test]
    fn system_columns_are_never_dropped() {
        let t = table(vec![col("no", "VARCHAR(20)", false)]);
        let mut l = live(vec![live_col("no", "character varying(20)", false)]);
        l.columns.push(live_col("_system_version", "bigint", false));
        assert!(plan_table_changes(&l, &t, &t.columns).is_empty());
    }

    #[test]
    fn foreign_key_columns_plan_like_other_columns() {
        let mut customer_no = col("customer_no", "VARCHAR(20)", true);
        customer_no.foreign_key = Some(ForeignKeyIR {
            target_table: "customer".to_string(),
            target_column: "no".to_string(),
            on_delete: "RESTRICT".to_string(),
        });
        let t = table(vec![col("no", "VARCHAR(20)", false), customer_no]);
        let l = live(vec![live_col("no", "character varying(20)", false)]);
        let steps = plan_table_changes(&l, &t, &t.columns);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].action, "add_column");
    }
}
