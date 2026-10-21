//! Schema synchronization: applies a package's declared tables to the live database in one
//! transaction.
//!
//! 1. Takes a cluster-wide advisory lock so concurrent deployments serialize.
//! 2. Renames tables/columns declared with `renamedFrom` in place.
//! 3. Diffs every physical copy of every table against its declaration and runs the ALTER plan;
//!    destructive steps (dropping fields, narrowing types) are refused unless explicitly allowed.
//! 4. Runs the package's idempotent DDL (indexes, views, RLS, triggers, procedures).
//! 5. Installs `relation` fields as real foreign keys and removes relations no longer declared.
//! 6. Journals the result in `_bos_schema_migrations`.
//!
//! Any failure rolls back the whole deployment. A dry run executes everything and rolls back, so
//! its report is exactly what a real run would do.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

use bosc_codegen_db::migration::{
    LiveColumn, LiveTable, MigrationStep, StepRisk, plan_table_changes,
};
use bosc_codegen_db::{SCHEMA_SUPPORT_DDL, relation_constraint_name};
use bosc_ir::{ColumnIR, TableExtensionIR, TableIR};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use crate::RuntimeError;

const SYNC_LOCK_KEY: &str = "bos.schema_sync";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaSyncOptions {
    /// Permit steps that can lose data (drop fields, narrow types, replace stored values).
    #[serde(default)]
    pub allow_destructive: bool,
    /// Execute and report, then roll everything back.
    #[serde(default)]
    pub dry_run: bool,
}

/// Everything one deployment wants the database to look like.
#[derive(Debug, Clone, Default)]
pub struct SchemaSyncRequest {
    pub package_name: String,
    /// Where the deployment came from (`cli`, `compile`, `register`, `extension`).
    pub source: String,
    pub tables: Vec<TableIR>,
    pub extensions: Vec<TableExtensionIR>,
    /// Idempotent DDL generated for the package (structures, indexes, views, procedures).
    pub ddl: String,
    pub applied_by: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationStatus {
    pub table: String,
    pub column: String,
    pub target: String,
    pub on_delete: String,
    /// `created`, `unchanged`, `created_not_validated` (existing rows violate it) or `dropped`.
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaSyncReport {
    pub package_name: String,
    pub applied: bool,
    pub dry_run: bool,
    pub steps: Vec<MigrationStep>,
    /// Destructive steps that were refused because `allow_destructive` was off.
    pub blocked: Vec<MigrationStep>,
    pub relations: Vec<RelationStatus>,
    pub migration_id: Option<Uuid>,
}

impl SchemaSyncReport {
    pub fn has_destructive_steps(&self) -> bool {
        self.steps.iter().any(|s| s.risk == StepRisk::Destructive)
    }
}

pub struct SchemaSync;

impl SchemaSync {
    /// Install the migration journal and the relation installer (idempotent).
    pub async fn ensure_support(pool: &PgPool) -> Result<(), RuntimeError> {
        sqlx::raw_sql(SCHEMA_SUPPORT_DDL).execute(pool).await?;
        Ok(())
    }

    pub async fn sync(
        pool: &PgPool,
        request: &SchemaSyncRequest,
        options: &SchemaSyncOptions,
    ) -> Result<SchemaSyncReport, RuntimeError> {
        let mut tx = pool.begin().await?;
        let mut report = SchemaSyncReport {
            package_name: request.package_name.clone(),
            dry_run: options.dry_run,
            ..Default::default()
        };

        let outcome = Self::run(&mut tx, request, options, &mut report).await;
        if let Err(err) = outcome {
            tx.rollback().await?;
            return Err(err);
        }

        if !report.blocked.is_empty() {
            tx.rollback().await?;
            if options.dry_run {
                return Ok(report);
            }
            let list = report
                .blocked
                .iter()
                .map(|s| format!("{}: {}", s.table, s.description))
                .collect::<Vec<_>>()
                .join("; ");
            return Err(RuntimeError::SchemaSyncBlocked(format!(
                "these changes would delete or truncate data and need an explicit allow-destructive deployment: {list}"
            )));
        }

        if options.dry_run {
            tx.rollback().await?;
            return Ok(report);
        }

        let steps_json = serde_json::to_value(&report.steps)
            .map_err(|e| RuntimeError::Internal(e.to_string()))?;
        let relations_json = serde_json::to_value(&report.relations)
            .map_err(|e| RuntimeError::Internal(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(request.ddl.as_bytes());
        hasher.update(steps_json.to_string().as_bytes());
        let checksum = format!("{:x}", hasher.finalize());
        let migration_id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO "_bos_schema_migrations"
                   ("package_name", "source", "checksum", "steps", "relations", "destructive", "applied_by")
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING "migration_id""#,
        )
        .bind(&request.package_name)
        .bind(&request.source)
        .bind(&checksum)
        .bind(&steps_json)
        .bind(&relations_json)
        .bind(report.has_destructive_steps())
        .bind(request.applied_by)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        report.migration_id = Some(migration_id);
        report.applied = true;
        Ok(report)
    }

    fn run<'a>(
        conn: &'a mut PgConnection,
        request: &'a SchemaSyncRequest,
        options: &'a SchemaSyncOptions,
        report: &'a mut SchemaSyncReport,
    ) -> BoxFuture<'a, Result<(), RuntimeError>> {
        Box::pin(Self::run_inner(conn, request, options, report))
    }

    async fn run_inner(
        conn: &mut PgConnection,
        request: &SchemaSyncRequest,
        options: &SchemaSyncOptions,
        report: &mut SchemaSyncReport,
    ) -> Result<(), RuntimeError> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(SYNC_LOCK_KEY)
            .execute(&mut *conn)
            .await?;
        exec_raw(&mut *conn, SCHEMA_SUPPORT_DDL).await?;

        let catalog_extensions = load_catalog_extensions(&mut *conn, &request.extensions).await?;

        for table in &request.tables {
            if let Some(old) = &table.renamed_from {
                rename_table(&mut *conn, old, &table.sql_table_name, report).await?;
            }

            let columns = declared_columns(table, &request.extensions, &catalog_extensions);
            for live in discover(&mut *conn, &table.sql_table_name).await? {
                let steps = plan_table_changes(&live, table, &columns);
                if steps.is_empty() {
                    continue;
                }
                let (allowed, blocked): (Vec<_>, Vec<_>) = steps
                    .into_iter()
                    .partition(|s| s.risk == StepRisk::Safe || options.allow_destructive);
                report.blocked.extend(blocked);
                if !report.blocked.is_empty() {
                    continue;
                }
                let drop_view = format!(
                    "DROP VIEW IF EXISTS {}.{};",
                    quote(&live.schema),
                    quote(&format!("{}_view", live.name))
                );
                exec_raw(&mut *conn, &drop_view).await?;
                for step in allowed {
                    exec_raw(&mut *conn, &step.sql).await.map_err(|e| {
                        RuntimeError::Internal(format!(
                            "Schema change failed on {} ({}): {e}",
                            step.table, step.description
                        ))
                    })?;
                    report.steps.push(step);
                }
            }
        }

        if !report.blocked.is_empty() {
            return Ok(());
        }

        let structure_ddl = without_relation_statements(&request.ddl);
        if !structure_ddl.trim().is_empty() {
            exec_raw(&mut *conn, &structure_ddl).await.map_err(|e| {
                RuntimeError::Internal(format!("Applying the package structure failed: {e}"))
            })?;
        }

        for table in &request.tables {
            let columns = declared_columns(table, &request.extensions, &catalog_extensions);
            sync_relations(&mut *conn, &table.sql_table_name, &columns, true, report).await?;
        }
        let mut handled: HashSet<String> = request
            .tables
            .iter()
            .map(|t| t.sql_table_name.clone())
            .collect();
        for ext in &request.extensions {
            let target = crate::to_snake_case(&ext.target_table);
            if !handled.insert(target.clone()) {
                continue;
            }
            match load_catalog_table(&mut *conn, &target).await? {
                Some(base) => {
                    let columns = declared_columns(&base, &request.extensions, &catalog_extensions);
                    sync_relations(&mut *conn, &target, &columns, true, report).await?;
                }
                None => {
                    // The base table's full declaration is unknown, so only add the extension's
                    // relations and leave every other constraint on the table untouched.
                    let columns: Vec<ColumnIR> = request
                        .extensions
                        .iter()
                        .chain(catalog_extensions.iter())
                        .filter(|e| crate::to_snake_case(&e.target_table) == target)
                        .flat_map(|e| e.added_columns.iter().cloned())
                        .collect();
                    sync_relations(&mut *conn, &target, &columns, false, report).await?;
                }
            }
        }
        Ok(())
    }
}

/// Relations are installed by [`sync_relations`] on every physical copy of a table, so the
/// generator's per-strategy `bos_ensure_relation` statements are not executed a second time.
fn without_relation_statements(ddl: &str) -> String {
    ddl.lines()
        .filter(|line| !line.trim_start().starts_with("SELECT bos_ensure_relation("))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Table fields plus every extension field targeting the table (from this deployment and the catalog).
fn declared_columns(
    table: &TableIR,
    request_extensions: &[TableExtensionIR],
    catalog_extensions: &[TableExtensionIR],
) -> Vec<ColumnIR> {
    let mut columns = table.columns.clone();
    let mut seen: HashSet<String> = columns.iter().map(|c| c.sql_name.clone()).collect();
    let request_names: HashSet<&str> = request_extensions.iter().map(|e| e.name.as_str()).collect();
    let extensions = request_extensions.iter().chain(
        catalog_extensions
            .iter()
            .filter(|e| !request_names.contains(e.name.as_str())),
    );
    for ext in extensions {
        if crate::to_snake_case(&ext.target_table) != table.sql_table_name {
            continue;
        }
        for col in &ext.added_columns {
            if seen.insert(col.sql_name.clone()) {
                let mut col = col.clone();
                if col.renamed_from.is_none() && col.name != col.sql_name {
                    col.renamed_from = Some(col.name.clone());
                }
                columns.push(col);
            }
        }
    }
    columns
}

async fn load_catalog_extensions(
    conn: &mut PgConnection,
    request_extensions: &[TableExtensionIR],
) -> Result<Vec<TableExtensionIR>, RuntimeError> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT to_regclass('_bos_meta_objects')::text")
            .fetch_one(&mut *conn)
            .await?;
    if exists.is_none() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        r#"SELECT "object_name", "definition" FROM "_bos_meta_objects"
           WHERE "object_type" = 'tableextension' AND "status" = 'active'"#,
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::new();
    for row in rows {
        let name: String = row.try_get("object_name")?;
        if request_extensions.iter().any(|e| e.name == name) {
            continue;
        }
        let definition: serde_json::Value = row.try_get("definition")?;
        let ext: TableExtensionIR = serde_json::from_value(definition).map_err(|e| {
            RuntimeError::Internal(format!(
                "Catalog table extension '{name}' has an unreadable definition: {e}"
            ))
        })?;
        out.push(ext);
    }
    Ok(out)
}

async fn load_catalog_table(
    conn: &mut PgConnection,
    sql_table: &str,
) -> Result<Option<TableIR>, RuntimeError> {
    let row = sqlx::query(
        r#"SELECT "definition" FROM "_bos_meta_objects"
           WHERE "object_type" = 'table'
             AND (LOWER("definition"->>'sql_table_name') = $1 OR LOWER("object_name") = $1)
           LIMIT 1"#,
    )
    .bind(sql_table)
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else { return Ok(None) };
    let definition: serde_json::Value = row.try_get("definition")?;
    Ok(serde_json::from_value::<TableIR>(definition).ok())
}

/// Every physical copy of a logical table: `schema.table` in any schema plus `company$table` copies.
async fn discover(
    conn: &mut PgConnection,
    sql_table: &str,
) -> Result<Vec<LiveTable>, RuntimeError> {
    let rows = sqlx::query(
        r#"SELECT n.nspname::text AS schema_name, c.relname::text AS table_name, c.relkind = 'p' AS is_partitioned
           FROM pg_class c
           JOIN pg_namespace n ON n.oid = c.relnamespace
           WHERE c.relkind IN ('r', 'p')
             AND NOT c.relispartition
             AND n.nspname NOT IN ('pg_catalog', 'information_schema')
             AND n.nspname NOT LIKE 'pg\_%'
             AND (c.relname = $1 OR right(c.relname, length($1) + 1) = '$' || $1)
           ORDER BY n.nspname, c.relname"#,
    )
    .bind(sql_table)
    .fetch_all(&mut *conn)
    .await?;

    let mut tables = Vec::with_capacity(rows.len());
    for row in rows {
        let schema: String = row.try_get("schema_name")?;
        let name: String = row.try_get("table_name")?;
        let is_partitioned: bool = row.try_get("is_partitioned")?;
        let columns = introspect_columns(&mut *conn, &schema, &name).await?;
        tables.push(LiveTable {
            schema,
            name,
            is_partitioned,
            columns,
        });
    }
    Ok(tables)
}

async fn introspect_columns(
    conn: &mut PgConnection,
    schema: &str,
    table: &str,
) -> Result<Vec<LiveColumn>, RuntimeError> {
    let rows = sqlx::query(
        r#"SELECT a.attname::text AS name,
                  format_type(a.atttypid, a.atttypmod) AS sql_type,
                  NOT a.attnotnull AS is_nullable,
                  CASE WHEN a.attgenerated = '' THEN pg_get_expr(d.adbin, d.adrelid) END AS default_expr,
                  CASE WHEN a.attgenerated <> '' THEN pg_get_expr(d.adbin, d.adrelid) END AS generated_expr,
                  EXISTS (
                      SELECT 1 FROM pg_index i
                      WHERE i.indrelid = a.attrelid AND i.indisunique AND i.indpred IS NULL
                        AND i.indnkeyatts = 1 AND i.indkey[0] = a.attnum
                  ) AS is_unique,
                  (SELECT con.conname::text FROM pg_constraint con
                   WHERE con.conrelid = a.attrelid AND con.contype = 'u' AND con.conkey = ARRAY[a.attnum]
                   ORDER BY con.conname LIMIT 1) AS unique_constraint
           FROM pg_attribute a
           LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum
           WHERE a.attrelid = to_regclass(format('%I.%I', $1::text, $2::text))
             AND a.attnum > 0 AND NOT a.attisdropped
           ORDER BY a.attnum"#,
    )
    .bind(schema)
    .bind(table)
    .fetch_all(&mut *conn)
    .await?;

    rows.iter()
        .map(|row| {
            Ok(LiveColumn {
                name: row.try_get("name")?,
                sql_type: row.try_get("sql_type")?,
                is_nullable: row.try_get("is_nullable")?,
                default_expr: row.try_get("default_expr")?,
                generated_expr: row.try_get("generated_expr")?,
                is_unique: row.try_get("is_unique")?,
                unique_constraint: row.try_get("unique_constraint")?,
            })
        })
        .collect()
}

/// Rename every physical copy of `old` (and its partitions, view and RLS policy) to `new`.
async fn rename_table(
    conn: &mut PgConnection,
    old: &str,
    new: &str,
    report: &mut SchemaSyncReport,
) -> Result<(), RuntimeError> {
    if !discover(&mut *conn, new).await?.is_empty() {
        return Ok(());
    }
    for live in discover(&mut *conn, old).await? {
        let prefix = &live.name[..live.name.len() - old.len()];
        let renamed = format!("{prefix}{new}");
        let schema = quote(&live.schema);
        let mut sql = format!(
            "DROP VIEW IF EXISTS {schema}.{};\nALTER TABLE {} RENAME TO {};\n",
            quote(&format!("{}_view", live.name)),
            live.qualified(),
            quote(&renamed)
        );

        let partitions: Vec<String> = sqlx::query_scalar(
            r#"SELECT c.relname::text FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid
               WHERE i.inhparent = to_regclass(format('%I.%I', $1::text, $2::text))"#,
        )
        .bind(&live.schema)
        .bind(&live.name)
        .fetch_all(&mut *conn)
        .await?;
        for part in partitions {
            if let Some(rest) = part.strip_prefix(&format!("{}_", live.name)) {
                sql.push_str(&format!(
                    "ALTER TABLE {schema}.{} RENAME TO {};\n",
                    quote(&part),
                    quote(&format!("{renamed}_{rest}"))
                ));
            }
        }
        sql.push_str(&format!(
            "DROP POLICY IF EXISTS {} ON {schema}.{};\n",
            quote(&format!("tenant_isolation_{}", live.name)),
            quote(&renamed)
        ));

        exec_raw(&mut *conn, &sql).await.map_err(|e| {
            RuntimeError::Internal(format!(
                "Renaming table {}.{} to {renamed} failed: {e}",
                live.schema, live.name
            ))
        })?;
        report.steps.push(MigrationStep {
            table: format!("{}.{}", live.schema, live.name),
            action: "rename_table".to_string(),
            description: format!("Rename table '{}' to '{}' (data kept)", live.name, renamed),
            sql,
            risk: StepRisk::Safe,
        });
    }
    Ok(())
}

/// Install declared relations on every physical copy and drop BOS relations no longer declared.
async fn sync_relations(
    conn: &mut PgConnection,
    sql_table: &str,
    columns: &[ColumnIR],
    drop_stale: bool,
    report: &mut SchemaSyncReport,
) -> Result<(), RuntimeError> {
    let declared: HashMap<String, &ColumnIR> = columns
        .iter()
        .filter(|c| c.foreign_key.is_some())
        .map(|c| (relation_constraint_name(sql_table, &c.sql_name), c))
        .collect();

    for live in discover(&mut *conn, sql_table).await? {
        let prefix = &live.name[..live.name.len() - sql_table.len()];
        for col in declared.values() {
            let Some(fk) = &col.foreign_key else { continue };
            let status: String =
                sqlx::query_scalar("SELECT bos_ensure_relation($1, $2, $3, $4, $5, $6, $7)")
                    .bind(&live.schema)
                    .bind(sql_table)
                    .bind(&col.sql_name)
                    .bind(&fk.target_table)
                    .bind(&fk.target_column)
                    .bind(&fk.on_delete)
                    .bind(prefix)
                    .fetch_one(&mut *conn)
                    .await
                    .map_err(|e| {
                        RuntimeError::Internal(format!(
                            "Installing relation {sql_table}.{} failed: {e}",
                            col.sql_name
                        ))
                    })?;
            report.relations.push(RelationStatus {
                table: format!("{}.{}", live.schema, live.name),
                column: col.sql_name.clone(),
                target: format!("{}.{}", fk.target_table, fk.target_column),
                on_delete: fk.on_delete.clone(),
                status,
            });
        }

        if !drop_stale {
            continue;
        }
        let prefix_pattern = relation_constraint_name(sql_table, "");
        let existing: Vec<String> = sqlx::query_scalar(
            r#"SELECT conname::text FROM pg_constraint
               WHERE conrelid = to_regclass(format('%I.%I', $1::text, $2::text))
                 AND contype = 'f' AND left(conname, length($3)) = $3"#,
        )
        .bind(&live.schema)
        .bind(&live.name)
        .bind(&prefix_pattern)
        .fetch_all(&mut *conn)
        .await?;
        for name in existing {
            if declared.contains_key(&name) {
                continue;
            }
            let drop = format!(
                "ALTER TABLE {} DROP CONSTRAINT {};",
                live.qualified(),
                quote(&name)
            );
            exec_raw(&mut *conn, &drop).await?;
            report.relations.push(RelationStatus {
                table: format!("{}.{}", live.schema, live.name),
                column: name.trim_start_matches(&prefix_pattern).to_string(),
                target: String::new(),
                on_delete: String::new(),
                status: "dropped".to_string(),
            });
        }
    }
    Ok(())
}

/// Run a multi-statement script on the connection (simple query protocol).
fn exec_raw<'a>(
    conn: &'a mut PgConnection,
    sql: &'a str,
) -> BoxFuture<'a, Result<(), sqlx::Error>> {
    Box::pin(async move { sqlx::Executor::execute(conn, sql).await.map(|_| ()) })
}

fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bosc_ir::TableTypeIR;

    fn column(name: &str, sql_name: &str) -> ColumnIR {
        ColumnIR {
            name: name.to_string(),
            sql_name: sql_name.to_string(),
            sql_type: "TEXT".to_string(),
            data_type: Some("String".to_string()),
            is_nullable: true,
            is_unique: false,
            default_val: None,
            caption: name.to_string(),
            foreign_key: None,
            check_constraint: None,
            formula: None,
            renamed_from: None,
        }
    }

    #[allow(dead_code)]
    fn sync_future_is_send(
        pool: &PgPool,
        request: &SchemaSyncRequest,
        options: &SchemaSyncOptions,
    ) {
        fn assert_send<T: Send>(_: T) {}
        assert_send(SchemaSync::sync(pool, request, options));
    }

    #[test]
    fn relation_statements_are_left_to_the_synchronizer() {
        let ddl = "CREATE TABLE IF NOT EXISTS \"a\" (\"x\" TEXT);\n  SELECT bos_ensure_relation(current_schema(), 'a', 'x', 'b', 'no', 'RESTRICT', '');\nCREATE INDEX i ON a (x);";
        let kept = without_relation_statements(ddl);
        assert!(!kept.contains("bos_ensure_relation"));
        assert!(kept.contains("CREATE TABLE") && kept.contains("CREATE INDEX"));
    }

    #[test]
    fn declared_columns_merge_extensions_and_adopt_legacy_names() {
        let table = TableIR {
            name: "Customer".to_string(),
            sql_table_name: "customer".to_string(),
            caption: "Customer".to_string(),
            table_type: TableTypeIR::Normal,
            data_per_company: true,
            columns: vec![column("no", "no")],
            primary_key: vec!["no".to_string()],
            indexes: vec![],
            flow_fields: vec![],
            triggers: vec![],
            check_constraints: vec![],
            on_delete: None,
            renamed_from: None,
            field_groups: vec![],
        };
        let request_ext = TableExtensionIR {
            name: "Loyalty".to_string(),
            package_name: "acme".to_string(),
            target_table: "Customer".to_string(),
            added_columns: vec![column("LoyaltyPoints", "loyalty_points")],
            added_indexes: vec![],
        };
        let catalog_ext = TableExtensionIR {
            name: "Credit".to_string(),
            package_name: "fin".to_string(),
            target_table: "customer".to_string(),
            added_columns: vec![column("credit_limit", "credit_limit"), column("no", "no")],
            added_indexes: vec![],
        };
        let other_ext = TableExtensionIR {
            name: "Vendor".to_string(),
            package_name: "fin".to_string(),
            target_table: "vendor".to_string(),
            added_columns: vec![column("rating", "rating")],
            added_indexes: vec![],
        };

        let cols = declared_columns(&table, &[request_ext], &[catalog_ext, other_ext]);
        let names: Vec<&str> = cols.iter().map(|c| c.sql_name.as_str()).collect();
        assert_eq!(names, vec!["no", "loyalty_points", "credit_limit"]);
        assert_eq!(cols[1].renamed_from.as_deref(), Some("LoyaltyPoints"));
        assert_eq!(cols[2].renamed_from, None);
    }
}
