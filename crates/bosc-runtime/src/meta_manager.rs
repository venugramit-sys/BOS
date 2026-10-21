use bosc_analyzer::SemanticAnalyzer;
use bosc_codegen_db::PostgresGenerator;
use bosc_ir::{
    ColumnIR, ForeignKeyIR, IndexIR, StateMachineIR, TableIR, TableTypeIR, sql_type_to_bos_type,
};
use bosc_syntax::{Lexer, Parser};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::RuntimeError;
use crate::schema_sync::{SchemaSync, SchemaSyncOptions, SchemaSyncReport, SchemaSyncRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProvisionStatus {
    pub is_provisioned: bool,
    pub kernel_version: Option<String>,
    pub schema_version: Option<String>,
    pub status: Option<String>,
    pub storage_strategy: Option<String>,
    pub provisioned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub meta_objects_count: i64,
    pub tenants_count: i64,
    pub companies_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: String,
    #[serde(default)]
    pub data_type: Option<String>,
    pub is_nullable: Option<bool>,
    pub is_unique: Option<bool>,
    pub default_val: Option<String>,
    pub caption: Option<String>,
    pub foreign_key_table: Option<String>,
    pub foreign_key_column: Option<String>,
    /// `restrict` (default), `cascade`, `setNull` or `noAction`.
    #[serde(default)]
    pub foreign_key_on_delete: Option<String>,
    pub check_constraint: Option<String>,
    pub formula: Option<String>,
    /// Previous name of this field; the column is renamed in place and keeps its data.
    #[serde(default)]
    pub renamed_from: Option<String>,
}

impl ColumnDefinition {
    fn to_column_ir(&self) -> Result<ColumnIR, RuntimeError> {
        let sql_name = crate::to_snake_case(&self.name);
        let is_nullable = self.is_nullable.unwrap_or(true);
        let foreign_key = match (&self.foreign_key_table, &self.foreign_key_column) {
            (Some(table), Some(column)) => {
                let raw = self.foreign_key_on_delete.as_deref().unwrap_or("restrict");
                let action = ForeignKeyIR::normalize_on_delete(raw).ok_or_else(|| {
                    RuntimeError::Internal(format!(
                        "Field '{}': onDelete '{raw}' is not one of restrict, cascade, setNull, noAction",
                        self.name
                    ))
                })?;
                if action == "SET NULL" && !is_nullable {
                    return Err(RuntimeError::Internal(format!(
                        "Field '{}': onDelete setNull needs a field that may be empty",
                        self.name
                    )));
                }
                Some(ForeignKeyIR {
                    target_table: crate::to_snake_case(table),
                    target_column: crate::to_snake_case(column),
                    on_delete: action.to_string(),
                })
            }
            (None, None) => None,
            _ => {
                return Err(RuntimeError::Internal(format!(
                    "Field '{}': a relation needs both foreign_key_table and foreign_key_column",
                    self.name
                )));
            }
        };
        let data_type = self
            .data_type
            .clone()
            .or_else(|| Some(sql_type_to_bos_type(&self.sql_type)));
        Ok(ColumnIR {
            name: self.name.clone(),
            sql_name: sql_name.clone(),
            sql_type: self.sql_type.clone(),
            data_type,
            is_nullable,
            is_unique: self.is_unique.unwrap_or(false),
            default_val: self.default_val.clone(),
            caption: self.caption.clone().unwrap_or_else(|| self.name.clone()),
            foreign_key,
            check_constraint: self.check_constraint.clone(),
            formula: self.formula.clone(),
            renamed_from: self
                .renamed_from
                .as_deref()
                .map(crate::to_snake_case)
                .filter(|old| *old != sql_name),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachineDefinition {
    pub name: String,
    pub initial_state: String,
    pub transitions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterMetaObjectRequest {
    pub object_type: String, // "table", "statemachine", "codeunit", "enum", "page", etc.
    pub name: String,
    pub package_name: Option<String>,
    pub version: Option<String>,
    pub caption: Option<String>,
    #[serde(alias = "table_type")]
    pub sub_type: Option<String>, // "Normal", "Document", "Ledger", "Card", "List", etc.
    pub data_per_company: Option<bool>,
    pub columns: Option<Vec<ColumnDefinition>>,
    pub primary_key: Option<Vec<String>>,
    pub check_constraints: Option<Vec<bosc_ir::CheckConstraintIR>>,
    pub state_machine: Option<StateMachineDefinition>,
    #[serde(alias = "definition_ir")]
    pub definition: Option<serde_json::Value>,
    /// Allow schema changes that delete or truncate data (dropping fields, narrowing types).
    #[serde(default)]
    pub allow_destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterTableExtensionRequest {
    pub name: String,
    pub package_name: Option<String>,
    pub target_table: String,
    pub added_columns: Vec<ColumnDefinition>,
    pub added_indexes: Option<Vec<bosc_ir::IndexIR>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyTableScriptRequest {
    pub script: String,
    pub package_name: Option<String>,
    pub extension_name: Option<String>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub allow_destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyTableScriptResponse {
    pub success: bool,
    pub table_name: String,
    pub extension_name: String,
    pub compiled_ddl: String,
    pub compiled_ir: serde_json::Value,
    pub added_columns: Vec<String>,
    pub registered_codeunits: Vec<String>,
    pub applied: bool,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_sync: Option<SchemaSyncReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileScriptRequest {
    pub script: String,
    pub package_name: Option<String>,
    #[serde(default)]
    pub apply: bool,
    #[serde(default)]
    pub allow_destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileScriptResponse {
    pub success: bool,
    pub compiled_ddl: String,
    pub compiled_ir: serde_json::Value,
    pub tables_extended: Vec<String>,
    pub tables_created: Vec<String>,
    pub codeunits_registered: Vec<String>,
    pub applied: bool,
    pub message: String,
    #[serde(default)]
    pub diagnostics: Vec<bosc_ir::CompilerDiagnosticIR>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_sync: Option<SchemaSyncReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaObjectSummary {
    pub object_id: Uuid,
    pub object_type: String,
    pub object_name: String,
    pub package_name: String,
    pub version: String,
    pub caption: Option<String>,
    #[serde(alias = "table_type")]
    pub sub_type: Option<String>,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaFieldInfo {
    pub field_id: Uuid,
    pub field_name: String,
    pub field_type: String,
    pub sql_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub is_flowfield: bool,
    pub default_val: Option<String>,
    pub caption: Option<String>,
    pub ordinal_position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaRelationInfo {
    pub relation_id: Uuid,
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
    pub cardinality: String,
    pub on_delete: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaObjectDetails {
    pub summary: MetaObjectSummary,
    #[serde(default)]
    pub object_type: String,
    pub fields: Vec<MetaFieldInfo>,
    pub relations: Vec<MetaRelationInfo>,
    #[serde(alias = "definition_ir")]
    pub definition: serde_json::Value,
    #[serde(default)]
    pub source_script: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSummary {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub status: String,
    pub installed_at: chrono::DateTime<chrono::Utc>,
}

pub struct MetaManager;

impl MetaManager {
    /// Provision the BOS System Kernel into the target PostgreSQL pool
    pub async fn provision_system_kernel(
        pool: &PgPool,
        strategy_str: Option<&str>,
    ) -> Result<SystemProvisionStatus, RuntimeError> {
        let strategy =
            bosc_ir::StorageStrategyIR::from_str_loose(strategy_str.unwrap_or("shared_rls"));
        let generator = PostgresGenerator::with_strategy(strategy);
        let ddl = generator.generate_system_kernel_ddl();

        sqlx::raw_sql(&ddl).execute(pool).await?;

        Self::get_system_status(pool).await
    }

    /// Check system provisioning status of the database
    pub async fn get_system_status(pool: &PgPool) -> Result<SystemProvisionStatus, RuntimeError> {
        // 1. Check if _bos_sys_info table exists
        let check_sys_table = r#"
            SELECT EXISTS (
                SELECT FROM information_schema.tables 
                WHERE table_schema = 'public' 
                AND table_name = '_bos_sys_info'
            ) as exists;
        "#;

        let row = sqlx::query(check_sys_table).fetch_one(pool).await?;
        let exists: bool = row.try_get("exists").unwrap_or(false);

        if !exists {
            return Ok(SystemProvisionStatus {
                is_provisioned: false,
                kernel_version: None,
                schema_version: None,
                status: None,
                storage_strategy: None,
                provisioned_at: None,
                meta_objects_count: 0,
                tenants_count: 0,
                companies_count: 0,
            });
        }

        // Check if storage_strategy column exists in _bos_sys_info
        let has_strategy_col: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = '_bos_sys_info' AND column_name = 'storage_strategy')"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        let query = if has_strategy_col {
            "SELECT kernel_version, schema_version, status, COALESCE(storage_strategy, 'shared_rls') as storage_strategy, provisioned_at FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1"
        } else {
            "SELECT kernel_version, schema_version, status, 'shared_rls' as storage_strategy, provisioned_at FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1"
        };

        // 2. Fetch kernel info
        let sys_info_row = sqlx::query(query).fetch_optional(pool).await?;

        let (kernel_version, schema_version, status, storage_strategy, provisioned_at) =
            match sys_info_row {
                Some(r) => (
                    Some(r.try_get("kernel_version")?),
                    Some(r.try_get("schema_version")?),
                    Some(r.try_get("status")?),
                    Some(
                        r.try_get("storage_strategy")
                            .unwrap_or_else(|_| "shared_rls".to_string()),
                    ),
                    Some(r.try_get("provisioned_at")?),
                ),
                None => (None, None, None, None, None),
            };

        // 3. Count meta objects, tenants, companies
        let meta_objs_cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"_bos_meta_objects\"")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let tenants_cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"_bos_tenants\"")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let companies_cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM \"_bos_companies\"")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        Ok(SystemProvisionStatus {
            is_provisioned: true,
            kernel_version,
            schema_version,
            status,
            storage_strategy,
            provisioned_at,
            meta_objects_count: meta_objs_cnt,
            tenants_count: tenants_cnt,
            companies_count: companies_cnt,
        })
    }

    /// Register a meta-object into the BOS catalog and execute its DDL if it is a table or state machine
    pub async fn register_meta_object(
        pool: &PgPool,
        req: RegisterMetaObjectRequest,
    ) -> Result<MetaObjectSummary, RuntimeError> {
        let object_type = req.object_type.to_lowercase();
        let object_name = req.name.trim().to_string();
        let package_name = req.package_name.unwrap_or_else(|| "bos.core".to_string());
        let version = req.version.unwrap_or_else(|| "1.0.0".to_string());
        let caption = req.caption.clone().unwrap_or_else(|| object_name.clone());
        let data_per_company = req.data_per_company.unwrap_or(true);

        let sub_type_str = req.sub_type.clone().unwrap_or_else(|| "Normal".to_string());
        let sql_table_name = crate::to_snake_case(&object_name);

        // 1. If object is a table, build IR and execute live DDL
        let mut ddl_to_exec = String::new();
        let strategy_str: String = sqlx::query_scalar(
            "SELECT COALESCE(storage_strategy, 'shared_rls') FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| "shared_rls".to_string());

        let strategy_ir = bosc_ir::StorageStrategyIR::from_str_loose(&strategy_str);
        let generator = PostgresGenerator::with_strategy(strategy_ir);

        if object_type == "table" {
            let table_type_ir = match sub_type_str.to_lowercase().as_str() {
                "document" => TableTypeIR::Document,
                "ledger" => TableTypeIR::Ledger,
                "temporal" => TableTypeIR::Temporal,
                "virtual" => TableTypeIR::Virtual,
                "temporary" => TableTypeIR::Temporary,
                _ => TableTypeIR::Normal,
            };

            let col_irs = req
                .columns
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(ColumnDefinition::to_column_ir)
                .collect::<Result<Vec<_>, _>>()?;

            let primary_key: Vec<String> = req
                .primary_key
                .as_ref()
                .map(|pk| pk.iter().map(|k| crate::to_snake_case(k)).collect())
                .unwrap_or_default();
            let mut indexes = Vec::new();

            // Create index for unique columns
            for col in &col_irs {
                if col.is_unique {
                    let mut cols = vec![col.sql_name.clone()];
                    if strategy_ir == bosc_ir::StorageStrategyIR::Partitioned {
                        cols.insert(0, "_tenant_id".to_string());
                    }
                    indexes.push(IndexIR {
                        name: format!("idx_{}_{}_unique", sql_table_name, col.sql_name),
                        columns: cols,
                        include_columns: Vec::new(),
                        is_unique: true,
                    });
                }
            }

            let table_ir = if let Some(ref def_val) = req.definition {
                if let Ok(mut t_ir) = serde_json::from_value::<TableIR>(def_val.clone()) {
                    t_ir.sql_table_name = sql_table_name.clone();
                    t_ir
                } else {
                    TableIR {
                        name: object_name.clone(),
                        sql_table_name: sql_table_name.clone(),
                        caption: caption.clone(),
                        table_type: table_type_ir,
                        data_per_company,
                        columns: col_irs,
                        primary_key,
                        indexes,
                        flow_fields: vec![],
                        triggers: vec![],
                        check_constraints: req.check_constraints.clone().unwrap_or_default(),
                        on_delete: None,
                        renamed_from: None,
                        field_groups: vec![],
                    }
                }
            } else {
                TableIR {
                    name: object_name.clone(),
                    sql_table_name: sql_table_name.clone(),
                    caption: caption.clone(),
                    table_type: table_type_ir,
                    data_per_company,
                    columns: col_irs,
                    primary_key,
                    indexes,
                    flow_fields: vec![],
                    triggers: vec![],
                    check_constraints: req.check_constraints.clone().unwrap_or_default(),
                    on_delete: None,
                    renamed_from: None,
                    field_groups: vec![],
                }
            };

            ddl_to_exec.push_str(&generator.generate_table_structure(&table_ir));
            ddl_to_exec.push('\n');

            // If state machine is specified alongside table
            if let Some(sm_def) = &req.state_machine {
                let sm_ir = StateMachineIR {
                    name: sm_def.name.clone(),
                    target_table: sql_table_name.clone(),
                    status_field: "status".to_string(),
                    initial_state: sm_def.initial_state.clone(),
                    states: vec![],
                    transitions: vec![],
                };
                ddl_to_exec.push_str(&generator.generate_state_machine_infrastructure(&sm_ir));
                ddl_to_exec.push('\n');
            }

            SchemaSync::sync(
                pool,
                &SchemaSyncRequest {
                    package_name: package_name.clone(),
                    source: "register".to_string(),
                    tables: vec![table_ir],
                    extensions: Vec::new(),
                    ddl: ddl_to_exec.clone(),
                    applied_by: None,
                },
                &SchemaSyncOptions {
                    allow_destructive: req.allow_destructive,
                    dry_run: false,
                },
            )
            .await?;

            // Multi-Strategy Topology Post-Processing
            if strategy_ir == bosc_ir::StorageStrategyIR::Partitioned {
                let tenant_rows = sqlx::query(
                    "SELECT tenant_id, code FROM \"_bos_tenants\" WHERE is_active = true",
                )
                .fetch_all(pool)
                .await
                .unwrap_or_default();
                for t_row in tenant_rows {
                    if let (Ok(t_id), Ok(code)) = (
                        t_row.try_get::<Uuid, _>("tenant_id"),
                        t_row.try_get::<String, _>("code"),
                    ) {
                        let clean_code = code.to_lowercase().replace('-', "_");
                        let part_ddl = format!(
                            "CREATE TABLE IF NOT EXISTS \"{table}_tenant_{code}\" PARTITION OF \"{table}\" FOR VALUES IN ('{t_id}');",
                            table = sql_table_name,
                            code = clean_code,
                            t_id = t_id
                        );
                        let _ = sqlx::raw_sql(&part_ddl).execute(pool).await;
                    }
                }
            } else if strategy_ir == bosc_ir::StorageStrategyIR::SchemaPerCompany
                && data_per_company
            {
                let comp_rows =
                    sqlx::query("SELECT code FROM \"_bos_companies\" WHERE is_active = true")
                        .fetch_all(pool)
                        .await
                        .unwrap_or_default();
                for c_row in comp_rows {
                    if let Ok(code) = c_row.try_get::<String, _>("code") {
                        let schema_name = format!("comp_{}", code.to_lowercase().replace('-', "_"));
                        let sch_ddl = format!(
                            "CREATE SCHEMA IF NOT EXISTS \"{schema}\"; CREATE TABLE IF NOT EXISTS \"{schema}\".\"{table}\" (LIKE \"public\".\"{table}\" INCLUDING ALL);",
                            schema = schema_name,
                            table = sql_table_name
                        );
                        let _ = sqlx::raw_sql(&sch_ddl).execute(pool).await;
                    }
                }
            } else if strategy_ir == bosc_ir::StorageStrategyIR::TablePerCompany && data_per_company
            {
                let comp_rows =
                    sqlx::query("SELECT code FROM \"_bos_companies\" WHERE is_active = true")
                        .fetch_all(pool)
                        .await
                        .unwrap_or_default();
                for c_row in comp_rows {
                    if let Ok(code) = c_row.try_get::<String, _>("code") {
                        let tbl_name = format!("{}${}", code.to_lowercase(), sql_table_name);
                        let tbl_ddl = format!(
                            "CREATE TABLE IF NOT EXISTS \"{tbl_name}\" (LIKE \"public\".\"{table}\" INCLUDING ALL);",
                            tbl_name = tbl_name,
                            table = sql_table_name
                        );
                        let _ = sqlx::raw_sql(&tbl_ddl).execute(pool).await;
                    }
                }
            }
        }

        // 2. Prepare definition JSON
        let mut definition = req.definition.unwrap_or_else(|| {
            serde_json::json!({
                "name": object_name,
                "type": object_type,
                "sub_type": sub_type_str,
                "data_per_company": data_per_company,
                "sql_table_name": sql_table_name,
                "columns": req.columns,
                "state_machine": req.state_machine,
            })
        });
        if let Some(obj) = definition.as_object_mut() {
            obj.entry("data_per_company")
                .or_insert_with(|| serde_json::json!(data_per_company));
            obj.entry("sql_table_name")
                .or_insert_with(|| serde_json::json!(sql_table_name));
            obj.entry("sub_type")
                .or_insert_with(|| serde_json::json!(sub_type_str));
        }

        // 3. Compute checksum
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&definition)
            .unwrap_or_default()
            .hash(&mut hasher);
        let checksum = format!("{:016x}", hasher.finish());

        // 4. Upsert into _bos_meta_objects
        let upsert_query = r#"
            INSERT INTO "_bos_meta_objects" (
                "object_type", "object_name", "package_name", "version", "caption",
                "sub_type", "definition", "checksum", "status", "updated_at"
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, 'active', NOW()
            )
            ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
            SET "version" = EXCLUDED."version",
                "caption" = EXCLUDED."caption",
                "sub_type" = EXCLUDED."sub_type",
                "definition" = EXCLUDED."definition",
                "checksum" = EXCLUDED."checksum",
                "updated_at" = NOW()
            RETURNING "object_id", "created_at";
        "#;

        let row = sqlx::query(upsert_query)
            .bind(&object_type)
            .bind(&object_name)
            .bind(&package_name)
            .bind(&version)
            .bind(&caption)
            .bind(&sub_type_str)
            .bind(&definition)
            .bind(&checksum)
            .fetch_one(pool)
            .await?;

        let object_id: Uuid = row.try_get("object_id")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

        // ADR 12: Zero Table Sprawl. Columns, relations, and state machines are
        // stored authoritatively inside _bos_meta_objects.definition (JSONB).
        // No auxiliary dictionary table writes are required.

        Ok(MetaObjectSummary {
            object_id,
            object_type,
            object_name,
            package_name,
            version,
            caption: Some(caption),
            sub_type: Some(sub_type_str),
            status: "active".to_string(),
            created_at,
        })
    }

    /// Register a Table Extension (AL/BC non-invasive schema augmentation)
    pub async fn register_table_extension(
        pool: &PgPool,
        req: RegisterTableExtensionRequest,
    ) -> Result<MetaObjectSummary, RuntimeError> {
        let pkg = req.package_name.unwrap_or_else(|| "bos.custom".to_string());
        let target_table = req.target_table.trim().to_string();

        // 1. Verify target table exists
        let target_row = sqlx::query(
            "SELECT object_id, definition FROM \"_bos_meta_objects\" WHERE LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1) LIMIT 1"
        )
        .bind(&target_table)
        .fetch_optional(pool)
        .await?;

        let (target_id, mut target_ir) = match target_row {
            Some(r) => (
                r.try_get::<Uuid, _>("object_id")?,
                r.try_get::<serde_json::Value, _>("definition")?,
            ),
            None => {
                return Err(RuntimeError::NotFound(
                    "Table".to_string(),
                    target_table.clone(),
                ));
            }
        };

        // 2. Fetch strategy and companies
        let strat_str: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
        let strategy = bosc_ir::StorageStrategyIR::from_str_loose(
            strat_str.as_deref().unwrap_or("shared_rls"),
        );

        let comp_rows = sqlx::query("SELECT code FROM \"_bos_companies\" WHERE is_active = true")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        let companies: Vec<String> = comp_rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("code").ok())
            .collect();

        // 3. Build TableExtensionIR and generate DDL
        let ir_columns = req
            .added_columns
            .iter()
            .map(ColumnDefinition::to_column_ir)
            .collect::<Result<Vec<_>, _>>()?;

        let ext_ir = bosc_ir::TableExtensionIR {
            name: req.name.clone(),
            package_name: pkg.clone(),
            target_table: target_table.clone(),
            added_columns: ir_columns,
            added_indexes: req.added_indexes.unwrap_or_default(),
        };

        let generator = PostgresGenerator::with_strategy(strategy);
        let ddl = generator.generate_table_extension_ddl(&ext_ir, &companies);

        SchemaSync::sync(
            pool,
            &SchemaSyncRequest {
                package_name: pkg.clone(),
                source: "extension".to_string(),
                tables: Vec::new(),
                extensions: vec![ext_ir.clone()],
                ddl,
                applied_by: None,
            },
            &SchemaSyncOptions::default(),
        )
        .await?;

        // 4. Save TableExtension meta-object
        let ext_def_ir = serde_json::to_value(&ext_ir).unwrap_or_default();
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&ext_def_ir)
            .unwrap_or_default()
            .hash(&mut hasher);
        let checksum = format!("{:016x}", hasher.finish());

        let upsert_query = r#"
            INSERT INTO "_bos_meta_objects" (
                "object_type", "object_name", "package_name", "version", "caption",
                "sub_type", "definition", "checksum", "status", "updated_at"
            ) VALUES (
                'tableextension', $1, $2, '1.0.0', $3, 'Extension', $4, $5, 'active', NOW()
            )
            ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
            SET "definition" = EXCLUDED."definition",
                "checksum" = EXCLUDED."checksum",
                "updated_at" = NOW()
            RETURNING "object_id", "created_at";
        "#;

        let caption = format!("Extension {} on {}", req.name, target_table);
        let row = sqlx::query(upsert_query)
            .bind(&req.name)
            .bind(&pkg)
            .bind(&caption)
            .bind(&ext_def_ir)
            .bind(&checksum)
            .fetch_one(pool)
            .await?;

        let object_id: Uuid = row.try_get("object_id")?;
        let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;

        // 5. Merge extension columns into base table's definition in _bos_meta_objects
        if let Some(cols) = target_ir.get_mut("columns").and_then(|c| c.as_array_mut()) {
            for new_col in &req.added_columns {
                let col_json = serde_json::json!({
                    "name": new_col.name,
                    "sql_type": new_col.sql_type,
                    "is_nullable": new_col.is_nullable.unwrap_or(true),
                    "is_unique": new_col.is_unique.unwrap_or(false),
                    "default_val": new_col.default_val,
                    "caption": new_col.caption.clone().unwrap_or_else(|| new_col.name.clone()),
                    "extension": req.name,
                });
                if !cols.iter().any(|existing| {
                    existing.get("name").and_then(|n| n.as_str()) == Some(&new_col.name)
                }) {
                    cols.push(col_json);
                }
            }

            let update_base_ir = "UPDATE \"_bos_meta_objects\" SET \"definition\" = $1, \"updated_at\" = NOW() WHERE \"object_id\" = $2;";
            let _ = sqlx::query(update_base_ir)
                .bind(&target_ir)
                .bind(target_id)
                .execute(pool)
                .await;
        }

        Ok(MetaObjectSummary {
            object_id,
            object_type: "tableextension".to_string(),
            object_name: req.name,
            package_name: pkg,
            version: "1.0.0".to_string(),
            caption: Some(caption),
            sub_type: Some("Extension".to_string()),
            status: "active".to_string(),
            created_at,
        })
    }

    /// List all meta-objects in catalog with optional type filter
    pub async fn list_meta_objects(
        pool: &PgPool,
        object_type: Option<&str>,
    ) -> Result<Vec<MetaObjectSummary>, RuntimeError> {
        let query = match object_type {
            Some(t) => {
                format!(
                    "SELECT object_id, object_type, object_name, package_name, version, caption, sub_type, status, created_at FROM \"_bos_meta_objects\" WHERE LOWER(object_type) = '{}' ORDER BY object_type, object_name;",
                    t.to_lowercase().replace("'", "''")
                )
            }
            None => {
                "SELECT object_id, object_type, object_name, package_name, version, caption, sub_type, status, created_at FROM \"_bos_meta_objects\" ORDER BY object_type, object_name;".to_string()
            }
        };

        let rows = sqlx::query(&query).fetch_all(pool).await?;
        let mut list = Vec::new();

        for r in rows {
            list.push(MetaObjectSummary {
                object_id: r.try_get("object_id")?,
                object_type: r.try_get("object_type")?,
                object_name: r.try_get("object_name")?,
                package_name: r.try_get("package_name")?,
                version: r.try_get("version")?,
                caption: r.try_get("caption").ok(),
                sub_type: r.try_get("sub_type").ok(),
                status: r.try_get("status")?,
                created_at: r.try_get("created_at")?,
            });
        }

        Ok(list)
    }

    /// Fetch details for a specific meta-object by name
    pub async fn get_meta_object(
        pool: &PgPool,
        name: &str,
    ) -> Result<Option<MetaObjectDetails>, RuntimeError> {
        let obj_query = r#"
            SELECT object_id, object_type, object_name, package_name, version, caption, sub_type, definition, status, created_at
            FROM "_bos_meta_objects"
            WHERE LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1)
            ORDER BY 
                CASE 
                    WHEN LOWER(object_name) = LOWER($1) AND object_type = 'table' THEN 0
                    WHEN LOWER(object_name) = LOWER($1) THEN 1
                    WHEN object_type = 'table' THEN 2
                    ELSE 3
                END,
                updated_at DESC
            LIMIT 1;
        "#;

        let row_opt = sqlx::query(obj_query)
            .bind(name)
            .fetch_optional(pool)
            .await?;
        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let summary = MetaObjectSummary {
            object_id: row.try_get("object_id")?,
            object_type: row.try_get("object_type")?,
            object_name: row.try_get("object_name")?,
            package_name: row.try_get("package_name")?,
            version: row.try_get("version")?,
            caption: row.try_get("caption").ok(),
            sub_type: row.try_get("sub_type").ok(),
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
        };

        let definition: serde_json::Value = row.try_get("definition").unwrap_or_default();

        // ADR 12: Project fields directly from definition JSONB (Single Source of Truth)
        let mut fields = Vec::new();
        let mut relations = Vec::new();

        if let Some(cols) = definition.get("columns").and_then(|c| c.as_array()) {
            for (idx, c) in cols.iter().enumerate() {
                let name = c
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if name.is_empty() {
                    continue;
                }

                let sql_type = c
                    .get("sql_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("TEXT")
                    .to_string();

                let field_type = if let Some(dt) = c.get("data_type") {
                    if let Some(prim) = dt.get("Primitive").and_then(|p| p.as_str()) {
                        prim.to_string()
                    } else if let Some(s) = dt.as_str() {
                        s.to_string()
                    } else {
                        sql_type_to_bos_type(&sql_type)
                    }
                } else {
                    sql_type_to_bos_type(&sql_type)
                };

                let is_pk = c
                    .get("is_primary_key")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let is_nullable = c
                    .get("is_nullable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(!is_pk);
                let is_unique = c
                    .get("is_unique")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let is_flowfield = c
                    .get("is_flowfield")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let default_val = c
                    .get("default_val")
                    .or_else(|| c.get("default_value"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let caption = c
                    .get("caption")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(name.clone()));

                let field_id = Uuid::new_v4();

                fields.push(MetaFieldInfo {
                    field_id,
                    field_name: name.clone(),
                    field_type,
                    sql_type,
                    is_primary_key: is_pk,
                    is_nullable,
                    is_unique,
                    is_flowfield,
                    default_val,
                    caption,
                    ordinal_position: (idx + 1) as i32,
                });

                // Foreign keys from column definition (foreign_key_table / foreign_key_column)
                if let (Some(fk_tbl), Some(fk_col)) = (
                    c.get("foreign_key_table").and_then(|v| v.as_str()),
                    c.get("foreign_key_column").and_then(|v| v.as_str()),
                ) {
                    let from_tbl = definition
                        .get("sql_table_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&summary.object_name)
                        .to_string();
                    let rel_id = Uuid::new_v4();
                    relations.push(MetaRelationInfo {
                        relation_id: rel_id,
                        from_table: from_tbl,
                        from_column: name.clone(),
                        to_table: fk_tbl.to_string(),
                        to_column: fk_col.to_string(),
                        cardinality: "ManyToOne".to_string(),
                        on_delete: "Restrict".to_string(),
                    });
                } else if let Some(fk) = c.get("foreign_key").and_then(|f| f.as_object()) {
                    if let (Some(fk_tbl), Some(fk_col)) = (
                        fk.get("target_table").and_then(|v| v.as_str()),
                        fk.get("target_column").and_then(|v| v.as_str()),
                    ) {
                        let from_tbl = definition
                            .get("sql_table_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&summary.object_name)
                            .to_string();
                        let rel_id = Uuid::new_v4();
                        let on_delete = fk
                            .get("on_delete")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Restrict")
                            .to_string();
                        relations.push(MetaRelationInfo {
                            relation_id: rel_id,
                            from_table: from_tbl,
                            from_column: name.clone(),
                            to_table: fk_tbl.to_string(),
                            to_column: fk_col.to_string(),
                            cardinality: "ManyToOne".to_string(),
                            on_delete,
                        });
                    }
                }
            }
        }

        // Additional relations from top-level foreign_keys if present
        if let Some(fks) = definition.get("foreign_keys").and_then(|f| f.as_array()) {
            for fk in fks {
                if let (Some(tbl), Some(col)) = (
                    fk.get("target_table").and_then(|v| v.as_str()),
                    fk.get("target_column").and_then(|v| v.as_str()),
                ) {
                    let from_tbl = definition
                        .get("sql_table_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&summary.object_name)
                        .to_string();
                    let from_col = fk
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let rel_id = Uuid::new_v4();
                    relations.push(MetaRelationInfo {
                        relation_id: rel_id,
                        from_table: from_tbl,
                        from_column: from_col,
                        to_table: tbl.to_string(),
                        to_column: col.to_string(),
                        cardinality: "ManyToOne".to_string(),
                        on_delete: fk
                            .get("on_delete")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Restrict")
                            .to_string(),
                    });
                }
            }
        }

        let object_type = summary.object_type.clone();
        let source_script = Self::extract_or_reconstruct_source(
            &summary.object_type,
            &summary.object_name,
            &definition,
        );

        Ok(Some(MetaObjectDetails {
            summary,
            object_type,
            fields,
            relations,
            definition,
            source_script: Some(source_script),
        }))
    }

    fn extract_or_reconstruct_source(
        object_type: &str,
        object_name: &str,
        definition: &serde_json::Value,
    ) -> String {
        if let Some(src) = definition.get("source_script").and_then(|v| v.as_str()) {
            if !src.trim().is_empty() {
                return src.to_string();
            }
        }
        match object_type {
            "table" => {
                let mut lines = vec![format!("table {} {{", object_name)];
                if let Some(cols) = definition.get("columns").and_then(|c| c.as_array()) {
                    for col in cols {
                        let name = col.get("name").and_then(|v| v.as_str()).unwrap_or("field");
                        let ty = col
                            .get("data_type")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| {
                                let sql_ty = col
                                    .get("sql_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("TEXT");
                                sql_type_to_bos_type(sql_ty)
                            });
                        lines.push(format!("    {}: {};", name, ty));
                    }
                }
                if let Some(pks) = definition.get("primary_key").and_then(|p| p.as_array()) {
                    let pk_list: Vec<&str> = pks.iter().filter_map(|v| v.as_str()).collect();
                    if !pk_list.is_empty() {
                        lines.push(format!("    primaryKey({});", pk_list.join(", ")));
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "codeunit" => {
                let mut lines = vec![format!("codeunit {} {{", object_name)];
                if let Some(methods) = definition.get("methods").and_then(|m| m.as_array()) {
                    for m in methods {
                        let m_name = m.get("name").and_then(|v| v.as_str()).unwrap_or("execute");
                        let params = m
                            .get("params")
                            .and_then(|p| p.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|item| {
                                        if let Some(pair) = item.as_array() {
                                            let p_name = pair.first().and_then(|v| v.as_str())?;
                                            let p_type = pair
                                                .get(1)
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("any");
                                            Some(format!("{}: {}", p_name, p_type))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        let ret = m
                            .get("return_type")
                            .and_then(|v| v.as_str())
                            .map(|r| format!(": {}", r))
                            .unwrap_or_default();
                        let body = m.get("script").and_then(|v| v.as_str()).unwrap_or("");
                        lines.push(format!("    function {}({}){} {{", m_name, params, ret));
                        for b_line in body.lines() {
                            lines.push(format!("        {}", b_line));
                        }
                        lines.push("    }".to_string());
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "statemachine" => {
                let target = definition
                    .get("target_table")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Document");
                let status_field = definition
                    .get("status_field")
                    .and_then(|v| v.as_str())
                    .unwrap_or("status");
                let initial = definition
                    .get("initial_state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Draft");
                let mut lines = vec![
                    format!(
                        "statemachine {} for {}.{} {{",
                        object_name, target, status_field
                    ),
                    format!("    initialState = \"{}\";", initial),
                ];
                if let Some(states) = definition.get("states").and_then(|s| s.as_array()) {
                    for st in states {
                        let st_name = st.get("name").and_then(|v| v.as_str()).unwrap_or("Draft");
                        let caption = st
                            .get("caption")
                            .and_then(|v| v.as_str())
                            .unwrap_or(st_name);
                        lines.push(format!(
                            "    state {} {{ caption = \"{}\"; }}",
                            st_name, caption
                        ));
                    }
                }
                if let Some(transitions) = definition.get("transitions").and_then(|t| t.as_array())
                {
                    for tr in transitions {
                        let tr_name = tr.get("name").and_then(|v| v.as_str()).unwrap_or("submit");
                        let to_st = tr
                            .get("to_state")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Approved");
                        let from_str = tr
                            .get("from_states")
                            .and_then(|f| f.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_else(|| "Draft".to_string());
                        lines.push(format!("    transition {} {{", tr_name));
                        lines.push(format!("        from = [{}];", from_str));
                        lines.push(format!("        to = {};", to_st));
                        lines.push("    }".to_string());
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "enum" => {
                let mut lines = vec![format!("enum {} {{", object_name)];
                if let Some(members) = definition.get("members").and_then(|m| m.as_array()) {
                    for mem in members {
                        if let Some(m_name) = mem.get("name").and_then(|v| v.as_str()) {
                            lines.push(format!("    {};", m_name));
                        } else if let Some(m_name) = mem.as_str() {
                            lines.push(format!("    {};", m_name));
                        }
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "interface" => {
                let mut lines = vec![format!("interface {} {{", object_name)];
                if let Some(methods) = definition.get("methods").and_then(|m| m.as_array()) {
                    for m in methods {
                        let m_name = m.get("name").and_then(|v| v.as_str()).unwrap_or("execute");
                        let params = m
                            .get("params")
                            .and_then(|p| p.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|item| {
                                        let pair = item.as_array()?;
                                        Some(format!(
                                            "{}: {}",
                                            pair.first()?.as_str()?,
                                            pair.get(1)?.as_str()?
                                        ))
                                    })
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        let ret = m
                            .get("return_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("void");
                        lines.push(format!("    {}({}): {};", m_name, params, ret));
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "permissionset" => {
                let mut lines = vec![format!("permissionset {} {{", object_name)];
                if let Some(cap) = definition.get("caption").and_then(|v| v.as_str()) {
                    lines.push(format!("    caption = \"{}\";", cap));
                }
                if let Some(perms) = definition.get("permissions").and_then(|p| p.as_array()) {
                    lines.push("    permissions = {".to_string());
                    for p in perms {
                        let Some(tbl) = p
                            .get("table_name")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                        else {
                            continue;
                        };
                        let mut rights = String::new();
                        if p.get("can_read").and_then(|v| v.as_bool()).unwrap_or(true) {
                            rights.push('R');
                        }
                        if p.get("can_insert")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            rights.push('I');
                        }
                        if p.get("can_modify")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            rights.push('M');
                        }
                        if p.get("can_delete")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                        {
                            rights.push('D');
                        }
                        if rights.is_empty() {
                            rights.push('R');
                        }
                        lines.push(format!("        {} = {};", tbl, rights));
                    }
                    lines.push("    };".to_string());
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            "view" => {
                let src_tbl = definition
                    .get("source_table")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(object_name);
                let cols: Vec<String> = definition
                    .get("columns")
                    .and_then(|c| c.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|col| {
                                let name = col.get("name").and_then(|v| v.as_str())?;
                                let expr = col
                                    .get("source_expr")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(name);
                                if expr == name {
                                    Some(format!("        {}", name))
                                } else {
                                    Some(format!("        {}: {}", name, expr))
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let select_body = if cols.is_empty() {
                    "        _id".to_string()
                } else {
                    cols.join(",\n")
                };
                format!(
                    "view {} {{\n    select {{\n{}\n    }}\n    from {};\n}}",
                    object_name, select_body, src_tbl
                )
            }
            "query" => {
                let mut elem_lines = Vec::new();
                if let Some(elements) = definition.get("elements").and_then(|e| e.as_array()) {
                    for el in elements {
                        let el_name = el.get("name").and_then(|v| v.as_str()).unwrap_or("row");
                        let tbl_name = el
                            .get("table_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(object_name);
                        elem_lines.push(format!("        dataitem({}; {}) {{", el_name, tbl_name));
                        if let Some(cols) = el.get("columns").and_then(|c| c.as_array()) {
                            for col in cols {
                                let c_name =
                                    col.get("name").and_then(|v| v.as_str()).unwrap_or("id");
                                let c_field = col
                                    .get("source_field")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(c_name);
                                elem_lines.push(format!(
                                    "            column({}; {}) {{}}",
                                    c_name, c_field
                                ));
                            }
                        }
                        elem_lines.push("        }".to_string());
                    }
                }
                if elem_lines.is_empty() {
                    let src_tbl = definition
                        .get("source_table")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .unwrap_or(object_name);
                    elem_lines.push(format!("        dataitem(row; {}) {{}}", src_tbl));
                }
                format!(
                    "query {} {{\n    elements {{\n{}\n    }}\n}}",
                    object_name,
                    elem_lines.join("\n")
                )
            }
            "page" => {
                let src_tbl = definition
                    .get("source_table")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty());
                let header = match src_tbl {
                    Some(t) => format!("page {} for {} {{", object_name, t),
                    None => format!("page {} {{", object_name),
                };
                let mut lines = vec![header];
                if let Some(pt) = definition
                    .get("page_type")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!("    pageType = {};", pt));
                }
                if let Some(cap) = definition
                    .get("caption")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!("    caption = \"{}\";", cap));
                }

                // Check modern definition.controls first
                if let Some(controls) = definition.get("controls").and_then(|c| c.as_array()) {
                    if !controls.is_empty() {
                        lines.push("    layout {".to_string());
                        for ctrl in controls {
                            lines.extend(Self::format_page_control(ctrl, 8));
                        }
                        lines.push("    }".to_string());
                    }
                } else if let Some(areas) = definition.get("layout").and_then(|l| l.as_array()) {
                    lines.push("    layout {".to_string());
                    for area in areas {
                        let a_name = area
                            .get("area_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Content");
                        lines.push(format!("        area({}) {{", a_name));
                        if let Some(groups) = area.get("groups").and_then(|g| g.as_array()) {
                            for grp in groups {
                                let g_name = grp
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("General");
                                let g_type = grp
                                    .get("group_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("group");
                                lines.push(format!("            {}({}) {{", g_type, g_name));
                                if let Some(ctrls) = grp.get("controls").and_then(|c| c.as_array())
                                {
                                    for ctrl in ctrls {
                                        let f_name = ctrl
                                            .get("field_name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("id");
                                        lines.push(format!("                field({});", f_name));
                                    }
                                }
                                lines.push("            }".to_string());
                            }
                        }
                        lines.push("        }".to_string());
                    }
                    lines.push("    }".to_string());
                }

                if let Some(actions) = definition.get("actions").and_then(|a| a.as_array()) {
                    if !actions.is_empty() {
                        lines.push("    actions {".to_string());
                        for act in actions {
                            let a_name = act.get("name").and_then(|v| v.as_str()).unwrap_or("Action");
                            lines.push(format!("        action({}) {{", a_name));
                            if let Some(cap) = act.get("caption").and_then(|v| v.as_str()) {
                                lines.push(format!("            caption = \"{}\";", cap));
                            }
                            if let Some(icon) = act.get("icon").and_then(|v| v.as_str()) {
                                lines.push(format!("            icon = \"{}\";", icon));
                            }
                            if let Some(script) = act.get("script").and_then(|v| v.as_str()) {
                                lines.push("            trigger onAction() {".to_string());
                                for s_line in script.lines() {
                                    lines.push(format!("                {}", s_line));
                                }
                                lines.push("            }".to_string());
                            }
                            lines.push("        }".to_string());
                        }
                        lines.push("    }".to_string());
                    }
                }

                lines.push("}".to_string());
                lines.join("\n")
            }
            "menu" => {
                let mut lines = vec![format!("menu {} {{", object_name)];
                if let Some(cap) = definition
                    .get("caption")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!("    caption = \"{}\";", cap));
                }
                if let Some(items) = definition.get("items").and_then(|i| i.as_array()) {
                    for item in items {
                        let kind = item.get("kind").and_then(|v| v.as_str()).unwrap_or("page");
                        let target = item.get("target").and_then(|v| v.as_str()).unwrap_or("");
                        lines.push(format!("    {}({});", kind, target));
                    }
                }
                lines.push("}".to_string());
                lines.join("\n")
            }
            _ => String::new(),
        }
    }

    fn format_page_control(ctrl: &serde_json::Value, indent: usize) -> Vec<String> {
        let mut out = Vec::new();
        let spaces = " ".repeat(indent);
        let kind = ctrl.get("kind").and_then(|v| v.as_str()).unwrap_or("control");
        let name = ctrl.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
        let caption = ctrl
            .get("caption")
            .and_then(|v| v.as_str())
            .or_else(|| {
                ctrl.get("properties")
                    .and_then(|p| p.get("caption"))
                    .and_then(|v| v.as_str())
            });
        let children = ctrl.get("children").and_then(|v| v.as_array());

        match kind {
            "area" => {
                out.push(format!("{}area({}) {{", spaces, name));
                if let Some(kids) = children {
                    for kid in kids {
                        out.extend(Self::format_page_control(kid, indent + 4));
                    }
                }
                out.push(format!("{}}}", spaces));
            }
            "group" | "section" => {
                out.push(format!("{}group({}) {{", spaces, name));
                if let Some(cap) = caption {
                    out.push(format!("{}    caption = \"{}\";", spaces, cap));
                }
                if let Some(kids) = children {
                    for kid in kids {
                        out.extend(Self::format_page_control(kid, indent + 4));
                    }
                }
                out.push(format!("{}}}", spaces));
            }
            "grid" => {
                out.push(format!("{}grid({}) {{", spaces, name));
                if let Some(cap) = caption {
                    out.push(format!("{}    caption = \"{}\";", spaces, cap));
                }
                if let Some(kids) = children {
                    for kid in kids {
                        out.extend(Self::format_page_control(kid, indent + 4));
                    }
                }
                out.push(format!("{}}}", spaces));
            }
            "part" => {
                let src = ctrl.get("source_expr").and_then(|v| v.as_str()).unwrap_or("");
                if !src.is_empty() {
                    out.push(format!("{}part({}; {}) {{", spaces, name, src));
                } else {
                    out.push(format!("{}part({}) {{", spaces, name));
                }
                if let Some(cap) = caption {
                    out.push(format!("{}    caption = \"{}\";", spaces, cap));
                }
                out.push(format!("{}}}", spaces));
            }
            "field" => {
                out.push(format!("{}field({}) {{", spaces, name));
                if let Some(cap) = caption {
                    out.push(format!("{}    caption = \"{}\";", spaces, cap));
                }
                out.push(format!("{}}}", spaces));
            }
            _ => {
                out.push(format!("{}{}({}) {{}}", spaces, kind, name));
            }
        }
        out
    }

    /// Retrieve installed packages from catalog (_bos_meta_packages)
    pub async fn list_packages(pool: &PgPool) -> Result<Vec<PackageSummary>, RuntimeError> {
        let rows = sqlx::query(
            "SELECT package_id, name, version, publisher, status, installed_at FROM \"_bos_meta_packages\" ORDER BY package_id"
        )
        .fetch_all(pool)
        .await?;

        let mut pkgs = Vec::new();
        for r in rows {
            pkgs.push(PackageSummary {
                package_id: r.try_get("package_id")?,
                name: r.try_get("name")?,
                version: r.try_get("version")?,
                publisher: r.try_get("publisher")?,
                status: r.try_get("status")?,
                installed_at: r.try_get("installed_at")?,
            });
        }
        Ok(pkgs)
    }

    /// Compile and apply BOS-Script to an already created table
    pub async fn apply_table_script(
        pool: &PgPool,
        table_name: &str,
        req: ApplyTableScriptRequest,
    ) -> Result<ApplyTableScriptResponse, RuntimeError> {
        let trimmed_table = table_name.trim();

        // 1. Verify target table exists in _bos_meta_objects or PostgreSQL
        let target_row = sqlx::query(
            "SELECT object_id, definition FROM \"_bos_meta_objects\" WHERE LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1) LIMIT 1"
        )
        .bind(trimmed_table)
        .fetch_optional(pool)
        .await?;

        let (target_id, mut target_ir) = match target_row {
            Some(r) => (
                Some(r.try_get::<Uuid, _>("object_id")?),
                r.try_get::<serde_json::Value, _>("definition")?,
            ),
            None => {
                // Check if physical table exists in PostgreSQL
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND LOWER(table_name) = LOWER($1))"
                )
                .bind(trimmed_table)
                .fetch_one(pool)
                .await
                .unwrap_or(false);

                if !exists {
                    return Err(RuntimeError::NotFound(
                        "Table".to_string(),
                        trimmed_table.to_string(),
                    ));
                }

                (
                    None,
                    serde_json::json!({
                        "name": trimmed_table,
                        "columns": []
                    }),
                )
            }
        };

        let pkg = req.package_name.as_deref().unwrap_or("bos.custom");
        let ext_name = req
            .extension_name
            .clone()
            .unwrap_or_else(|| format!("{}Ext", trimmed_table));

        // 2. Normalize script if needed
        let raw_script = req.script.trim();
        let normalized_script =
            if raw_script.contains("tableextension ") || raw_script.contains("table ") {
                raw_script.to_string()
            } else if let Some(cu_pos) = raw_script.find("codeunit ") {
                let (fields_part, cu_part) = raw_script.split_at(cu_pos);
                format!(
                    "tableextension {} extends {} {{\n{}\n}}\n\n{}",
                    ext_name,
                    trimmed_table,
                    fields_part.trim(),
                    cu_part.trim()
                )
            } else {
                format!(
                    "tableextension {} extends {} {{\n{}\n}}",
                    ext_name, trimmed_table, raw_script
                )
            };

        // 3. Tokenize, parse, and analyze
        let mut lexer = Lexer::new(&normalized_script);
        let tokens = lexer
            .tokenize()
            .map_err(|e| RuntimeError::Internal(format!("Lexer error: {:?}", e)))?;
        let mut parser = Parser::with_source(tokens, normalized_script.clone());
        let program = parser
            .parse_program()
            .map_err(|e| RuntimeError::Internal(format!("Parser syntax error: {:?}", e)))?;
        let analyzer = SemanticAnalyzer::new(pkg);
        let model_ir = analyzer
            .analyze_and_lower(&program)
            .map_err(|e| RuntimeError::Internal(format!("Analyzer error: {:?}", e)))?;

        // 4. Determine storage strategy and active companies for adaptive DDL
        let strat_str: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
        let strategy = bosc_ir::StorageStrategyIR::from_str_loose(
            strat_str.as_deref().unwrap_or("shared_rls"),
        );

        let comp_rows = sqlx::query("SELECT code FROM \"_bos_companies\" WHERE is_active = true")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        let companies: Vec<String> = comp_rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("code").ok())
            .collect();

        let generator = PostgresGenerator::with_strategy(strategy);

        let mut compiled_ddl = String::new();
        let mut added_columns_list = Vec::new();
        let mut actual_ext_name = ext_name.clone();

        for ext in &model_ir.table_extensions {
            actual_ext_name = ext.name.clone();
            let ddl = generator.generate_table_extension_ddl(ext, &companies);
            compiled_ddl.push_str(&ddl);
            compiled_ddl.push('\n');
            for col in &ext.added_columns {
                added_columns_list.push(format!("{} ({})", col.name, col.sql_type));
            }
        }

        compiled_ddl.push_str(&generator.generate_tables(&model_ir.tables));

        // BOS-AMDP: Generate PL/pgSQL stored procedures for pushdown Codeunit methods
        for cu in &model_ir.codeunits {
            for m in &cu.methods {
                if (m.is_pushdown || cu.is_pushdown) && bosc_codegen_db::pushdown::body(m).is_ok() {
                    let proc_ddl = generator.generate_plpgsql_procedure(&cu.name, m);
                    compiled_ddl.push_str(&proc_ddl);
                    compiled_ddl.push('\n');
                }
            }
        }

        let compiled_ir = serde_json::to_value(&model_ir).unwrap_or_default();
        let mut registered_codeunits = Vec::new();
        let mut schema_report = None;

        // 5. Apply changes if not dry_run
        if !req.dry_run {
            schema_report = Some(
                SchemaSync::sync(
                    pool,
                    &SchemaSyncRequest {
                        package_name: pkg.to_string(),
                        source: "table_script".to_string(),
                        tables: model_ir.tables.clone(),
                        extensions: model_ir.table_extensions.clone(),
                        ddl: compiled_ddl.clone(),
                        applied_by: None,
                    },
                    &SchemaSyncOptions {
                        allow_destructive: req.allow_destructive,
                        dry_run: false,
                    },
                )
                .await?,
            );

            // Register table extensions & augment target table metadata
            for ext in &model_ir.table_extensions {
                let ext_def_ir = serde_json::to_value(ext).unwrap_or_default();
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&ext_def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        'tableextension', $1, $2, '1.0.0', $3, 'Extension', $4, $5, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                let caption = format!("Extension {} on {}", ext.name, ext.target_table);
                sqlx::query(upsert_query)
                    .bind(&ext.name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&ext_def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await?;

                // Merge columns into target table definition
                if let Some(cols) = target_ir.get_mut("columns").and_then(|c| c.as_array_mut()) {
                    for new_col in &ext.added_columns {
                        let col_json = serde_json::json!({
                            "name": new_col.name,
                            "sql_type": new_col.sql_type,
                            "is_nullable": new_col.is_nullable,
                            "is_unique": new_col.is_unique,
                            "default_val": new_col.default_val,
                            "caption": new_col.caption,
                            "extension": ext.name,
                        });
                        if !cols.iter().any(|existing| {
                            existing.get("name").and_then(|n| n.as_str()) == Some(&new_col.name)
                        }) {
                            cols.push(col_json);
                        }
                    }

                    if let Some(t_id) = target_id {
                        let _ = sqlx::query(
                            "UPDATE \"_bos_meta_objects\" SET \"definition\" = $1, \"updated_at\" = NOW() WHERE \"object_id\" = $2"
                        )
                        .bind(&target_ir)
                        .bind(t_id)
                        .execute(pool)
                        .await;
                    } else {
                        let base_checksum = format!("{:016x}", 0u64);
                        let _ = sqlx::query(
                            r#"INSERT INTO "_bos_meta_objects" (
                                "object_type", "object_name", "package_name", "version", "caption",
                                "sub_type", "definition", "checksum", "status", "updated_at"
                            ) VALUES ('table', $1, $2, '1.0.0', $1, 'Normal', $3, $4, 'active', NOW())
                            ON CONFLICT ("object_type", "object_name", "package_name") DO NOTHING"#
                        )
                        .bind(trimmed_table)
                        .bind(pkg)
                        .bind(&target_ir)
                        .bind(&base_checksum)
                        .execute(pool)
                        .await;
                    }
                }
            }

            // Register Codeunits
            for cu in &model_ir.codeunits {
                crate::execution_engine::ExecutionEngine::register_codeunit(pool, cu.clone())
                    .await?;
                registered_codeunits.push(cu.name.clone());
            }
        }

        Ok(ApplyTableScriptResponse {
            success: true,
            table_name: trimmed_table.to_string(),
            extension_name: actual_ext_name,
            compiled_ddl: compiled_ddl.trim().to_string(),
            compiled_ir,
            added_columns: added_columns_list,
            registered_codeunits,
            applied: !req.dry_run,
            message: if req.dry_run {
                format!(
                    "Successfully compiled BOS-Script for table '{}' (dry run preview)",
                    trimmed_table
                )
            } else {
                format!(
                    "Successfully compiled and applied BOS-Script to table '{}'",
                    trimmed_table
                )
            },
            schema_sync: schema_report,
        })
    }

    /// Compile any arbitrary BOS-Script, returning compiled DDL and IR, with optional live execution
    pub async fn compile_script(
        pool: &PgPool,
        req: CompileScriptRequest,
    ) -> Result<CompileScriptResponse, RuntimeError> {
        let pkg = req.package_name.as_deref().unwrap_or("bos.custom");
        let script = req.script.trim();

        let mut lexer = Lexer::new(script);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => {
                let line = e.line();
                let col = e.column();
                let msg = format!("line {}, column {}: {}", line, col, e);
                if req.apply {
                    return Err(RuntimeError::Internal(msg));
                }
                let diag = bosc_ir::CompilerDiagnosticIR {
                    severity: "error".to_string(),
                    code: "BOS1001".to_string(),
                    message: e.to_string(),
                    line,
                    column: col,
                    end_line: line,
                    end_column: col + 1,
                    hint: e.hint(),
                };
                return Ok(CompileScriptResponse {
                    success: false,
                    compiled_ddl: String::new(),
                    compiled_ir: serde_json::json!({}),
                    tables_extended: Vec::new(),
                    tables_created: Vec::new(),
                    codeunits_registered: Vec::new(),
                    applied: false,
                    message: msg,
                    diagnostics: vec![diag],
                    schema_sync: None,
                });
            }
        };

        let mut parser = Parser::with_source(tokens, script.to_string());
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                let line = e.line();
                let col = e.column();
                let msg = format!("line {}, column {}: {}", line, col, e);
                if req.apply {
                    return Err(RuntimeError::Internal(msg));
                }
                let diag = bosc_ir::CompilerDiagnosticIR {
                    severity: "error".to_string(),
                    code: "BOS1002".to_string(),
                    message: e.to_string(),
                    line,
                    column: col,
                    end_line: line,
                    end_column: col + 1,
                    hint: e.hint(),
                };
                return Ok(CompileScriptResponse {
                    success: false,
                    compiled_ddl: String::new(),
                    compiled_ir: serde_json::json!({}),
                    tables_extended: Vec::new(),
                    tables_created: Vec::new(),
                    codeunits_registered: Vec::new(),
                    applied: false,
                    message: msg,
                    diagnostics: vec![diag],
                    schema_sync: None,
                });
            }
        };

        let analyzer = SemanticAnalyzer::new(pkg);
        let model_ir = match analyzer.analyze_and_lower(&program) {
            Ok(m) => m,
            Err(e) => {
                let msg = format!("line 1, column 1: {}", e);
                if req.apply {
                    return Err(RuntimeError::Internal(msg));
                }
                let diag = bosc_ir::CompilerDiagnosticIR {
                    severity: "error".to_string(),
                    code: "BOS1003".to_string(),
                    message: e.to_string(),
                    line: 1,
                    column: 1,
                    end_line: 1,
                    end_column: 10,
                    hint: None,
                };
                return Ok(CompileScriptResponse {
                    success: false,
                    compiled_ddl: String::new(),
                    compiled_ir: serde_json::json!({}),
                    tables_extended: Vec::new(),
                    tables_created: Vec::new(),
                    codeunits_registered: Vec::new(),
                    applied: false,
                    message: msg,
                    diagnostics: vec![diag],
                    schema_sync: None,
                });
            }
        };

        let mut diagnostics = model_ir.diagnostics.clone();

        // Cross-check referenced tables, `models.<Table>`, and field accesses against the active database catalog
        if let Ok(cat_rows) = sqlx::query(
            r#"SELECT object_name, definition FROM "_bos_meta_objects" WHERE object_type = 'table' AND status = 'active'"#
        )
        .fetch_all(pool)
        .await
        {
            let mut active_tables: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut table_columns: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

            for r in &cat_rows {
                if let Ok(obj_name) = r.try_get::<String, _>("object_name") {
                    let key = obj_name.to_lowercase();
                    active_tables.insert(key.clone());
                    let mut cols = Vec::new();
                    if let Ok(def) = r.try_get::<serde_json::Value, _>("definition") {
                        if let Some(c_arr) = def.get("columns").and_then(|v| v.as_array()) {
                            for c in c_arr {
                                if let Some(cn) = c.get("name").and_then(|v| v.as_str()) {
                                    cols.push(cn.to_string());
                                }
                            }
                        }
                        if let Some(ff_arr) = def.get("flow_fields").and_then(|v| v.as_array()) {
                            for ff in ff_arr {
                                if let Some(fnm) = ff.get("name").and_then(|v| v.as_str()) {
                                    cols.push(fnm.to_string());
                                }
                            }
                        }
                    }
                    if !cols.is_empty() {
                        table_columns.insert(key, cols);
                    }
                }
            }

            for t in &model_ir.tables {
                let key = t.name.to_lowercase();
                active_tables.insert(key.clone());
                let entry = table_columns.entry(key).or_default();
                for col in &t.columns {
                    if !entry.iter().any(|c| c.eq_ignore_ascii_case(&col.name)) {
                        entry.push(col.name.clone());
                    }
                }
                for ff in &t.flow_fields {
                    if !entry.iter().any(|c| c.eq_ignore_ascii_case(&ff.name)) {
                        entry.push(ff.name.clone());
                    }
                }
            }
            for ext in &model_ir.table_extensions {
                let key = ext.target_table.to_lowercase();
                if let Some(entry) = table_columns.get_mut(&key) {
                    for col in &ext.added_columns {
                        if !entry.iter().any(|c| c.eq_ignore_ascii_case(&col.name)) {
                            entry.push(col.name.clone());
                        }
                    }
                }
            }

            let find_token_pos = |needle: &str| -> (usize, usize) {
                for (idx, line_str) in script.lines().enumerate() {
                    if let Some(col_idx) = line_str.find(needle) {
                        return (idx + 1, col_idx + 1);
                    }
                }
                (1, 1)
            };

            let mut check_table_ref = |target: &str, context_desc: &str| {
                let clean = target.trim();
                if clean.is_empty() || active_tables.contains(&clean.to_lowercase()) {
                    return;
                }
                let (line, col) = find_token_pos(clean);
                diagnostics.push(bosc_ir::CompilerDiagnosticIR {
                    severity: "warning".to_string(),
                    code: "BOS2012".to_string(),
                    message: format!(
                        "{} references table `{}` which does not exist in the active database catalog.",
                        context_desc, clean
                    ),
                    line,
                    column: col,
                    end_line: line,
                    end_column: col + clean.len(),
                    hint: Some(format!(
                        "Create or apply `table {}` in the active database before referencing it.",
                        clean
                    )),
                });
            };

            for ext in &model_ir.table_extensions {
                check_table_ref(&ext.target_table, &format!("tableextension `{}`", ext.name));
            }
            for sm in &model_ir.state_machines {
                if !sm.target_table.is_empty() {
                    check_table_ref(&sm.target_table, &format!("statemachine `{}`", sm.name));
                }
            }
            for pg in &model_ir.pages {
                if let Some(ref src) = pg.source_table {
                    check_table_ref(src, &format!("page `{}`", pg.name));
                }
            }
            for rp in &model_ir.reports {
                if let Some(ref src) = rp.source_table {
                    check_table_ref(src, &format!("report `{}`", rp.name));
                }
            }
            for vw in &model_ir.views {
                check_table_ref(&vw.source_table, &format!("view `{}`", vw.name));
            }
            for tbl in &model_ir.tables {
                for ff in &tbl.flow_fields {
                    check_table_ref(&ff.target_table, &format!("FlowField `{}.{}`", tbl.name, ff.name));
                }
            }

            // Determine primary target table for `rec.` / `xRec.` validation if in a table, tableextension, or statemachine
            let mut primary_target_table: Option<String> = None;
            if let Some(first_tbl) = model_ir.tables.first() {
                primary_target_table = Some(first_tbl.name.clone());
            } else if let Some(first_ext) = model_ir.table_extensions.first() {
                primary_target_table = Some(first_ext.target_table.clone());
            } else if let Some(first_sm) = model_ir.state_machines.first() {
                if !first_sm.target_table.is_empty() {
                    primary_target_table = Some(first_sm.target_table.clone());
                }
            }
            if primary_target_table.is_none() {
                for line_str in script.lines() {
                    let trimmed = line_str.trim();
                    if trimmed.starts_with("tablelogic ") {
                        if let Some(for_idx) = trimmed.find(" for ") {
                            let rest = &trimmed[for_idx + 5..];
                            let target: String = rest
                                .trim_start()
                                .chars()
                                .take_while(|c| c.is_alphanumeric() || *c == '_')
                                .collect();
                            if !target.is_empty() {
                                primary_target_table = Some(target);
                                break;
                            }
                        }
                    }
                }
            }

            let built_in_record_members = [
                "testfield", "calcfields", "validate", "save", "modify", "insert", "delete",
                "get", "find", "findset", "next", "reset", "init", "setrange", "setfilter",
                "_id", "id", "_version", "created_at", "updated_at",
            ];

            let suggest_closest_col = |typo: &str, cols: &[String]| -> Option<String> {
                let typo_low = typo.to_lowercase();
                let mut best: Option<(usize, &String)> = None;
                for c in cols {
                    let c_low = c.to_lowercase();
                    if c_low.starts_with(&typo_low) || typo_low.starts_with(&c_low) {
                        return Some(c.clone());
                    }
                    // Simple character diff distance
                    let len_diff = (c_low.len() as isize - typo_low.len() as isize).unsigned_abs();
                    if len_diff <= 3 {
                        let common = c_low.chars().zip(typo_low.chars()).filter(|(a, b)| a == b).count();
                        let dist = c_low.len().max(typo_low.len()) - common;
                        if dist <= 3 {
                            if best.map_or(true, |(b_dist, _)| dist < b_dist) {
                                best = Some((dist, c));
                            }
                        }
                    }
                }
                best.map(|(_, s)| s.clone())
            };

            // Also scan procedural lines for `models.<TableName>` references and `rec.<field>` / `xRec.<field>` accesses
            for (idx, line_str) in script.lines().enumerate() {
                let mut search_from = 0;
                while let Some(pos) = line_str[search_from..].find("models.") {
                    let abs_pos = search_from + pos;
                    let after = &line_str[abs_pos + 7..];
                    let ident: String = after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !ident.is_empty() && !active_tables.contains(&ident.to_lowercase()) {
                        let col = abs_pos + 8;
                        diagnostics.push(bosc_ir::CompilerDiagnosticIR {
                            severity: "warning".to_string(),
                            code: "BOS2013".to_string(),
                            message: format!(
                                "Model `models.{}` does not exist in the active database catalog.",
                                ident
                            ),
                            line: idx + 1,
                            column: col,
                            end_line: idx + 1,
                            end_column: col + ident.len(),
                            hint: Some(
                                "Only tables deployed to the active database are available on `models.*`.".to_string()
                            ),
                        });
                    }
                    search_from = abs_pos + 7 + ident.len().max(1);
                }

                // Validate `rec.<field>` and `xRec.<field>` when primary_target_table has known columns
                if let Some(ref target_tbl) = primary_target_table {
                    if let Some(known_cols) = table_columns.get(&target_tbl.to_lowercase()) {
                        if !known_cols.is_empty() {
                            for prefix in ["rec.", "xRec."] {
                                let mut p_from = 0;
                                while let Some(pos) = line_str[p_from..].find(prefix) {
                                    let abs_pos = p_from + pos;
                                    // Ensure `rec.` is preceded by non-identifier char
                                    let valid_start = if abs_pos == 0 {
                                        true
                                    } else {
                                        line_str[..abs_pos]
                                            .chars()
                                            .last()
                                            .map_or(true, |ch| !ch.is_alphanumeric() && ch != '_')
                                    };
                                    let after = &line_str[abs_pos + prefix.len()..];
                                    let field_ident: String = after
                                        .chars()
                                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                                        .collect();
                                    if valid_start && !field_ident.is_empty() {
                                        let fl = field_ident.to_lowercase();
                                        let is_known = built_in_record_members.contains(&fl.as_str())
                                            || known_cols.iter().any(|c| c.eq_ignore_ascii_case(&field_ident));
                                        if !is_known {
                                            let col = abs_pos + prefix.len() + 1;
                                            let suggestion = suggest_closest_col(&field_ident, known_cols);
                                            let hint_msg = if let Some(sug) = suggestion {
                                                format!("Did you mean `{}`? Available fields on `{}`: {}", sug, target_tbl, known_cols.join(", "))
                                            } else {
                                                format!("Available fields on `{}`: {}", target_tbl, known_cols.join(", "))
                                            };
                                            diagnostics.push(bosc_ir::CompilerDiagnosticIR {
                                                severity: "warning".to_string(),
                                                code: "BOS2014".to_string(),
                                                message: format!(
                                                    "Field `{}` does not exist on table `{}`.",
                                                    field_ident, target_tbl
                                                ),
                                                line: idx + 1,
                                                column: col,
                                                end_line: idx + 1,
                                                end_column: col + field_ident.len(),
                                                hint: Some(hint_msg),
                                            });
                                        }
                                    }
                                    p_from = abs_pos + prefix.len() + field_ident.len().max(1);
                                }
                            }
                        }
                    }
                }
            }
        }

        let has_semantic_errors = diagnostics.iter().any(|d| d.severity == "error");
        if req.apply && has_semantic_errors {
            if let Some(first_err) = diagnostics.iter().find(|d| d.severity == "error") {
                return Err(RuntimeError::Internal(format!(
                    "line {}, column {}: {}",
                    first_err.line, first_err.column, first_err.message
                )));
            }
        }

        let strat_str: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
        let strategy = bosc_ir::StorageStrategyIR::from_str_loose(
            strat_str.as_deref().unwrap_or("shared_rls"),
        );

        let comp_rows = sqlx::query("SELECT code FROM \"_bos_companies\" WHERE is_active = true")
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        let companies: Vec<String> = comp_rows
            .iter()
            .filter_map(|r| r.try_get::<String, _>("code").ok())
            .collect();

        let generator = PostgresGenerator::with_strategy(strategy);

        let mut compiled_ddl = String::new();
        let mut tables_extended = Vec::new();
        let mut tables_created = Vec::new();
        let mut codeunits_registered = Vec::new();

        for ext in &model_ir.table_extensions {
            let ddl = generator.generate_table_extension_ddl(ext, &companies);
            compiled_ddl.push_str(&ddl);
            compiled_ddl.push('\n');
            tables_extended.push(format!("{} (extends {})", ext.name, ext.target_table));
        }

        compiled_ddl.push_str(&generator.generate_tables(&model_ir.tables));
        tables_created.extend(model_ir.tables.iter().map(|tbl| tbl.name.clone()));

        for sm in &model_ir.state_machines {
            let ddl = generator.generate_state_machine_infrastructure(sm);
            compiled_ddl.push_str(&ddl);
            compiled_ddl.push('\n');
        }

        // BOS-AMDP: Generate PL/pgSQL stored procedures for pushdown Codeunit methods
        for cu in &model_ir.codeunits {
            for m in &cu.methods {
                if (m.is_pushdown || cu.is_pushdown) && bosc_codegen_db::pushdown::body(m).is_ok() {
                    let proc_ddl = generator.generate_plpgsql_procedure(&cu.name, m);
                    compiled_ddl.push_str(&proc_ddl);
                    compiled_ddl.push('\n');
                }
            }
        }

        for v in &model_ir.views {
            let v_ddl = generator.generate_view_ddl(v);
            compiled_ddl.push_str(&v_ddl);
            compiled_ddl.push('\n');
        }

        for ps in &model_ir.permission_sets {
            let ps_ddl = generator.generate_permission_set_ddl(ps);
            compiled_ddl.push_str(&ps_ddl);
            compiled_ddl.push('\n');
        }

        for q in &model_ir.queries {
            let q_ddl = generator.generate_query_catalog_ddl(q);
            compiled_ddl.push_str(&q_ddl);
            compiled_ddl.push('\n');
        }

        let mut schema_report = None;
        let mut structure_warning: Option<String> = None;
        if req.apply {
            for ext in &model_ir.table_extensions {
                let mut ext_def_ir = serde_json::to_value(ext).unwrap_or_default();
                if let Some(m) = ext_def_ir.as_object_mut() {
                    m.insert(
                        "source_script".to_string(),
                        serde_json::Value::String(script.to_string()),
                    );
                }
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&ext_def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        'tableextension', $1, $2, '1.0.0', $3, 'Extension', $4, $5, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                let caption = format!("Extension {} on {}", ext.name, ext.target_table);
                let _ = sqlx::query(upsert_query)
                    .bind(&ext.name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&ext_def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await;
            }

            for tbl in &model_ir.tables {
                let mut stored = tbl.clone();
                let companion_triggers: std::collections::HashSet<String> = model_ir
                    .table_logics
                    .iter()
                    .filter(|logic| logic.target_table.eq_ignore_ascii_case(&tbl.name))
                    .flat_map(|logic| {
                        logic
                            .triggers
                            .iter()
                            .map(|trigger| trigger.name.to_ascii_lowercase())
                    })
                    .chain(
                        model_ir
                            .table_logic_extensions
                            .iter()
                            .filter(|logic| logic.target_table.eq_ignore_ascii_case(&tbl.name))
                            .flat_map(|logic| {
                                logic
                                    .triggers
                                    .iter()
                                    .map(|trigger| trigger.name.to_ascii_lowercase())
                            }),
                    )
                    .collect();
                stored.triggers.retain(|trigger| {
                    !companion_triggers.contains(&trigger.name.to_ascii_lowercase())
                        && !trigger.name.to_ascii_lowercase().starts_with("onvalidate_")
                });

                let mut tbl_def_ir = serde_json::to_value(&stored).unwrap_or_default();
                if let Some(m) = tbl_def_ir.as_object_mut() {
                    m.insert(
                        "source_script".to_string(),
                        serde_json::Value::String(script.to_string()),
                    );
                }
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&tbl_def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());
                let sub_type = tbl_def_ir
                    .get("table_type")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Normal")
                    .to_string();
                let caption = if tbl.caption.trim().is_empty() {
                    tbl.name.clone()
                } else {
                    tbl.caption.clone()
                };

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        'table', $1, $2, '1.0.0', $3, $4, $5, $6, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "caption" = EXCLUDED."caption",
                        "sub_type" = EXCLUDED."sub_type",
                        "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                sqlx::query(upsert_query)
                    .bind(&tbl.name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&sub_type)
                    .bind(&tbl_def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await?;
            }

            for tl in &model_ir.table_logics {
                let mut tl_def_ir = serde_json::to_value(tl).unwrap_or_default();
                if let Some(m) = tl_def_ir.as_object_mut() {
                    m.insert(
                        "source_script".to_string(),
                        serde_json::Value::String(script.to_string()),
                    );
                }
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&tl_def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        'tablelogic', $1, $2, '1.0.0', $3, 'Logic', $4, $5, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                let caption = format!("Logic for table {}", tl.target_table);
                sqlx::query(upsert_query)
                    .bind(&tl.name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&tl_def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await?;
            }

            for tle in &model_ir.table_logic_extensions {
                let mut tle_def_ir = serde_json::to_value(tle).unwrap_or_default();
                if let Some(m) = tle_def_ir.as_object_mut() {
                    m.insert(
                        "source_script".to_string(),
                        serde_json::Value::String(script.to_string()),
                    );
                }
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&tle_def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        'tablelogicextension', $1, $2, '1.0.0', $3, 'LogicExtension', $4, $5, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                let caption = format!("Logic Extension {} for {}", tle.name, tle.target_table);
                sqlx::query(upsert_query)
                    .bind(&tle.name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&tle_def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await?;
            }

            for cu in &model_ir.codeunits {
                let obj_id =
                    crate::execution_engine::ExecutionEngine::register_codeunit(pool, cu.clone())
                        .await?;
                let _ = sqlx::query(
                    "UPDATE \"_bos_meta_objects\" SET \"definition\" = jsonb_set(\"definition\", '{source_script}', to_jsonb($1::text)) WHERE \"object_id\" = $2"
                )
                .bind(script)
                .bind(obj_id)
                .execute(pool)
                .await;
                codeunits_registered.push(cu.name.clone());
            }

            // Persist StateMachines, StateMachineExtensions, Enums, EnumExtensions, Interfaces, PermissionSets, Views, Queries, Pages, PageExtensions, Reports, CustomMetadata
            let mut extra_objects: Vec<(&str, String, String, String, serde_json::Value)> =
                Vec::new();
            for sm in &model_ir.state_machines {
                extra_objects.push((
                    "statemachine",
                    sm.name.clone(),
                    format!("State Machine {} for {}", sm.name, sm.target_table),
                    "Workflow".to_string(),
                    serde_json::to_value(sm).unwrap_or_default(),
                ));
            }
            for sme in &model_ir.state_machine_extensions {
                extra_objects.push((
                    "statemachineextension",
                    sme.name.clone(),
                    format!(
                        "State Machine Extension {} extends {}",
                        sme.name, sme.target_state_machine
                    ),
                    "WorkflowExtension".to_string(),
                    serde_json::to_value(sme).unwrap_or_default(),
                ));
                // Also merge states and transitions into the target statemachine in _bos_meta_objects if it exists
                if let Ok(Some(row)) = sqlx::query(
                    "SELECT object_id, definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND LOWER(object_name) = LOWER($1) LIMIT 1"
                )
                .bind(&sme.target_state_machine)
                .fetch_optional(pool)
                .await
                {
                    if let (Ok(sm_id), Ok(mut sm_def)) = (row.try_get::<Uuid, _>("object_id"), row.try_get::<serde_json::Value, _>("definition")) {
                        if let Some(states) = sm_def.get_mut("states").and_then(|v| v.as_array_mut()) {
                            for st in &sme.states {
                                if !states.iter().any(|ex| ex.get("name").and_then(|n| n.as_str()) == Some(&st.name)) {
                                    states.push(serde_json::to_value(st).unwrap_or_default());
                                }
                            }
                        }
                        if let Some(transitions) = sm_def.get_mut("transitions").and_then(|v| v.as_array_mut()) {
                            for tr in &sme.transitions {
                                if !transitions.iter().any(|ex| ex.get("name").and_then(|n| n.as_str()) == Some(&tr.name)) {
                                    transitions.push(serde_json::to_value(tr).unwrap_or_default());
                                }
                            }
                        }
                        let _ = sqlx::query("UPDATE \"_bos_meta_objects\" SET \"definition\" = $1, \"updated_at\" = NOW() WHERE \"object_id\" = $2")
                            .bind(&sm_def)
                            .bind(sm_id)
                            .execute(pool)
                            .await;
                    }
                }
            }
            for en in &model_ir.enums {
                extra_objects.push((
                    "enum",
                    en.name.clone(),
                    en.name.clone(),
                    "Enum".to_string(),
                    serde_json::to_value(en).unwrap_or_default(),
                ));
            }
            for ene in &model_ir.enum_extensions {
                extra_objects.push((
                    "enumextension",
                    ene.name.clone(),
                    format!("Enum Extension {} extends {}", ene.name, ene.target_enum),
                    "EnumExtension".to_string(),
                    serde_json::to_value(ene).unwrap_or_default(),
                ));
                if let Ok(Some(row)) = sqlx::query(
                    "SELECT object_id, definition FROM \"_bos_meta_objects\" WHERE object_type = 'enum' AND LOWER(object_name) = LOWER($1) LIMIT 1"
                )
                .bind(&ene.target_enum)
                .fetch_optional(pool)
                .await
                {
                    if let (Ok(en_id), Ok(mut en_def)) = (row.try_get::<Uuid, _>("object_id"), row.try_get::<serde_json::Value, _>("definition")) {
                        if let Some(members) = en_def.get_mut("members").and_then(|v| v.as_array_mut()) {
                            for m in &ene.members {
                                if !members.iter().any(|ex| ex.get("name").and_then(|n| n.as_str()) == Some(&m.name)) {
                                    members.push(serde_json::to_value(m).unwrap_or_default());
                                }
                            }
                        }
                        let _ = sqlx::query("UPDATE \"_bos_meta_objects\" SET \"definition\" = $1, \"updated_at\" = NOW() WHERE \"object_id\" = $2")
                            .bind(&en_def)
                            .bind(en_id)
                            .execute(pool)
                            .await;
                    }
                }
            }
            for iface in &model_ir.interfaces {
                extra_objects.push((
                    "interface",
                    iface.name.clone(),
                    iface.name.clone(),
                    "Interface".to_string(),
                    serde_json::to_value(iface).unwrap_or_default(),
                ));
            }
            for ps in &model_ir.permission_sets {
                extra_objects.push((
                    "permissionset",
                    ps.name.clone(),
                    ps.caption.clone().unwrap_or_else(|| ps.name.clone()),
                    "Security".to_string(),
                    serde_json::to_value(ps).unwrap_or_default(),
                ));
            }
            for vw in &model_ir.views {
                extra_objects.push((
                    "view",
                    vw.name.clone(),
                    vw.name.clone(),
                    if vw.is_materialized {
                        "Materialized".to_string()
                    } else {
                        "Standard".to_string()
                    },
                    serde_json::to_value(vw).unwrap_or_default(),
                ));
            }
            for qr in &model_ir.queries {
                extra_objects.push((
                    "query",
                    qr.name.clone(),
                    qr.caption.clone().unwrap_or_else(|| qr.name.clone()),
                    "Analytical".to_string(),
                    serde_json::to_value(qr).unwrap_or_default(),
                ));
            }
            for pg in &model_ir.pages {
                extra_objects.push((
                    "page",
                    pg.name.clone(),
                    pg.caption.clone().unwrap_or_else(|| pg.name.clone()),
                    pg.page_type.clone(),
                    serde_json::to_value(pg).unwrap_or_default(),
                ));
            }
            for pge in &model_ir.page_extensions {
                extra_objects.push((
                    "pageextension",
                    pge.name.clone(),
                    format!("Page Extension {} on {}", pge.name, pge.target_page),
                    "PageExtension".to_string(),
                    serde_json::to_value(pge).unwrap_or_default(),
                ));
            }
            for rp in &model_ir.reports {
                extra_objects.push((
                    "report",
                    rp.name.clone(),
                    rp.caption.clone().unwrap_or_else(|| rp.name.clone()),
                    "Report".to_string(),
                    serde_json::to_value(rp).unwrap_or_default(),
                ));
            }
            for cm in &model_ir.custom_metadata {
                extra_objects.push((
                    "custommetadata",
                    cm.name.clone(),
                    cm.caption.clone().unwrap_or_else(|| cm.name.clone()),
                    "Metadata".to_string(),
                    serde_json::to_value(cm).unwrap_or_default(),
                ));
            }
            for mn in &model_ir.menus {
                extra_objects.push((
                    "menu",
                    mn.name.clone(),
                    mn.label.clone().unwrap_or_else(|| mn.name.clone()),
                    "Menu".to_string(),
                    serde_json::to_value(mn).unwrap_or_default(),
                ));
            }

            for (obj_type, obj_name, caption, sub_type, mut def_ir) in extra_objects {
                if let Some(obj_map) = def_ir.as_object_mut() {
                    obj_map.insert(
                        "source_script".to_string(),
                        serde_json::Value::String(script.to_string()),
                    );
                }
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&def_ir)
                    .unwrap_or_default()
                    .hash(&mut hasher);
                let checksum = format!("{:016x}", hasher.finish());

                let upsert_query = r#"
                    INSERT INTO "_bos_meta_objects" (
                        "object_type", "object_name", "package_name", "version", "caption",
                        "sub_type", "definition", "checksum", "status", "updated_at"
                    ) VALUES (
                        $1, $2, $3, '1.0.0', $4, $5, $6, $7, 'active', NOW()
                    )
                    ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
                    SET "caption" = EXCLUDED."caption",
                        "sub_type" = EXCLUDED."sub_type",
                        "definition" = EXCLUDED."definition",
                        "checksum" = EXCLUDED."checksum",
                        "updated_at" = NOW();
                "#;
                let _ = sqlx::query(upsert_query)
                    .bind(obj_type)
                    .bind(&obj_name)
                    .bind(pkg)
                    .bind(&caption)
                    .bind(&sub_type)
                    .bind(&def_ir)
                    .bind(&checksum)
                    .execute(pool)
                    .await;
            }

            match SchemaSync::sync(
                pool,
                &SchemaSyncRequest {
                    package_name: pkg.to_string(),
                    source: "compile".to_string(),
                    tables: model_ir.tables.clone(),
                    extensions: model_ir.table_extensions.clone(),
                    ddl: compiled_ddl.clone(),
                    applied_by: None,
                },
                &SchemaSyncOptions {
                    allow_destructive: req.allow_destructive,
                    dry_run: false,
                },
            )
            .await
            {
                Ok(report) => schema_report = Some(report),
                Err(err) => {
                    structure_warning = Some(format!(
                        "The script was saved, but the database structure was not updated: {err}"
                    ));
                }
            }
        }

        let compiled_ir = serde_json::to_value(&model_ir).unwrap_or_default();
        let message = if has_semantic_errors {
            if let Some(first_err) = diagnostics.iter().find(|d| d.severity == "error") {
                format!(
                    "line {}, column {}: {}",
                    first_err.line, first_err.column, first_err.message
                )
            } else {
                "Semantic validation errors detected".to_string()
            }
        } else if let Some(warning) = structure_warning {
            warning
        } else if req.apply {
            "Successfully compiled and applied BOS-Script to database".to_string()
        } else {
            "Successfully compiled BOS-Script (preview)".to_string()
        };

        Ok(CompileScriptResponse {
            success: !has_semantic_errors,
            compiled_ddl: compiled_ddl.trim().to_string(),
            compiled_ir,
            tables_extended,
            tables_created,
            codeunits_registered,
            applied: req.apply && !has_semantic_errors,
            message,
            diagnostics,
            schema_sync: schema_report,
        })
    }

    /// Generate live catalog-aware TypeScript ambient declarations (.d.ts) for Monaco IntelliSense
    pub async fn generate_live_ambient_dts(pool: &PgPool) -> String {
        let mut dts = String::from(crate::quickjs_engine::QuickJsEngine::generate_ambient_dts());
        dts.push_str(
            "\n// ============================================================================\n",
        );
        dts.push_str(
            "// Live Catalog Schema Declarations (Auto-Generated from _bos_meta_objects)\n",
        );
        dts.push_str(
            "// ============================================================================\n\n",
        );

        let rows = sqlx::query(
            r#"SELECT object_type, object_name, caption, definition FROM "_bos_meta_objects" WHERE status = 'active' ORDER BY object_type, object_name"#
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let mut table_names: Vec<String> = Vec::new();
        let mut seen_tables: std::collections::HashSet<String> = std::collections::HashSet::new();

        for row in &rows {
            let obj_type: String = row.try_get("object_type").unwrap_or_default();
            let obj_name: String = row.try_get("object_name").unwrap_or_default();
            let caption: Option<String> = row.try_get("caption").ok();
            let def: serde_json::Value = row.try_get("definition").unwrap_or_default();

            if obj_name.is_empty() || !obj_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }

            if obj_type == "table" {
                if !seen_tables.insert(obj_name.clone()) {
                    continue;
                }
                table_names.push(obj_name.clone());
                if let Some(cap) = caption {
                    dts.push_str(&format!("/** Table `{}`: {} */\n", obj_name, cap));
                }
                dts.push_str(&format!("declare interface {}Record {{\n", obj_name));
                if let Some(cols) = def.get("columns").and_then(|c| c.as_array()) {
                    for col in cols {
                        let col_name = col.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                        if col_name.is_empty()
                            || !col_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            continue;
                        }
                        let sql_type = col
                            .get("sql_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("TEXT");
                        let is_nullable = col
                            .get("is_nullable")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let col_cap = col
                            .get("caption")
                            .and_then(|v| v.as_str())
                            .unwrap_or(col_name);
                        let ts_type = Self::sql_type_to_ts(sql_type);
                        let opt = if is_nullable { "?" } else { "" };
                        dts.push_str(&format!("    /** {} ({}) */\n", col_cap, sql_type));
                        dts.push_str(&format!("    {}{}: {};\n", col_name, opt, ts_type));
                    }
                }
                if let Some(ffs) = def.get("flow_fields").and_then(|f| f.as_array()) {
                    for ff in ffs {
                        let ff_name = ff.get("name").and_then(|v| v.as_str()).unwrap_or_default();
                        if !ff_name.is_empty()
                            && ff_name.chars().all(|c| c.is_alphanumeric() || c == '_')
                        {
                            dts.push_str(&format!("    /** FlowField `{}` */\n", ff_name));
                            dts.push_str(&format!("    readonly {}?: number;\n", ff_name));
                        }
                    }
                }
                dts.push_str("    testField(field: string, expectedValue?: any): void;\n");
                dts.push_str("    calcFields(...fields: string[]): void;\n");
                dts.push_str("    validate(field: string, newValue?: any): this;\n");
                dts.push_str("    save(): this;\n");
                dts.push_str("    insert(): this;\n");
                dts.push_str("    modify(): this;\n");
                dts.push_str("    [field: string]: any;\n");
                dts.push_str("}\n\n");

                dts.push_str(&format!(
                    "declare type {} = {}Record;\n",
                    obj_name, obj_name
                ));
                dts.push_str(&format!(
                    "declare const {}: TableModel<{}Record>;\n\n",
                    obj_name, obj_name
                ));
            } else if obj_type == "enum" {
                if let Some(members) = def.get("members").and_then(|m| m.as_array()) {
                    let literals: Vec<String> = members
                        .iter()
                        .filter_map(|m| {
                            m.get("name")
                                .and_then(|v| v.as_str())
                                .or_else(|| m.as_str())
                        })
                        .map(|s| format!("\"{}\"", s))
                        .collect();
                    if !literals.is_empty() {
                        dts.push_str(&format!(
                            "declare type {}Enum = {};\n\n",
                            obj_name,
                            literals.join(" | ")
                        ));
                    }
                }
            }
        }

        dts.push_str("declare interface BosModelsRegistry {\n");
        for tbl in &table_names {
            dts.push_str(&format!("    {}: TableModel<{}Record>;\n", tbl, tbl));
        }
        dts.push_str("    [tableName: string]: TableModel<any>;\n");
        dts.push_str("}\n\n");
        dts.push_str("declare const models: BosModelsRegistry;\n");
        dts
    }

    fn sql_type_to_ts(sql_type: &str) -> &'static str {
        let up = sql_type.to_ascii_uppercase();
        if up.contains("UUID") {
            "UUID"
        } else if up.contains("INT")
            || up.contains("NUMERIC")
            || up.contains("DECIMAL")
            || up.contains("FLOAT")
            || up.contains("DOUBLE")
        {
            "number"
        } else if up.contains("BOOL") {
            "boolean"
        } else if up.contains("JSON") {
            "Record<string, any>"
        } else {
            "string"
        }
    }
}
