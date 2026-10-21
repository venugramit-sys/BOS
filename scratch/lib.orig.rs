use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

pub mod detector;
pub mod db_manager;
pub mod db_session;
pub mod schema_sync;
pub mod meta_manager;
pub mod package_manager;
pub mod execution_engine;
pub mod quickjs_engine;
pub mod server;
pub mod event_engine;
pub mod wasm_engine;
pub mod sidecar_host;
pub use detector::{DatabaseDetector, DetectedDatabase, ProviderKind};
pub use db_manager::{
    CompanyInfo, ConnectionTestResult, CreateCompanyRequest, CreateDatabaseRequest,
    CreateTenantRequest, DatabaseDetails, DatabaseInfo, DatabaseManager, DropDatabaseQuery,
    ManageDatabaseRequest, SwitchDatabaseRequest, TableStats, TenantInfo, TestConnectionRequest,
    UpdateDatabaseRequest,
};
pub use meta_manager::{
    ApplyTableScriptRequest, ApplyTableScriptResponse, ColumnDefinition, CompileScriptRequest,
    CompileScriptResponse, MetaFieldInfo, MetaManager, MetaObjectDetails, MetaObjectSummary,
    MetaRelationInfo,
    RegisterMetaObjectRequest, RegisterTableExtensionRequest, StateMachineDefinition, SystemProvisionStatus,
};
pub use package_manager::{PackageInstallResult, PackageManager};
pub use execution_engine::{ExecutionEngine, ExecutionResult};
pub use event_engine::{EventRegistry, EventSubscriber};
pub use wasm_engine::WasmEngine;
pub use sidecar_host::{SidecarDescriptor, SidecarHost, SidecarStatus};
pub use server::create_api_router;
pub use db_session::DbSession;
pub use schema_sync::{SchemaSync, SchemaSyncOptions, SchemaSyncReport};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub entity: String,
    #[serde(default)]
    pub select: Option<Vec<String>>,
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
    #[serde(default)]
    pub include: Option<Vec<IncludeRelation>>,
    #[serde(default, alias = "orderBy")]
    pub order_by: Option<Vec<OrderByClause>>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeRelation {
    pub relation: String,
    pub entity: Option<String>,
    #[serde(default, alias = "foreignKey")]
    pub foreign_key: Option<String>,
    pub select: Option<Vec<String>>,
    pub filter: Option<serde_json::Value>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByClause {
    pub field: String,
    #[serde(default)]
    pub desc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityColumnSchema {
    pub name: String,
    pub sql_name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_unique: bool,
    pub default_val: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySchemaResponse {
    pub table_name: String,
    pub physical_table: String,
    pub package_name: Option<String>,
    pub caption: Option<String>,
    pub data_per_company: bool,
    pub columns: Vec<EntityColumnSchema>,
    pub state_machine: Option<serde_json::Value>,
    pub sample_create_payload: serde_json::Value,
    pub sample_batch_payload: Vec<serde_json::Value>,
    pub sample_update_payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityQueryParams {
    pub fields: Option<Vec<String>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub page: Option<i64>,
    pub sort: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRecords {
    pub records: Vec<serde_json::Value>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub page: i64,
    pub total_pages: i64,
    pub has_more: bool,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Record not found in {0}: {1}")]
    NotFound(String, String),
    #[error("State machine transition error: {0}")]
    TransitionError(String),
    #[error("Security / Permission error: {0}")]
    SecurityError(String),
    #[error("Optimistic concurrency conflict: {0}")]
    ConcurrencyConflict(String),
    #[error("Immutable ledger violation: {0}")]
    ImmutableLedger(String),
    #[error("Record version required: {0}")]
    VersionRequired(String),
    #[error("Transaction rolled back after database error: {0}")]
    TransactionAborted(String),
    #[error("Schema synchronization blocked: {0}")]
    SchemaSyncBlocked(String),
    #[error("{0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub tenant_id: Uuid,
    pub company_id: Uuid,
    pub user_id: Uuid,
    pub roles: Vec<String>,
}

impl Default for SessionContext {
    fn default() -> Self {
        Self {
            tenant_id: Uuid::nil(),
            company_id: Uuid::nil(),
            user_id: Uuid::nil(),
            roles: vec!["Admin".to_string()],
        }
    }
}

pub fn sanitize_identifier(ident: &str) -> Result<String, RuntimeError> {
    let clean = ident.trim();
    if clean.is_empty() {
        return Err(RuntimeError::SecurityError("SQL identifier cannot be empty".to_string()));
    }
    if clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
        Ok(clean.to_string())
    } else {
        Err(RuntimeError::SecurityError(format!("Invalid characters in SQL identifier: '{}'", ident)))
    }
}

pub fn to_snake_case(s: &str) -> String {
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

/// Securely hash plain-text password using salted SHA-256 with CSPRNG salt
pub fn hash_password(plain: &str) -> String {
    use sha2::{Sha256, Digest};
    let salt = Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(plain.as_bytes());
    let hex_hash = format!("{:x}", hasher.finalize());
    format!("$sha256${}${}", salt, hex_hash)
}

/// Verify plain-text password against stored hash (supporting $sha256$, legacy bcrypt prefixes, or exact match)
pub fn verify_password(plain: &str, stored_hash: &str) -> bool {
    if stored_hash.starts_with("$sha256$") {
        let parts: Vec<&str> = stored_hash.split('$').collect();
        if parts.len() == 4 {
            let salt = parts[2];
            let expected_hex = parts[3];
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(salt.as_bytes());
            hasher.update(plain.as_bytes());
            let hex_hash = format!("{:x}", hasher.finalize());
            return hex_hash == expected_hex;
        }
    }
    // Reject invalid or unhashed passwords
    false
}

/// Transparently calculate password hash if client sends plain password ("password", "plainPassword", "plain_password")
pub fn preprocess_password_fields(data: &mut serde_json::Value) {
    if let Some(obj) = data.as_object_mut() {
        let plain_pwd = obj.get("password")
            .or_else(|| obj.get("plainPassword"))
            .or_else(|| obj.get("plain_password"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(pwd) = plain_pwd {
            if !pwd.trim().is_empty() {
                let computed_hash = hash_password(&pwd);
                // Set both snake_case and camelCase so triggers and SQL statements resolve seamlessly
                obj.insert("password_hash".to_string(), serde_json::json!(computed_hash));
                obj.insert("passwordHash".to_string(), serde_json::json!(computed_hash));
            }
            obj.remove("password");
            obj.remove("plainPassword");
            obj.remove("plain_password");
        }
    }
}

#[derive(Clone)]
pub struct BosEngine {
    pool: Arc<tokio::sync::RwLock<PgPool>>,
    active_url: Arc<tokio::sync::RwLock<String>>,
    meta_cache: Arc<dashmap::DashMap<String, MetaObjectDetails>>,
    pub event_registry: Arc<EventRegistry>,
    pub wasm_engine: Arc<WasmEngine>,
    pub sidecar_host: Arc<SidecarHost>,
}

impl BosEngine {
    pub async fn connect(database_url: &str) -> Result<Self, RuntimeError> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await?;

        // Auto-upgrade storage_strategy column if _bos_sys_info exists
        let _ = sqlx::query(
            "ALTER TABLE IF EXISTS \"_bos_sys_info\" ADD COLUMN IF NOT EXISTS \"storage_strategy\" VARCHAR(50) NOT NULL DEFAULT 'shared_rls';"
        )
        .execute(&pool)
        .await;

        Self::run_kernel_migrations(&pool).await;

        let meta_cache = Arc::new(dashmap::DashMap::new());
        let listener_cache = meta_cache.clone();
        let listener_pool = pool.clone();

        // High-Performance Event Bus: Real-Time Cache Invalidation via PostgreSQL LISTEN / NOTIFY
        tokio::spawn(async move {
            if let Ok(mut listener) = sqlx::postgres::PgListener::connect_with(&listener_pool).await {
                if listener.listen("bos_meta_channel").await.is_ok() {
                    while let Ok(notification) = listener.recv().await {
                        let payload = notification.payload();
                        if payload == "*" || payload.is_empty() {
                            listener_cache.clear();
                        } else {
                            listener_cache.remove(&payload.to_lowercase());
                        }
                    }
                }
            }
        });

        let event_registry = Arc::new(EventRegistry::load_from_db(&pool).await.unwrap_or_else(|_| EventRegistry::new()));
        let wasm_engine = Arc::new(WasmEngine::new()?);
        let sidecar_host = Arc::new(SidecarHost::new());

        Ok(Self {
            pool: Arc::new(tokio::sync::RwLock::new(pool)),
            active_url: Arc::new(tokio::sync::RwLock::new(database_url.to_string())),
            meta_cache,
            event_registry,
            wasm_engine,
            sidecar_host,
        })
    }

    pub async fn pool(&self) -> PgPool {
        self.pool.read().await.clone()
    }

    pub async fn active_url(&self) -> String {
        self.active_url.read().await.clone()
    }

    pub async fn active_db_name(&self) -> String {
        let url = self.active_url().await;
        DatabaseManager::extract_database_name(&url)
    }

    /// Dynamically switch active database or connect to another URL
    pub async fn switch_database(&self, target: &str) -> Result<String, RuntimeError> {
        let current_url = self.active_url().await;
        let new_url = if target.starts_with("postgres://") || target.starts_with("postgresql://") {
            target.to_string()
        } else {
            DatabaseManager::validate_db_name(target)?;
            DatabaseManager::replace_database_name(&current_url, target)
        };

        let new_pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(&new_url)
            .await?;

        let _ = sqlx::query(
            "ALTER TABLE IF EXISTS \"_bos_sys_info\" ADD COLUMN IF NOT EXISTS \"storage_strategy\" VARCHAR(50) NOT NULL DEFAULT 'shared_rls';"
        )
        .execute(&new_pool)
        .await;

        Self::run_kernel_migrations(&new_pool).await;

        let old_pool = {
            let mut pool_guard = self.pool.write().await;
            let mut url_guard = self.active_url.write().await;
            let old = pool_guard.clone();
            *pool_guard = new_pool;
            *url_guard = new_url.clone();
            old
        };

        self.meta_cache.clear();

        tokio::spawn(async move {
            old_pool.close().await;
        });

        Ok(DatabaseManager::extract_database_name(&new_url))
    }

    /// Automatically migrate schema evolution changes for _bos_meta_objects and kernel tables
    pub async fn run_kernel_migrations(pool: &PgPool) {
        let migration_sql = r#"
            DO $$
            BEGIN
                IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = '_bos_meta_objects' AND column_name = 'table_type') THEN
                    ALTER TABLE "_bos_meta_objects" RENAME COLUMN "table_type" TO "sub_type";
                END IF;
                IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = '_bos_meta_objects' AND column_name = 'definition_ir') THEN
                    ALTER TABLE "_bos_meta_objects" RENAME COLUMN "definition_ir" TO "definition";
                END IF;
                IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = '_bos_meta_objects' AND column_name = 'data_per_company') THEN
                    ALTER TABLE "_bos_meta_objects" DROP COLUMN "data_per_company";
                END IF;
                IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = '_bos_meta_objects' AND column_name = 'sql_table_name') THEN
                    ALTER TABLE "_bos_meta_objects" DROP COLUMN "sql_table_name";
                END IF;
            END $$;
            DROP INDEX IF EXISTS "idx_bos_meta_objects_ir";
            CREATE INDEX IF NOT EXISTS "idx_bos_meta_objects_def" ON "_bos_meta_objects" USING gin ("definition" jsonb_path_ops);

            CREATE TABLE IF NOT EXISTS "_bos_job_queue" (
                "job_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                "job_type" VARCHAR(100) NOT NULL,
                "codeunit_name" VARCHAR(100) NOT NULL,
                "method_name" VARCHAR(100) NOT NULL,
                "payload" JSONB NOT NULL DEFAULT '{}',
                "status" VARCHAR(50) NOT NULL DEFAULT 'ready',
                "chunk_size" INTEGER NOT NULL DEFAULT 500,
                "attempts" INTEGER NOT NULL DEFAULT 0,
                "error_message" TEXT,
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                "started_at" TIMESTAMPTZ,
                "completed_at" TIMESTAMPTZ
            );
            CREATE INDEX IF NOT EXISTS "idx_bos_job_queue_status" ON "_bos_job_queue" ("status", "created_at");
        "#;
        let _ = sqlx::raw_sql(migration_sql).execute(pool).await;
    }

    /// List all databases on the connected PostgreSQL cluster
    pub async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, RuntimeError> {
        let pool = self.pool().await;
        let current_db = self.active_db_name().await;
        DatabaseManager::list_databases(&pool, &current_db).await
    }

    /// Create a new PostgreSQL database with optional extensions and user-specified tenant/company
    pub async fn create_database(
        &self,
        req: &db_manager::CreateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let current_url = self.active_url().await;
        DatabaseManager::create_database(&current_url, req).await
    }

    /// List all tenants registered in active database
    pub async fn list_tenants(&self) -> Result<Vec<db_manager::TenantInfo>, RuntimeError> {
        let pool = self.pool().await;
        DatabaseManager::list_tenants(&pool).await
    }

    /// Create a new tenant in active database
    pub async fn create_tenant(&self, req: db_manager::CreateTenantRequest) -> Result<db_manager::TenantInfo, RuntimeError> {
        let pool = self.pool().await;
        DatabaseManager::create_tenant(&pool, req).await
    }

    /// List companies registered in active database
    pub async fn list_companies(&self, tenant_id: Option<Uuid>) -> Result<Vec<db_manager::CompanyInfo>, RuntimeError> {
        let pool = self.pool().await;
        DatabaseManager::list_companies(&pool, tenant_id).await
    }

    /// Create a new company under a tenant in active database
    pub async fn create_company(&self, req: db_manager::CreateCompanyRequest) -> Result<db_manager::CompanyInfo, RuntimeError> {
        let pool = self.pool().await;
        let strat: Option<String> = sqlx::query_scalar("SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
        DatabaseManager::create_company(&pool, req, strat.as_deref()).await
    }

    /// Provision BOS system kernel on active database
    pub async fn provision_system(&self, storage_strategy: Option<&str>) -> Result<SystemProvisionStatus, RuntimeError> {
        let pool = self.pool().await;
        MetaManager::provision_system_kernel(&pool, storage_strategy).await
    }

    /// Check system provisioning status of active database
    pub async fn get_system_status(&self) -> Result<SystemProvisionStatus, RuntimeError> {
        let pool = self.pool().await;
        MetaManager::get_system_status(&pool).await
    }

    /// Invalidate in-memory meta cache across cluster via PostgreSQL LISTEN / NOTIFY
    pub async fn invalidate_meta_cache(&self, object_name: &str) {
        self.meta_cache.remove(&object_name.to_lowercase());
        let pool = self.pool().await;
        let _ = sqlx::query("SELECT pg_notify('bos_meta_channel', $1);")
            .bind(object_name)
            .execute(&pool)
            .await;
    }

    /// Register a meta-object (table, statemachine, codeunit, enum) and create database objects
    pub async fn register_meta_object(&self, req: RegisterMetaObjectRequest) -> Result<MetaObjectSummary, RuntimeError> {
        let name = req.name.clone();
        let pool = self.pool().await;
        let res = MetaManager::register_meta_object(&pool, req).await?;
        self.invalidate_meta_cache(&name).await;
        Ok(res)
    }

    /// List all registered meta-objects with optional type filter
    pub async fn list_meta_objects(&self, object_type: Option<&str>) -> Result<Vec<MetaObjectSummary>, RuntimeError> {
        let pool = self.pool().await;
        MetaManager::list_meta_objects(&pool, object_type).await
    }

    /// Inspect details of a specific meta-object with high-speed DashMap read caching
    pub async fn get_meta_object(&self, name: &str) -> Result<Option<MetaObjectDetails>, RuntimeError> {
        let key = name.to_lowercase();
        if let Some(entry) = self.meta_cache.get(&key) {
            return Ok(Some(entry.clone()));
        }

        let pool = self.pool().await;
        let res = MetaManager::get_meta_object(&pool, name).await?;
        if let Some(ref details) = res {
            self.meta_cache.insert(key, details.clone());
        }
        Ok(res)
    }

    /// List installed packages from catalog (_bos_meta_packages)
    pub async fn list_packages(&self) -> Result<Vec<meta_manager::PackageSummary>, RuntimeError> {
        let pool = self.pool().await;
        MetaManager::list_packages(&pool).await
    }


    /// Install a .bospkg package bundle
    pub async fn install_package(&self, bundle: bosc_ir::PackageBundleIR) -> Result<package_manager::PackageInstallResult, RuntimeError> {
        let pool = self.pool().await;
        let strat: Option<String> = sqlx::query_scalar("SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1")
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);
        self.meta_cache.clear();
        let _ = sqlx::query("SELECT pg_notify('bos_meta_channel', '*');")
            .execute(&pool)
            .await;
        package_manager::PackageManager::install_package(&pool, bundle, strat.as_deref()).await
    }

    /// Export an installed package into a .bospkg bundle
    pub async fn export_package(&self, package_id: &str) -> Result<bosc_ir::PackageBundleIR, RuntimeError> {
        let pool = self.pool().await;
        package_manager::PackageManager::export_package(&pool, package_id).await
    }

    /// Register a Table Extension (AL/BC non-invasive schema evolution)
    pub async fn register_table_extension(&self, req: meta_manager::RegisterTableExtensionRequest) -> Result<meta_manager::MetaObjectSummary, RuntimeError> {
        let target = req.target_table.clone();
        let pool = self.pool().await;
        let res = meta_manager::MetaManager::register_table_extension(&pool, req).await?;
        self.invalidate_meta_cache(&target).await;
        Ok(res)
    }

    /// Compile and apply BOS-Script to an already created table
    pub async fn apply_table_script(&self, table_name: &str, req: meta_manager::ApplyTableScriptRequest) -> Result<meta_manager::ApplyTableScriptResponse, RuntimeError> {
        let pool = self.pool().await;
        let res = meta_manager::MetaManager::apply_table_script(&pool, table_name, req).await?;
        if res.applied {
            self.invalidate_meta_cache(table_name).await;
        }
        Ok(res)
    }

    /// Compile any arbitrary BOS-Script (preview or apply)
    pub async fn compile_script(&self, req: meta_manager::CompileScriptRequest) -> Result<meta_manager::CompileScriptResponse, RuntimeError> {
        let pool = self.pool().await;
        meta_manager::MetaManager::compile_script(&pool, req).await
    }

    /// Generate live catalog-aware ambient TypeScript declarations (.d.ts)
    pub async fn generate_ambient_dts(&self) -> String {
        let pool = self.pool().await;
        meta_manager::MetaManager::generate_live_ambient_dts(&pool).await
    }

    /// Register a Codeunit in catalog
    pub async fn register_codeunit(&self, cu: bosc_ir::CodeunitIR) -> Result<Uuid, RuntimeError> {
        let pool = self.pool().await;
        execution_engine::ExecutionEngine::register_codeunit(&pool, cu).await
    }

    /// Execute a Codeunit procedure
    pub async fn execute_procedure(&self, codeunit: &str, method: &str, params: serde_json::Value, ctx: &SessionContext) -> Result<execution_engine::ExecutionResult, RuntimeError> {
        let pool = self.pool().await;
        execution_engine::ExecutionEngine::execute_procedure(&pool, codeunit, method, params, ctx).await
    }

    /// Inspect details and table statistics of a database
    pub async fn get_database_details(&self, name: &str) -> Result<DatabaseDetails, RuntimeError> {
        let current_url = self.active_url().await;
        let current_db = self.active_db_name().await;
        DatabaseManager::get_database_details(&current_url, name, &current_db).await
    }

    /// Inspect details and table statistics of the active current database
    pub async fn get_current_database_details(&self) -> Result<DatabaseDetails, RuntimeError> {
        let current_db = self.active_db_name().await;
        self.get_database_details(&current_db).await
    }

    /// Drop a database from the PostgreSQL cluster
    pub async fn drop_database(&self, name: &str, force: bool) -> Result<(), RuntimeError> {
        let current_db = self.active_db_name().await;
        if name == current_db {
            // Switch to maintenance DB first so active connection doesn't block dropping
            self.switch_database("postgres").await?;
        }
        let updated_url = self.active_url().await;
        DatabaseManager::drop_database(&updated_url, name, force).await
    }

    /// Delete the currently active database and automatically switch engine to fallback 'postgres'
    pub async fn delete_current_database(&self, force: bool) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;
        if current_db.to_lowercase() == "postgres" {
            return Err(RuntimeError::SecurityError(
                "Cannot drop the primary system maintenance database 'postgres'".into(),
            ));
        }

        self.drop_database(&current_db, force).await?;
        let active_db = self.active_db_name().await;

        Ok(serde_json::json!({
            "success": true,
            "deleted_database": current_db,
            "active_database": active_db,
            "message": format!("Current database '{}' dropped successfully. Active database is now '{}'.", current_db, active_db)
        }))
    }

    /// Test an arbitrary connection string
    pub async fn test_connection(&self, url: &str) -> ConnectionTestResult {
        DatabaseManager::test_connection(url).await
    }

    /// Manage database operations (rename, truncate, custom SQL, vacuum, reindex, update_strategy, reset)
    pub async fn manage_database(&self, name: &str, req: ManageDatabaseRequest) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;
        let is_current = name == current_db;

        if req.action == "rename" {
            if let Some(ref new_name) = req.new_name {
                let target_new = new_name.clone();
                if is_current {
                    self.switch_database("postgres").await?;
                    let maint_url = self.active_url().await;
                    let res = DatabaseManager::manage_database(&maint_url, name, req).await?;
                    self.switch_database(&target_new).await?;
                    return Ok(res);
                }
            }
        }

        let current_url = self.active_url().await;
        DatabaseManager::manage_database(&current_url, name, req).await
    }

    /// Manage operations on current active database
    pub async fn manage_current_database(&self, req: ManageDatabaseRequest) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;
        self.manage_database(&current_db, req).await
    }

    /// Update database properties (rename, storage strategy, or status) for a specific database
    pub async fn update_database(&self, name: &str, req: UpdateDatabaseRequest) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;
        let is_current = name == current_db;

        if let Some(ref new_name) = req.new_name {
            if new_name != name {
                if is_current {
                    self.switch_database("postgres").await?;
                    let maint_url = self.active_url().await;
                    DatabaseManager::manage_database(&maint_url, name, ManageDatabaseRequest {
                        action: "rename".to_string(),
                        new_name: Some(new_name.clone()),
                        table: None,
                        sql: None,
                        storage_strategy: None,
                        full: None,
                    }).await?;
                    self.switch_database(new_name).await?;
                } else {
                    let current_url = self.active_url().await;
                    DatabaseManager::manage_database(&current_url, name, ManageDatabaseRequest {
                        action: "rename".to_string(),
                        new_name: Some(new_name.clone()),
                        table: None,
                        sql: None,
                        storage_strategy: None,
                        full: None,
                    }).await?;
                }
            }
        }

        let target_db = req.new_name.as_deref().unwrap_or(name);
        if req.storage_strategy.is_some() || req.status.is_some() {
            let target_url = DatabaseManager::replace_database_name(&self.active_url().await, target_db);
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let _ = sqlx::query(
                r#"
                UPDATE "_bos_sys_info"
                SET "storage_strategy" = COALESCE($1, "storage_strategy"),
                    "status" = COALESCE($2, "status")
                WHERE "id" = 'core';
                "#
            )
            .bind(req.storage_strategy.as_deref())
            .bind(req.status.as_deref())
            .execute(&pool)
            .await;

            pool.close().await;
        }

        Ok(serde_json::json!({
            "success": true,
            "database": target_db,
            "old_name": name,
            "new_name": req.new_name,
            "storage_strategy": req.storage_strategy,
            "status": req.status,
            "message": format!("Database '{}' updated successfully", target_db)
        }))
    }

    /// Update database properties for the currently active database
    pub async fn update_current_database(&self, req: UpdateDatabaseRequest) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;
        self.update_database(&current_db, req).await
    }

    /// Execute raw DDL statements (e.g. from bosc-codegen-db)
    pub async fn execute_migration(&self, ddl: &str) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        sqlx::raw_sql(ddl).execute(&pool).await?;
        Ok(())
    }

    /// Check if user has permission on entity for given action (Read, Insert, Modify, Delete, Execute)
    pub async fn check_permission(&self, ctx: &SessionContext, entity: &str, action: &str) -> Result<bool, RuntimeError> {
        // If user has 'Admin' or 'Super' role, bypass permission checks
        if ctx.roles.iter().any(|r| r.eq_ignore_ascii_case("Admin") || r.eq_ignore_ascii_case("Super")) {
            return Ok(true);
        }

        let psets = self.list_meta_objects(Some("permissionset")).await?;
        for ps_summary in psets {
            if let Some(details) = self.get_meta_object(&ps_summary.object_name).await? {
                if let Ok(ps_ir) = serde_json::from_value::<bosc_ir::PermissionSetIR>(details.definition.clone()) {
                    if action.eq_ignore_ascii_case("execute") {
                        if ps_ir.execute.iter().any(|cu| cu.eq_ignore_ascii_case(entity)) {
                            return Ok(true);
                        }
                    } else {
                        for perm in &ps_ir.permissions {
                            if perm.table_name.eq_ignore_ascii_case(entity) || perm.sql_table_name.eq_ignore_ascii_case(entity) {
                                let allowed = match action.to_lowercase().as_str() {
                                    "read" => perm.can_read,
                                    "insert" => perm.can_insert,
                                    "modify" | "update" => perm.can_modify,
                                    "delete" => perm.can_delete,
                                    _ => false,
                                };
                                if allowed {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }

        // If no explicit permission found and user is not admin, deny
        Ok(false)
    }

    /// Execute a BOS View by name with pagination, filter, and sorting
    pub async fn execute_view(
        &self,
        view_name: &str,
        filter: Option<serde_json::Value>,
        order_by: Option<&str>,
        limit: i64,
        offset: i64,
        ctx: &SessionContext,
    ) -> Result<Vec<serde_json::Value>, RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let sql_view = to_snake_case(view_name);
        let mut sql = format!("SELECT to_jsonb(v.*) AS json_data FROM \"{}\" v", sql_view);
        let mut conditions = Vec::new();

        if let Some(f) = filter {
            if let Some(obj) = f.as_object() {
                for (k, v) in obj {
                    let col_name = to_snake_case(k);
                    match v {
                        serde_json::Value::String(s) => {
                            conditions.push(format!("v.\"{}\" = '{}'", col_name, s.replace('\'', "''")));
                        }
                        serde_json::Value::Number(n) => {
                            conditions.push(format!("v.\"{}\" = {}", col_name, n));
                        }
                        serde_json::Value::Bool(b) => {
                            conditions.push(format!("v.\"{}\" = {}", col_name, if *b { "TRUE" } else { "FALSE" }));
                        }
                        serde_json::Value::Null => {
                            conditions.push(format!("v.\"{}\" IS NULL", col_name));
                        }
                        _ => {}
                    }
                }
            }
        }

        if !conditions.is_empty() {
            sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        if let Some(ord) = order_by {
            sql.push_str(&format!(" ORDER BY {}", ord));
        }

        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let rows = sqlx::query(&sql).fetch_all(&mut *tx).await?;
        let results = rows
            .iter()
            .map(|r| r.try_get::<serde_json::Value, _>("json_data").unwrap_or(serde_json::Value::Null))
            .collect();

        tx.commit().await?;
        Ok(results)
    }

    /// Refresh a Materialized View
    pub async fn refresh_view(&self, view_name: &str, concurrently: bool) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        let sql_view = to_snake_case(view_name);
        let conc_str = if concurrently { "CONCURRENTLY " } else { "" };
        let sql = format!("REFRESH MATERIALIZED VIEW {}\"{}\";", conc_str, sql_view);
        sqlx::query(&sql).execute(&pool).await?;
        Ok(())
    }

    /// Execute a registered Query meta-object by name with dynamic parameters
    pub async fn execute_named_query(
        &self,
        query_name: &str,
        params: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<Vec<serde_json::Value>, RuntimeError> {
        let meta = self.get_meta_object(query_name).await?
            .ok_or_else(|| RuntimeError::NotFound("Query".to_string(), query_name.to_string()))?;

        let query_ir: bosc_ir::QueryIR = serde_json::from_value(meta.definition)
            .map_err(|e| RuntimeError::Internal(format!("Failed to parse query definition: {}", e)))?;

        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let mut final_sql = query_ir.sql_query.clone();

        // Parameter substitution (e.g. :minAmount -> value)
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                let placeholder = format!(":{}", k);
                let val_str = match v {
                    serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => (if *b { "TRUE" } else { "FALSE" }).to_string(),
                    serde_json::Value::Null => "NULL".to_string(),
                    _ => v.to_string(),
                };
                final_sql = final_sql.replace(&placeholder, &val_str);
            }
        }

        let wrapped_sql = format!("SELECT to_jsonb(q.*) AS json_data FROM ({}) q", final_sql);
        let rows = sqlx::query(&wrapped_sql).fetch_all(&mut *tx).await?;

        let results = rows
            .iter()
            .map(|r| r.try_get::<serde_json::Value, _>("json_data").unwrap_or(serde_json::Value::Null))
            .collect();

        tx.commit().await?;
        Ok(results)
    }

    /// Apply multi-tenant and multi-strategy session context to the transaction
    pub async fn apply_isolation_context(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        ctx: &SessionContext,
    ) -> Result<(), RuntimeError> {
        let set_tenant = format!("SET LOCAL bos.current_tenant_id = '{}';", ctx.tenant_id);
        let set_company = format!("SET LOCAL bos.current_company_id = '{}';", ctx.company_id);
        let set_user = format!("SET LOCAL bos.current_user_id = '{}';", ctx.user_id);
        sqlx::query(&set_tenant).execute(&mut **tx).await?;
        sqlx::query(&set_company).execute(&mut **tx).await?;
        sqlx::query(&set_user).execute(&mut **tx).await?;

        let pool = self.pool().await;
        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1"
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if strat.as_deref() == Some("schema_per_company") {
            let comp_code: Option<String> = sqlx::query_scalar(
                "SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1"
            )
            .bind(ctx.company_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let schema_name = format!(
                "comp_{}",
                comp_code.unwrap_or_else(|| "default".to_string()).to_lowercase().replace('-', "_")
            );
            let set_search_path = format!("SET LOCAL search_path = \"{}\", \"public\";", schema_name);
            let _ = sqlx::query(&set_search_path).execute(&mut **tx).await;
        }

        Ok(())
    }

    /// Resolve physical table target and company isolation requirement
    pub async fn resolve_table_info(&self, table: &str, ctx: &SessionContext) -> Result<(String, bool), RuntimeError> {
        let pool = self.pool().await;

        let meta_row = sqlx::query(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'table' AND (LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1)) LIMIT 1"
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        let (base_table, data_per_company) = match meta_row {
            Some(r) => {
                let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
                let sql_name = def.get("sql_table_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(table)
                    .to_lowercase();
                let dpc = def.get("data_per_company")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let has_company_col: bool = if dpc {
                    sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = '_company_id')"
                    )
                    .bind(&sql_name)
                    .fetch_one(&pool)
                    .await
                    .unwrap_or(true)
                } else {
                    false
                };

                (sql_name, has_company_col)
            }
            None => {
                let snake = to_snake_case(table);
                let exists_snake: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = $1)"
                )
                .bind(&snake)
                .fetch_one(&pool)
                .await
                .unwrap_or(false);

                let resolved_name = if exists_snake { snake } else { table.to_lowercase() };

                let has_company_id: bool = sqlx::query_scalar(
                    "SELECT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = $1 AND column_name = '_company_id')"
                )
                .bind(&resolved_name)
                .fetch_one(&pool)
                .await
                .unwrap_or(false);

                (resolved_name, has_company_id)
            }
        };

        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1"
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if strat.as_deref() == Some("table_per_company") && data_per_company {
            let comp_code: Option<String> = sqlx::query_scalar(
                "SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1"
            )
            .bind(ctx.company_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let prefix = comp_code.unwrap_or_else(|| "default".to_string()).to_lowercase();
            return Ok((format!("{}${}", prefix, base_table), data_per_company));
        }

        if strat.as_deref() == Some("schema_per_company") && data_per_company {
            let comp_code: Option<String> = sqlx::query_scalar(
                "SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1"
            )
            .bind(ctx.company_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let schema_name = format!(
                "comp_{}",
                comp_code.unwrap_or_else(|| "default".to_string()).to_lowercase().replace('-', "_")
            );
            return Ok((format!("\"{}\".\"{}\"", schema_name, base_table), data_per_company));
        }

        Ok((base_table, data_per_company))
    }

    /// Resolve physical table target according to active multi-company storage strategy
    pub async fn resolve_physical_table(&self, table: &str, ctx: &SessionContext) -> Result<String, RuntimeError> {
        let (tbl, _) = self.resolve_table_info(table, ctx).await?;
        Ok(tbl)
    }

    /// Check if a table is marked as an immutable ledger (sub_type == 'Ledger' or immutable == true)
    pub async fn is_immutable_ledger(&self, table: &str) -> bool {
        let pool = self.pool().await;
        let row_opt = sqlx::query(
            "SELECT sub_type, definition FROM \"_bos_meta_objects\" WHERE object_type = 'table' AND (LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($1)) LIMIT 1"
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some(r) = row_opt {
            let sub_type: Option<String> = r.try_get("sub_type").ok();
            if sub_type.as_deref().map(|s| s.eq_ignore_ascii_case("ledger")).unwrap_or(false) {
                return true;
            }
            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
            if def.get("immutable").and_then(|v| v.as_bool()).unwrap_or(false) {
                return true;
            }
            if def.get("sub_type").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("ledger")).unwrap_or(false) {
                return true;
            }
        }
        false
    }

    /// Introspect complete table schema, combining high-level BOS meta catalog with physical PostgreSQL columns
    pub async fn get_entity_schema(&self, table: &str, ctx: &SessionContext) -> Result<EntitySchemaResponse, RuntimeError> {
        let pool = self.pool().await;
        let (physical_table, data_per_company) = self.resolve_table_info(table, ctx).await?;

        // 1. Check if there is a registered meta object for this table
        let meta_row = sqlx::query(
            "SELECT object_name, package_name, caption, definition FROM \"_bos_meta_objects\" WHERE object_type = 'table' AND (LOWER(object_name) = LOWER($1) OR LOWER(definition->>'sql_table_name') = LOWER($2) OR LOWER(object_name) = LOWER($2)) LIMIT 1"
        )
        .bind(table)
        .bind(&physical_table)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        // 2. Query physical columns from information_schema
        let phys_cols: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
            "SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position"
        )
        .bind(&physical_table)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        let mut columns = Vec::new();
        let mut table_name = table.to_string();
        let mut package_name = None;
        let mut caption = None;
        let mut state_machine = None;

        if let Some(r) = meta_row {
            table_name = r.try_get("object_name").unwrap_or_else(|_| table.to_string());
            package_name = r.try_get("package_name").ok();
            caption = r.try_get("caption").ok();
            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();
            state_machine = def.get("state_machine").cloned();

            if let Some(meta_cols) = def.get("columns").and_then(|c| c.as_array()) {
                for mc in meta_cols {
                    let col_name = mc.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let sql_name = mc.get("sql_name").and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| to_snake_case(&col_name));
                    let data_type = mc.get("sql_type").or_else(|| mc.get("data_type"))
                        .and_then(|v| v.as_str()).unwrap_or("varchar(255)").to_string();
                    let is_nullable = mc.get("is_nullable").and_then(|v| v.as_bool()).unwrap_or(true);
                    let is_pk = mc.get("is_primary_key").and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_unique = mc.get("is_unique").and_then(|v| v.as_bool()).unwrap_or(false);
                    let def_val = mc.get("default_val").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let col_caption = mc.get("caption").and_then(|v| v.as_str()).map(|s| s.to_string());

                    columns.push(EntityColumnSchema {
                        name: col_name,
                        sql_name,
                        data_type,
                        is_nullable,
                        is_primary_key: is_pk,
                        is_unique,
                        default_val: def_val,
                        caption: col_caption,
                    });
                }
            }
        }

        // If no meta columns registered, populate from physical database columns
        if columns.is_empty() {
            for (c_name, d_type, is_null, c_def) in &phys_cols {
                if c_name.starts_with('_') {
                    continue;
                }
                columns.push(EntityColumnSchema {
                    name: c_name.clone(),
                    sql_name: c_name.clone(),
                    data_type: d_type.clone(),
                    is_nullable: is_null == "YES",
                    is_primary_key: false,
                    is_unique: false,
                    default_val: c_def.clone(),
                    caption: Some(c_name.clone()),
                });
            }
        }

        let mut sample_create_map = serde_json::Map::new();
        let mut sample_update_map = serde_json::Map::new();

        for col in &columns {
            if col.name.starts_with('_') || col.sql_name.starts_with('_') || col.is_primary_key {
                continue;
            }
            let key = if !col.name.is_empty() { col.name.clone() } else { col.sql_name.clone() };
            let dt = col.data_type.to_lowercase();
            let sample_val = if dt.contains("int") || dt.contains("serial") {
                serde_json::json!(1)
            } else if dt.contains("numeric") || dt.contains("decimal") || dt.contains("float") || dt.contains("double") || dt.contains("real") {
                serde_json::json!(100.0)
            } else if dt.contains("bool") {
                serde_json::json!(true)
            } else if dt.contains("uuid") {
                serde_json::json!(Uuid::new_v4().to_string())
            } else if dt.contains("json") {
                serde_json::json!({})
            } else if dt.contains("date") || dt.contains("time") {
                serde_json::json!("2026-10-10T12:00:00Z")
            } else {
                serde_json::json!(format!("Sample {}", key))
            };

            sample_create_map.insert(key.clone(), sample_val.clone());
            if !col.is_unique {
                sample_update_map.insert(key, sample_val);
            }
        }

        let sample_create_payload = serde_json::Value::Object(sample_create_map.clone());
        let sample_update_payload = serde_json::Value::Object(sample_update_map);
        let sample_batch_payload = vec![
            serde_json::Value::Object(sample_create_map.clone()),
            serde_json::Value::Object(sample_create_map),
        ];

        Ok(EntitySchemaResponse {
            table_name,
            physical_table,
            package_name,
            caption,
            data_per_company,
            columns,
            state_machine,
            sample_create_payload,
            sample_batch_payload,
            sample_update_payload,
        })
    }

    /// Insert record with tenant and audit isolation and business logic trigger dispatch
    pub async fn insert_record(
        &self,
        table: &str,
        data: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<Uuid, RuntimeError> {
        self.insert_record_with_messages(table, data, ctx)
            .await
            .map(|(id, _)| id)
    }

    pub async fn insert_record_with_messages(
        &self,
        table: &str,
        data: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<(Uuid, Vec<serde_json::Value>), RuntimeError> {
        let pool = self.pool().await;

        let mut payload = data;

        // 1. Dispatch pre-insert business logic and table triggers (OnBeforeInsert, and OnDraftSave if draft)
        let is_draft_insert = payload.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || payload.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        // 0. Dispatch field-level OnValidate cascades for all incoming fields
        if let Some(obj) = payload.as_object().cloned() {
            for (k, _) in obj {
                if !k.starts_with('_') {
                    execution_engine::ExecutionEngine::dispatch_triggers(
                        &pool,
                        table,
                        &bosc_ir::TriggerEventIR::OnValidate(k.clone()),
                        &mut payload,
                        None,
                        ctx,
                    )
                    .await?;
                }
            }
        }

        if is_draft_insert {
            execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnDraftSave,
                &mut payload,
                None,
                ctx,
            )
            .await?;
        }

        execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnBeforeInsert,
            &mut payload,
            None,
            ctx,
        )
        .await?;

        let mut ev_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnBeforeInsert",
                &mut payload,
                None,
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_logs,
            );
        }

        let mut tx = pool.begin().await?;

        // 2. Set session context for RLS & Schema
        self.apply_isolation_context(&mut tx, ctx).await?;

        // 3. Resolve physical table name according to strategy and company isolation
        let (physical_table, data_per_company) = self.resolve_table_info(table, ctx).await?;

        let id = payload
            .get("_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let obj = payload.as_object().ok_or_else(|| {
            RuntimeError::DatabaseError(sqlx::Error::Configuration("Expected JSON object".into()))
        })?;

        let mut added_columns = std::collections::HashSet::new();
        let mut columns = Vec::new();
        let mut values = Vec::new();

        columns.push("\"_id\"".to_string());
        values.push(format!("'{}'::uuid", id));
        added_columns.insert("_id".to_string());

        columns.push("\"_tenant_id\"".to_string());
        values.push(format!("'{}'::uuid", ctx.tenant_id));
        added_columns.insert("_tenant_id".to_string());

        if data_per_company {
            columns.push("\"_company_id\"".to_string());
            values.push(format!("'{}'::uuid", ctx.company_id));
            added_columns.insert("_company_id".to_string());
        }

        for (k, v) in obj {
            let col_name = if k.starts_with('_') { k.clone() } else { to_snake_case(k) };
            if added_columns.contains(&col_name) {
                continue;
            }

            // Skip transient UI messaging fields and system audit/version columns if null
            if col_name == "_system_messages" || col_name == "_messages" {
                continue;
            }
            if (col_name == "_created_at" || col_name == "_updated_at") && (v.is_null() || v.as_str().is_none()) {
                continue;
            }
            if (col_name == "_created_by" || col_name == "_updated_by") && v.is_null() {
                continue;
            }
            if col_name == "_system_version" && (v.is_null() || v.as_i64().is_none()) {
                continue;
            }

            let safe_col = sanitize_identifier(&col_name)?;
            columns.push(format!("\"{}\"", safe_col));
            added_columns.insert(col_name);
            match v {
                serde_json::Value::String(s) => values.push(format!("'{}'", s.replace("'", "''"))),
                serde_json::Value::Number(n) => values.push(n.to_string()),
                serde_json::Value::Bool(b) => values.push(if *b { "TRUE".to_string() } else { "FALSE".to_string() }),
                serde_json::Value::Null => values.push("NULL".to_string()),
                _ => values.push(format!("'{}'::jsonb", v.to_string().replace("'", "''"))),
            }
        }

        let query = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({});",
            physical_table,
            columns.join(", "),
            values.join(", ")
        );

        sqlx::query(&query).execute(&mut *tx).await?;
        tx.commit().await?;

        // 4. Dispatch post-insert business logic triggers (OnAfterInsert)
        let _ = execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnAfterInsert,
            &mut payload,
            None,
            ctx,
        )
        .await;

        let mut ev_post_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnAfterInsert",
                &mut payload,
                None,
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_post_logs,
            );
        }

        let messages = if let Some(msgs) = payload.as_object_mut().and_then(|o| o.remove("_system_messages")) {
            msgs.as_array().cloned().unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok((id, messages))
    }

    /// Bulk insert records in a single vectorized multi-row query with tenant isolation (Salesforce/BC Bulkification pattern)
    pub async fn insert_records_batch(
        &self,
        table: &str,
        records: Vec<serde_json::Value>,
        ctx: &SessionContext,
    ) -> Result<Vec<Uuid>, RuntimeError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }

        let pool = self.pool().await;
        let mut payloads = records;

        // 1. Dispatch pre-insert triggers (including BOS-Script table triggers)
        for payload in &mut payloads {
            execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnBeforeInsert,
                payload,
                None,
                ctx,
            )
            .await?;
        }

        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;
        let (physical_table, data_per_company) = self.resolve_table_info(table, ctx).await?;

        // 2. Determine all distinct user columns across all records
        let mut column_set = std::collections::BTreeSet::new();
        for p in &payloads {
            if let Some(obj) = p.as_object() {
                for k in obj.keys() {
                    if !k.starts_with('_') {
                        column_set.insert(to_snake_case(k));
                    }
                }
            }
        }
        let user_columns: Vec<String> = column_set.into_iter().collect();

        // 3. Build multi-row INSERT query
        let mut cols = vec!["\"_id\"".to_string(), "\"_tenant_id\"".to_string()];
        if data_per_company {
            cols.push("\"_company_id\"".to_string());
        }
        for c in &user_columns {
            let safe_col = sanitize_identifier(c)?;
            cols.push(format!("\"{}\"", safe_col));
        }

        let mut generated_ids = Vec::with_capacity(payloads.len());
        let mut row_tuples = Vec::with_capacity(payloads.len());

        for p in &payloads {
            let id = Uuid::new_v4();
            generated_ids.push(id);
            let mut val_tokens = vec![
                format!("'{}'::uuid", id),
                format!("'{}'::uuid", ctx.tenant_id),
            ];
            if data_per_company {
                val_tokens.push(format!("'{}'::uuid", ctx.company_id));
            }

            let obj = p.as_object();
            for c in &user_columns {
                let v_opt = obj.and_then(|o| o.get(c).or_else(|| {
                    o.iter().find(|(k, _)| &to_snake_case(k) == c).map(|(_, v)| v)
                }));
                match v_opt {
                    Some(serde_json::Value::String(s)) => {
                        val_tokens.push(format!("'{}'", s.replace("'", "''")));
                    }
                    Some(serde_json::Value::Number(n)) => {
                        val_tokens.push(n.to_string());
                    }
                    Some(serde_json::Value::Bool(b)) => {
                        val_tokens.push(if *b { "TRUE".to_string() } else { "FALSE".to_string() });
                    }
                    Some(serde_json::Value::Null) | None => {
                        val_tokens.push("NULL".to_string());
                    }
                    Some(v) => {
                        val_tokens.push(format!("'{}'::jsonb", v.to_string().replace("'", "''")));
                    }
                }
            }
            row_tuples.push(format!("({})", val_tokens.join(", ")));
        }

        let insert_query = format!(
            "INSERT INTO \"{}\" ({}) VALUES {};",
            physical_table,
            cols.join(", "),
            row_tuples.join(", ")
        );

        sqlx::query(&insert_query).execute(&mut *tx).await?;
        tx.commit().await?;

        // 4. Dispatch post-insert business logic triggers for all records
        for payload in &mut payloads {
            let _ = execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnAfterInsert,
                payload,
                None,
                ctx,
            )
            .await;
        }

        Ok(generated_ids)
    }

    /// Update record with compute-first trigger validation, xRec diffing, and Optimistic Concurrency Control
    pub async fn update_record(
        &self,
        table: &str,
        record_id: Uuid,
        patch: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let pool = self.pool().await;
        let physical_table = self.resolve_physical_table(table, ctx).await?;

        if self.is_immutable_ledger(table).await {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Table '{}' is an immutable ledger. Updates and in-place modifications are forbidden by accounting design. Compensating or reversal entries must be posted instead.",
                table
            )));
        }

        let mut patch = patch;

        // 1. Fetch pristine record (xRec)
        let get_query = format!("SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE \"_id\" = $1 LIMIT 1;", physical_table);
        let x_rec_opt = sqlx::query_scalar::<_, serde_json::Value>(&get_query)
            .bind(record_id)
            .fetch_optional(&pool)
            .await?;

        let x_rec = match x_rec_opt {
            Some(x) => x,
            None => return Err(RuntimeError::NotFound(table.to_string(), record_id.to_string())),
        };

        // 1b. Enforce State Machine `allowModify` policy if a StateMachine is bound to this table
        if let Ok(Some(sm_def)) = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1"
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        {
            let status_col = sm_def.get("status_field").and_then(|v| v.as_str()).unwrap_or("status");
            if let Some(cur_state) = x_rec.get(status_col).and_then(|v| v.as_str()) {
                if let Some(states) = sm_def.get("states").and_then(|v| v.as_array()) {
                    for st in states {
                        if st.get("name").and_then(|v| v.as_str()).map_or(false, |n| n.eq_ignore_ascii_case(cur_state)) {
                            if st.get("allow_modify").and_then(|v| v.as_bool()) == Some(false) {
                                return Err(RuntimeError::TransitionError(format!(
                                    "State Machine Policy Violation: Record in table '{}' is in state '{}' which sets allowModify = false.",
                                    table, cur_state
                                )));
                            }
                        }
                    }
                }
            }
        }

        // 2. Stage target record (Rec) and dispatch field-level OnValidate cascades for changed fields
        let mut rec = x_rec.clone();
        let mut changed_fields = Vec::new();
        if let (Some(patch_obj), Some(x_obj)) = (patch.as_object(), x_rec.as_object()) {
            for (k, v) in patch_obj {
                if !k.starts_with('_') && x_obj.get(k) != Some(v) {
                    changed_fields.push((k.clone(), v.clone()));
                }
            }
        }

        for (k, v) in changed_fields {
            if let Some(rec_obj) = rec.as_object_mut() {
                rec_obj.insert(k.clone(), v);
            }
            execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnValidate(k.clone()),
                &mut rec,
                Some(&x_rec),
                ctx,
            )
            .await?;

            let mut ev_logs = Vec::new();
            if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
                let _ = self.event_registry.dispatch(
                    table,
                    &format!("OnValidate{}", k),
                    &mut rec,
                    Some(&x_rec),
                    ctx,
                    Some(&pool),
                    &qjs,
                    &mut ev_logs,
                );
            }
        }

        // 3. Compute-first: Dispatch OnDraftSave (if draft) and OnBeforeModify with (Rec, Some(&xRec))
        let is_draft_edit = rec.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || rec.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        if is_draft_edit {
            execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnDraftSave,
                &mut rec,
                Some(&x_rec),
                ctx,
            )
            .await?;
        }

        execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnBeforeModify,
            &mut rec,
            Some(&x_rec),
            ctx,
        )
        .await?;

        let mut ev_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnBeforeModify",
                &mut rec,
                Some(&x_rec),
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_logs,
            );
        }

        // Sync any fields mutated or stripped by triggers in rec (e.g. hashPassword) back into patch
        if let (Some(rec_obj), Some(patch_obj)) = (rec.as_object(), patch.as_object_mut()) {
            for (k, v) in rec_obj {
                if !k.starts_with('_') && x_rec.as_object().and_then(|x| x.get(k)) != Some(v) {
                    patch_obj.insert(k.clone(), v.clone());
                }
            }
            patch_obj.retain(|k, _| rec_obj.contains_key(k) || k.starts_with('_'));
        }

        // 4. Short-lived write transaction with Optimistic Concurrency Control (OCC)
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let expected_version: Option<i64> = patch.get("_system_version")
            .or_else(|| patch.get("version"))
            .and_then(|v| v.as_i64());

        if let Some(obj) = patch.as_object() {
            let mut set_clauses = Vec::new();
            for (k, v) in obj {
                let col_name = if k.starts_with('_') { k.clone() } else { to_snake_case(k) };
                if col_name == "_system_version"
                    || col_name == "version"
                    || col_name == "_id"
                    || col_name == "_tenant_id"
                    || col_name == "_company_id"
                    || col_name == "_created_at"
                    || col_name == "_created_by"
                    || col_name == "_updated_at"
                    || col_name == "_updated_by"
                    || col_name == "_system_messages"
                    || col_name == "_messages"
                {
                    continue;
                }
                let safe_col = sanitize_identifier(&col_name)?;
                let v_sql = match v {
                    serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
                    serde_json::Value::Null => "NULL".to_string(),
                    _ => format!("'{}'::jsonb", v.to_string().replace("'", "''")),
                };
                set_clauses.push(format!("\"{}\" = {}", safe_col, v_sql));
            }
            set_clauses.push("\"_updated_at\" = NOW()".to_string());
            set_clauses.push("\"_system_version\" = COALESCE(\"_system_version\", 0) + 1".to_string());

            let where_clause = match expected_version {
                Some(ver) => format!("\"_id\" = '{}' AND (\"_system_version\" = {} OR \"_system_version\" IS NULL)", record_id, ver),
                None => format!("\"_id\" = '{}'", record_id),
            };

            let update_query = format!(
                "UPDATE \"{}\" SET {} WHERE {};",
                physical_table,
                set_clauses.join(", "),
                where_clause
            );
            let affected = sqlx::query(&update_query).execute(&mut *tx).await?.rows_affected();
            if affected == 0 {
                return Err(RuntimeError::ConcurrencyConflict(format!(
                    "Record {} in table '{}' has been modified or locked by another concurrent transaction (Optimistic Concurrency Control failure)",
                    record_id, physical_table
                )));
            }
        }

        tx.commit().await?;

        // 5. Dispatch OnAfterModify
        let _ = execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnAfterModify,
            &mut rec,
            Some(&x_rec),
            ctx,
        )
        .await;

        let mut ev_post_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnAfterModify",
                &mut rec,
                Some(&x_rec),
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_post_logs,
            );
        }

        Ok(rec)
    }

    /// Delete record with OnBeforeDelete and OnAfterDelete triggers and referential protection
    pub async fn delete_record(
        &self,
        table: &str,
        record_id: Uuid,
        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let pool = self.pool().await;
        let physical_table = self.resolve_physical_table(table, ctx).await?;

        if self.is_immutable_ledger(table).await {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Table '{}' is an immutable ledger. Deletions are strictly prohibited. Correcting or reversal entries must be posted instead.",
                table
            )));
        }

        // 1. Fetch pristine record (xRec)
        let get_query = format!("SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE \"_id\" = $1 LIMIT 1;", physical_table);
        let x_rec_opt = sqlx::query_scalar::<_, serde_json::Value>(&get_query)
            .bind(record_id)
            .fetch_optional(&pool)
            .await?;

        let x_rec = match x_rec_opt {
            Some(x) => x,
            None => return Err(RuntimeError::NotFound(table.to_string(), record_id.to_string())),
        };

        // 1b. Enforce State Machine `allowDelete` policy if a StateMachine is bound to this table
        if let Ok(Some(sm_def)) = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1"
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        {
            let status_col = sm_def.get("status_field").and_then(|v| v.as_str()).unwrap_or("status");
            if let Some(cur_state) = x_rec.get(status_col).and_then(|v| v.as_str()) {
                if let Some(states) = sm_def.get("states").and_then(|v| v.as_array()) {
                    for st in states {
                        if st.get("name").and_then(|v| v.as_str()).map_or(false, |n| n.eq_ignore_ascii_case(cur_state)) {
                            if st.get("allow_delete").and_then(|v| v.as_bool()) == Some(false) {
                                return Err(RuntimeError::TransitionError(format!(
                                    "State Machine Policy Violation: Record in table '{}' is in state '{}' which sets allowDelete = false.",
                                    table, cur_state
                                )));
                            }
                        }
                    }
                }
            }
        }

        let mut data = x_rec.clone();

        // 2. Dispatch OnBeforeDelete trigger (and OnDraftDiscard if in Draft status)
        let is_draft = x_rec.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || x_rec.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        if is_draft {
            execution_engine::ExecutionEngine::dispatch_triggers(
                &pool,
                table,
                &bosc_ir::TriggerEventIR::OnDraftDiscard,
                &mut data,
                Some(&x_rec),
                ctx,
            )
            .await?;
        }

        execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnBeforeDelete,
            &mut data,
            Some(&x_rec),
            ctx,
        )
        .await?;

        let mut ev_del_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnBeforeDelete",
                &mut data,
                Some(&x_rec),
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_del_logs,
            );
        }

        // 3. Delete in isolation transaction
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let del_query = format!("DELETE FROM \"{}\" WHERE \"_id\" = $1;", physical_table);
        let affected = sqlx::query(&del_query)
            .bind(record_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

        if affected == 0 {
            return Err(RuntimeError::NotFound(table.to_string(), record_id.to_string()));
        }

        tx.commit().await?;

        // 4. Dispatch OnAfterDelete trigger
        let _ = execution_engine::ExecutionEngine::dispatch_triggers(
            &pool,
            table,
            &bosc_ir::TriggerEventIR::OnAfterDelete,
            &mut data,
            Some(&x_rec),
            ctx,
        )
        .await;

        let mut ev_post_del_logs = Vec::new();
        if let Ok(qjs) = crate::quickjs_engine::QuickJsEngine::new() {
            let _ = self.event_registry.dispatch(
                table,
                "OnAfterDelete",
                &mut data,
                Some(&x_rec),
                ctx,
                Some(&pool),
                &qjs,
                &mut ev_post_del_logs,
            );
        }

        Ok(x_rec)
    }

    /// Pessimistic record locking (Dynamics NAV LOCKTABLE row lock pattern: SELECT ... FOR UPDATE)
    pub async fn lock_record(
        &self,
        table: &str,
        record_id: Uuid,
        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;
        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let lock_query = format!(
            "SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE \"_id\" = $1 FOR UPDATE;",
            physical_table
        );
        let row = sqlx::query_scalar::<_, serde_json::Value>(&lock_query)
            .bind(record_id)
            .fetch_optional(&mut *tx)
            .await?;

        tx.commit().await?;

        match row {
            Some(data) => Ok(data),
            None => Err(RuntimeError::NotFound(table.to_string(), record_id.to_string())),
        }
    }

    /// Pessimistic table locking (Dynamics NAV LOCKTABLE table lock pattern)
    pub async fn lock_table(&self, table: &str, ctx: &SessionContext) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;
        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let lock_sql = format!("LOCK TABLE \"{}\" IN ROW EXCLUSIVE MODE;", physical_table);
        sqlx::query(&lock_sql).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    /// GraphQL-like Base Query Protocol Engine with Native SIFT and FlowField Pushdown
    pub async fn execute_query(
        &self,
        req: &QueryRequest,
        ctx: &SessionContext,
    ) -> Result<Vec<serde_json::Value>, RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let physical_table = self.resolve_physical_table(&req.entity, ctx).await?;

        // 1. Look up table definition in catalog for FlowFields (SIFT)
        let meta_obj = self.get_meta_object(&req.entity).await.unwrap_or(None);
        let mut flow_fields_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        if let Some(meta) = meta_obj {
            if let Some(ff_arr) = meta.definition.get("flow_fields").and_then(|v| v.as_array()) {
                for ff in ff_arr {
                    if let (Some(name), Some(subq)) = (ff.get("name").and_then(|v| v.as_str()), ff.get("sql_subquery").and_then(|v| v.as_str())) {
                        flow_fields_map.insert(name.to_lowercase(), subq.to_string());
                        flow_fields_map.insert(to_snake_case(name), subq.to_string());
                    }
                }
            }
        }

        // 2. Build projections list
        let mut projections = Vec::new();

        match &req.select {
            Some(fields) if !fields.is_empty() => {
                let mut has_id = false;
                for f in fields {
                    let f_snake = to_snake_case(f);
                    let safe_f = sanitize_identifier(&f_snake)?;
                    if safe_f == "_id" || safe_f == "id" {
                        has_id = true;
                    }

                    if let Some(subquery) = flow_fields_map.get(&f.to_lowercase()).or_else(|| flow_fields_map.get(&f_snake)) {
                        let rewritten_sub = subquery
                            .replace(&format!("\"{}\".", physical_table), "t.")
                            .replace(&format!("\"{}\".", req.entity.to_lowercase()), "t.");
                        projections.push(format!("'{}', {}", safe_f, rewritten_sub));
                    } else {
                        projections.push(format!("'{}', t.\"{}\"", safe_f, safe_f));
                    }
                }
                if !has_id {
                    projections.push("'_id', t.\"_id\"".to_string());
                }
            }
            _ => {
                projections.push("'_id', t.\"_id\"".to_string());
                for (name, subq) in &flow_fields_map {
                    let rewritten_sub = subq
                        .replace(&format!("\"{}\".", physical_table), "t.")
                        .replace(&format!("\"{}\".", req.entity.to_lowercase()), "t.");
                    projections.push(format!("'{}', {}", name, rewritten_sub));
                }
            }
        }

        // 3. Process nested relations (include) using json_agg lateral subqueries (0 N+1 overhead)
        if let Some(includes) = &req.include {
            for inc in includes {
                let rel_entity = inc.entity.as_deref().unwrap_or(&inc.relation);
                let (rel_physical, _) = self.resolve_table_info(rel_entity, ctx).await?;
                let rel_alias = sanitize_identifier(&inc.relation)?;
                let fk_col = inc.foreign_key.as_deref()
                    .map(to_snake_case)
                    .unwrap_or_else(|| format!("{}_id", to_snake_case(&req.entity)));
                let safe_fk = sanitize_identifier(&fk_col)?;
                let rel_limit = inc.limit.unwrap_or(100);

                let rel_select = match &inc.select {
                    Some(s_cols) if !s_cols.is_empty() => {
                        let pairs: Vec<String> = s_cols.iter()
                            .filter_map(|c| sanitize_identifier(&to_snake_case(c)).ok())
                            .map(|sc| format!("'{}', rel.\"{}\"", sc, sc))
                            .collect();
                        format!("json_build_object({})", pairs.join(", "))
                    }
                    _ => "to_jsonb(rel.*)".to_string(),
                };

                let parent_join_col = if safe_fk.ends_with("_code") { "\"code\"" } else { "\"_id\"" };
                let rel_subquery = format!(
                    "COALESCE((SELECT json_agg({}) FROM \"{}\" rel WHERE rel.\"{}\"::text = t.{}::text LIMIT {}), '[]'::json)",
                    rel_select, rel_physical, safe_fk, parent_join_col, rel_limit
                );

                projections.push(format!("'{}', {}", rel_alias, rel_subquery));
            }
        }

        let select_expr = if projections.is_empty() || req.select.is_none() {
            if flow_fields_map.is_empty() && req.include.is_none() {
                "to_jsonb(t.*)".to_string()
            } else {
                format!("to_jsonb(t.*) || json_build_object({})", projections.join(", "))
            }
        } else {
            format!("json_build_object({})", projections.join(", "))
        };

        // 4. Build WHERE clauses
        let mut where_clauses = vec!["TRUE".to_string()];
        if let Some(filter_val) = &req.filter {
            if let Some(obj) = filter_val.as_object() {
                for (k, v) in obj {
                    let safe_col = sanitize_identifier(&to_snake_case(k))?;
                    match v {
                        serde_json::Value::String(s) => {
                            where_clauses.push(format!("t.\"{}\" = '{}'", safe_col, s.replace('\'', "''")));
                        }
                        serde_json::Value::Number(n) => {
                            where_clauses.push(format!("t.\"{}\" = {}", safe_col, n));
                        }
                        serde_json::Value::Bool(b) => {
                            where_clauses.push(format!("t.\"{}\" = {}", safe_col, if *b { "TRUE" } else { "FALSE" }));
                        }
                        serde_json::Value::Object(op_map) => {
                            for (op, val) in op_map {
                                match op.as_str() {
                                    "$gt" => where_clauses.push(format!("t.\"{}\" > {}", safe_col, val)),
                                    "$gte" => where_clauses.push(format!("t.\"{}\" >= {}", safe_col, val)),
                                    "$lt" => where_clauses.push(format!("t.\"{}\" < {}", safe_col, val)),
                                    "$lte" => where_clauses.push(format!("t.\"{}\" <= {}", safe_col, val)),
                                    "$ne" => where_clauses.push(format!("t.\"{}\" != {}", safe_col, val)),
                                    "$like" => {
                                        if let Some(pat) = val.as_str() {
                                            where_clauses.push(format!("t.\"{}\" LIKE '{}'", safe_col, pat.replace('\'', "''")));
                                        }
                                    }
                                    "$ilike" => {
                                        if let Some(pat) = val.as_str() {
                                            where_clauses.push(format!("t.\"{}\" ILIKE '{}'", safe_col, pat.replace('\'', "''")));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // 5. Build ORDER BY clauses
        let order_clause = if let Some(order_by) = &req.order_by {
            let mut parts = Vec::new();
            for ob in order_by {
                let safe_col = sanitize_identifier(&to_snake_case(&ob.field))?;
                let dir = if ob.desc { "DESC" } else { "ASC" };
                parts.push(format!("t.\"{}\" {}", safe_col, dir));
            }
            if parts.is_empty() {
                String::new()
            } else {
                format!("ORDER BY {}", parts.join(", "))
            }
        } else {
            "ORDER BY t.\"_id\" ASC".to_string()
        };

        // 6. Limit and Offset
        let limit_clause = format!("LIMIT {}", req.limit.unwrap_or(100));
        let offset_clause = req.offset.map(|o| format!("OFFSET {}", o)).unwrap_or_default();

        let full_sql = format!(
            "SELECT {} as data FROM \"{}\" t WHERE {} {} {} {};",
            select_expr,
            physical_table,
            where_clauses.join(" AND "),
            order_clause,
            limit_clause,
            offset_clause
        );

        let rows = sqlx::query_scalar::<_, serde_json::Value>(&full_sql)
            .fetch_all(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(rows)
    }

    /// Enqueue a job into the background asynchronous job queue
    pub async fn enqueue_job(
        &self,
        job_type: &str,
        codeunit_name: &str,
        method_name: &str,
        payload: serde_json::Value,
        chunk_size: usize,
    ) -> Result<Uuid, RuntimeError> {
        let pool = self.pool().await;
        let job_id = Uuid::new_v4();

        let insert_query = r#"
            INSERT INTO "_bos_job_queue" (
                "job_id", "job_type", "codeunit_name", "method_name", "payload", "chunk_size", "status"
            ) VALUES ($1, $2, $3, $4, $5, $6, 'ready');
        "#;

        sqlx::query(insert_query)
            .bind(job_id)
            .bind(job_type)
            .bind(codeunit_name)
            .bind(method_name)
            .bind(payload)
            .bind(chunk_size as i32)
            .execute(&pool)
            .await?;

        Ok(job_id)
    }

    /// Process next ready job from the background job queue
    pub async fn execute_next_job(&self, ctx: &SessionContext) -> Result<Option<Uuid>, RuntimeError> {
        let pool = self.pool().await;

        let claim_query = r#"
            UPDATE "_bos_job_queue"
            SET "status" = 'running', "started_at" = NOW(), "attempts" = "attempts" + 1
            WHERE "job_id" = (
                SELECT "job_id" FROM "_bos_job_queue"
                WHERE "status" = 'ready'
                ORDER BY "created_at" ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING "job_id", "codeunit_name", "method_name", "payload";
        "#;

        let row_opt = sqlx::query(claim_query).fetch_optional(&pool).await?;
        let row = match row_opt {
            Some(r) => r,
            None => return Ok(None),
        };

        let job_id: Uuid = row.try_get("job_id")?;
        let cu_name: String = row.try_get("codeunit_name")?;
        let method: String = row.try_get("method_name")?;
        let payload: serde_json::Value = row.try_get("payload")?;

        let exec_result = self.execute_procedure(&cu_name, &method, payload, ctx).await;

        match exec_result {
            Ok(_) => {
                let _ = sqlx::query(
                    "UPDATE \"_bos_job_queue\" SET \"status\" = 'completed', \"completed_at\" = NOW() WHERE \"job_id\" = $1"
                )
                .bind(job_id)
                .execute(&pool)
                .await;
            }
            Err(e) => {
                let _ = sqlx::query(
                    "UPDATE \"_bos_job_queue\" SET \"status\" = 'failed', \"error_message\" = $1, \"completed_at\" = NOW() WHERE \"job_id\" = $2"
                )
                .bind(e.to_string())
                .bind(job_id)
                .execute(&pool)
                .await;
            }
        }

        Ok(Some(job_id))
    }

    /// Privileged update for immutable ledger records
    pub async fn update_privileged(
        &self,
        table: &str,
        record_id: Uuid,
        column: &str,
        new_value: &str,
        reason: &str,
        ticket: &str,
        authorizer: &str,
        ctx: &SessionContext,
    ) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;

        self.apply_isolation_context(&mut tx, ctx).await?;
        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let set_override = "SET LOCAL bos.privileged_override = 'true';";
        let set_reason = format!("SET LOCAL bos.privileged_reason = '{}';", reason.replace("'", "''"));
        let set_ticket = format!("SET LOCAL bos.privileged_ticket = '{}';", ticket.replace("'", "''"));
        let set_auth = format!("SET LOCAL bos.privileged_authorizer = '{}';", authorizer.replace("'", "''"));

        sqlx::query(set_override).execute(&mut *tx).await?;
        sqlx::query(&set_reason).execute(&mut *tx).await?;
        sqlx::query(&set_ticket).execute(&mut *tx).await?;
        sqlx::query(&set_auth).execute(&mut *tx).await?;

        let safe_col = sanitize_identifier(column)?;
        let query = format!(
            "UPDATE \"{}\" SET \"{}\" = '{}', \"_updated_at\" = NOW() WHERE \"_id\" = '{}';",
            physical_table, safe_col, new_value.replace("'", "''"), record_id
        );

        let rows = sqlx::query(&query).execute(&mut *tx).await?.rows_affected();
        if rows == 0 {
            return Err(RuntimeError::NotFound(table.to_string(), record_id.to_string()));
        }

        tx.commit().await?;
        Ok(())
    }

    /// Atomic State Machine Transition
    pub async fn transition_state(
        &self,
        table: &str,
        record_id: Uuid,
        status_column: &str,
        from_state: &str,
        to_state: &str,
        action: &str,
        comments: Option<&str>,
        params: Option<serde_json::Value>,
        ctx: &SessionContext,
    ) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        let physical_table = self.resolve_physical_table(table, ctx).await?;
        let safe_status_col = sanitize_identifier(status_column)?;

        // Fetch current record for StateMachine guard / onTransition / draft evaluation
        let get_sql = format!("SELECT to_jsonb(t.*) as data FROM \"{}\" t WHERE \"_id\" = $1 LIMIT 1;", physical_table);
        let current_rec_opt = sqlx::query_scalar::<_, serde_json::Value>(&get_sql)
            .bind(record_id)
            .fetch_optional(&pool)
            .await?;

        // Check if an active StateMachine meta-object is registered for this table
        if let Ok(Some(sm_def)) = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1"
        )
        .bind(table)
        .fetch_optional(&pool)
        .await
        {
            if let Some(transitions) = sm_def.get("transitions").and_then(|v| v.as_array()) {
                let matched_tr = transitions.iter().find(|tr| {
                    let tr_name = tr.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let tr_to = tr.get("to_state").and_then(|v| v.as_str()).unwrap_or("");
                    let from_ok = tr.get("from_states").and_then(|v| v.as_array()).map_or(true, |arr| {
                        arr.iter().any(|f| f.as_str().map_or(false, |s| s.eq_ignore_ascii_case(from_state)))
                    });
                    (tr_name.eq_ignore_ascii_case(action) || tr_to.eq_ignore_ascii_case(to_state)) && from_ok
                });

                if let Some(tr) = matched_tr {
                    // 1. Enforce required_roles if specified
                    if let Some(req_roles) = tr.get("required_roles").and_then(|v| v.as_array()) {
                        let non_empty_roles: Vec<&str> = req_roles.iter().filter_map(|r| r.as_str()).filter(|s| !s.is_empty()).collect();
                        if !non_empty_roles.is_empty() && !ctx.roles.is_empty() {
                            let has_role = ctx.roles.iter().any(|ur| {
                                ur.eq_ignore_ascii_case("admin")
                                    || ur.eq_ignore_ascii_case("super_admin")
                                    || non_empty_roles.iter().any(|rr| rr.eq_ignore_ascii_case(ur))
                            });
                            if !has_role {
                                return Err(RuntimeError::TransitionError(format!(
                                    "Transition '{}' requires one of roles {:?}, held roles: {:?}",
                                    action, non_empty_roles, ctx.roles
                                )));
                            }
                        }
                    }

                    // 2. Execute guard_script if defined
                    if let Some(guard_src) = tr.get("guard_script").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
                        let rec_for_guard = current_rec_opt.clone().unwrap_or_else(|| serde_json::json!({}));
                        let wrapped_guard = if guard_src.contains("return ") || guard_src.contains(';') {
                            format!(
                                "const __guardRes = (() => {{ {} }})(); if (__guardRes === false) {{ throw new Error('Transition guard returned false'); }}",
                                guard_src
                            )
                        } else {
                            format!(
                                "const __guardRes = ({}); if (!__guardRes) {{ throw new Error('Transition guard failed: ' + {}); }}",
                                guard_src,
                                serde_json::to_string(guard_src).unwrap_or_else(|_| "\"guard\"".to_string())
                            )
                        };
                        crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                            None,
                            None,
                            None,
                            table.to_string(),
                            format!("guard_{}", action),
                            wrapped_guard,
                            rec_for_guard,
                            current_rec_opt.clone(),
                            ctx.clone(),
                            Some(pool.clone()),
                        )
                        .await
                        .map_err(|e| RuntimeError::TransitionError(format!("Guard rejected transition '{}': {}", action, e)))?;
                    }

                    // 3. Execute on_transition_script if defined
                    if let Some(effect_src) = tr.get("on_transition_script").and_then(|v| v.as_str()).filter(|s| !s.trim().is_empty()) {
                        let rec_for_effect = current_rec_opt.clone().unwrap_or_else(|| serde_json::json!({}));
                        let _ = crate::quickjs_engine::QuickJsEngine::execute_trigger_async(
                            None,
                            None,
                            None,
                            table.to_string(),
                            format!("onTransition_{}", action),
                            effect_src.to_string(),
                            rec_for_effect,
                            current_rec_opt.clone(),
                            ctx.clone(),
                            Some(pool.clone()),
                        )
                        .await?;
                    }
                }
            }
        }

        // If activating from Draft, run OnDraftActivate invariants
        if from_state.eq_ignore_ascii_case("Draft") {
            if let Some(mut draft_data) = current_rec_opt {
                let x_draft = draft_data.clone();
                execution_engine::ExecutionEngine::dispatch_triggers(
                    &pool,
                    table,
                    &bosc_ir::TriggerEventIR::OnDraftActivate,
                    &mut draft_data,
                    Some(&x_draft),
                    ctx,
                )
                .await?;
            }
        }

        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        // 1. Atomic conditional update
        let update_query = format!(
            "UPDATE \"{}\" SET \"{}\" = '{}', \"_system_version\" = \"_system_version\" + 1, \"_updated_at\" = NOW() WHERE \"_id\" = '{}' AND \"{}\" = '{}';",
            physical_table, safe_status_col, to_state, record_id, safe_status_col, from_state
        );

        let affected = sqlx::query(&update_query).execute(&mut *tx).await?.rows_affected();
        if affected == 0 {
            return Err(RuntimeError::TransitionError(format!(
                "Failed transition '{}' on {}: record not found or current state is not '{}'",
                action, record_id, from_state
            )));
        }

        // 2. Insert into State History
        let history_table = format!("{}_state_history", table);
        let comments_val = comments.map(|c| format!("'{}'", c.replace("'", "''"))).unwrap_or("NULL".to_string());
        let params_val = params.map(|p| format!("'{}'::jsonb", p.to_string().replace("'", "''"))).unwrap_or("NULL".to_string());

        let history_query = format!(
            "INSERT INTO \"{}\" (\"entity_id\", \"from_state\", \"to_state\", \"action\", \"actor_id\", \"comments\", \"params\") VALUES ('{}', '{}', '{}', '{}', '{}', {}, {});",
            history_table, record_id, from_state, to_state, action, ctx.user_id, comments_val, params_val
        );

        sqlx::query(&history_query).execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }

    /// Query records with pagination, sorting, filtering, and tenant isolation
    pub async fn query_records_paginated(
        &self,
        table: &str,
        params: &EntityQueryParams,
        ctx: &SessionContext,
    ) -> Result<PaginatedRecords, RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let (physical_table, data_per_company) = self.resolve_table_info(table, ctx).await?;

        let limit = params.limit.unwrap_or(50).clamp(1, 1000);
        let offset = if let Some(p) = params.page {
            (p.max(1) - 1) * limit
        } else {
            params.offset.unwrap_or(0).max(0)
        };
        let page = (offset / limit) + 1;

        let projection = match &params.fields {
            Some(f) if !f.is_empty() => {
                let valid_fields: Vec<&String> = f
                    .iter()
                    .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                    .collect();
                if valid_fields.is_empty() {
                    "to_jsonb(t.*)".to_string()
                } else {
                    let mut pairs = Vec::new();
                    let mut has_id = false;
                    for col in &valid_fields {
                        if *col == "_id" || *col == "id" {
                            has_id = true;
                        }
                        pairs.push(format!("'{}', t.\"{}\"", col, col));
                    }
                    if !has_id {
                        pairs.push("'_id', t.\"_id\"".to_string());
                    }
                    format!("json_build_object({})", pairs.join(", "))
                }
            }
            _ => "to_jsonb(t.*)".to_string(),
        };

        // Multi-tenant isolation WHERE conditions
        let mut where_clauses = vec![format!("t.\"_tenant_id\" = '{}'", ctx.tenant_id)];
        if data_per_company {
            where_clauses.push(format!("t.\"_company_id\" = '{}'", ctx.company_id));
        }

        // Optional filter (supports JSON object or key:value)
        if let Some(f_str) = &params.filter {
            if let Ok(filter_obj) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(f_str) {
                for (k, v) in filter_obj {
                    let col_name = if k.starts_with('_') { k.clone() } else { to_snake_case(&k) };
                    if let Ok(safe_col) = sanitize_identifier(&col_name) {
                        match v {
                            serde_json::Value::String(s) => {
                                where_clauses.push(format!("t.\"{}\"::text ILIKE '%{}%'", safe_col, s.replace('\'', "''")));
                            }
                            serde_json::Value::Number(n) => {
                                where_clauses.push(format!("t.\"{}\" = {}", safe_col, n));
                            }
                            serde_json::Value::Bool(b) => {
                                where_clauses.push(format!("t.\"{}\" = {}", safe_col, if b { "TRUE" } else { "FALSE" }));
                            }
                            serde_json::Value::Null => {
                                where_clauses.push(format!("t.\"{}\" IS NULL", safe_col));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let where_sql = where_clauses.join(" AND ");

        // Sort / Order By parsing
        let mut order_sql = "t.\"_created_at\" DESC".to_string();
        if let Some(s) = &params.sort {
            let parts: Vec<&str> = s.split(',').collect();
            let mut order_parts = Vec::new();
            for p in parts {
                let trimmed = p.trim();
                if trimmed.is_empty() { continue; }
                let (col, dir) = if let Some((c, d)) = trimmed.split_once(':') {
                    (c.trim(), if d.trim().eq_ignore_ascii_case("desc") { "DESC" } else { "ASC" })
                } else if let Some((c, d)) = trimmed.split_once(' ') {
                    (c.trim(), if d.trim().eq_ignore_ascii_case("desc") { "DESC" } else { "ASC" })
                } else {
                    (trimmed, "ASC")
                };
                let col_name = if col.starts_with('_') { col.to_string() } else { to_snake_case(col) };
                if let Ok(safe_col) = sanitize_identifier(&col_name) {
                    order_parts.push(format!("t.\"{}\" {}", safe_col, dir));
                }
            }
            if !order_parts.is_empty() {
                order_sql = order_parts.join(", ");
            }
        }

        // Total count
        let count_query = format!("SELECT COUNT(*)::bigint FROM \"{}\" t WHERE {};", physical_table, where_sql);
        let total: i64 = sqlx::query_scalar::<_, i64>(&count_query)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0);

        // Paginated query
        let data_query = format!(
            "SELECT {} as data FROM \"{}\" t WHERE {} ORDER BY {} LIMIT {} OFFSET {};",
            projection, physical_table, where_sql, order_sql, limit, offset
        );
        let records = sqlx::query_scalar::<_, serde_json::Value>(&data_query)
            .fetch_all(&mut *tx)
            .await?;

        tx.commit().await?;

        let total_pages = if total == 0 { 1 } else { (total + limit - 1) / limit };
        let has_more = offset + (records.len() as i64) < total;

        Ok(PaginatedRecords {
            records,
            total,
            limit,
            offset,
            page,
            total_pages,
            has_more,
        })
    }

    /// Query records (unpaginated wrapper forwarding to query_records_paginated)
    pub async fn query_records(
        &self,
        table: &str,
        fields: Option<&[String]>,
        ctx: &SessionContext,
    ) -> Result<Vec<serde_json::Value>, RuntimeError> {
        let params = EntityQueryParams {
            fields: fields.map(|f| f.to_vec()),
            limit: Some(100),
            ..Default::default()
        };
        let paginated = self.query_records_paginated(table, &params, ctx).await?;
        Ok(paginated.records)
    }

    /// Get single record by ID with tenant isolation, strategy routing, and partial record projection (LoadFields)
    pub async fn get_record(
        &self,
        table: &str,
        id: Uuid,
        fields: Option<&[String]>,
        ctx: &SessionContext,
    ) -> Result<Option<serde_json::Value>, RuntimeError> {
        let pool = self.pool().await;
        let mut tx = pool.begin().await?;
        self.apply_isolation_context(&mut tx, ctx).await?;

        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let projection = match fields {
            Some(f) if !f.is_empty() => {
                let valid_fields: Vec<&String> = f
                    .iter()
                    .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
                    .collect();
                if valid_fields.is_empty() {
                    "to_jsonb(t.*)".to_string()
                } else {
                    let mut pairs = Vec::new();
                    let mut has_id = false;
                    for col in &valid_fields {
                        if *col == "_id" || *col == "id" {
                            has_id = true;
                        }
                        pairs.push(format!("'{}', t.\"{}\"", col, col));
                    }
                    if !has_id {
                        pairs.push("'_id', t.\"_id\"".to_string());
                    }
                    format!("json_build_object({})", pairs.join(", "))
                }
            }
            _ => "to_jsonb(t.*)".to_string(),
        };

        let query = format!("SELECT {} as data FROM \"{}\" t WHERE \"_id\" = $1;", projection, physical_table);
        let row = sqlx::query_scalar::<_, serde_json::Value>(&query)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Compute FlowField live via generated subquery
    pub async fn calculate_flowfield(
        &self,
        table: &str,
        record_id: Uuid,
        computed_subquery: &str,
    ) -> Result<f64, RuntimeError> {
        let pool = self.pool().await;
        let query = format!("SELECT {} as val FROM \"{}\" WHERE \"_id\" = '{}';", computed_subquery, table, record_id);
        let row = sqlx::query(&query).fetch_one(&pool).await?;
        let val: Option<f64> = row.try_get("val").unwrap_or(Some(0.0));
        Ok(val.unwrap_or(0.0))
    }

    /// Execute business logic test suite in an auto-rollback transaction (Gap 6: @test runner)
    pub async fn run_tests(
        &self,
        codeunit_filter: Option<&str>,
        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let pool = self.pool().await;
        execution_engine::ExecutionEngine::execute_tests(&pool, codeunit_filter, ctx).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_identifier() {
        assert!(sanitize_identifier("code").is_ok());
        assert!(sanitize_identifier("customer_ledger_entry").is_ok());
        assert!(sanitize_identifier("comp$table").is_ok());
        assert!(sanitize_identifier("").is_err());
        assert!(sanitize_identifier("drop table users").is_err());
        assert!(sanitize_identifier("col\" = 1; --").is_err());
        assert!(sanitize_identifier("name' OR '1'='1").is_err());
    }

    #[test]
    fn test_validate_db_name() {
        assert!(DatabaseManager::validate_db_name("bos_dev").is_ok());
        assert!(DatabaseManager::validate_db_name("crm-db_2").is_ok());
        assert!(DatabaseManager::validate_db_name("").is_err());
        assert!(DatabaseManager::validate_db_name("template0").is_err());
        assert!(DatabaseManager::validate_db_name("template1").is_err());
        assert!(DatabaseManager::validate_db_name("db; DROP DATABASE x;").is_err());
    }

    #[test]
    fn test_replace_database_name() {
        let url = "postgres://postgres:postgres@localhost:5432/bos_dev?sslmode=disable";
        let replaced = DatabaseManager::replace_database_name(url, "bos_prod");
        assert_eq!(replaced, "postgres://postgres:postgres@localhost:5432/bos_prod?sslmode=disable");

        let url2 = "postgres://postgres:postgres@localhost:5432/bos_dev";
        let replaced2 = DatabaseManager::replace_database_name(url2, "postgres");
        assert_eq!(replaced2, "postgres://postgres:postgres@localhost:5432/postgres");
    }

    #[tokio::test]
    async fn test_router_construction() {
        // Build test BosEngine instance to verify router compiles and builds with no route conflicts
        let engine = BosEngine {
            pool: Arc::new(tokio::sync::RwLock::new(
                sqlx::postgres::PgPoolOptions::new()
                    .connect_lazy("postgres://postgres:postgres@localhost:5432/postgres")
                    .unwrap(),
            )),
            active_url: Arc::new(tokio::sync::RwLock::new("postgres://postgres:postgres@localhost:5432/postgres".to_string())),
            meta_cache: Arc::new(dashmap::DashMap::new()),
            event_registry: Arc::new(EventRegistry::new()),
            wasm_engine: Arc::new(WasmEngine::new().unwrap()),
            sidecar_host: Arc::new(SidecarHost::new()),
        };

        // Verifies no panics during Axum router route registration (matchit conflict checks)
        let _router = create_api_router(engine);
    }

    #[test]
    fn test_meta_cache_operations() {
        let cache = Arc::new(dashmap::DashMap::new());
        let sample = meta_manager::MetaObjectDetails {
            summary: meta_manager::MetaObjectSummary {
                object_id: Uuid::new_v4(),
                object_type: "table".to_string(),
                object_name: "Customer".to_string(),
                package_name: "base".to_string(),
                version: "1.0".to_string(),
                caption: None,
                sub_type: Some("Normal".to_string()),
                status: "active".to_string(),
                created_at: chrono::Utc::now(),
            },
            fields: vec![],
            relations: vec![],
            definition: serde_json::json!({ "name": "Customer" }),
            object_type: "table".to_string(),
            source_script: None,
        };

        cache.insert("customer".to_string(), sample.clone());
        assert!(cache.contains_key("customer"));
        assert_eq!(cache.get("customer").unwrap().summary.object_name, "Customer");

        cache.remove("customer");
        assert!(!cache.contains_key("customer"));
    }
}
