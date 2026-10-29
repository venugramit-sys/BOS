#![recursion_limit = "256"]

use serde::{Deserialize, Serialize};

use sqlx::{PgPool, Row, postgres::PgPoolOptions};

use std::sync::Arc;

use thiserror::Error;

use uuid::Uuid;

use bosc_ir::sql_type_to_bos_type;

pub mod detector;

pub mod db_manager;

pub mod db_session;

pub mod schema_sync;

pub mod meta_manager;

pub mod package_manager;
pub mod package_vault;
pub mod bytecode_cache;
pub mod prepared_statement_registry;
pub mod page_compiler;
pub mod page_artifacts;
mod page_routes;
pub mod page_designer;

pub mod execution_engine;

pub mod quickjs_engine;

pub mod server;

pub mod auth_guard;

pub mod event_engine;

pub mod wasm_engine;

pub mod sidecar_host;

pub mod node_config;

pub mod nas_worker;

pub mod notification;

pub use notification::{
    Notification, NotificationAction, NotificationHub, NotificationSeverity, NotificationTarget,
};

pub use detector::{DatabaseDetector, DetectedDatabase, ProviderKind};

pub use node_config::{
    DatabaseSection, DefaultsSection, NasWorkerSection, NetworkSection, NodeConfig,
    NodeConfigError, NodeRole, NodeSection, SecuritySection, UiSection,
};

pub use nas_worker::{NasWorkerDaemon, NasWorkerHandle};

pub use db_manager::{
    CompanyInfo, ConnectionTestResult, CreateCompanyRequest, CreateDatabaseRequest,
    CreateTenantRequest, DatabaseDetails, DatabaseInfo, DatabaseManager, DropDatabaseQuery,
    ManageDatabaseRequest, SwitchDatabaseRequest, TableStats, TenantInfo, TestConnectionRequest,
    UpdateDatabaseRequest,
};

pub use meta_manager::{
    ApplyTableScriptRequest, ApplyTableScriptResponse, ColumnDefinition, CompileScriptRequest,
    CompileScriptResponse, MetaFieldInfo, MetaManager, MetaObjectDetails, MetaObjectSummary,
    MetaRelationInfo, RegisterMetaObjectRequest, RegisterTableExtensionRequest,
    StateMachineDefinition, SystemProvisionStatus,
};

pub use package_manager::{PackageInstallResult, PackageManager};
pub use bytecode_cache::BytecodeCache;
pub use prepared_statement_registry::PreparedStatementRegistry;
pub use page_artifacts::{PageManifest, PageService, PageServiceConfig, PageServiceError, RefreshReport};
pub use page_compiler::{PageCompilerError, SvelteCompiler};
pub use node_config::PagesSection;

/// Outcome of a server-side page trigger (`POST /api/v1/pages/{page}/triggers/{trigger}`).
#[derive(Debug, Clone, Serialize)]
pub struct PageTriggerResult {
    /// The record after the trigger ran (triggers such as `onValidate` may change fields).
    pub record: serde_json::Value,
    /// Page variables after the trigger ran.
    pub variables: serde_json::Value,
    /// `ctx.ui` toasts and messages.
    pub messages: serde_json::Value,
    /// `ctx.ui` directives (`modal`, `navigate`, `fieldVisible`, `fieldEditable`, `confirm`).
    pub directives: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_value: Option<serde_json::Value>,
    pub logs: Vec<String>,
    pub fuel_used: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notifications: Vec<crate::Notification>,
}

pub use execution_engine::{ExecScope, ExecutionEngine, ExecutionResult};

pub use event_engine::{EventRegistry, EventSubscriber};

pub use quickjs_engine::{ExecutionLimits, ScriptServices};

use bosc_ir::TriggerEventIR;

use quickjs_engine::{QuickJsEngine, ScriptCall};

pub use wasm_engine::WasmEngine;

pub use sidecar_host::{SidecarDescriptor, SidecarHost, SidecarStatus};

pub use server::{create_api_router, create_node_router};

pub use db_session::DbSession;

pub use schema_sync::{
    RelationStatus, SchemaSync, SchemaSyncOptions, SchemaSyncReport, SchemaSyncRequest,
};

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

pub struct FirstRunProvisionRequest {
    pub database: Option<String>,

    pub connection_url: Option<String>,

    pub storage_strategy: Option<String>,

    #[serde(default = "default_provision_true")]
    pub import_base_packages: bool,

    #[serde(default)]
    pub packages: Vec<String>,

    pub tenant_code: Option<String>,

    pub tenant_name: Option<String>,

    pub company_code: Option<String>,

    pub company_name: Option<String>,
}

fn default_provision_true() -> bool {
    true
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

            roles: Vec::new(),
        }
    }
}

pub fn sanitize_identifier(ident: &str) -> Result<String, RuntimeError> {
    let clean = ident.trim();

    if clean.is_empty() {
        return Err(RuntimeError::SecurityError(
            "SQL identifier cannot be empty".to_string(),
        ));
    }

    if clean
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        Ok(clean.to_string())
    } else {
        Err(RuntimeError::SecurityError(format!(
            "Invalid characters in SQL identifier: '{}'",
            ident
        )))
    }
}

/// Columns the runtime owns; client patches can never overwrite them.

const SYSTEM_WRITE_PROTECTED: &[&str] = &[
    "_system_version",
    "system_version",
    "systemversion",
    "_id",
    "id",
    "_tenant_id",
    "tenant_id",
    "tenantid",
    "_company_id",
    "company_id",
    "companyid",
    "_created_at",
    "created_at",
    "createdat",
    "_created_by",
    "created_by",
    "createdby",
    "_updated_at",
    "updated_at",
    "updatedat",
    "_updated_by",
    "updated_by",
    "updatedby",
    "_system_messages",
    "system_messages",
    "systemmessages",
    "_messages",
    "messages",
];

/// Quote a resolved physical table name; schema-qualified names arrive already quoted.

fn quote_physical(physical: &str) -> String {
    if physical.starts_with('"') {
        physical.to_string()
    } else {
        format!("\"{}\"", physical)
    }
}

/// Render a JSON value as an escaped SQL literal whose type PostgreSQL infers from the target column.

fn sql_literal(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),

        serde_json::Value::Number(n) => n.to_string(),

        serde_json::Value::Bool(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }

        serde_json::Value::Null => "NULL".to_string(),

        other => format!("'{}'::jsonb", other.to_string().replace('\'', "''")),
    }
}

/// Turn `:name` markers in a compiler-built SELECT into `$n` bindings.

fn bind_named_query(
    sql: &str,
    params: &serde_json::Value,
) -> Result<(String, Vec<serde_json::Value>), RuntimeError> {
    let trimmed = sql.trim().trim_end_matches(';').trim();

    if trimmed.is_empty() {
        return Err(RuntimeError::Internal(
            "The query has no SQL to run".to_string(),
        ));
    }

    if trimmed.contains(';') {
        return Err(RuntimeError::SecurityError(
            "A query runs one statement".to_string(),
        ));
    }

    let head = trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();

    if head != "SELECT" && head != "WITH" {
        return Err(RuntimeError::SecurityError(
            "A query runs a SELECT".to_string(),
        ));
    }

    let map = params.as_object();

    let chars: Vec<char> = trimmed.chars().collect();

    let mut out = String::new();

    let mut bound = Vec::new();

    let mut i = 0;

    while i < chars.len() {
        if chars[i] == ':' && i + 1 < chars.len() && chars[i + 1] == ':' {
            out.push(':');

            out.push(':');

            i += 2;

            continue;
        }

        if chars[i] == ':'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
        {
            let mut j = i + 1;

            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }

            let name: String = chars[i + 1..j].iter().collect();

            let value = map
                .and_then(|m| m.get(&name))
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            bound.push(value);

            out.push('$');

            out.push_str(&bound.len().to_string());

            i = j;

            continue;
        }

        out.push(chars[i]);

        i += 1;
    }

    Ok((out, bound))
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

/// Securely hash plain-text password using standard Blowfish bcrypt (cost: 10, memory: 4 KB)
pub fn hash_password_bcrypt(plain: &str) -> String {
    match bcrypt::hash(plain, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!("bcrypt hashing fallback to sha256: {:?}", e);
            hash_password_sha256(plain)
        }
    }
}

pub fn hash_password_sha256(plain: &str) -> String {
    use sha2::{Digest, Sha256};
    let salt = Uuid::new_v4().to_string().replace('-', "")[..16].to_string();
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(plain.as_bytes());
    let hex_hash = format!("{:x}", hasher.finalize());
    format!("$sha256${}${}", salt, hex_hash)
}

pub fn hash_password(plain: &str) -> String {
    hash_password_bcrypt(plain)
}

/// Securely hash plain-text password using official RFC 9106 Argon2id
pub fn hash_password_argon2(plain: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    match argon2.hash_password(plain.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            tracing::warn!("argon2 hashing error: {:?}, fallback to bcrypt", e);
            hash_password_bcrypt(plain)
        }
    }
}

/// Verify plain-text password against stored hash (supporting standard Blowfish $2a$/$2b$/$2y$, RFC 9106 $argon2id$, and $sha256$)
pub fn verify_password(plain: &str, stored_hash: &str) -> bool {
    // 1. Standard Blowfish bcrypt hash verification ($2a$, $2b$, $2y$)
    if stored_hash.starts_with("$2a$") || stored_hash.starts_with("$2b$") || stored_hash.starts_with("$2y$") {
        return bcrypt::verify(plain, stored_hash).unwrap_or(false);
    }

    // 2. Official RFC 9106 Argon2id hash verification
    if stored_hash.starts_with("$argon2") {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };
        if let Ok(parsed_hash) = PasswordHash::new(stored_hash) {
            if Argon2::default().verify_password(plain.as_bytes(), &parsed_hash).is_ok() {
                return true;
            }
        }
        // Backward-compatible fallback for prior simulated bos-argon2id-v19 hashes
        if stored_hash.contains("$m=4096,t=3,p=1$") {
            return verify_legacy_pseudo_argon2(plain, stored_hash);
        }
        return false;
    }

    // 2. Salted SHA-256 verification ($sha256$salt$hash)
    let parts: Vec<&str> = stored_hash.split('$').collect();
    match parts.as_slice() {
        ["", "sha256", salt, expected_hex] => {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(salt.as_bytes());
            hasher.update(plain.as_bytes());
            let hex_hash = format!("{:x}", hasher.finalize());
            if hex_hash.len() != expected_hex.len() {
                return false;
            }
            let mut diff = 0u8;
            for (a, b) in hex_hash.bytes().zip(expected_hex.bytes()) {
                diff |= a ^ b;
            }
            diff == 0
        }
        _ => false,
    }
}

fn verify_legacy_pseudo_argon2(plain: &str, stored_hash: &str) -> bool {
    use sha2::{Digest, Sha256};
    let parts: Vec<&str> = stored_hash.split('$').collect();
    if let ["", "argon2id", _v, _params, salt, expected_hex] = parts.as_slice() {
        let mut state = {
            let mut h = Sha256::new();
            h.update(b"bos-argon2id-v19:");
            h.update(salt.as_bytes());
            h.update(b":");
            h.update(plain.as_bytes());
            h.finalize()
        };
        for round in 0u32..4096 {
            let mut h = Sha256::new();
            h.update(state);
            h.update(salt.as_bytes());
            h.update(plain.as_bytes());
            h.update(round.to_le_bytes());
            state = h.finalize();
        }
        let mut computed_hex = String::with_capacity(64);
        for b in state {
            computed_hex.push_str(&format!("{:02x}", b));
        }
        if computed_hex.len() != expected_hex.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in computed_hex.bytes().zip(expected_hex.bytes()) {
            diff |= a ^ b;
        }
        diff == 0
    } else {
        false
    }
}

/// Transparently calculate password hash if client sends plain password ("password", "plainPassword", "plain_password")

pub fn preprocess_password_fields(data: &mut serde_json::Value) {
    if let Some(obj) = data.as_object_mut() {
        let plain_pwd = obj
            .get("password")
            .or_else(|| obj.get("plainPassword"))
            .or_else(|| obj.get("plain_password"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        if let Some(pwd) = plain_pwd {
            if !pwd.trim().is_empty() {
                let computed_hash = hash_password(&pwd);

                // Set both snake_case and camelCase so triggers and SQL statements resolve seamlessly

                obj.insert(
                    "password_hash".to_string(),
                    serde_json::json!(computed_hash),
                );

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

    pub notification_hub: Arc<NotificationHub>,

    pub bytecode_cache: Arc<BytecodeCache>,

    pub prepared_statements: Arc<PreparedStatementRegistry>,

    emergency_sessions: Arc<dashmap::DashMap<String, (tokio::time::Instant, Vec<String>)>>,

    /// Compiled page service (Svelte page artifacts); absent until [`BosEngine::start_page_service`].
    pages: Arc<arc_swap::ArcSwapOption<page_artifacts::PageService>>,
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
            if let Ok(mut listener) = sqlx::postgres::PgListener::connect_with(&listener_pool).await
            {
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

        let event_registry = Arc::new(
            EventRegistry::load_from_db(&pool)
                .await
                .unwrap_or_else(|_| EventRegistry::new()),
        );

        let wasm_engine = Arc::new(WasmEngine::new()?);

        let sidecar_host = Arc::new(SidecarHost::new());

        let bytecode_cache = Arc::new(BytecodeCache::new());
        let _ = bytecode_cache.load_from_dir(std::path::Path::new("dist/packages"));
        let _ = bytecode_cache.load_from_dir(std::path::Path::new("../../dist/packages"));

        let prepared_statements = Arc::new(PreparedStatementRegistry::new());
        let _ = prepared_statements.load_from_dir(std::path::Path::new("dist/packages"));
        let _ = prepared_statements.load_from_dir(std::path::Path::new("../../dist/packages"));

        Ok(Self {
            pool: Arc::new(tokio::sync::RwLock::new(pool)),

            active_url: Arc::new(tokio::sync::RwLock::new(database_url.to_string())),

            meta_cache,

            event_registry,

            wasm_engine,

            sidecar_host,

            notification_hub: Arc::new(NotificationHub::default_hub()),

            bytecode_cache,

            prepared_statements,

            emergency_sessions: Arc::new(dashmap::DashMap::new()),

            pages: Arc::new(arc_swap::ArcSwapOption::empty()),
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

    /// Register an active emergency Super Admin session with a TTL.
    pub fn register_emergency_session(&self, token: &str, roles: Vec<String>, ttl: std::time::Duration) {
        let expires_at = tokio::time::Instant::now() + ttl;
        self.emergency_sessions.insert(token.to_string(), (expires_at, roles));
    }

    /// Validate an emergency session token. Returns the assigned roles if still valid.
    pub fn validate_emergency_session(&self, token: &str) -> Option<Vec<String>> {
        if let Some(entry) = self.emergency_sessions.get(token) {
            let (expires_at, roles) = entry.value();
            if tokio::time::Instant::now() < *expires_at {
                return Some(roles.clone());
            }
        }
        None
    }

    /// Clear all active emergency sessions (e.g. when exiting maintenance mode).
    pub fn clear_emergency_sessions(&self) {
        self.emergency_sessions.clear();
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
            .acquire_timeout(std::time::Duration::from_secs(10))
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

        // Pages belong to a database: resolve the new database's catalog (no NOTIFY: nothing changed there).
        self.refresh_pages().await;

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

        if let Err(err) = sqlx::raw_sql(migration_sql).execute(pool).await {
            tracing::debug!("kernel catalog upgrade skipped (database not provisioned yet): {err}");
        }

        if let Err(err) = SchemaSync::ensure_support(pool).await {
            tracing::warn!("could not install schema migration support objects: {err}");
        }

        if let Err(err) = EventRegistry::init_table(pool).await {
            tracing::warn!("could not install event outbox support objects: {err}");
        }
    }

    /// List all databases on the connected PostgreSQL cluster

    pub async fn list_databases(&self) -> Result<Vec<DatabaseInfo>, RuntimeError> {
        let pool = self.pool().await;

        let current_db = self.active_db_name().await;

        DatabaseManager::list_databases(&pool, &current_db).await
    }

    /// List all databases on a specified PostgreSQL server URL

    pub async fn list_databases_for_url(
        &self,
        url: &str,
    ) -> Result<Vec<DatabaseInfo>, RuntimeError> {
        let pool = PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(8))
            .max_connections(2)
            .connect(url)
            .await?;

        let current_db = DatabaseManager::extract_database_name(url);

        let res = DatabaseManager::list_databases(&pool, &current_db).await;

        pool.close().await;

        res
    }

    /// Create a new PostgreSQL database with optional extensions and user-specified tenant/company

    pub async fn create_database(
        &self,

        req: &db_manager::CreateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let current_url = self.active_url().await;

        DatabaseManager::create_database(&current_url, req).await
    }

    /// List all tenants registered in active or specified database

    pub async fn list_tenants(&self) -> Result<Vec<db_manager::TenantInfo>, RuntimeError> {
        self.list_tenants_for_db(None).await
    }

    pub async fn list_tenants_for_db(
        &self,
        database: Option<&str>,
    ) -> Result<Vec<db_manager::TenantInfo>, RuntimeError> {
        let active_db = self.active_db_name().await;

        if let Some(target_db) = database.filter(|d| *d != active_db.as_str()) {
            DatabaseManager::validate_db_name(target_db)?;

            let current_url = self.active_url().await;

            let target_url = DatabaseManager::replace_database_name(&current_url, target_db);

            let target_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let res = DatabaseManager::list_tenants(&target_pool).await;

            target_pool.close().await;

            res
        } else {
            let pool = self.pool().await;

            DatabaseManager::list_tenants(&pool).await
        }
    }

    /// Create a new tenant in active or specified database

    pub async fn create_tenant(
        &self,
        req: db_manager::CreateTenantRequest,
    ) -> Result<db_manager::TenantInfo, RuntimeError> {
        self.create_tenant_for_db(None, req).await
    }

    pub async fn create_tenant_for_db(
        &self,
        database: Option<&str>,
        req: db_manager::CreateTenantRequest,
    ) -> Result<db_manager::TenantInfo, RuntimeError> {
        let target_db = req.database.as_deref().or(database);

        let active_db = self.active_db_name().await;

        if let Some(db_name) = target_db.filter(|d| *d != active_db.as_str()) {
            DatabaseManager::validate_db_name(db_name)?;

            let current_url = self.active_url().await;

            let target_url = DatabaseManager::replace_database_name(&current_url, db_name);

            let target_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let res = DatabaseManager::create_tenant(&target_pool, req).await;

            target_pool.close().await;

            res
        } else {
            let pool = self.pool().await;

            DatabaseManager::create_tenant(&pool, req).await
        }
    }

    /// List companies registered in active or specified database

    pub async fn list_companies(
        &self,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<db_manager::CompanyInfo>, RuntimeError> {
        self.list_companies_for_db(None, tenant_id).await
    }

    pub async fn list_companies_for_db(
        &self,
        database: Option<&str>,
        tenant_id: Option<Uuid>,
    ) -> Result<Vec<db_manager::CompanyInfo>, RuntimeError> {
        let active_db = self.active_db_name().await;

        if let Some(target_db) = database.filter(|d| *d != active_db.as_str()) {
            DatabaseManager::validate_db_name(target_db)?;

            let current_url = self.active_url().await;

            let target_url = DatabaseManager::replace_database_name(&current_url, target_db);

            let target_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let res = DatabaseManager::list_companies(&target_pool, tenant_id).await;

            target_pool.close().await;

            res
        } else {
            let pool = self.pool().await;

            DatabaseManager::list_companies(&pool, tenant_id).await
        }
    }

    /// Create a new company under a tenant in active or specified database

    pub async fn create_company(
        &self,
        req: db_manager::CreateCompanyRequest,
    ) -> Result<db_manager::CompanyInfo, RuntimeError> {
        self.create_company_for_db(None, req).await
    }

    pub async fn create_company_for_db(
        &self,
        database: Option<&str>,
        req: db_manager::CreateCompanyRequest,
    ) -> Result<db_manager::CompanyInfo, RuntimeError> {
        let target_db = req.database.as_deref().or(database);

        let active_db = self.active_db_name().await;

        if let Some(db_name) = target_db.filter(|d| *d != active_db.as_str()) {
            DatabaseManager::validate_db_name(db_name)?;

            let current_url = self.active_url().await;

            let target_url = DatabaseManager::replace_database_name(&current_url, db_name);

            let target_pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&target_url)
                .await?;

            let strat: Option<String> = sqlx::query_scalar(
                "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
            )
            .fetch_optional(&target_pool)
            .await
            .unwrap_or(None);

            let res = DatabaseManager::create_company(&target_pool, req, strat.as_deref()).await;

            target_pool.close().await;

            res
        } else {
            let pool = self.pool().await;

            let strat: Option<String> = sqlx::query_scalar(
                "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
            )
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            DatabaseManager::create_company(&pool, req, strat.as_deref()).await
        }
    }

    /// Clone an existing company with optional ledger history
    pub async fn clone_company(
        &self,
        req: db_manager::CloneCompanyRequest,
    ) -> Result<db_manager::CompanyInfo, RuntimeError> {
        let pool = self.pool().await;
        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        DatabaseManager::clone_company(&pool, req, strat.as_deref()).await
    }

    /// Drop a company and its underlying isolated storage
    pub async fn drop_company(&self, code: &str) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        DatabaseManager::drop_company(&pool, code, strat.as_deref()).await
    }

    /// Run the schema drift sentinel report
    pub async fn check_schema_drift(&self) -> Result<db_manager::SchemaDriftReport, RuntimeError> {
        let pool = self.pool().await;
        DatabaseManager::check_schema_drift(&pool).await
    }

    /// Provision BOS system kernel on active database

    pub async fn provision_system(
        &self,
        storage_strategy: Option<&str>,
    ) -> Result<SystemProvisionStatus, RuntimeError> {
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

    pub async fn register_meta_object(
        &self,
        req: RegisterMetaObjectRequest,
    ) -> Result<MetaObjectSummary, RuntimeError> {
        let name = req.name.clone();

        let pool = self.pool().await;

        let res = MetaManager::register_meta_object(&pool, req).await?;

        self.invalidate_meta_cache(&name).await;

        self.republish_pages().await;

        Ok(res)
    }

    /// List all registered meta-objects with optional type filter

    pub async fn list_meta_objects(
        &self,
        object_type: Option<&str>,
    ) -> Result<Vec<MetaObjectSummary>, RuntimeError> {
        let pool = self.pool().await;

        MetaManager::list_meta_objects(&pool, object_type).await
    }

    /// Inspect details of a specific meta-object with high-speed DashMap read caching

    pub async fn get_meta_object(
        &self,
        name: &str,
    ) -> Result<Option<MetaObjectDetails>, RuntimeError> {
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

    /// Assign or remove a feature tag on a meta-object
    pub async fn assign_meta_object_feature(
        &self,
        name: &str,
        feature: Option<&str>,
    ) -> Result<(), RuntimeError> {
        let pool = self.pool().await;
        MetaManager::assign_meta_object_feature(&pool, name, feature).await?;
        self.invalidate_meta_cache(name).await;
        Ok(())
    }

    /// Resolve a page definition with all matching pageextensions and source table metadata
    pub async fn resolve_page(&self, page_name: &str) -> Result<Option<bosc_ir::ResolvedPageIR>, RuntimeError> {
        let pool = self.pool().await;

        // 1. Fetch base page
        let page_row = sqlx::query(
            r#"SELECT definition FROM "_bos_meta_objects" 
               WHERE LOWER(object_name) = LOWER($1) AND object_type = 'page' 
               ORDER BY updated_at DESC LIMIT 1"#
        )
        .bind(page_name)
        .fetch_optional(&pool)
        .await?;

        let page_def = match page_row {
            Some(row) => {
                let def: serde_json::Value = row.try_get("definition").map_err(|e| RuntimeError::Internal(e.to_string()))?;
                serde_json::from_value::<bosc_ir::PageIR>(def)
                    .map_err(|e| RuntimeError::Internal(format!("Failed to parse PageIR for {}: {}", page_name, e)))?
            }
            None => return Ok(None),
        };

        // 2. Fetch all matching page extensions targeting this page
        let ext_rows = sqlx::query(
            r#"SELECT definition FROM "_bos_meta_objects" 
               WHERE object_type = 'pageextension' AND LOWER(definition->>'target_page') = LOWER($1)
               ORDER BY created_at ASC"#
        )
        .bind(page_name)
        .fetch_all(&pool)
        .await?;

        let mut extensions = Vec::new();
        for row in ext_rows {
            if let Ok(def) = row.try_get::<serde_json::Value, _>("definition") {
                if let Ok(ext_ir) = serde_json::from_value::<bosc_ir::PageExtensionIR>(def) {
                    extensions.push(ext_ir);
                }
            }
        }

        // 3. Merge base page with extensions using bosc-analyzer merger
        let merged_page = bosc_analyzer::merge_page_with_extensions(&page_def, &extensions);

        // 4. Resolve field metadata from source_table if available
        let mut resolved_fields = std::collections::BTreeMap::new();
        let mut state_machine = None;
        let mut initial_state = None;
        let mut transitions = Vec::new();

        if let Some(ref tbl_name) = merged_page.source_table {
            if let Ok(Some(tbl_details)) = self.get_meta_object(tbl_name).await {
                if let Ok(tbl_ir) = serde_json::from_value::<bosc_ir::TableIR>(tbl_details.definition) {
                    for col in &tbl_ir.columns {
                        let (rel_tbl, rel_col) = match col.foreign_key {
                            Some(ref fk) => (Some(fk.target_table.clone()), Some(fk.target_column.clone())),
                            None => (None, None),
                        };
                        resolved_fields.insert(
                            col.name.clone(),
                            bosc_ir::ResolvedFieldMetaIR {
                                name: col.name.clone(),
                                caption: Some(col.caption.clone()),
                                data_type: col.data_type.clone().unwrap_or_else(|| col.sql_type.clone()),
                                not_null: !col.is_nullable,
                                is_primary_key: tbl_ir.primary_key.contains(&col.name),
                                default_value: col.default_val.clone(),
                                relation_table: rel_tbl,
                                relation_field: rel_col,
                                is_flow_field: false,
                                flow_field_expr: None,
                                editable: true,
                                visible: true,
                            },
                        );
                    }
                    for ff in &tbl_ir.flow_fields {
                        resolved_fields.insert(
                            ff.name.clone(),
                            bosc_ir::ResolvedFieldMetaIR {
                                name: ff.name.clone(),
                                caption: Some(ff.name.clone()),
                                data_type: "Decimal(18,2)".to_string(),
                                not_null: false,
                                is_primary_key: false,
                                default_value: None,
                                relation_table: None,
                                relation_field: None,
                                is_flow_field: true,
                                flow_field_expr: Some(ff.sql_subquery.clone()),
                                editable: false,
                                visible: true,
                            },
                        );
                    }
                }
            }

            // Also check for state machine attached to this table
            let sm_row = sqlx::query(
                r#"SELECT definition FROM "_bos_meta_objects" 
                   WHERE object_type = 'statemachine' AND LOWER(definition->>'target_table') = LOWER($1)
                   ORDER BY updated_at DESC LIMIT 1"#
            )
            .bind(tbl_name)
            .fetch_optional(&pool)
            .await?;

            if let Some(row) = sm_row {
                if let Ok(def) = row.try_get::<serde_json::Value, _>("definition") {
                    if let Ok(sm_ir) = serde_json::from_value::<bosc_ir::StateMachineIR>(def) {
                        state_machine = Some(sm_ir.name);
                        initial_state = Some(sm_ir.initial_state);
                        transitions = sm_ir.transitions;
                    }
                }
            }
        }

        Ok(Some(bosc_ir::ResolvedPageIR {
            page: merged_page,
            resolved_fields,
            state_machine,
            initial_state,
            transitions,
        }))
    }

    // ----------------------------------------------------
    // COMPILED PAGES (Svelte 5 page artifacts)
    // ----------------------------------------------------

    /// The running page service, once [`Self::start_page_service`] has been called.
    pub fn page_service(&self) -> Option<Arc<page_artifacts::PageService>> {
        self.pages.load_full()
    }

    /// Starts the Svelte page compiler, resolves every page of the catalog once (no recompilation of
    /// unchanged pages: artifacts are content-addressed), warms missing artifacts in the background and
    /// subscribes to page changes published by other nodes.
    pub async fn start_page_service(
        &self,
        cfg: page_artifacts::PageServiceConfig,
    ) -> Result<Arc<page_artifacts::PageService>, RuntimeError> {
        let service = page_artifacts::PageService::start(cfg).await;
        self.pages.store(Some(service.clone()));

        let pool = self.pool().await;
        let url = self.active_url().await;
        let report = service
            .refresh(&pool, &url)
            .await
            .map_err(|e| RuntimeError::Internal(format!("page catalog load failed: {e}")))?;
        tracing::info!(target: "bos::pages", pages = report.pages, db = %report.db_key, "page catalog resolved");

        let warm = service.clone();
        tokio::spawn(async move {
            let failures = warm.warm().await;
            let removed = warm.cleanup();
            tracing::info!(target: "bos::pages", compiled = warm.compile_count(), failures = failures.len(), removed, "page artifacts warmed");
        });

        let engine = self.clone();
        tokio::spawn(async move { engine.listen_for_page_changes().await });
        Ok(service)
    }

    /// Re-resolves pages after a catalog change and tells the other nodes.
    pub async fn republish_pages(&self) -> Option<page_artifacts::RefreshReport> {
        let report = self.refresh_pages().await?;
        if !report.changed.is_empty() || !report.removed.is_empty() {
            if let Some(service) = self.page_service() {
                let pool = self.pool().await;
                let payload = format!("{}|{}", report.db_key, service.instance());
                if let Err(e) = sqlx::query("SELECT pg_notify('bos_pages_channel', $1)").bind(payload).execute(&pool).await {
                    tracing::warn!(target: "bos::pages", "could not notify other nodes about page changes: {e}");
                }
            }
        }
        Some(report)
    }

    async fn refresh_pages(&self) -> Option<page_artifacts::RefreshReport> {
        let service = self.page_service()?;
        let pool = self.pool().await;
        let url = self.active_url().await;
        match service.refresh(&pool, &url).await {
            Ok(report) => {
                if !report.changed.is_empty() {
                    tracing::info!(target: "bos::pages", changed = ?report.changed, "pages changed");
                    let warm = service.clone();
                    tokio::spawn(async move {
                        warm.warm().await;
                    });
                }
                Some(report)
            }
            Err(e) => {
                tracing::error!(target: "bos::pages", "page refresh failed: {e}");
                None
            }
        }
    }

    /// LISTEN loop for `bos_pages_channel` and `bos_meta_channel`; reconnects to the active database after errors or switches.
    async fn listen_for_page_changes(&self) {
        loop {
            let pool = self.pool().await;
            match sqlx::postgres::PgListener::connect_with(&pool).await {
                Ok(mut listener) => {
                    let _ = listener.listen("bos_pages_channel").await;
                    let _ = listener.listen("bos_meta_channel").await;
                    while let Ok(n) = listener.recv().await {
                        let Some(service) = self.page_service() else { continue };
                        let payload = n.payload();
                        if n.channel() == "bos_meta_channel" || payload.contains("external") || payload.starts_with('*') {
                            tracing::info!(target: "bos::pages", "Dynamic package/page update detected. Hot-reloading page catalog...");
                            self.meta_cache.clear();
                            self.refresh_pages().await;
                            continue;
                        }
                        let Some((db, instance)) = payload.split_once('|') else { continue };
                        let own = instance == service.instance().to_string();
                        if !own && (service.db_key().as_deref() == Some(db) || db == "*") {
                            self.refresh_pages().await;
                        }
                    }
                }
                Err(e) => tracing::warn!(target: "bos::pages", "page change listener: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    /// Runs a server-side page trigger, action script or control trigger in one transaction.
    pub async fn run_page_trigger(
        &self,
        page: &str,
        reference: &str,
        record: serde_json::Value,
        variables: serde_json::Value,
        ui_answers: std::collections::HashMap<String, bool>,
        ctx: &SessionContext,
    ) -> Result<PageTriggerResult, RuntimeError> {
        let service = self
            .page_service()
            .ok_or_else(|| RuntimeError::Internal("page service is not running".to_string()))?;
        let entry = service
            .page(page)
            .ok_or_else(|| RuntimeError::Internal(format!("page '{page}' does not exist")))?;
        let script = page_artifacts::find_page_script(&entry, reference).map_err(|e| RuntimeError::Internal(e.to_string()))?;
        let wrapped = page_artifacts::wrap_page_script(&entry.name, reference, &variables, &script.script);

        let mut scope = self.begin_scope(ctx).await?;
        scope.ui_answers = ui_answers.clone();
        let data = if record.is_object() { record } else { serde_json::json!({}) };
        let outcome = QuickJsEngine::run_script(
            &scope,
            ScriptCall {
                table_name: script.table.clone(),
                trigger_name: reference.to_string(),
                script: wrapped,
                bytecode: None,
                data,
                x_rec: None,
                limits: quickjs_engine::ExecutionLimits::with_fuel(None),
                ui_answers,
            },
        )
        .await;
        let run = scope.db.finish(outcome).await?;

        let mut record = run.data;
        let take = |rec: &mut serde_json::Value, key: &str| rec.as_object_mut().and_then(|o| o.remove(key));
        let messages = take(&mut record, "_system_messages").unwrap_or_else(|| serde_json::json!([]));
        let directives = take(&mut record, "_ui_directives").unwrap_or_else(|| serde_json::json!([]));
        let variables = take(&mut record, "_page_variables").unwrap_or(variables);
        Ok(PageTriggerResult {
            record,
            variables,
            messages,
            directives,
            return_value: run.return_value,
            logs: run.logs,
            fuel_used: run.fuel_used,
            notifications: run.notifications,
        })
    }

    /// List installed packages from catalog (_bos_meta_packages)

    pub async fn list_packages(&self) -> Result<Vec<meta_manager::PackageSummary>, RuntimeError> {
        let pool = self.pool().await;

        MetaManager::list_packages(&pool).await
    }

    /// Register or update a package in the catalog (_bos_meta_packages)
    pub async fn register_package(
        &self,
        req: meta_manager::RegisterPackageRequest,
    ) -> Result<meta_manager::PackageSummary, RuntimeError> {
        let pool = self.pool().await;

        MetaManager::register_package(&pool, req).await
    }

    /// Install a .bospkg package bundle

    pub async fn install_package(
        &self,

        bundle: bosc_ir::PackageBundleIR,

        allow_destructive: bool,
    ) -> Result<package_manager::PackageInstallResult, RuntimeError> {
        let pool = self.pool().await;

        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        self.meta_cache.clear();

        let _ = sqlx::query("SELECT pg_notify('bos_meta_channel', '*');")
            .execute(&pool)
            .await;
        let _ = sqlx::query("SELECT pg_notify('bos_pages_channel', '*|external');")
            .execute(&pool)
            .await;

        let result = package_manager::PackageManager::install_package(
            &pool,
            bundle,
            strat.as_deref(),
            allow_destructive,
        )
        .await;

        if result.is_ok() {
            self.republish_pages().await;
        }

        result
    }

    /// Bring the database in line with a package's declared tables (see [`schema_sync`]).

    pub async fn sync_schema(
        &self,

        request: &SchemaSyncRequest,

        options: &SchemaSyncOptions,
    ) -> Result<SchemaSyncReport, RuntimeError> {
        let pool = self.pool().await;

        let report = SchemaSync::sync(&pool, request, options).await?;

        if report.applied {
            self.meta_cache.clear();
            self.republish_pages().await;
        }

        Ok(report)
    }

    /// Export an installed package into a .bospkg bundle

    pub async fn export_package(
        &self,
        package_id: &str,
    ) -> Result<bosc_ir::PackageBundleIR, RuntimeError> {
        let pool = self.pool().await;

        package_manager::PackageManager::export_package(&pool, package_id).await
    }

    /// Register a Table Extension (AL/BC non-invasive schema evolution)

    pub async fn register_table_extension(
        &self,
        req: meta_manager::RegisterTableExtensionRequest,
    ) -> Result<meta_manager::MetaObjectSummary, RuntimeError> {
        let target = req.target_table.clone();

        let pool = self.pool().await;

        let res = meta_manager::MetaManager::register_table_extension(&pool, req).await?;

        self.invalidate_meta_cache(&target).await;

        self.republish_pages().await;

        Ok(res)
    }

    /// Compile and apply BOS-Script to an already created table

    pub async fn apply_table_script(
        &self,
        table_name: &str,
        req: meta_manager::ApplyTableScriptRequest,
    ) -> Result<meta_manager::ApplyTableScriptResponse, RuntimeError> {
        let pool = self.pool().await;

        let res = meta_manager::MetaManager::apply_table_script(&pool, table_name, req).await?;

        if res.applied {
            self.invalidate_meta_cache(table_name).await;
            self.republish_pages().await;
        }

        Ok(res)
    }

    /// Compile any arbitrary BOS-Script (preview or apply)

    pub async fn compile_script(
        &self,
        req: meta_manager::CompileScriptRequest,
    ) -> Result<meta_manager::CompileScriptResponse, RuntimeError> {
        let apply = req.apply;

        let pool = self.pool().await;

        let result = meta_manager::MetaManager::compile_script(&pool, req).await;

        if apply {
            self.meta_cache.clear();
            if result.is_ok() {
                self.republish_pages().await;
            }
        }

        result
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

    /// Host services (WASM, sidecars, event registry) exposed to BOS-Script.

    pub fn script_services(&self) -> ScriptServices {
        ScriptServices {
            wasm: Some(self.wasm_engine.clone()),

            sidecars: Some(self.sidecar_host.clone()),

            events: Some(self.event_registry.clone()),

            bytecode_cache: Some(self.bytecode_cache.clone()),

            meta_cache: Some(self.meta_cache.clone()),

            prepared_statements: Some(self.prepared_statements.clone()),
        }
    }

    /// First-ever load provisioning routine (ADR 29).

    /// Atomically provisions the database, storage strategy, organizational hierarchy (Tenant/Company),

    /// imports base packages (bos.security, bos.ledger), and creates the initial Superuser account.

    pub async fn first_run_provision(
        &self,

        req: FirstRunProvisionRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let strat = req.storage_strategy.as_deref().unwrap_or("shared_rls");

        let target_url = req.connection_url.as_deref().unwrap_or("").trim();

        let target_db = req.database.as_deref().unwrap_or("").trim();

        let current_db = self.active_db_name().await;

        // 2. Database handling (Cloud Provider vs Local Database)

        if !target_url.is_empty() {
            // User provided a full cloud connection URL (Supabase, Neon, AWS RDS, Azure, etc.)

            self.switch_database(target_url).await?;

            let _ = self.provision_system(Some(strat)).await?;

            // Create initial Tenant & Company

            let t_code = req.tenant_code.as_deref().unwrap_or("DEFAULT");

            let t_name = req.tenant_name.as_deref().unwrap_or("Default Tenant");

            let tenant = self
                .create_tenant(CreateTenantRequest {
                    database: None,

                    tenant_id: None,

                    code: t_code.to_string(),

                    name: t_name.to_string(),
                })
                .await
                .ok();

            let tenant_id = tenant.map(|t| t.tenant_id).unwrap_or_else(Uuid::nil);

            let c_code = req.company_code.as_deref().unwrap_or("CRONUS");

            let c_name = req.company_name.as_deref().unwrap_or("Cronus US");

            let _ = self
                .create_company(CreateCompanyRequest {
                    database: None,

                    company_id: None,

                    tenant_id: Some(tenant_id),

                    code: c_code.to_string(),

                    name: c_name.to_string(),
                })
                .await
                .ok();

            // Import base packages if requested

            if req.import_base_packages {
                let mut pkgs = Vec::new();

                pkgs.push("bos.core".to_string());

                for p in &req.packages {
                    let pt = p.trim().to_string();

                    if !pt.is_empty() && !pkgs.iter().any(|x| x.eq_ignore_ascii_case(&pt)) {
                        pkgs.push(pt);
                    }
                }

                let pool = self.pool().await;

                for pkg_id in &pkgs {
                    if let Some(bundle) = DatabaseManager::load_base_package_bundle(pkg_id).await {
                        let _ = crate::package_manager::PackageManager::install_package(
                            &pool,
                            bundle,
                            Some(strat),
                            false,
                        )
                        .await;
                    }
                }
            }
        } else if !target_db.is_empty() && !target_db.eq_ignore_ascii_case(&current_db) {
            let current_url = self.active_url().await;

            let dbs = self.list_databases().await.unwrap_or_default();

            let db_exists = dbs.iter().any(|d| d.name.eq_ignore_ascii_case(target_db));

            if !db_exists {
                let create_req = CreateDatabaseRequest {
                    name: target_db.to_string(),

                    init_extensions: true,

                    provision_system: true,

                    storage_strategy: Some(strat.to_string()),

                    tenant_id: None,

                    tenant_code: req.tenant_code.clone(),

                    tenant_name: req.tenant_name.clone(),

                    company_id: None,

                    company_code: req.company_code.clone(),

                    company_name: req.company_name.clone(),

                    import_base_packages: req.import_base_packages,

                    packages: req.packages.clone(),
                };

                let _ = DatabaseManager::create_database(&current_url, &create_req).await?;
            }

            self.switch_database(target_db).await?;
        } else {
            // Target is current active database: provision system kernel

            let _ = self.provision_system(Some(strat)).await?;

            // Create initial Tenant & Company

            let t_code = req.tenant_code.as_deref().unwrap_or("DEFAULT");

            let t_name = req.tenant_name.as_deref().unwrap_or("Default Tenant");

            let tenant = self
                .create_tenant(CreateTenantRequest {
                    database: None,

                    tenant_id: None,

                    code: t_code.to_string(),

                    name: t_name.to_string(),
                })
                .await
                .ok();

            let tenant_id = tenant.map(|t| t.tenant_id).unwrap_or_else(Uuid::nil);

            let c_code = req.company_code.as_deref().unwrap_or("CRONUS");

            let c_name = req.company_name.as_deref().unwrap_or("Cronus US");

            let _ = self
                .create_company(CreateCompanyRequest {
                    database: None,

                    company_id: None,

                    tenant_id: Some(tenant_id),

                    code: c_code.to_string(),

                    name: c_name.to_string(),
                })
                .await
                .ok();

            // Import base packages if requested

            if req.import_base_packages {
                let mut pkgs = Vec::new();

                pkgs.push("bos.core".to_string());

                for p in &req.packages {
                    let pt = p.trim().to_string();

                    if !pt.is_empty() && !pkgs.iter().any(|x| x.eq_ignore_ascii_case(&pt)) {
                        pkgs.push(pt);
                    }
                }

                let pool = self.pool().await;

                for pkg_id in &pkgs {
                    if let Some(bundle) = DatabaseManager::load_base_package_bundle(pkg_id).await {
                        let _ = crate::package_manager::PackageManager::install_package(
                            &pool,
                            bundle,
                            Some(strat),
                            false,
                        )
                        .await;
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "provisioned": true,
            "database": if !target_db.is_empty() { target_db } else { &current_db },
            "storage_strategy": strat,
            "packages_imported": req.import_base_packages,
            "message": "System provisioned successfully."
        }))
    }

    /// Begin a unit of work: one transaction carrying the caller identity (for RLS) and,

    /// under `schema_per_company`, the company schema search path.

    pub async fn begin_scope(&self, ctx: &SessionContext) -> Result<ExecScope, RuntimeError> {
        let pool = self.pool().await;

        let db = DbSession::begin_with_context(&pool, ctx).await?;

        if let Some(schema) = self.company_schema(&pool, ctx).await? {
            let mut conn = db.conn().await?;

            let res = sqlx::query(&format!(
                "SET LOCAL search_path = \"{}\", \"public\"",
                schema
            ))
            .execute(&mut *conn)
            .await;

            db.track(res)?;
        }

        Ok(ExecScope::new(db, ctx.clone(), self.script_services()))
    }

    /// Company schema name when the `schema_per_company` storage strategy is active.

    async fn company_schema(
        &self,
        pool: &PgPool,
        ctx: &SessionContext,
    ) -> Result<Option<String>, RuntimeError> {
        let has_sys_info: bool =
            sqlx::query_scalar("SELECT to_regclass('_bos_sys_info') IS NOT NULL")
                .fetch_one(pool)
                .await?;

        if !has_sys_info {
            return Ok(None);
        }

        let strat: Option<String> = sqlx::query_scalar(
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        if strat.as_deref() != Some("schema_per_company") {
            return Ok(None);
        }

        let comp_code: Option<String> =
            sqlx::query_scalar("SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1")
                .bind(ctx.company_id)
                .fetch_optional(pool)
                .await?;

        let schema = format!(
            "comp_{}",
            comp_code
                .unwrap_or_else(|| "default".to_string())
                .to_lowercase()
                .replace('-', "_")
        );

        Ok(Some(sanitize_identifier(&schema)?))
    }

    /// Execute a Codeunit procedure as one unit of work: the method, every record it touches,

    /// nested codeunit calls, triggers and event subscribers commit together or roll back together.

    pub async fn execute_procedure(
        &self,
        codeunit: &str,
        method: &str,
        params: serde_json::Value,
        ctx: &SessionContext,
    ) -> Result<execution_engine::ExecutionResult, RuntimeError> {
        self.execute_procedure_with(
            codeunit,
            method,
            params,
            ctx,
            std::collections::HashMap::new(),
        )
        .await
    }

    /// Same as [`Self::execute_procedure`], with answers for `ctx.ui.confirm`.

    /// A confirm whose message is absent from `ui_answers` throws `ConfirmRequired`.

    pub async fn execute_procedure_with(
        &self,

        codeunit: &str,

        method: &str,

        params: serde_json::Value,

        ctx: &SessionContext,

        ui_answers: std::collections::HashMap<String, bool>,
    ) -> Result<execution_engine::ExecutionResult, RuntimeError> {
        let mut scope = self.begin_scope(ctx).await?;

        scope.ui_answers = ui_answers;

        let outcome = ExecutionEngine::execute_procedure_in(&scope, codeunit, method, params).await;

        scope.db.finish(outcome).await
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

    pub async fn delete_current_database(
        &self,
        force: bool,
    ) -> Result<serde_json::Value, RuntimeError> {
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

    pub async fn manage_database(
        &self,
        name: &str,
        req: ManageDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
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

    pub async fn manage_current_database(
        &self,
        req: ManageDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;

        self.manage_database(&current_db, req).await
    }

    /// Update database properties (rename, storage strategy, or status) for a specific database

    pub async fn update_database(
        &self,
        name: &str,
        req: UpdateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
        let current_db = self.active_db_name().await;

        let is_current = name == current_db;

        if let Some(ref new_name) = req.new_name {
            if new_name != name {
                if is_current {
                    self.switch_database("postgres").await?;

                    let maint_url = self.active_url().await;

                    DatabaseManager::manage_database(
                        &maint_url,
                        name,
                        ManageDatabaseRequest {
                            action: "rename".to_string(),

                            new_name: Some(new_name.clone()),

                            table: None,

                            sql: None,

                            storage_strategy: None,

                            full: None,
                        },
                    )
                    .await?;

                    self.switch_database(new_name).await?;
                } else {
                    let current_url = self.active_url().await;

                    DatabaseManager::manage_database(
                        &current_url,
                        name,
                        ManageDatabaseRequest {
                            action: "rename".to_string(),

                            new_name: Some(new_name.clone()),

                            table: None,

                            sql: None,

                            storage_strategy: None,

                            full: None,
                        },
                    )
                    .await?;
                }
            }
        }

        let target_db = req.new_name.as_deref().unwrap_or(name);

        if req.storage_strategy.is_some() || req.status.is_some() {
            let target_url =
                DatabaseManager::replace_database_name(&self.active_url().await, target_db);

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

                "#,
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

    pub async fn update_current_database(
        &self,
        req: UpdateDatabaseRequest,
    ) -> Result<serde_json::Value, RuntimeError> {
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

    pub async fn check_permission(
        &self,
        ctx: &SessionContext,
        entity: &str,
        action: &str,
    ) -> Result<bool, RuntimeError> {
        // If user has 'Admin' or 'Super' role, bypass permission checks

        if ctx.roles.iter().any(|r| {
            r.eq_ignore_ascii_case("Admin")
                || r.eq_ignore_ascii_case("Super")
                || r.eq_ignore_ascii_case("Super_Admin")
                || r.eq_ignore_ascii_case("System")
        }) {
            return Ok(true);
        }

        let psets = self.list_meta_objects(Some("permissionset")).await?;

        let mut parsed_sets = Vec::new();

        for ps_summary in psets {
            if let Some(details) = self.get_meta_object(&ps_summary.object_name).await? {
                if let Ok(ps_ir) =
                    serde_json::from_value::<bosc_ir::PermissionSetIR>(details.definition)
                {
                    parsed_sets.push(ps_ir);
                }
            }
        }

        let role_matched: Vec<&bosc_ir::PermissionSetIR> = parsed_sets
            .iter()
            .filter(|ps| ctx.roles.iter().any(|r| r.eq_ignore_ascii_case(&ps.name)))
            .collect();

        let active_sets: Vec<&bosc_ir::PermissionSetIR> = if !role_matched.is_empty() {
            role_matched
        } else {
            parsed_sets.iter().collect()
        };

        for ps_ir in active_sets {
            if action.eq_ignore_ascii_case("execute") {
                if ps_ir
                    .execute
                    .iter()
                    .any(|cu| cu.eq_ignore_ascii_case(entity))
                {
                    return Ok(true);
                }
            } else {
                for perm in &ps_ir.permissions {
                    if perm.table_name.eq_ignore_ascii_case(entity)
                        || perm.sql_table_name.eq_ignore_ascii_case(entity)
                    {
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

        // If no explicit permission found and user is not admin, deny
        Ok(false)
    }

    /// Return the list of excluded (masked/redacted) fields for a given entity and security context
    pub async fn get_excluded_fields(
        &self,
        ctx: &SessionContext,
        entity: &str,
    ) -> Result<Vec<String>, RuntimeError> {
        // Super/Admin roles have full access without field exclusions
        if ctx.roles.iter().any(|r| {
            r.eq_ignore_ascii_case("Admin")
                || r.eq_ignore_ascii_case("Administrator")
                || r.eq_ignore_ascii_case("Super")
                || r.eq_ignore_ascii_case("Super_Admin")
                || r.eq_ignore_ascii_case("System")
                || r.eq_ignore_ascii_case("SystemWorker")
        }) {
            return Ok(Vec::new());
        }

        let psets = self.list_meta_objects(Some("permissionset")).await?;
        let mut parsed_sets = Vec::new();
        for ps_summary in psets {
            if let Some(details) = self.get_meta_object(&ps_summary.object_name).await? {
                if let Ok(ps_ir) = serde_json::from_value::<bosc_ir::PermissionSetIR>(details.definition) {
                    parsed_sets.push(ps_ir);
                }
            }
        }

        let role_matched: Vec<&bosc_ir::PermissionSetIR> = parsed_sets
            .iter()
            .filter(|ps| ctx.roles.iter().any(|r| r.eq_ignore_ascii_case(&ps.name)))
            .collect();

        let active_sets: Vec<&bosc_ir::PermissionSetIR> = if !role_matched.is_empty() {
            role_matched
        } else {
            parsed_sets.iter().collect()
        };

        let mut excluded = Vec::new();
        for ps in active_sets {
            for perm in &ps.permissions {
                if perm.table_name.eq_ignore_ascii_case(entity) || perm.sql_table_name.eq_ignore_ascii_case(entity) {
                    for f in &perm.exclude_fields {
                        let f_lower = f.to_lowercase();
                        if !excluded.contains(&f_lower) {
                            excluded.push(f_lower);
                        }
                    }
                }
            }
        }
        Ok(excluded)
    }

    /// Strip excluded fields from a JSON record row
    pub fn mask_excluded_fields(record: &mut serde_json::Value, excluded_fields: &[String]) {
        if excluded_fields.is_empty() {
            return;
        }
        if let serde_json::Value::Object(map) = record {
            for ex in excluded_fields {
                let ex_clean = ex.to_lowercase().replace('_', "");
                map.retain(|k, _| {
                    let k_clean = k.to_lowercase().replace('_', "");
                    k_clean != ex_clean
                });
            }
        }
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

        let sql_view = sanitize_identifier(&to_snake_case(view_name))?;

        let mut sql = format!("SELECT to_jsonb(v.*) AS json_data FROM \"{}\" v", sql_view);

        let mut conditions = Vec::new();

        if let Some(f) = filter {
            if let Some(obj) = f.as_object() {
                for (k, v) in obj {
                    let col_name = sanitize_identifier(&to_snake_case(k))?;

                    match v {
                        serde_json::Value::String(s) => {
                            conditions.push(format!(
                                "v.\"{}\" = '{}'",
                                col_name,
                                s.replace('\'', "''")
                            ));
                        }

                        serde_json::Value::Number(n) => {
                            conditions.push(format!("v.\"{}\" = {}", col_name, n));
                        }

                        serde_json::Value::Bool(b) => {
                            conditions.push(format!(
                                "v.\"{}\" = {}",
                                col_name,
                                if *b { "TRUE" } else { "FALSE" }
                            ));
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
            let mut order_parts = Vec::new();

            for clause in ord.split(',') {
                let trimmed = clause.trim();

                if trimmed.is_empty() {
                    continue;
                }

                let mut tokens = trimmed.split_whitespace();

                if let Some(raw_col) = tokens.next() {
                    let clean_col = raw_col.trim_start_matches("v.").trim_matches('"');

                    let safe_col = sanitize_identifier(&to_snake_case(clean_col))?;

                    let dir = match tokens.next().map(|d| d.to_ascii_uppercase()).as_deref() {
                        Some("DESC") => "DESC",

                        Some("ASC") | None => "ASC",

                        Some(other) => {
                            return Err(RuntimeError::SecurityError(format!(
                                "Invalid ORDER BY direction: '{}'",
                                other
                            )));
                        }
                    };

                    if tokens.next().is_some() {
                        return Err(RuntimeError::SecurityError(format!(
                            "Invalid ORDER BY clause: '{}'",
                            trimmed
                        )));
                    }

                    order_parts.push(format!("v.\"{}\" {}", safe_col, dir));
                }
            }

            if !order_parts.is_empty() {
                sql.push_str(&format!(" ORDER BY {}", order_parts.join(", ")));
            }
        }

        sql.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        let rows = sqlx::query(&sql).fetch_all(&mut *tx).await?;

        let results = rows
            .iter()
            .map(|r| {
                r.try_get::<serde_json::Value, _>("json_data")
                    .unwrap_or(serde_json::Value::Null)
            })
            .collect();

        tx.commit().await?;

        Ok(results)
    }

    /// Refresh a Materialized View

    pub async fn refresh_view(
        &self,
        view_name: &str,
        concurrently: bool,
    ) -> Result<(), RuntimeError> {
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
        let meta = self
            .get_meta_object(query_name)
            .await?
            .ok_or_else(|| RuntimeError::NotFound("Query".to_string(), query_name.to_string()))?;

        let query_ir: bosc_ir::QueryIR = serde_json::from_value(meta.definition).map_err(|e| {
            RuntimeError::Internal(format!("Failed to parse query definition: {}", e))
        })?;

        let pool = self.pool().await;

        let mut tx = pool.begin().await?;

        self.apply_isolation_context(&mut tx, ctx).await?;

        let (final_sql, bound) = bind_named_query(&query_ir.sql_query, &params)?;

        let wrapped_sql = format!("SELECT to_jsonb(q.*) AS json_data FROM ({}) q", final_sql);

        let mut query = sqlx::query(&wrapped_sql);

        for value in &bound {
            query = match value {
                serde_json::Value::String(s) => query.bind(s.clone()),

                serde_json::Value::Number(n) if n.is_i64() => query.bind(n.as_i64()),

                serde_json::Value::Number(n) => query.bind(n.as_f64()),

                serde_json::Value::Bool(b) => query.bind(*b),

                serde_json::Value::Null => query.bind(Option::<String>::None),

                other => query.bind(other.to_string()),
            };
        }

        let rows = query.fetch_all(&mut *tx).await?;

        let results = rows
            .iter()
            .map(|r| {
                r.try_get::<serde_json::Value, _>("json_data")
                    .unwrap_or(serde_json::Value::Null)
            })
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

        if let Some(schema_name) = self.company_schema(&pool, ctx).await? {
            let set_search_path =
                format!("SET LOCAL search_path = \"{}\", \"public\";", schema_name);

            sqlx::query(&set_search_path).execute(&mut **tx).await?;
        }

        Ok(())
    }

    /// Resolve physical table target and company isolation requirement

    pub async fn resolve_table_info(
        &self,
        table: &str,
        ctx: &SessionContext,
    ) -> Result<(String, bool), RuntimeError> {
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

                let sql_name = def
                    .get("sql_table_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(table)
                    .to_lowercase();

                let dpc = def
                    .get("data_per_company")
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
                    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = $1)",
                )
                .bind(&snake)
                .fetch_one(&pool)
                .await
                .unwrap_or(false);

                let resolved_name = if exists_snake {
                    snake
                } else {
                    table.to_lowercase()
                };

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
            "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if strat.as_deref() == Some("table_per_company") && data_per_company {
            let comp_code: Option<String> = sqlx::query_scalar(
                "SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1",
            )
            .bind(ctx.company_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let prefix = comp_code
                .unwrap_or_else(|| "default".to_string())
                .to_lowercase();

            return Ok((format!("{}${}", prefix, base_table), data_per_company));
        }

        if strat.as_deref() == Some("schema_per_company") && data_per_company {
            let comp_code: Option<String> = sqlx::query_scalar(
                "SELECT code FROM \"_bos_companies\" WHERE company_id = $1 LIMIT 1",
            )
            .bind(ctx.company_id)
            .fetch_optional(&pool)
            .await
            .unwrap_or(None);

            let schema_name = format!(
                "comp_{}",
                comp_code
                    .unwrap_or_else(|| "default".to_string())
                    .to_lowercase()
                    .replace('-', "_")
            );

            return Ok((
                format!("\"{}\".\"{}\"", schema_name, base_table),
                data_per_company,
            ));
        }

        Ok((base_table, data_per_company))
    }

    /// Resolve physical table target according to active multi-company storage strategy

    pub async fn resolve_physical_table(
        &self,
        table: &str,
        ctx: &SessionContext,
    ) -> Result<String, RuntimeError> {
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

            if sub_type
                .as_deref()
                .map(|s| s.eq_ignore_ascii_case("ledger"))
                .unwrap_or(false)
            {
                return true;
            }

            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();

            if def
                .get("immutable")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                return true;
            }

            if def
                .get("sub_type")
                .and_then(|v| v.as_str())
                .map(|s| s.eq_ignore_ascii_case("ledger"))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }

    /// Introspect complete table schema, combining high-level BOS meta catalog with physical PostgreSQL columns

    pub async fn get_entity_schema(
        &self,
        table: &str,
        ctx: &SessionContext,
    ) -> Result<EntitySchemaResponse, RuntimeError> {
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
            table_name = r
                .try_get("object_name")
                .unwrap_or_else(|_| table.to_string());

            package_name = r.try_get("package_name").ok();

            caption = r.try_get("caption").ok();

            let def: serde_json::Value = r.try_get("definition").unwrap_or_default();

            state_machine = def.get("state_machine").cloned();

            if let Some(meta_cols) = def.get("columns").and_then(|c| c.as_array()) {
                for mc in meta_cols {
                    let col_name = mc
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let sql_name = mc
                        .get("sql_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| to_snake_case(&col_name));

                    let data_type = mc
                        .get("data_type")
                        .or_else(|| mc.get("sql_type"))
                        .and_then(|v| v.as_str())
                        .map(sql_type_to_bos_type)
                        .unwrap_or_else(|| "String(50)".to_string());

                    let is_nullable = mc
                        .get("is_nullable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);

                    let is_pk = mc
                        .get("is_primary_key")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let is_unique = mc
                        .get("is_unique")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let def_val = mc
                        .get("default_val")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    let col_caption = mc
                        .get("caption")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

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

                    data_type: sql_type_to_bos_type(d_type),

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

            let key = if !col.name.is_empty() {
                col.name.clone()
            } else {
                col.sql_name.clone()
            };

            let dt = col.data_type.to_lowercase();

            let sample_val = if dt.contains("int") || dt.contains("serial") {
                serde_json::json!(1)
            } else if dt.contains("numeric")
                || dt.contains("decimal")
                || dt.contains("float")
                || dt.contains("double")
                || dt.contains("real")
            {
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

    /// Insert one record as a single unit of work: validation cascades, before-triggers, the

    /// INSERT, after-triggers and event subscribers all commit together or not at all.

    pub async fn insert_record_with_messages(
        &self,

        table: &str,

        data: serde_json::Value,

        ctx: &SessionContext,
    ) -> Result<(Uuid, Vec<serde_json::Value>), RuntimeError> {
        let scope = self.begin_scope(ctx).await?;

        let outcome = self.insert_in(&scope, table, data).await;

        scope.db.finish(outcome).await
    }

    /// Insert inside an existing unit of work.

    pub async fn insert_in(
        &self,

        scope: &ExecScope,

        table: &str,

        data: serde_json::Value,
    ) -> Result<(Uuid, Vec<serde_json::Value>), RuntimeError> {
        // C.4: Concurrency-safe Linear Audit Hash Chain Serialization
        if table.eq_ignore_ascii_case("SecurityAuditLog") || table.eq_ignore_ascii_case("security_audit_log") {
            let mut conn = scope.db.conn().await?;
            // Advisory transaction lock 0x424F535F415544 (18654261559981876) locks audit insertion sequentially until COMMIT
            let _ = sqlx::query("SELECT pg_advisory_xact_lock(18654261559981876)")
                .execute(&mut *conn)
                .await;
        }

        let mut payload = data;

        let is_draft_insert = payload.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || payload.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        let incoming: Vec<String> = payload
            .as_object()
            .map(|o| o.keys().filter(|k| !k.starts_with('_')).cloned().collect())
            .unwrap_or_default();

        for k in incoming {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnValidate(k),
                &mut payload,
                None,
            )
            .await?;
        }

        if is_draft_insert {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnDraftSave,
                &mut payload,
                None,
            )
            .await?;
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnBeforeInsert,
            &mut payload,
            None,
        )
        .await?;

        self.publish_record_event(scope, table, "OnBeforeInsert", &mut payload, None)
            .await?;

        let (physical_table, data_per_company) = self.resolve_table_info(table, &scope.ctx).await?;

        let id = payload
            .get("_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        let obj = payload.as_object().ok_or_else(|| {
            RuntimeError::Internal(format!("Record for '{}' must be a JSON object", table))
        })?;

        // High-Performance Dynamic Tuple Insertion (robust against partial payloads, default values & typed nulls)
        let executed_prepared = false;

        if !executed_prepared {
            let (columns, values) = Self::insert_tuple(obj, id, &scope.ctx, data_per_company)?;

            let query = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                quote_physical(&physical_table),
                columns.join(", "),
                values.join(", ")
            );

            let mut conn = scope.db.conn().await?;

            let res = sqlx::query(&query).execute(&mut *conn).await;

            scope.db.track(res)?;
        }

        if let Some(o) = payload.as_object_mut() {
            o.insert("_id".to_string(), serde_json::json!(id.to_string()));
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnAfterInsert,
            &mut payload,
            None,
        )
        .await?;

        self.publish_record_event(scope, table, "OnAfterInsert", &mut payload, None)
            .await?;

        let messages = payload
            .as_object_mut()
            .and_then(|o| o.remove("_system_messages"))
            .and_then(|m| m.as_array().cloned())
            .unwrap_or_default();

        Ok((id, messages))
    }

    /// Column list and SQL literals for one INSERT row (system columns first).

    fn insert_tuple(
        obj: &serde_json::Map<String, serde_json::Value>,

        id: Uuid,

        ctx: &SessionContext,

        data_per_company: bool,
    ) -> Result<(Vec<String>, Vec<String>), RuntimeError> {
        let mut added = std::collections::HashSet::new();

        let mut columns = vec!["\"_id\"".to_string(), "\"_tenant_id\"".to_string()];

        let tenant_val = if ctx.tenant_id.is_nil() {
            "NULL".to_string()
        } else {
            format!("'{}'::uuid", ctx.tenant_id)
        };

        let mut values = vec![
            format!("'{}'::uuid", id),
            tenant_val,
        ];

        added.insert("_id".to_string());

        added.insert("_tenant_id".to_string());

        if data_per_company {
            columns.push("\"_company_id\"".to_string());

            let comp_val = if ctx.company_id.is_nil() {
                "NULL".to_string()
            } else {
                format!("'{}'::uuid", ctx.company_id)
            };

            values.push(comp_val);

            added.insert("_company_id".to_string());
        }

        for (k, v) in obj {
            let col_name = if k.starts_with('_') {
                k.clone()
            } else {
                to_snake_case(k)
            };

            if added.contains(&col_name)
                || col_name == "_system_messages"
                || col_name == "system_messages"
                || col_name == "_messages"
                || col_name == "messages"
            {
                continue;
            }

            if (col_name == "_created_at" || col_name == "_updated_at") && v.as_str().is_none() {
                continue;
            }

            if (col_name == "_created_by" || col_name == "_updated_by") && v.is_null() {
                continue;
            }

            if col_name == "_system_version" {
                continue;
            }

            let safe_col = sanitize_identifier(&col_name)?;

            columns.push(format!("\"{}\"", safe_col));

            values.push(sql_literal(v));

            added.insert(col_name);
        }

        Ok((columns, values))
    }

    /// Bulk insert records in a single multi-row statement (Salesforce/BC bulkification pattern).

    /// Triggers for every record and the INSERT share one transaction.

    pub async fn insert_records_batch(
        &self,

        table: &str,

        records: Vec<serde_json::Value>,

        ctx: &SessionContext,
    ) -> Result<Vec<Uuid>, RuntimeError> {
        if records.is_empty() {
            return Ok(Vec::new());
        }

        let scope = self.begin_scope(ctx).await?;

        let outcome = self.insert_batch_in(&scope, table, records).await;

        scope.db.finish(outcome).await
    }

    async fn insert_batch_in(
        &self,

        scope: &ExecScope,

        table: &str,

        records: Vec<serde_json::Value>,
    ) -> Result<Vec<Uuid>, RuntimeError> {
        let mut payloads = records;

        for payload in &mut payloads {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnBeforeInsert,
                payload,
                None,
            )
            .await?;

            self.publish_record_event(scope, table, "OnBeforeInsert", payload, None)
                .await?;
        }

        let (physical_table, data_per_company) = self.resolve_table_info(table, &scope.ctx).await?;

        let mut column_set = std::collections::BTreeSet::new();

        for p in &payloads {
            let obj = p.as_object().ok_or_else(|| {
                RuntimeError::Internal(format!(
                    "Every record for '{}' must be a JSON object",
                    table
                ))
            })?;

            for k in obj.keys().filter(|k| !k.starts_with('_')) {
                let sc = to_snake_case(k);
                if sc != "system_messages" && sc != "messages" {
                    column_set.insert(sc);
                }
            }
        }

        let user_columns: Vec<String> = column_set.into_iter().collect();

        let mut cols = vec!["\"_id\"".to_string(), "\"_tenant_id\"".to_string()];

        if data_per_company {
            cols.push("\"_company_id\"".to_string());
        }

        for c in &user_columns {
            cols.push(format!("\"{}\"", sanitize_identifier(c)?));
        }

        let mut generated_ids = Vec::with_capacity(payloads.len());

        let mut row_tuples = Vec::with_capacity(payloads.len());

        for p in &mut payloads {
            let id = p
                .get("_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::now_v7);

            generated_ids.push(id);

            let tenant_token = if scope.ctx.tenant_id.is_nil() {
                "NULL".to_string()
            } else {
                format!("'{}'::uuid", scope.ctx.tenant_id)
            };

            let mut val_tokens = vec![
                format!("'{}'::uuid", id),
                tenant_token,
            ];

            if data_per_company {
                let comp_token = if scope.ctx.company_id.is_nil() {
                    "NULL".to_string()
                } else {
                    format!("'{}'::uuid", scope.ctx.company_id)
                };
                val_tokens.push(comp_token);
            }

            let obj = p.as_object();

            for c in &user_columns {
                let v_opt = obj.and_then(|o| {
                    o.iter()
                        .find(|(k, _)| {
                            k.chars().any(|ch| ch.is_ascii_uppercase()) && &to_snake_case(k) == c
                        })
                        .map(|(_, v)| v)
                        .or_else(|| o.get(c))
                });

                val_tokens.push(
                    v_opt
                        .map(sql_literal)
                        .unwrap_or_else(|| "DEFAULT".to_string()),
                );
            }

            row_tuples.push(format!("({})", val_tokens.join(", ")));

            if let Some(o) = p.as_object_mut() {
                o.insert("_id".to_string(), serde_json::json!(id.to_string()));
            }
        }

        let insert_query = format!(
            "INSERT INTO {} ({}) VALUES {}",
            quote_physical(&physical_table),
            cols.join(", "),
            row_tuples.join(", ")
        );

        {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query(&insert_query).execute(&mut *conn).await;

            scope.db.track(res)?;
        }

        for payload in &mut payloads {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnAfterInsert,
                payload,
                None,
            )
            .await?;

            self.publish_record_event(scope, table, "OnAfterInsert", payload, None)
                .await?;
        }

        Ok(generated_ids)
    }

    /// Update one record as a single unit of work with mandatory optimistic concurrency:

    /// the patch must carry the `_system_version` the client last read.

    pub async fn update_record(
        &self,

        table: &str,

        record_id: Uuid,

        patch: serde_json::Value,

        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let scope = self.begin_scope(ctx).await?;

        let outcome = self.update_in(&scope, table, record_id, patch).await;

        scope.db.finish(outcome).await
    }

    /// Fetch a record by `_id` inside the session, locking it against concurrent writers.

    async fn lock_for_write(
        &self,

        scope: &ExecScope,

        table: &str,

        physical_table: &str,

        record_id: Uuid,
    ) -> Result<serde_json::Value, RuntimeError> {
        let get_query = format!(
            "SELECT to_jsonb(t.*) FROM {} t WHERE t.\"_id\" = $1 FOR UPDATE",
            quote_physical(physical_table)
        );

        let mut conn = scope.db.conn().await?;

        let res = sqlx::query_scalar::<_, serde_json::Value>(&get_query)
            .bind(record_id)
            .fetch_optional(&mut *conn)
            .await;

        scope
            .db
            .track(res)?
            .ok_or_else(|| RuntimeError::NotFound(table.to_string(), record_id.to_string()))
    }

    /// Reject the write when the record's current state forbids it (`allowModify` / `allowDelete`).

    async fn enforce_state_policy(
        &self,

        scope: &ExecScope,

        table: &str,

        x_rec: &serde_json::Value,

        policy: &str,

        label: &str,
    ) -> Result<(), RuntimeError> {
        let sm_def = {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query_scalar::<_, serde_json::Value>(

                "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1",

            )

            .bind(table)

            .fetch_optional(&mut *conn)

            .await;

            scope.db.track(res)?
        };

        let Some(sm_def) = sm_def else {
            return Ok(());
        };

        let status_col = sm_def
            .get("status_field")
            .and_then(|v| v.as_str())
            .unwrap_or("status");

        let Some(cur_state) = x_rec.get(status_col).and_then(|v| v.as_str()) else {
            return Ok(());
        };

        let forbidden = sm_def
            .get("states")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .any(|st| {
                st.get("name")
                    .and_then(|v| v.as_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(cur_state))
                    && st.get(policy).and_then(|v| v.as_bool()) == Some(false)
            });

        if forbidden {
            return Err(RuntimeError::TransitionError(format!(
                "State Machine Policy Violation: Record in table '{}' is in state '{}' which sets {} = false.",
                table, cur_state, label
            )));
        }

        Ok(())
    }

    /// Update inside an existing unit of work.

    pub async fn update_in(
        &self,

        scope: &ExecScope,

        table: &str,

        record_id: Uuid,

        patch: serde_json::Value,
    ) -> Result<serde_json::Value, RuntimeError> {
        if self.is_immutable_ledger(table).await {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Table '{}' is an immutable ledger. Updates and in-place modifications are forbidden by accounting design. Compensating or reversal entries must be posted instead.",
                table
            )));
        }

        let mut patch = patch;

        if !patch.is_object() {
            return Err(RuntimeError::Internal(format!(
                "Patch for '{}' must be a JSON object",
                table
            )));
        }

        let physical_table = self.resolve_physical_table(table, &scope.ctx).await?;

        // 1. Lock the pristine record (xRec) for the rest of the unit of work

        let x_rec = self
            .lock_for_write(scope, table, &physical_table, record_id)
            .await?;

        // 2. Optimistic concurrency: the client must prove it saw the current version

        let versioned = x_rec.get("_system_version").is_some();

        let expected_version = match patch.get("_system_version") {
            None | Some(serde_json::Value::Null) => None,

            Some(v) => Some(
                v.as_i64()
                    .or_else(|| v.as_str().and_then(|s| s.trim().parse::<i64>().ok()))
                    .ok_or_else(|| {
                        RuntimeError::VersionRequired(format!(
                            "'_system_version' must be an integer, got {}",
                            v
                        ))
                    })?,
            ),
        };

        if versioned {
            let current = x_rec
                .get("_system_version")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            match expected_version {
                None => {
                    return Err(RuntimeError::VersionRequired(format!(
                        "Updates to '{}' must include the record's current '_system_version' (currently {})",
                        table, current
                    )));
                }

                Some(ver) if ver != current => {
                    return Err(RuntimeError::ConcurrencyConflict(format!(
                        "Record {} in table '{}' was changed by another user (you have version {}, current version is {})",
                        record_id, table, ver, current
                    )));
                }

                Some(_) => {}
            }
        }

        self.enforce_state_policy(scope, table, &x_rec, "allow_modify", "allowModify")
            .await?;

        // 3. Stage Rec and run field-level OnValidate cascades for changed fields

        let mut rec = x_rec.clone();

        let changed_fields: Vec<(String, serde_json::Value)> =
            match (patch.as_object(), x_rec.as_object()) {
                (Some(p), Some(x)) => p
                    .iter()
                    .filter(|(k, v)| !k.starts_with('_') && x.get(*k) != Some(*v))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),

                _ => Vec::new(),
            };

        for (k, v) in changed_fields {
            if let Some(rec_obj) = rec.as_object_mut() {
                rec_obj.insert(k.clone(), v);
            }

            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnValidate(k.clone()),
                &mut rec,
                Some(&x_rec),
            )
            .await?;

            self.publish_record_event(
                scope,
                table,
                &format!("OnValidate{}", k),
                &mut rec,
                Some(&x_rec),
            )
            .await?;
        }

        // 4. Compute-first: OnDraftSave (drafts) and OnBeforeModify with (Rec, xRec)

        let is_draft_edit = rec.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || rec.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        if is_draft_edit {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnDraftSave,
                &mut rec,
                Some(&x_rec),
            )
            .await?;
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnBeforeModify,
            &mut rec,
            Some(&x_rec),
        )
        .await?;

        self.publish_record_event(scope, table, "OnBeforeModify", &mut rec, Some(&x_rec))
            .await?;

        // Fields mutated or stripped by triggers (e.g. hashPassword) flow back into the patch

        if let (Some(rec_obj), Some(patch_obj)) = (rec.as_object(), patch.as_object_mut()) {
            for (k, v) in rec_obj {
                let snake = to_snake_case(k);
                if SYSTEM_WRITE_PROTECTED.contains(&k.as_str())
                    || SYSTEM_WRITE_PROTECTED.contains(&snake.as_str())
                    || k.starts_with('_')
                {
                    continue;
                }
                let in_x_rec = x_rec
                    .as_object()
                    .and_then(|x| x.get(k).or_else(|| x.get(&snake)));
                if in_x_rec != Some(v) {
                    patch_obj.insert(k.clone(), v.clone());
                }
            }

            patch_obj.retain(|k, _| rec_obj.contains_key(k) || k.starts_with('_'));
        }

        // 5. Write with the version predicate (the row lock makes this a final safety net)

        let mut set_clauses = Vec::new();

        if let Some(obj) = patch.as_object() {
            for (k, v) in obj {
                let col_name = if k.starts_with('_') {
                    k.clone()
                } else {
                    to_snake_case(k)
                };

                if SYSTEM_WRITE_PROTECTED.contains(&col_name.as_str()) {
                    continue;
                }

                let safe_col = sanitize_identifier(&col_name)?;

                set_clauses.push(format!("\"{}\" = {}", safe_col, sql_literal(v)));
            }
        }

        if x_rec.get("_updated_at").is_some() {
            set_clauses.push("\"_updated_at\" = NOW()".to_string());
        }

        if x_rec.get("_updated_by").is_some() {
            set_clauses.push(format!("\"_updated_by\" = '{}'::uuid", scope.ctx.user_id));
        }

        if versioned {
            set_clauses
                .push("\"_system_version\" = COALESCE(t.\"_system_version\", 0) + 1".to_string());
        }

        if set_clauses.is_empty() {
            return Ok(x_rec);
        }

        let mut where_clause = format!("t.\"_id\" = '{}'::uuid", record_id);

        if let (true, Some(ver)) = (versioned, expected_version) {
            where_clause.push_str(&format!(
                " AND COALESCE(t.\"_system_version\", 0) = {}",
                ver
            ));
        }

        let update_query = format!(
            "UPDATE {} AS t SET {} WHERE {} RETURNING to_jsonb(t.*)",
            quote_physical(&physical_table),
            set_clauses.join(", "),
            where_clause
        );

        let updated = {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query_scalar::<_, serde_json::Value>(&update_query)
                .fetch_optional(&mut *conn)
                .await;

            scope.db.track(res)?
        };

        let mut updated = updated.ok_or_else(|| {
            RuntimeError::ConcurrencyConflict(format!(
                "Record {} in table '{}' was modified by a concurrent transaction",
                record_id, physical_table
            ))
        })?;

        // 6. OnAfterModify and subscribers see the persisted row; failures roll everything back

        let mut after = updated.clone();

        if let (Some(a), Some(msgs)) = (after.as_object_mut(), rec.get("_system_messages")) {
            a.insert("_system_messages".to_string(), msgs.clone());
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnAfterModify,
            &mut after,
            Some(&x_rec),
        )
        .await?;

        self.publish_record_event(scope, table, "OnAfterModify", &mut after, Some(&x_rec))
            .await?;

        if let (Some(u), Some(msgs)) = (updated.as_object_mut(), after.get("_system_messages")) {
            u.insert("_system_messages".to_string(), msgs.clone());
        }

        Ok(updated)
    }

    /// Delete one record as a single unit of work (triggers, delete and subscribers together).

    pub async fn delete_record(
        &self,

        table: &str,

        record_id: Uuid,

        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let scope = self.begin_scope(ctx).await?;

        let outcome = self.delete_in(&scope, table, record_id).await;

        scope.db.finish(outcome).await
    }

    /// Delete inside an existing unit of work.

    pub async fn delete_in(
        &self,

        scope: &ExecScope,

        table: &str,

        record_id: Uuid,
    ) -> Result<serde_json::Value, RuntimeError> {
        if self.is_immutable_ledger(table).await {
            return Err(RuntimeError::ImmutableLedger(format!(
                "Table '{}' is an immutable ledger. Deletions are strictly prohibited. Correcting or reversal entries must be posted instead.",
                table
            )));
        }

        let physical_table = self.resolve_physical_table(table, &scope.ctx).await?;

        let x_rec = self
            .lock_for_write(scope, table, &physical_table, record_id)
            .await?;

        self.enforce_state_policy(scope, table, &x_rec, "allow_delete", "allowDelete")
            .await?;

        let mut data = x_rec.clone();

        let is_draft = x_rec.get("status").and_then(|v| v.as_str()) == Some("Draft")
            || x_rec.get("is_draft").and_then(|v| v.as_bool()) == Some(true);

        if is_draft {
            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnDraftDiscard,
                &mut data,
                Some(&x_rec),
            )
            .await?;
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnBeforeDelete,
            &mut data,
            Some(&x_rec),
        )
        .await?;

        self.publish_record_event(scope, table, "OnBeforeDelete", &mut data, Some(&x_rec))
            .await?;

        let del_query = format!(
            "DELETE FROM {} WHERE \"_id\" = $1",
            quote_physical(&physical_table)
        );

        let affected = {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query(&del_query)
                .bind(record_id)
                .execute(&mut *conn)
                .await;

            scope.db.track(res)?.rows_affected()
        };

        if affected == 0 {
            return Err(RuntimeError::NotFound(
                table.to_string(),
                record_id.to_string(),
            ));
        }

        ExecutionEngine::dispatch_triggers(
            scope,
            table,
            &TriggerEventIR::OnAfterDelete,
            &mut data,
            Some(&x_rec),
        )
        .await?;

        self.publish_record_event(scope, table, "OnAfterDelete", &mut data, Some(&x_rec))
            .await?;

        Ok(x_rec)
    }

    /// Run event subscribers for a record lifecycle event inside the unit of work.

    async fn publish_record_event(
        &self,

        scope: &ExecScope,

        table: &str,

        event: &str,

        data: &mut serde_json::Value,

        x_rec: Option<&serde_json::Value>,
    ) -> Result<(), RuntimeError> {
        if self.event_registry.get_subscribers(table, event).is_empty() {
            return Ok(());
        }

        let (out, _) = self
            .event_registry
            .dispatch_in(table, event, data.clone(), x_rec.cloned(), scope)
            .await?;

        *data = out;

        Ok(())
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

            None => Err(RuntimeError::NotFound(
                table.to_string(),
                record_id.to_string(),
            )),
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

        let mut flow_fields_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        if let Some(meta) = meta_obj {
            if let Some(ff_arr) = meta
                .definition
                .get("flow_fields")
                .and_then(|v| v.as_array())
            {
                for ff in ff_arr {
                    if let (Some(name), Some(subq)) = (
                        ff.get("name").and_then(|v| v.as_str()),
                        ff.get("sql_subquery").and_then(|v| v.as_str()),
                    ) {
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

                    if let Some(subquery) = flow_fields_map
                        .get(&f.to_lowercase())
                        .or_else(|| flow_fields_map.get(&f_snake))
                    {
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

                let fk_col = inc
                    .foreign_key
                    .as_deref()
                    .map(to_snake_case)
                    .unwrap_or_else(|| format!("{}_id", to_snake_case(&req.entity)));

                let safe_fk = sanitize_identifier(&fk_col)?;

                let rel_limit = inc.limit.unwrap_or(100);

                let rel_select = match &inc.select {
                    Some(s_cols) if !s_cols.is_empty() => {
                        let pairs: Vec<String> = s_cols
                            .iter()
                            .filter_map(|c| sanitize_identifier(&to_snake_case(c)).ok())
                            .map(|sc| format!("'{}', rel.\"{}\"", sc, sc))
                            .collect();

                        format!("json_build_object({})", pairs.join(", "))
                    }

                    _ => "to_jsonb(rel.*)".to_string(),
                };

                let parent_join_col = if safe_fk.ends_with("_code") {
                    "\"code\""
                } else {
                    "\"_id\""
                };

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
                format!(
                    "to_jsonb(t.*) || json_build_object({})",
                    projections.join(", ")
                )
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
                            where_clauses.push(format!(
                                "t.\"{}\" = '{}'",
                                safe_col,
                                s.replace('\'', "''")
                            ));
                        }

                        serde_json::Value::Number(n) => {
                            where_clauses.push(format!("t.\"{}\" = {}", safe_col, n));
                        }

                        serde_json::Value::Bool(b) => {
                            where_clauses.push(format!(
                                "t.\"{}\" = {}",
                                safe_col,
                                if *b { "TRUE" } else { "FALSE" }
                            ));
                        }

                        serde_json::Value::Object(op_map) => {
                            for (op, val) in op_map {
                                let lit = sql_literal(val);

                                match op.as_str() {
                                    "$gt" => {
                                        where_clauses.push(format!("t.\"{}\" > {}", safe_col, lit))
                                    }

                                    "$gte" => {
                                        where_clauses.push(format!("t.\"{}\" >= {}", safe_col, lit))
                                    }

                                    "$lt" => {
                                        where_clauses.push(format!("t.\"{}\" < {}", safe_col, lit))
                                    }

                                    "$lte" => {
                                        where_clauses.push(format!("t.\"{}\" <= {}", safe_col, lit))
                                    }

                                    "$ne" => {
                                        where_clauses.push(format!("t.\"{}\" != {}", safe_col, lit))
                                    }

                                    "$eq" => {
                                        if val.is_null() {
                                            where_clauses.push(format!("t.\"{}\" IS NULL", safe_col));
                                        } else {
                                            where_clauses.push(format!("t.\"{}\" = {}", safe_col, lit));
                                        }
                                    }

                                    // `{ "$in": [..] }`: text comparison, so UUIDs, codes and numbers all match
                                    // their JSON form (compiled pages fetch FlowFields of the visible rows).
                                    "$in" => {
                                        let items: Vec<String> = val
                                            .as_array()
                                            .map(|a| {
                                                a.iter()
                                                    .filter(|v| !v.is_null())
                                                    .map(|v| match v {
                                                        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                                                        other => format!("'{}'", other.to_string().replace('\'', "''")),
                                                    })
                                                    .collect()
                                            })
                                            .unwrap_or_default();
                                        if items.is_empty() {
                                            where_clauses.push("FALSE".to_string());
                                        } else {
                                            where_clauses.push(format!("t.\"{}\"::text IN ({})", safe_col, items.join(", ")));
                                        }
                                    }

                                    "$like" => {
                                        if let Some(pat) = val.as_str() {
                                            where_clauses.push(format!(
                                                "t.\"{}\" LIKE '{}'",
                                                safe_col,
                                                pat.replace('\'', "''")
                                            ));
                                        }
                                    }

                                    "$ilike" => {
                                        if let Some(pat) = val.as_str() {
                                            where_clauses.push(format!(
                                                "t.\"{}\" ILIKE '{}'",
                                                safe_col,
                                                pat.replace('\'', "''")
                                            ));
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

        let offset_clause = req
            .offset
            .map(|o| format!("OFFSET {}", o))
            .unwrap_or_default();

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

        let job_id = Uuid::now_v7();

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

    pub async fn execute_next_job(
        &self,
        ctx: &SessionContext,
    ) -> Result<Option<Uuid>, RuntimeError> {
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

        // The job's business work and its 'completed' status commit atomically; on failure the

        // work is rolled back and the failure is recorded separately.

        let scope = self.begin_scope(ctx).await?;

        let work = async {

            ExecutionEngine::execute_procedure_in(&scope, &cu_name, &method, payload).await?;

            let mut conn = scope.db.conn().await?;

            let res = sqlx::query(

                "UPDATE \"_bos_job_queue\" SET \"status\" = 'completed', \"completed_at\" = NOW(), \"error_message\" = NULL WHERE \"job_id\" = $1"

            )

            .bind(job_id)

            .execute(&mut *conn)

            .await;

            scope.db.track(res).map(|_| ())

        }

        .await;

        if let Err(e) = scope.db.finish(work).await {
            sqlx::query(

                "UPDATE \"_bos_job_queue\" SET \"status\" = 'failed', \"error_message\" = $1, \"completed_at\" = NOW() WHERE \"job_id\" = $2"

            )

            .bind(e.to_string())

            .bind(job_id)

            .execute(&pool)

            .await?;
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

        let set_reason = format!(
            "SET LOCAL bos.privileged_reason = '{}';",
            reason.replace("'", "''")
        );

        let set_ticket = format!(
            "SET LOCAL bos.privileged_ticket = '{}';",
            ticket.replace("'", "''")
        );

        let set_auth = format!(
            "SET LOCAL bos.privileged_authorizer = '{}';",
            authorizer.replace("'", "''")
        );

        sqlx::query(set_override).execute(&mut *tx).await?;

        sqlx::query(&set_reason).execute(&mut *tx).await?;

        sqlx::query(&set_ticket).execute(&mut *tx).await?;

        sqlx::query(&set_auth).execute(&mut *tx).await?;

        let safe_col = sanitize_identifier(column)?;

        let query = format!(
            "UPDATE \"{}\" SET \"{}\" = '{}', \"_updated_at\" = NOW() WHERE \"_id\" = '{}';",
            physical_table,
            safe_col,
            new_value.replace("'", "''"),
            record_id
        );

        let rows = sqlx::query(&query).execute(&mut *tx).await?.rows_affected();

        if rows == 0 {
            return Err(RuntimeError::NotFound(
                table.to_string(),
                record_id.to_string(),
            ));
        }

        tx.commit().await?;

        Ok(())
    }

    /// Atomic State Machine Transition: guard, transition effect, draft-activation triggers,

    /// the status update and the history row share one transaction.

    #[allow(clippy::too_many_arguments)]

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
        let scope = self.begin_scope(ctx).await?;

        let outcome = self
            .transition_in(
                &scope,
                table,
                record_id,
                status_column,
                from_state,
                to_state,
                action,
                comments,
                params,
            )
            .await;

        scope.db.finish(outcome).await
    }

    #[allow(clippy::too_many_arguments)]

    async fn transition_in(
        &self,

        scope: &ExecScope,

        table: &str,

        record_id: Uuid,

        status_column: &str,

        from_state: &str,

        to_state: &str,

        action: &str,

        comments: Option<&str>,

        params: Option<serde_json::Value>,
    ) -> Result<(), RuntimeError> {
        let ctx = &scope.ctx;

        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let safe_status_col = sanitize_identifier(status_column)?;

        let current_rec = self
            .lock_for_write(scope, table, &physical_table, record_id)
            .await?;

        let sm_def = {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query_scalar::<_, serde_json::Value>(

                "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1"

            )

            .bind(table)

            .fetch_optional(&mut *conn)

            .await;

            scope.db.track(res)?
        };

        if let Some(tr) = sm_def.as_ref().and_then(|sm| {
            sm.get("transitions")
                .and_then(|v| v.as_array())?
                .iter()
                .find(|tr| {
                    let tr_name = tr.get("name").and_then(|v| v.as_str()).unwrap_or("");

                    let tr_to = tr.get("to_state").and_then(|v| v.as_str()).unwrap_or("");

                    let from_ok =
                        tr.get("from_states")
                            .and_then(|v| v.as_array())
                            .is_none_or(|arr| {
                                arr.iter().any(|f| {
                                    f.as_str()
                                        .is_some_and(|s| s.eq_ignore_ascii_case(from_state))
                                })
                            });

                    (tr_name.eq_ignore_ascii_case(action) || tr_to.eq_ignore_ascii_case(to_state))
                        && from_ok
                })
        }) {
            // 1. Enforce required_roles if specified

            if let Some(req_roles) = tr.get("required_roles").and_then(|v| v.as_array()) {
                let non_empty_roles: Vec<&str> = req_roles
                    .iter()
                    .filter_map(|r| r.as_str())
                    .filter(|s| !s.is_empty())
                    .collect();

                if !non_empty_roles.is_empty() {
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

            // 2. Guard script must not throw / return false

            if let Some(guard_src) = tr
                .get("guard_script")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                let wrapped_guard = if guard_src.contains("return ") || guard_src.contains(';') {
                    format!(
                        "const __guardRes = (() => {{ {} }})(); if (__guardRes === false) {{ throw new Error('Transition guard returned false'); }}",
                        guard_src
                    )
                } else {
                    format!(
                        "const __guardRes = ({}); if (!__guardRes) {{ throw new Error('Transition guard failed: ' + {}); }}",
                        guard_src,
                        serde_json::to_string(guard_src)
                            .map_err(|e| RuntimeError::Internal(e.to_string()))?
                    )
                };

                QuickJsEngine::run_script(
                    scope,
                    ScriptCall {
                        table_name: table.to_string(),

                        trigger_name: format!("guard_{}", action),

                        script: wrapped_guard,

                        bytecode: None,

                        data: current_rec.clone(),

                        x_rec: Some(current_rec.clone()),

                        limits: ExecutionLimits::default(),

                        ui_answers: scope.ui_answers.clone(),
                    },
                )
                .await
                .map_err(|e| {
                    RuntimeError::TransitionError(format!(
                        "Guard rejected transition '{}': {}",
                        action, e
                    ))
                })?;
            }

            // 3. Transition effect script runs inside the same transaction

            if let Some(effect_src) = tr
                .get("on_transition_script")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                QuickJsEngine::run_script(
                    scope,
                    ScriptCall {
                        table_name: table.to_string(),

                        trigger_name: format!("onTransition_{}", action),

                        script: effect_src.to_string(),

                        bytecode: None,

                        data: current_rec.clone(),

                        x_rec: Some(current_rec.clone()),

                        limits: ExecutionLimits::default(),

                        ui_answers: scope.ui_answers.clone(),
                    },
                )
                .await?;
            }
        }

        // 4. Activating a draft runs OnDraftActivate invariants

        if from_state.eq_ignore_ascii_case("Draft") {
            let mut draft_data = current_rec.clone();

            ExecutionEngine::dispatch_triggers(
                scope,
                table,
                &TriggerEventIR::OnDraftActivate,
                &mut draft_data,
                Some(&current_rec),
            )
            .await?;
        }

        // 5. Conditional status update (fails if the record is no longer in from_state)

        let update_query = format!(
            "UPDATE {} SET \"{}\" = $1, \"_system_version\" = COALESCE(\"_system_version\", 0) + 1, \"_updated_at\" = NOW() WHERE \"_id\" = $2 AND \"{}\" = $3",
            quote_physical(&physical_table),
            safe_status_col,
            safe_status_col
        );

        let affected = {
            let mut conn = scope.db.conn().await?;

            let res = sqlx::query(&update_query)
                .bind(to_state)
                .bind(record_id)
                .bind(from_state)
                .execute(&mut *conn)
                .await;

            scope.db.track(res)?.rows_affected()
        };

        if affected == 0 {
            return Err(RuntimeError::TransitionError(format!(
                "Failed transition '{}' on {}: record not found or current state is not '{}'",
                action, record_id, from_state
            )));
        }

        // 6. State history row

        let history_table = sanitize_identifier(&format!(
            "{}_state_history",
            to_snake_case(table).to_lowercase()
        ))?;

        let history_query = format!(
            "INSERT INTO \"{}\" (\"entity_id\", \"from_state\", \"to_state\", \"action\", \"actor_id\", \"comments\", \"params\") VALUES ($1, $2, $3, $4, $5, $6, $7)",
            history_table
        );

        let mut conn = scope.db.conn().await?;

        let res = sqlx::query(&history_query)
            .bind(record_id)
            .bind(from_state)
            .bind(to_state)
            .bind(action)
            .bind(ctx.user_id)
            .bind(comments)
            .bind(params)
            .execute(&mut *conn)
            .await;

        scope.db.track(res)?;

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
                    .filter(|s| {
                        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
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

        // Multi-tenant and company isolation WHERE conditions
        let mut where_clauses = Vec::new();
        if !ctx.tenant_id.is_nil() {
            where_clauses.push(format!("t.\"_tenant_id\" = '{}'", ctx.tenant_id));
        } else {
            where_clauses.push("t.\"_tenant_id\" IS NULL".to_string());
        }

        if data_per_company {
            if !ctx.company_id.is_nil() {
                where_clauses.push(format!("t.\"_company_id\" = '{}'", ctx.company_id));
            } else {
                where_clauses.push("t.\"_company_id\" IS NULL".to_string());
            }
        }

        // Optional filter (supports JSON object or key:value)

        if let Some(f_str) = &params.filter {
            if let Ok(filter_obj) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(f_str)
            {
                for (k, v) in filter_obj {
                    let col_name = if k.starts_with('_') {
                        k.clone()
                    } else {
                        to_snake_case(&k)
                    };

                    if let Ok(safe_col) = sanitize_identifier(&col_name) {
                        match v {
                            serde_json::Value::String(s) => {
                                where_clauses.push(format!(
                                    "t.\"{}\"::text ILIKE '%{}%'",
                                    safe_col,
                                    s.replace('\'', "''")
                                ));
                            }

                            serde_json::Value::Number(n) => {
                                where_clauses.push(format!("t.\"{}\" = {}", safe_col, n));
                            }

                            serde_json::Value::Bool(b) => {
                                where_clauses.push(format!(
                                    "t.\"{}\" = {}",
                                    safe_col,
                                    if b { "TRUE" } else { "FALSE" }
                                ));
                            }

                            serde_json::Value::Null => {
                                where_clauses.push(format!("t.\"{}\" IS NULL", safe_col));
                            }

                            // `{ "$eq": v }`: exact match (page part links / SubPageLink), unlike the
                            // substring match of a plain string.
                            serde_json::Value::Object(op) => match op.get("$eq") {
                                Some(serde_json::Value::Null) => {
                                    where_clauses.push(format!("t.\"{}\" IS NULL", safe_col));
                                }
                                Some(serde_json::Value::String(s)) => {
                                    where_clauses.push(format!("t.\"{}\"::text = '{}'", safe_col, s.replace('\'', "''")));
                                }
                                Some(serde_json::Value::Number(n)) => {
                                    where_clauses.push(format!("t.\"{}\" = {}", safe_col, n));
                                }
                                Some(serde_json::Value::Bool(b)) => {
                                    where_clauses.push(format!("t.\"{}\" = {}", safe_col, if *b { "TRUE" } else { "FALSE" }));
                                }
                                _ => {}
                            },

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

                if trimmed.is_empty() {
                    continue;
                }

                let (col, dir) = if let Some((c, d)) = trimmed.split_once(':') {
                    (
                        c.trim(),
                        if d.trim().eq_ignore_ascii_case("desc") {
                            "DESC"
                        } else {
                            "ASC"
                        },
                    )
                } else if let Some((c, d)) = trimmed.split_once(' ') {
                    (
                        c.trim(),
                        if d.trim().eq_ignore_ascii_case("desc") {
                            "DESC"
                        } else {
                            "ASC"
                        },
                    )
                } else {
                    (trimmed, "ASC")
                };

                let col_name = if col.starts_with('_') {
                    col.to_string()
                } else {
                    to_snake_case(col)
                };

                if let Ok(safe_col) = sanitize_identifier(&col_name) {
                    order_parts.push(format!("t.\"{}\" {}", safe_col, dir));
                }
            }

            if !order_parts.is_empty() {
                order_sql = order_parts.join(", ");
            }
        }

        // Total count

        let count_query = format!(
            "SELECT COUNT(*)::bigint FROM \"{}\" t WHERE {};",
            physical_table, where_sql
        );

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

        let total_pages = if total == 0 {
            1
        } else {
            (total + limit - 1) / limit
        };

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

        // Fast path: AOT Prepared Statement for full record lookup (only if keyed on _id/id)
        let is_full_record = fields.map(|f| f.is_empty()).unwrap_or(true);
        if is_full_record {
            if let Some(stmt) = self.prepared_statements.get_table_kind(table, "get_by_id") {
                let first_param = stmt.param_names.first().map(|s| s.as_str());
                if first_param == Some("_id") {
                    let sql = format!("SELECT to_jsonb(t.*) FROM ({}) t", stmt.sql.trim_end_matches(';'));
                    let mut q = sqlx::query_scalar::<_, serde_json::Value>(&sql);
                    for p in &stmt.param_names {
                        if p == "_tenant_id" {
                            if ctx.tenant_id.is_nil() {
                                q = q.bind(Option::<Uuid>::None);
                            } else {
                                q = q.bind(Some(ctx.tenant_id));
                            }
                        } else if p == "_company_id" {
                            if ctx.company_id.is_nil() {
                                q = q.bind(Option::<Uuid>::None);
                            } else {
                                q = q.bind(Some(ctx.company_id));
                            }
                        } else {
                            q = q.bind(id.to_string());
                        }
                    }
                    let row = q.fetch_optional(&mut *tx).await?;
                    tx.commit().await?;
                    return Ok(row);
                }
            }
        }

        let physical_table = self.resolve_physical_table(table, ctx).await?;

        let projection = match fields {
            Some(f) if !f.is_empty() => {
                let valid_fields: Vec<&String> = f
                    .iter()
                    .filter(|s| {
                        !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
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

        let query = format!(
            "SELECT {} as data FROM \"{}\" t WHERE \"_id\" = $1;",
            projection, physical_table
        );

        let mut row = sqlx::query_scalar::<_, serde_json::Value>(&query)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        tx.commit().await?;

        if let Some(ref mut record) = row {
            let excluded = self.get_excluded_fields(ctx, table).await.unwrap_or_default();
            Self::mask_excluded_fields(record, &excluded);
        }

        Ok(row)
    }

    /// Compute FlowField live via generated subquery with exact decimal arithmetic
    pub async fn calculate_flowfield(
        &self,
        table: &str,
        record_id: Uuid,
        computed_subquery: &str,
    ) -> Result<rust_decimal::Decimal, RuntimeError> {
        let pool = self.pool().await;
        let safe_table = sanitize_identifier(table)?;
        let query = format!(
            "SELECT COALESCE(({}), 0) as val FROM \"{}\" WHERE \"_id\" = $1;",
            computed_subquery, safe_table
        );
        let row = sqlx::query(&query)
            .bind(record_id)
            .fetch_one(&pool)
            .await?;
        let val: Option<rust_decimal::Decimal> = row.try_get("val").ok();
        Ok(val.unwrap_or(rust_decimal::Decimal::ZERO))
    }

    /// Execute the business logic test suite. Everything the tests write is rolled back:

    /// each test to its own savepoint, and the whole run's transaction at the end.

    pub async fn run_tests(
        &self,

        codeunit_filter: Option<&str>,

        ctx: &SessionContext,
    ) -> Result<serde_json::Value, RuntimeError> {
        let scope = self.begin_scope(ctx).await?;

        let outcome = ExecutionEngine::execute_tests(&scope, codeunit_filter).await;

        scope.db.rollback().await?;

        outcome
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn test_company_only_mode_and_uuid_v7() {
        // 1. UUIDv7 generation
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        assert!(id2 >= id1, "UUIDv7 must be time-ordered");

        // 2. CompanyInfo in company-only mode (tenant_id: None)
        let company = CompanyInfo {
            company_id: id1,
            tenant_id: None,
            code: "CRONUS".to_string(),
            name: "Cronus Enterprise".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
        };
        let serialized = serde_json::to_string(&company).expect("serializes");
        let deserialized: CompanyInfo = serde_json::from_str(&serialized).expect("deserializes");
        assert_eq!(deserialized.company_id, id1);
        assert_eq!(deserialized.tenant_id, None);
        assert_eq!(deserialized.code, "CRONUS");

        // 3. insert_tuple in company-only mode emits NULL for _tenant_id and UUID for _company_id
        let ctx = SessionContext {
            tenant_id: Uuid::nil(),
            company_id: id1,
            user_id: id2,
            roles: vec!["admin".to_string()],
        };
        let mut obj = serde_json::Map::new();
        obj.insert("code".to_string(), serde_json::json!("CUST001"));
        obj.insert("name".to_string(), serde_json::json!("Acme Corp"));

        let (cols, vals) = BosEngine::insert_tuple(&obj, id2, &ctx, true).expect("insert_tuple");
        assert!(cols.contains(&"\"_tenant_id\"".to_string()));
        assert!(cols.contains(&"\"_company_id\"".to_string()));

        let tenant_val_idx = cols.iter().position(|c| c == "\"_tenant_id\"").unwrap();
        assert_eq!(vals[tenant_val_idx], "NULL");

        let comp_val_idx = cols.iter().position(|c| c == "\"_company_id\"").unwrap();
        assert_eq!(vals[comp_val_idx], format!("'{}'::uuid", id1));
    }

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

        assert_eq!(
            replaced,
            "postgres://postgres:postgres@localhost:5432/bos_prod?sslmode=disable"
        );

        let url2 = "postgres://postgres:postgres@localhost:5432/bos_dev";

        let replaced2 = DatabaseManager::replace_database_name(url2, "postgres");

        assert_eq!(
            replaced2,
            "postgres://postgres:postgres@localhost:5432/postgres"
        );
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

            active_url: Arc::new(tokio::sync::RwLock::new(
                "postgres://postgres:postgres@localhost:5432/postgres".to_string(),
            )),

            meta_cache: Arc::new(dashmap::DashMap::new()),

            event_registry: Arc::new(EventRegistry::new()),

            wasm_engine: Arc::new(WasmEngine::new().unwrap()),

            sidecar_host: Arc::new(SidecarHost::new()),

            notification_hub: Arc::new(NotificationHub::default_hub()),

            bytecode_cache: Arc::new(BytecodeCache::new()),

            prepared_statements: Arc::new(PreparedStatementRegistry::new()),

            emergency_sessions: Arc::new(dashmap::DashMap::new()),

            pages: Arc::new(arc_swap::ArcSwapOption::empty()),
        };

        // Verifies no panics during Axum router route registration (matchit conflict checks)

        let _router = create_api_router(engine);
    }

    #[test]

    fn test_meta_cache_operations() {
        let cache = Arc::new(dashmap::DashMap::new());

        let sample = meta_manager::MetaObjectDetails {
            summary: meta_manager::MetaObjectSummary {
                object_id: Uuid::now_v7(),

                object_type: "table".to_string(),

                object_name: "Customer".to_string(),

                package_name: "base".to_string(),

                version: "1.0".to_string(),

                caption: None,

                sub_type: Some("Normal".to_string()),

                status: "active".to_string(),

                created_at: chrono::Utc::now(),
                feature: None,
                definition: None,
            },

            fields: vec![],

            relations: vec![],

            definition: serde_json::json!({ "name": "Customer" }),

            object_type: "table".to_string(),

            source_script: None,
        };

        cache.insert("customer".to_string(), sample.clone());

        assert!(cache.contains_key("customer"));

        assert_eq!(
            cache.get("customer").unwrap().summary.object_name,
            "Customer"
        );

        cache.remove("customer");

        assert!(!cache.contains_key("customer"));
    }
}
