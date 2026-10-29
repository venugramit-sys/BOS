use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::time::Instant;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub size_pretty: String,
    pub size_bytes: i64,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStats {
    pub table_name: String,
    pub table_type: String,
    pub row_count: i64,
    pub total_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseDetails {
    pub name: String,
    pub size_pretty: String,
    pub size_bytes: i64,
    pub connection_count: i64,
    pub is_current: bool,
    pub is_provisioned: bool,
    pub kernel_version: Option<String>,
    pub storage_strategy: Option<String>,
    pub meta_objects_count: Option<i64>,
    pub tables: Vec<TableStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub success: bool,
    pub latency_ms: u64,
    pub server_version: Option<String>,
    pub database_name: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    #[serde(default = "default_true")]
    pub init_extensions: bool,
    #[serde(default = "default_true")]
    pub provision_system: bool,
    pub storage_strategy: Option<String>,
    // Optional initial tenant created at database provisioning
    pub tenant_id: Option<Uuid>,
    pub tenant_code: Option<String>,
    pub tenant_name: Option<String>,
    // Optional initial company created at database provisioning
    pub company_id: Option<Uuid>,
    pub company_code: Option<String>,
    pub company_name: Option<String>,
    // Optional base package importation at database creation
    #[serde(default = "default_true")]
    pub import_base_packages: bool,
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantInfo {
    pub tenant_id: Uuid,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInfo {
    pub company_id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub database: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCompanyRequest {
    pub database: Option<String>,
    pub company_id: Option<Uuid>,
    pub tenant_id: Option<Uuid>,
    pub code: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CloneCompanyRequest {
    pub database: Option<String>,
    pub source_code: String,
    pub target_code: String,
    pub target_name: String,
    pub tenant_id: Option<Uuid>,
    #[serde(default)]
    pub copy_data: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SchemaDriftReport {
    pub status: String,
    pub total_tables: usize,
    pub unindexed_foreign_keys: Vec<String>,
    pub issues_count: usize,
    pub checked_at: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct SwitchDatabaseRequest {
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TestConnectionRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManageDatabaseRequest {
    pub action: String,
    pub new_name: Option<String>,
    pub table: Option<String>,
    pub sql: Option<String>,
    pub storage_strategy: Option<String>,
    pub full: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDatabaseRequest {
    pub new_name: Option<String>,
    pub storage_strategy: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropDatabaseQuery {
    pub force: Option<bool>,
}

pub struct DatabaseManager;

impl DatabaseManager {
    /// Validates database identifier name to prevent SQL injection
    pub fn validate_db_name(name: &str) -> Result<(), RuntimeError> {
        if name.is_empty() || name.len() > 63 {
            return Err(RuntimeError::DatabaseError(sqlx::Error::Configuration(
                "Database name must be between 1 and 63 characters".into(),
            )));
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RuntimeError::DatabaseError(sqlx::Error::Configuration(
                "Database name must contain only alphanumeric characters, underscores, or dashes"
                    .into(),
            )));
        }
        let lower = name.to_lowercase();
        if lower == "template0" || lower == "template1" {
            return Err(RuntimeError::SecurityError(
                "Cannot modify or drop template databases".into(),
            ));
        }
        Ok(())
    }

    /// Extract database name from connection URL
    pub fn extract_database_name(url: &str) -> String {
        let clean = if let Some(q_pos) = url.find('?') {
            &url[..q_pos]
        } else {
            url
        };
        if let Some(slash_pos) = clean.rfind('/') {
            clean[slash_pos + 1..].to_string()
        } else {
            "postgres".to_string()
        }
    }

    /// Replace database name in connection URL
    pub fn replace_database_name(url: &str, new_db: &str) -> String {
        if let Some(q_pos) = url.find('?') {
            let base = &url[..q_pos];
            let query = &url[q_pos..];
            if let Some(slash_pos) = base.rfind('/') {
                format!("{}/{}{}", &base[..slash_pos], new_db, query)
            } else {
                format!("{}/{}{}", base, new_db, query)
            }
        } else if let Some(slash_pos) = url.rfind('/') {
            format!("{}/{}", &url[..slash_pos], new_db)
        } else {
            format!("{}/{}", url, new_db)
        }
    }

    /// List all databases on the PostgreSQL instance
    pub async fn list_databases(
        pool: &PgPool,
        current_db: &str,
    ) -> Result<Vec<DatabaseInfo>, RuntimeError> {
        let query = r#"
            SELECT 
                datname::text as name, 
                CASE 
                    WHEN datname = current_database() THEN COALESCE(pg_size_pretty(pg_database_size(datname)), '0 bytes')
                    ELSE 'Cached'
                END as size_pretty,
                CASE 
                    WHEN datname = current_database() THEN COALESCE(pg_database_size(datname), 0)
                    ELSE 0
                END as size_bytes
            FROM pg_database 
            WHERE datistemplate = false 
            ORDER BY datname;
        "#;

        let rows = sqlx::query(query).fetch_all(pool).await?;
        let mut dbs = Vec::new();

        for row in rows {
            let name: String = row.try_get("name")?;
            let size_pretty: String = row
                .try_get("size_pretty")
                .unwrap_or_else(|_| "0 bytes".to_string());
            let size_bytes: i64 = row.try_get("size_bytes").unwrap_or(0);
            let is_current = name == current_db;

            dbs.push(DatabaseInfo {
                name,
                size_pretty,
                size_bytes,
                is_current,
            });
        }

        Ok(dbs)
    }

    /// Inspect details and table statistics of a specific database
    pub async fn get_database_details(
        current_url: &str,
        target_name: &str,
        current_db: &str,
    ) -> Result<DatabaseDetails, RuntimeError> {
        Self::validate_db_name(target_name)?;

        let target_url = Self::replace_database_name(current_url, target_name);
        let target_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&target_url)
            .await?;

        // 1. Fetch size and connection count
        let meta_query = r#"
            SELECT 
                pg_size_pretty(pg_database_size($1)) as size_pretty,
                pg_database_size($1) as size_bytes,
                (SELECT count(*)::bigint FROM pg_stat_activity WHERE datname = $1) as connection_count;
        "#;

        let meta_row = sqlx::query(meta_query)
            .bind(target_name)
            .fetch_one(&target_pool)
            .await?;

        let size_pretty: String = meta_row
            .try_get("size_pretty")
            .unwrap_or_else(|_| "0 bytes".to_string());
        let size_bytes: i64 = meta_row.try_get("size_bytes").unwrap_or(0);
        let connection_count: i64 = meta_row.try_get("connection_count").unwrap_or(1);

        // 2. Fetch tables with row count estimates and sizes
        let tables_query = r#"
            SELECT 
                t.table_name::text as table_name,
                t.table_type::text as table_type,
                COALESCE(s.n_live_tup, 0)::bigint as row_count,
                pg_size_pretty(pg_total_relation_size(quote_ident(t.table_name)::regclass)) as total_size
            FROM information_schema.tables t
            LEFT JOIN pg_stat_user_tables s ON s.relname = t.table_name
            WHERE t.table_schema = 'public'
            ORDER BY t.table_name;
        "#;

        let table_rows = sqlx::query(tables_query).fetch_all(&target_pool).await?;
        let mut tables = Vec::new();

        for r in table_rows {
            tables.push(TableStats {
                table_name: r.try_get("table_name")?,
                table_type: r.try_get("table_type")?,
                row_count: r.try_get("row_count").unwrap_or(0),
                total_size: r
                    .try_get("total_size")
                    .unwrap_or_else(|_| "0 bytes".to_string()),
            });
        }

        let sys_status = crate::meta_manager::MetaManager::get_system_status(&target_pool)
            .await
            .unwrap_or(crate::meta_manager::SystemProvisionStatus {
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

        target_pool.close().await;

        Ok(DatabaseDetails {
            name: target_name.to_string(),
            size_pretty,
            size_bytes,
            connection_count,
            is_current: target_name == current_db,
            is_provisioned: sys_status.is_provisioned,
            kernel_version: sys_status.kernel_version,
            storage_strategy: sys_status.storage_strategy,
            meta_objects_count: if sys_status.is_provisioned {
                Some(sys_status.meta_objects_count)
            } else {
                None
            },
            tables,
        })
    }

    /// Create a new PostgreSQL database with optional extensions and BOS system kernel provisioning
    pub async fn create_database(
        current_url: &str,
        req: &CreateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let name = &req.name;
        Self::validate_db_name(name)?;

        // Connect to maintenance database 'postgres' to issue CREATE DATABASE outside transaction block
        let maint_url = Self::replace_database_name(current_url, "postgres");
        let maint_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&maint_url)
            .await?;

        let create_cmd = format!("CREATE DATABASE \"{}\";", name);
        sqlx::query(&create_cmd).execute(&maint_pool).await?;
        maint_pool.close().await;

        let mut created_tenant_id = None;
        let mut created_company_id = None;

        if req.init_extensions || req.provision_system {
            let new_db_url = Self::replace_database_name(current_url, name);
            if let Ok(new_pool) = PgPoolOptions::new()
                .max_connections(2)
                .connect(&new_db_url)
                .await
            {
                if req.init_extensions {
                    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";")
                        .execute(&new_pool)
                        .await;
                    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";")
                        .execute(&new_pool)
                        .await;
                }
                if req.provision_system {
                    let strat = req.storage_strategy.as_deref().unwrap_or("shared_rls");
                    let _ = crate::meta_manager::MetaManager::provision_system_kernel(
                        &new_pool,
                        Some(strat),
                    )
                    .await?;

                    // If user specified initial tenant details, create them now
                    let has_tenant = req.tenant_code.is_some()
                        || req.tenant_name.is_some()
                        || req.tenant_id.is_some();
                    if has_tenant {
                        let t_id = req.tenant_id.unwrap_or_else(Uuid::now_v7);
                        let t_code = req
                            .tenant_code
                            .clone()
                            .unwrap_or_else(|| "DEFAULT".to_string());
                        let t_name = req
                            .tenant_name
                            .clone()
                            .unwrap_or_else(|| "Default Tenant".to_string());

                        let _ = sqlx::query(
                            "INSERT INTO \"_bos_tenants\" (\"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\") VALUES ($1, $2, $3, true, NOW()) ON CONFLICT (\"tenant_id\") DO NOTHING;"
                        )
                        .bind(t_id)
                        .bind(&t_code)
                        .bind(&t_name)
                        .execute(&new_pool)
                        .await;
                        created_tenant_id = Some(t_id);
                    }

                    // If user specified initial company details, create them (with or without tenant)
                    let has_company = req.company_code.is_some()
                        || req.company_name.is_some()
                        || req.company_id.is_some();
                    if has_company {
                        let c_id = req.company_id.unwrap_or_else(Uuid::now_v7);
                        let c_code = req
                            .company_code
                            .clone()
                            .unwrap_or_else(|| "DEFAULT".to_string());
                        let c_name = req
                            .company_name
                            .clone()
                            .unwrap_or_else(|| "Default Company".to_string());

                        let _ = sqlx::query(
                            "INSERT INTO \"_bos_companies\" (\"company_id\", \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\") VALUES ($1, $2, $3, $4, true, NOW()) ON CONFLICT (\"company_id\") DO NOTHING;"
                        )
                        .bind(c_id)
                        .bind(created_tenant_id)
                        .bind(&c_code)
                        .bind(&c_name)
                        .execute(&new_pool)
                        .await;
                        created_company_id = Some(c_id);

                        if strat == "schema_per_company" {
                            let schema =
                                format!("comp_{}", c_code.to_lowercase().replace('-', "_"));
                            let _ = sqlx::raw_sql(&format!(
                                "CREATE SCHEMA IF NOT EXISTS \"{}\";",
                                schema
                            ))
                            .execute(&new_pool)
                            .await;
                        }
                    }

                    // 4. Import base packages if requested (ADR 12 & ADR 24: Enterprise Security Package)
                    if req.import_base_packages {
                        let mut pkgs_to_install = Vec::new();
                        // Always include canonical bos.core mega package by default
                        pkgs_to_install.push("bos.core".to_string());

                        // Add any additional packages specified in request
                        for p in &req.packages {
                            let p_trimmed = p.trim().to_string();
                            if !p_trimmed.is_empty()
                                && !pkgs_to_install
                                    .iter()
                                    .any(|existing| existing.eq_ignore_ascii_case(&p_trimmed))
                            {
                                pkgs_to_install.push(p_trimmed);
                            }
                        }

                        for pkg_id in &pkgs_to_install {
                            if let Some(bundle) = Self::load_base_package_bundle(pkg_id).await {
                                match crate::package_manager::PackageManager::install_package(
                                    &new_pool,
                                    bundle,
                                    Some(strat),
                                    false,
                                )
                                .await
                                {
                                    Ok(res) => {
                                        tracing::info!(
                                            "Successfully imported base package '{}' ({} objects) into database '{}'",
                                            res.package_id,
                                            res.objects_installed,
                                            name
                                        );
                                    }
                                    Err(err) => {
                                        tracing::warn!(
                                            "Failed to import base package '{}' into database '{}': {}",
                                            pkg_id,
                                            name,
                                            err
                                        );
                                    }
                                }
                            } else {
                                tracing::warn!(
                                    "Base package bundle for '{}' not found on system during database creation",
                                    pkg_id
                                );
                            }
                        }
                    }
                }
                new_pool.close().await;
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "name": name,
            "provisioned": req.provision_system,
            "storage_strategy": req.storage_strategy.as_deref().unwrap_or("shared_rls"),
            "tenant_id": created_tenant_id,
            "company_id": created_company_id,
            "packages_imported": req.import_base_packages,
            "message": format!("Database '{}' created successfully", name)
        }))
    }

    /// Load or compile a base package bundle for database creation
    pub async fn load_base_package_bundle(pkg_id: &str) -> Option<bosc_ir::PackageBundleIR> {
        let pkg_id_clean = pkg_id.trim();

        // 1. Look for pre-compiled .bospkg binary bundle in packages/
        let mut bospkg_candidates = vec![
            format!("packages/{}.bospkg", pkg_id_clean),
            format!("packages/{}/{}.bospkg", pkg_id_clean, pkg_id_clean),
            format!("./packages/{}.bospkg", pkg_id_clean),
            format!("./packages/{}/{}.bospkg", pkg_id_clean, pkg_id_clean),
            format!("../../packages/{}.bospkg", pkg_id_clean),
            format!("../../packages/{}/{}.bospkg", pkg_id_clean, pkg_id_clean),
            format!("../packages/{}.bospkg", pkg_id_clean),
            format!("../packages/{}/{}.bospkg", pkg_id_clean, pkg_id_clean),
        ];
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let p = std::path::Path::new(&manifest_dir);
            if let Some(parent) = p.parent() {
                if let Some(root) = parent.parent() {
                    bospkg_candidates.push(format!(
                        "{}/packages/{}.bospkg",
                        root.display(),
                        pkg_id_clean
                    ));
                    bospkg_candidates.push(format!(
                        "{}/packages/{}/{}.bospkg",
                        root.display(),
                        pkg_id_clean,
                        pkg_id_clean
                    ));
                }
            }
        }

        for path in &bospkg_candidates {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(bundle) = serde_json::from_slice::<bosc_ir::PackageBundleIR>(&bytes) {
                    return Some(bundle);
                }
            }
        }

        // 2. If pre-compiled .bospkg does not exist, compile dynamically from package source
        let pkg_suffix = pkg_id_clean.split('.').last().unwrap_or(pkg_id_clean);
        let mut package_bos_candidates = vec![
                        format!("packages/{}/{}.bos", pkg_id_clean, pkg_suffix),
            format!("packages/{}/package.bos", pkg_id_clean),
            format!("packages/{}.bos", pkg_id_clean),
                        format!("../../packages/{}/{}.bos", pkg_id_clean, pkg_suffix),
            format!("../../packages/{}/package.bos", pkg_id_clean),
                        format!("../packages/{}/{}.bos", pkg_id_clean, pkg_suffix),
            format!("../packages/{}/package.bos", pkg_id_clean),
        ];
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let p = std::path::Path::new(&manifest_dir);
            if let Some(parent) = p.parent() {
                if let Some(root) = parent.parent() {
                    package_bos_candidates.push(format!(
                        "{}/packages/{}/package.bos",
                        root.display(),
                        pkg_id_clean
                    ));
                }
            }
        }

        for path in &package_bos_candidates {
            if let Ok(script) = std::fs::read_to_string(path) {
                let mut lexer = bosc_syntax::lexer::Lexer::new(&script);
                if let Ok(tokens) = lexer.tokenize() {
                    let mut parser =
                        bosc_syntax::parser::Parser::with_source(tokens, script.clone());
                    if let Ok(program) = parser.parse_program() {
                        let analyzer = bosc_analyzer::SemanticAnalyzer::new(pkg_id_clean);
                        if let Ok(ir_model) = analyzer.analyze_and_lower(&program) {
                            let mut objects = Vec::new();
                            for table in ir_model.tables {
                                if let Ok(val) = serde_json::to_value(table) {
                                    objects.push(val);
                                }
                            }
                            for cu in ir_model.codeunits {
                                if let Ok(val) = serde_json::to_value(cu) {
                                    objects.push(val);
                                }
                            }
                            for ps in ir_model.permission_sets {
                                if let Ok(val) = serde_json::to_value(ps) {
                                    objects.push(val);
                                }
                            }

                            let manifest = bosc_ir::PackageManifestIR {
                                package_id: pkg_id_clean.to_string(),
                                name: format!("BOS Package {}", pkg_id_clean),
                                version: "1.0.0".to_string(),
                                publisher: "BOS Core Kernel".to_string(),
                                dependencies: Vec::new(),
                                optional_features: Vec::new(),
                            };

                            if let Ok(bundle) = crate::package_manager::PackageManager::build_bundle(
                                manifest, objects,
                            ) {
                                return Some(bundle);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// List all tenants registered in active database
    pub async fn list_tenants(pool: &PgPool) -> Result<Vec<TenantInfo>, RuntimeError> {
        let rows = match sqlx::query("SELECT tenant_id, code, name, is_active, created_at FROM \"_bos_tenants\" ORDER BY created_at")
            .fetch_all(pool)
            .await
        {
            Ok(r) => r,
            Err(sqlx::Error::Database(ref db_err)) if db_err.code().as_deref() == Some("42P01") => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(RuntimeError::DatabaseError(e)),
        };
        let mut list = Vec::new();
        for r in rows {
            list.push(TenantInfo {
                tenant_id: r.try_get("tenant_id")?,
                code: r.try_get("code")?,
                name: r.try_get("name")?,
                is_active: r.try_get("is_active")?,
                created_at: r.try_get("created_at")?,
            });
        }
        Ok(list)
    }

    /// Create a new tenant in active database
    pub async fn create_tenant(
        pool: &PgPool,
        req: CreateTenantRequest,
    ) -> Result<TenantInfo, RuntimeError> {
        let tenant_id = req.tenant_id.unwrap_or_else(Uuid::now_v7);
        let row = sqlx::query(
            "INSERT INTO \"_bos_tenants\" (\"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\") VALUES ($1, $2, $3, true, NOW()) RETURNING \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\";"
        )
        .bind(tenant_id)
        .bind(&req.code)
        .bind(&req.name)
        .fetch_one(pool)
        .await?;

        Ok(TenantInfo {
            tenant_id: row.try_get("tenant_id")?,
            code: row.try_get("code")?,
            name: row.try_get("name")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
        })
    }

    /// List companies registered in active database
    pub async fn list_companies(
        pool: &PgPool,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<CompanyInfo>, RuntimeError> {
        let res = if let Some(t_id) = tenant_id {
            sqlx::query("SELECT company_id, tenant_id, code, name, is_active, created_at FROM \"_bos_companies\" WHERE tenant_id = $1 ORDER BY created_at")
                .bind(t_id)
                .fetch_all(pool)
                .await
        } else {
            sqlx::query("SELECT company_id, tenant_id, code, name, is_active, created_at FROM \"_bos_companies\" ORDER BY created_at")
                .fetch_all(pool)
                .await
        };
        let rows = match res {
            Ok(r) => r,
            Err(sqlx::Error::Database(ref db_err)) if db_err.code().as_deref() == Some("42P01") => {
                return Ok(Vec::new());
            }
            Err(e) => return Err(RuntimeError::DatabaseError(e)),
        };
        let mut list = Vec::new();
        for r in rows {
            list.push(CompanyInfo {
                company_id: r.try_get("company_id")?,
                tenant_id: r.try_get("tenant_id").ok(),
                code: r.try_get("code")?,
                name: r.try_get("name")?,
                is_active: r.try_get("is_active")?,
                created_at: r.try_get("created_at")?,
            });
        }
        Ok(list)
    }

    /// Create a new company under a tenant in active database
    pub async fn create_company(
        pool: &PgPool,
        req: CreateCompanyRequest,
        strategy_str: Option<&str>,
    ) -> Result<CompanyInfo, RuntimeError> {
        let company_id = req.company_id.unwrap_or_else(Uuid::now_v7);
        let row = sqlx::query(
            "INSERT INTO \"_bos_companies\" (\"company_id\", \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\") VALUES ($1, $2, $3, $4, true, NOW()) RETURNING \"company_id\", \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\";"
        )
        .bind(company_id)
        .bind(req.tenant_id)
        .bind(&req.code)
        .bind(&req.name)
        .fetch_one(pool)
        .await?;

        let res = CompanyInfo {
            company_id: row.try_get("company_id")?,
            tenant_id: row.try_get("tenant_id").ok(),
            code: row.try_get("code")?,
            name: row.try_get("name")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
        };

        if strategy_str == Some("schema_per_company") {
            let schema = format!("comp_{}", req.code.to_lowercase().replace('-', "_"));
            let _ = sqlx::raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema))
                .execute(pool)
                .await;
        }

        Ok(res)
    }

    /// Clone an existing company (optionally with ledger transactions)
    pub async fn clone_company(
        pool: &PgPool,
        req: CloneCompanyRequest,
        strategy_str: Option<&str>,
    ) -> Result<CompanyInfo, RuntimeError> {
        Self::validate_db_name(&req.target_code)?;
        let new_company_id = Uuid::now_v7();

        // 1. Insert target company into _bos_companies
        let row = sqlx::query(
            "INSERT INTO \"_bos_companies\" (\"company_id\", \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\") VALUES ($1, $2, $3, $4, true, NOW()) RETURNING \"company_id\", \"tenant_id\", \"code\", \"name\", \"is_active\", \"created_at\";"
        )
        .bind(new_company_id)
        .bind(req.tenant_id)
        .bind(&req.target_code)
        .bind(&req.target_name)
        .fetch_one(pool)
        .await?;

        let res = CompanyInfo {
            company_id: row.try_get("company_id")?,
            tenant_id: row.try_get("tenant_id").ok(),
            code: row.try_get("code")?,
            name: row.try_get("name")?,
            is_active: row.try_get("is_active")?,
            created_at: row.try_get("created_at")?,
        };

        // 2. Physical cloning based on strategy
        if strategy_str == Some("schema_per_company") {
            let src_schema = format!("comp_{}", req.source_code.to_lowercase().replace('-', "_"));
            let tgt_schema = format!("comp_{}", req.target_code.to_lowercase().replace('-', "_"));

            sqlx::raw_sql(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", tgt_schema))
                .execute(pool)
                .await?;

            // Fetch all base tables in source schema
            let tables: Vec<String> = sqlx::query_scalar(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = $1 AND table_type = 'BASE TABLE';"
            )
            .bind(&src_schema)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            for tbl in tables {
                let ddl = format!(
                    "CREATE TABLE IF NOT EXISTS \"{}\".\"{}\" (LIKE \"{}\".\"{}\" INCLUDING ALL);",
                    tgt_schema, tbl, src_schema, tbl
                );
                sqlx::raw_sql(&ddl).execute(pool).await?;

                if req.copy_data {
                    let copy_sql = format!(
                        "INSERT INTO \"{}\".\"{}\" SELECT * FROM \"{}\".\"{}\";",
                        tgt_schema, tbl, src_schema, tbl
                    );
                    let _ = sqlx::raw_sql(&copy_sql).execute(pool).await;
                }
            }
        } else if strategy_str == Some("table_per_company") {
            let src_prefix = format!("{}$", req.source_code.to_lowercase().replace('-', "_"));
            let tgt_prefix = format!("{}$", req.target_code.to_lowercase().replace('-', "_"));

            let tables: Vec<String> = sqlx::query_scalar(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE $1;"
            )
            .bind(format!("{}%", src_prefix))
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            for tbl in tables {
                if let Some(suffix) = tbl.strip_prefix(&src_prefix) {
                    let tgt_tbl = format!("{}{}", tgt_prefix, suffix);
                    let ddl = format!(
                        "CREATE TABLE IF NOT EXISTS \"{}\" (LIKE \"{}\" INCLUDING ALL);",
                        tgt_tbl, tbl
                    );
                    sqlx::raw_sql(&ddl).execute(pool).await?;

                    if req.copy_data {
                        let copy_sql = format!(
                            "INSERT INTO \"{}\" SELECT * FROM \"{}\";",
                            tgt_tbl, tbl
                        );
                        let _ = sqlx::raw_sql(&copy_sql).execute(pool).await;
                    }
                }
            }
        }

        Ok(res)
    }

    /// Drop a company and its physical schemas/tables
    pub async fn drop_company(
        pool: &PgPool,
        code: &str,
        strategy_str: Option<&str>,
    ) -> Result<(), RuntimeError> {
        // Delete company record
        sqlx::query("DELETE FROM \"_bos_companies\" WHERE LOWER(\"code\") = LOWER($1);")
            .bind(code)
            .execute(pool)
            .await?;

        if strategy_str == Some("schema_per_company") {
            let schema = format!("comp_{}", code.to_lowercase().replace('-', "_"));
            let drop_sql = format!("DROP SCHEMA IF EXISTS \"{}\" CASCADE;", schema);
            let _ = sqlx::raw_sql(&drop_sql).execute(pool).await;
        } else if strategy_str == Some("table_per_company") {
            let prefix = format!("{}$", code.to_lowercase().replace('-', "_"));
            let tables: Vec<String> = sqlx::query_scalar(
                "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE $1;"
            )
            .bind(format!("{}%", prefix))
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            for tbl in tables {
                let _ = sqlx::raw_sql(&format!("DROP TABLE IF EXISTS \"{}\" CASCADE;", tbl))
                    .execute(pool)
                    .await;
            }
        }

        Ok(())
    }

    /// Check for schema drifts and unindexed foreign keys across PostgreSQL tables
    pub async fn check_schema_drift(pool: &PgPool) -> Result<SchemaDriftReport, RuntimeError> {
        let total_tables: i64 = sqlx::query_scalar(
            "SELECT count(*)::bigint FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog', 'information_schema') AND table_type = 'BASE TABLE';"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let unindexed_fks: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT 
                format('%s (%s)', c.conrelid::regclass::text, c.conname) AS issue
            FROM pg_constraint c
            WHERE c.contype = 'f'
              AND NOT EXISTS (
                  SELECT 1 FROM pg_index i 
                  WHERE i.indrelid = c.conrelid 
                    AND (i.indkey::int2[])[0] = (c.conkey)[1]
              )
            LIMIT 20;
            "#
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let issues_count = unindexed_fks.len();
        let status = if issues_count == 0 { "OPTIMAL".to_string() } else { "WARNING_UNINDEXED_FK".to_string() };

        Ok(SchemaDriftReport {
            status,
            total_tables: total_tables as usize,
            unindexed_foreign_keys: unindexed_fks,
            issues_count,
            checked_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Drop a PostgreSQL database (optionally with force termination of open connections)
    pub async fn drop_database(
        current_url: &str,
        name: &str,
        force: bool,
    ) -> Result<(), RuntimeError> {
        Self::validate_db_name(name)?;
        if name.to_lowercase() == "postgres" {
            return Err(RuntimeError::SecurityError(
                "Cannot drop the primary system maintenance database 'postgres'".into(),
            ));
        }

        // Connect to maintenance database 'postgres'
        let maint_url = Self::replace_database_name(current_url, "postgres");
        let maint_pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&maint_url)
            .await?;

        let drop_cmd = if force {
            format!("DROP DATABASE \"{}\" WITH (FORCE);", name)
        } else {
            format!("DROP DATABASE \"{}\";", name)
        };

        sqlx::query(&drop_cmd).execute(&maint_pool).await?;
        maint_pool.close().await;

        Ok(())
    }

    /// Test an arbitrary database connection URL
    pub async fn test_connection(url: &str) -> ConnectionTestResult {
        let start = Instant::now();
        match PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(5))
            .max_connections(1)
            .connect(url)
            .await
        {
            Ok(pool) => {
                let latency_ms = start.elapsed().as_millis() as u64;
                let version_row = sqlx::query("SELECT version() as ver, current_database() as db;")
                    .fetch_one(&pool)
                    .await;

                pool.close().await;

                match version_row {
                    Ok(r) => {
                        let ver: Option<String> = r.try_get("ver").ok();
                        let db: Option<String> = r.try_get("db").ok();
                        ConnectionTestResult {
                            success: true,
                            latency_ms,
                            server_version: ver,
                            database_name: db,
                            message: "Connected successfully to PostgreSQL database".to_string(),
                        }
                    }
                    Err(e) => ConnectionTestResult {
                        success: false,
                        latency_ms,
                        server_version: None,
                        database_name: None,
                        message: format!("Connected but failed executing query: {}", e),
                    },
                }
            }
            Err(e) => ConnectionTestResult {
                success: false,
                latency_ms: start.elapsed().as_millis() as u64,
                server_version: None,
                database_name: None,
                message: format!("Connection failed: {}", e),
            },
        }
    }

    /// Manage database operations (rename, truncate, custom SQL)
    pub async fn manage_database(
        current_url: &str,
        target_name: &str,
        req: ManageDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        Self::validate_db_name(target_name)?;

        match req.action.as_str() {
            "rename" => {
                let new_name = req.new_name.as_deref().ok_or_else(|| {
                    RuntimeError::DatabaseError(sqlx::Error::Configuration(
                        "new_name is required for rename action".into(),
                    ))
                })?;
                Self::validate_db_name(new_name)?;

                let maint_url = Self::replace_database_name(current_url, "postgres");
                let maint_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&maint_url)
                    .await?;

                let rename_cmd = format!(
                    "ALTER DATABASE \"{}\" RENAME TO \"{}\";",
                    target_name, new_name
                );
                sqlx::query(&rename_cmd).execute(&maint_pool).await?;
                maint_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "rename",
                    "old_name": target_name,
                    "new_name": new_name,
                    "message": format!("Database '{}' renamed to '{}'", target_name, new_name)
                }))
            }
            "truncate_table" => {
                let table = req.table.as_deref().ok_or_else(|| {
                    RuntimeError::DatabaseError(sqlx::Error::Configuration(
                        "table is required for truncate_table action".into(),
                    ))
                })?;
                Self::validate_db_name(table)?;

                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let trunc_cmd = format!("TRUNCATE TABLE \"{}\" CASCADE;", table);
                sqlx::query(&trunc_cmd).execute(&target_pool).await?;
                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "truncate_table",
                    "database": target_name,
                    "table": table,
                    "message": format!("Table '{}' truncated in database '{}'", table, target_name)
                }))
            }
            "execute_sql" => {
                let sql = req.sql.as_deref().ok_or_else(|| {
                    RuntimeError::DatabaseError(sqlx::Error::Configuration(
                        "sql statement is required for execute_sql action".into(),
                    ))
                })?;

                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let affected = sqlx::query(sql)
                    .execute(&target_pool)
                    .await?
                    .rows_affected();
                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "execute_sql",
                    "database": target_name,
                    "rows_affected": affected,
                    "message": "SQL executed successfully"
                }))
            }
            "vacuum" | "vacuum_analyze" => {
                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let cmd = if req.full.unwrap_or(false) {
                    "VACUUM FULL ANALYZE;"
                } else {
                    "VACUUM ANALYZE;"
                };
                sqlx::query(cmd).execute(&target_pool).await?;
                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "vacuum",
                    "database": target_name,
                    "full": req.full.unwrap_or(false),
                    "message": format!("Database '{}' vacuumed and analyzed successfully", target_name)
                }))
            }
            "reindex" => {
                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let cmd = if let Some(ref t) = req.table {
                    Self::validate_db_name(t)?;
                    format!("REINDEX TABLE \"{}\";", t)
                } else {
                    "REINDEX SCHEMA public;".to_string()
                };
                sqlx::query(&cmd).execute(&target_pool).await?;
                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "reindex",
                    "database": target_name,
                    "target": req.table.as_deref().unwrap_or("schema public"),
                    "message": format!("Database '{}' reindexed successfully", target_name)
                }))
            }
            "update_strategy" => {
                let strat = req.storage_strategy.as_deref().ok_or_else(|| {
                    RuntimeError::DatabaseError(sqlx::Error::Configuration(
                        "storage_strategy is required for update_strategy action".into(),
                    ))
                })?;

                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let rows = sqlx::query(
                    "UPDATE \"_bos_sys_info\" SET \"storage_strategy\" = $1 WHERE \"id\" = 'core';",
                )
                .bind(strat)
                .execute(&target_pool)
                .await?
                .rows_affected();

                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "update_strategy",
                    "database": target_name,
                    "storage_strategy": strat,
                    "updated": rows > 0,
                    "message": format!("Storage strategy updated to '{}' for database '{}'", strat, target_name)
                }))
            }
            "reset" => {
                let target_url = Self::replace_database_name(current_url, target_name);
                let target_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&target_url)
                    .await?;

                let _ = sqlx::query(
                    r#"
                    TRUNCATE TABLE "_system_privileged_audit_log", "_bos_meta_objects", "_bos_meta_packages" CASCADE;
                    "#
                )
                .execute(&target_pool)
                .await;

                target_pool.close().await;

                Ok(serde_json::json!({
                    "success": true,
                    "action": "reset",
                    "database": target_name,
                    "message": format!("Database '{}' meta tables reset successfully", target_name)
                }))
            }
            unknown => Err(RuntimeError::DatabaseError(sqlx::Error::Configuration(
                format!("Unknown database management action: {}", unknown).into(),
            ))),
        }
    }

    /// Update database properties (rename, storage strategy, or status)
    pub async fn update_database(
        current_url: &str,
        target_name: &str,
        req: UpdateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        Self::validate_db_name(target_name)?;

        let mut actual_name = target_name.to_string();

        // 1. Handle rename if requested
        if let Some(ref new_name) = req.new_name {
            if new_name != target_name {
                Self::validate_db_name(new_name)?;
                let maint_url = Self::replace_database_name(current_url, "postgres");
                let maint_pool = PgPoolOptions::new()
                    .max_connections(2)
                    .connect(&maint_url)
                    .await?;

                let rename_cmd = format!(
                    "ALTER DATABASE \"{}\" RENAME TO \"{}\";",
                    target_name, new_name
                );
                sqlx::query(&rename_cmd).execute(&maint_pool).await?;
                maint_pool.close().await;
                actual_name = new_name.clone();
            }
        }

        // 2. Handle storage_strategy or status updates if provided
        if req.storage_strategy.is_some() || req.status.is_some() {
            let target_url = Self::replace_database_name(current_url, &actual_name);
            let target_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let _ = sqlx::query(
                r#"
                UPDATE "_bos_sys_info"
                SET "storage_strategy" = COALESCE($1, "storage_strategy"),
                    "status" = COALESCE($2, "status")
                WHERE "id" = 'core';
                "#,
            )
            .bind(req.storage_strategy.as_deref())
            .bind(req.status.as_deref())
            .execute(&target_pool)
            .await;

            target_pool.close().await;
        }

        Ok(serde_json::json!({
            "success": true,
            "database": actual_name,
            "old_name": target_name,
            "new_name": req.new_name,
            "storage_strategy": req.storage_strategy,
            "status": req.status,
            "message": format!("Database '{}' updated successfully", actual_name)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_database_request_defaults() {
        let json = r#"{"name": "crm_dev"}"#;
        let req: CreateDatabaseRequest = serde_json::from_str(json).expect("valid parse");
        assert_eq!(req.name, "crm_dev");
        assert!(req.init_extensions);
        assert!(req.provision_system);
        assert!(req.import_base_packages);
        assert!(req.packages.is_empty());
    }

    #[test]
    fn test_create_database_request_custom_packages() {
        let json = r#"{
            "name": "finance_db",
            "import_base_packages": true,
            "packages": ["bos.security", "bos.ledger"]
        }"#;
        let req: CreateDatabaseRequest = serde_json::from_str(json).expect("valid parse");
        assert_eq!(req.name, "finance_db");
        assert!(req.import_base_packages);
        assert_eq!(req.packages.len(), 2);
        assert_eq!(req.packages[0], "bos.security");
        assert_eq!(req.packages[1], "bos.ledger");
    }

    #[tokio::test]
    async fn test_load_base_package_bundle() {
        let bundle_opt = DatabaseManager::load_base_package_bundle("bos.security").await;
        if let Some(bundle) = bundle_opt {
            assert_eq!(bundle.manifest.package_id, "bos.security");
            assert!(!bundle.objects.is_empty(), "objects should not be empty");
        }
    }
}
