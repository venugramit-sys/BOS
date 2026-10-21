use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeConfigError {
    #[error("Failed to read node configuration file '{0}': {1}")]
    IoError(String, #[source] std::io::Error),
    #[error("Failed to parse TOML configuration from '{0}': {1}")]
    ParseError(String, #[source] toml::de::Error),
    #[error("Configuration validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeRole {
    /// Serves UI static files, API endpoints, and runs background NAS worker
    AllInOne,
    /// Interactive user node: serves UI static files and API endpoints (no NAS worker)
    Interactive,
    /// Headless Navision Application Server (NAS) worker: runs background batch jobs and outbox
    NasWorker,
    /// Headless API Gateway node optimized for external integrations
    ApiGateway,
}

impl Default for NodeRole {
    fn default() -> Self {
        NodeRole::AllInOne
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSection {
    #[serde(default = "default_node_id")]
    pub id: String,
    #[serde(default = "default_node_name")]
    pub name: String,
    #[serde(default)]
    pub role: NodeRole,
}

fn default_node_id() -> String {
    "node-primary-01".to_string()
}

fn default_node_name() -> String {
    "BOS Enterprise Application Server Node".to_string()
}

impl Default for NodeSection {
    fn default() -> Self {
        Self {
            id: default_node_id(),
            name: default_node_name(),
            role: NodeRole::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_cors")]
    pub cors_origins: Vec<String>,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_cors() -> Vec<String> {
    vec!["*".to_string()]
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            cors_origins: default_cors(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSection {
    #[serde(default = "default_ui_enabled")]
    pub enabled: bool,
    #[serde(default = "default_static_dir")]
    pub static_dir: String,
    #[serde(default = "default_index_file")]
    pub index_file: String,
}

fn default_ui_enabled() -> bool {
    true
}

fn default_static_dir() -> String {
    "./ui/dist".to_string()
}

fn default_index_file() -> String {
    "index.html".to_string()
}

impl Default for UiSection {
    fn default() -> Self {
        Self {
            enabled: default_ui_enabled(),
            static_dir: default_static_dir(),
            index_file: default_index_file(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSection {
    pub url: String,
    #[serde(default = "default_min_pool")]
    pub min_pool_size: u32,
    #[serde(default = "default_max_pool")]
    pub max_pool_size: u32,
    #[serde(default = "default_storage_strategy")]
    pub storage_strategy: String,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_seconds: u64,
}

fn default_min_pool() -> u32 {
    5
}

fn default_max_pool() -> u32 {
    30
}

fn default_storage_strategy() -> String {
    "shared_rls".to_string()
}

fn default_connection_timeout() -> u64 {
    10
}

impl Default for DatabaseSection {
    fn default() -> Self {
        Self {
            url: "postgres://postgres:postgres@localhost:5432/bos_dev".to_string(),
            min_pool_size: default_min_pool(),
            max_pool_size: default_max_pool(),
            storage_strategy: default_storage_strategy(),
            connection_timeout_seconds: default_connection_timeout(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSection {
    #[serde(default = "default_tenant_code")]
    pub default_tenant: String,
    #[serde(default = "default_company_code")]
    pub default_company: String,
}

fn default_tenant_code() -> String {
    "DEFAULT_TENANT".to_string()
}

fn default_company_code() -> String {
    "MAIN_COMPANY".to_string()
}

impl Default for DefaultsSection {
    fn default() -> Self {
        Self {
            default_tenant: default_tenant_code(),
            default_company: default_company_code(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NasWorkerSection {
    #[serde(default = "default_nas_enabled")]
    pub enabled: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_batch_size")]
    pub batch_size: i64,
    #[serde(default = "default_bool_true")]
    pub process_outbox: bool,
    #[serde(default = "default_bool_true")]
    pub process_job_queue: bool,
}

fn default_nas_enabled() -> bool {
    true
}

fn default_poll_interval() -> u64 {
    1000
}

fn default_batch_size() -> i64 {
    50
}

fn default_bool_true() -> bool {
    true
}

impl Default for NasWorkerSection {
    fn default() -> Self {
        Self {
            enabled: default_nas_enabled(),
            poll_interval_ms: default_poll_interval(),
            batch_size: default_batch_size(),
            process_outbox: default_bool_true(),
            process_job_queue: default_bool_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    /// When true, enables emergency fallback super-admin access or bypasses auth in maintenance mode
    #[serde(default)]
    pub maintenance_mode: bool,
    /// Fallback emergency username
    #[serde(default = "default_emergency_username")]
    pub emergency_username: String,
    /// Fallback emergency display name
    #[serde(default = "default_emergency_name")]
    pub emergency_name: String,
    /// Roles assigned to the emergency super admin session
    #[serde(default = "default_emergency_roles")]
    pub emergency_roles: Vec<String>,
    /// When true, automatically activates emergency super-admin session if the .bos auth module fails/breaks
    #[serde(default = "default_bool_true")]
    pub auto_fallback_on_auth_failure: bool,
}

fn default_emergency_username() -> String {
    "emergency.admin".to_string()
}

fn default_emergency_name() -> String {
    "Emergency Maintenance Administrator".to_string()
}

fn default_emergency_roles() -> Vec<String> {
    vec![
        "SUPER".to_string(),
        "Admin".to_string(),
        "Global Admin".to_string(),
    ]
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            maintenance_mode: false,
            emergency_username: default_emergency_username(),
            emergency_name: default_emergency_name(),
            emergency_roles: default_emergency_roles(),
            auto_fallback_on_auth_failure: default_bool_true(),
        }
    }
}

/// Compiled page artifacts (Svelte 5 page modules) served from disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesSection {
    #[serde(default = "default_bool_true")]
    pub enabled: bool,
    /// Root of the content-addressed artifact store (`<root>/<db_key>/<Page>.<hash>.js`).
    #[serde(default = "default_artifact_dir")]
    pub artifact_dir: String,
    /// Folder holding `svelte-compiler.js` and `version.json` from the UI runtime build.
    /// Defaults to `<ui.static_dir>/bos-runtime`.
    #[serde(default)]
    pub runtime_dir: Option<String>,
    /// Svelte compiler worker threads; 0 = min(CPU cores, 4).
    #[serde(default)]
    pub compiler_workers: usize,
    /// Folders searched for pre-built artifacts (`bosc package build` output) before compiling.
    #[serde(default = "default_seed_dirs")]
    pub seed_dirs: Vec<String>,
}

fn default_artifact_dir() -> String {
    "./data/page-artifacts".to_string()
}

fn default_seed_dirs() -> Vec<String> {
    vec!["./dist/packages".to_string()]
}

impl Default for PagesSection {
    fn default() -> Self {
        Self {
            enabled: true,
            artifact_dir: default_artifact_dir(),
            runtime_dir: None,
            compiler_workers: 0,
            seed_dirs: default_seed_dirs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    #[serde(default)]
    pub node: NodeSection,
    #[serde(default)]
    pub network: NetworkSection,
    #[serde(default)]
    pub ui: UiSection,
    #[serde(default)]
    pub database: DatabaseSection,
    #[serde(default)]
    pub defaults: DefaultsSection,
    #[serde(default)]
    pub nas_worker: NasWorkerSection,
    #[serde(default)]
    pub security: SecuritySection,
    #[serde(default)]
    pub pages: PagesSection,
}

impl NodeConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, NodeConfigError> {
        let path_ref = path.as_ref();
        let display_path = path_ref.display().to_string();
        let content = fs::read_to_string(path_ref)
            .map_err(|e| NodeConfigError::IoError(display_path.clone(), e))?;
        let config: NodeConfig =
            toml::from_str(&content).map_err(|e| NodeConfigError::ParseError(display_path, e))?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), NodeConfigError> {
        if self.node.id.trim().is_empty() {
            return Err(NodeConfigError::ValidationError(
                "Node ID must not be empty".to_string(),
            ));
        }
        if self.database.url.trim().is_empty() {
            return Err(NodeConfigError::ValidationError(
                "Database URL must not be empty".to_string(),
            ));
        }
        if self.database.max_pool_size < self.database.min_pool_size {
            return Err(NodeConfigError::ValidationError(
                "database.max_pool_size cannot be smaller than min_pool_size".to_string(),
            ));
        }
        if self.network.port == 0 {
            return Err(NodeConfigError::ValidationError(
                "network.port cannot be 0".to_string(),
            ));
        }
        Ok(())
    }

    pub fn default_template() -> &'static str {
        r#"# =============================================================================
# BOS Server Node Profile (Navision Application Server / NST Equivalent)
# =============================================================================

[node]
id = "node-main-01"
name = "BOS Enterprise Primary Node"
# Operational Roles:
# - "all_in_one": Serves Svelte 5 UI, REST/GraphQL API, and runs background NAS worker
# - "interactive": Serves Svelte 5 UI and API (no background batch worker)
# - "nas_worker": Headless background batch daemon (processes job queue & outbox, no UI)
# - "api_gateway": Headless API-only node optimized for external integrations
role = "all_in_one"

[network]
host = "0.0.0.0"
port = 3000
cors_origins = ["*"]

[ui]
# When true, the node serves compiled Svelte 5 UI assets on the same port
enabled = true
static_dir = "./ui/dist"
index_file = "index.html"

[database]
# Securely isolated on this node - clients never receive or see this connection string
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
min_pool_size = 5
max_pool_size = 30
storage_strategy = "shared_rls" # "shared_rls" | "schema_per_tenant" | "db_per_tenant"
connection_timeout_seconds = 10

[defaults]
# Auto-resolve tenant & company for clients logging in with just User ID & Password
default_tenant = "DEFAULT_TENANT"
default_company = "MAIN_COMPANY"

[nas_worker]
# Equivalent to Navision Application Server (NAS) background batch processor
enabled = true
poll_interval_ms = 1000
batch_size = 50
process_outbox = true
process_job_queue = true

[security]
# Emergency maintenance mode: bypass auth and create emergency super admin session to fix broken .bos scripts
maintenance_mode = false
emergency_username = "emergency.admin"
emergency_name = "Emergency Maintenance Administrator"
emergency_roles = ["SUPER", "Admin", "Global Admin"]
auto_fallback_on_auth_failure = true

[pages]
# Pages are compiled to Svelte 5 modules at node start (only when their content changed) and served from disk
enabled = true
artifact_dir = "./data/page-artifacts"
# runtime_dir = "./ui/dist/bos-runtime"   # svelte-compiler.js + version.json (defaults to <ui.static_dir>/bos-runtime)
compiler_workers = 0                      # 0 = min(CPU cores, 4)
seed_dirs = ["./dist/packages"]           # pre-built artifacts from `bosc package build`
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_template() {
        let template = NodeConfig::default_template();
        let cfg: NodeConfig =
            toml::from_str(template).expect("Default template must parse cleanly");
        assert_eq!(cfg.node.id, "node-main-01");
        assert_eq!(cfg.node.role, NodeRole::AllInOne);
        assert_eq!(cfg.network.port, 3000);
        assert!(cfg.ui.enabled);
        assert_eq!(cfg.ui.static_dir, "./ui/dist");
        assert!(cfg.nas_worker.enabled);
        assert_eq!(cfg.defaults.default_tenant, "DEFAULT_TENANT");
        assert_eq!(cfg.defaults.default_company, "MAIN_COMPANY");
        assert!(!cfg.security.maintenance_mode);
        assert_eq!(cfg.security.emergency_username, "emergency.admin");
        assert!(cfg.security.auto_fallback_on_auth_failure);
        assert!(cfg.pages.enabled);
        assert_eq!(cfg.pages.artifact_dir, "./data/page-artifacts");
        assert_eq!(cfg.pages.compiler_workers, 0);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_node_roles() {
        let roles = [
            ("all_in_one", NodeRole::AllInOne),
            ("interactive", NodeRole::Interactive),
            ("nas_worker", NodeRole::NasWorker),
            ("api_gateway", NodeRole::ApiGateway),
        ];

        for (str_val, expected) in roles {
            let toml_str = format!(
                "[node]\nrole = \"{}\"\n[database]\nurl = \"postgres://test\"\n",
                str_val
            );
            let cfg: NodeConfig = toml::from_str(&toml_str).unwrap();
            assert_eq!(cfg.node.role, expected);
        }
    }

    #[test]
    fn test_validation_empty_id() {
        let mut cfg = NodeConfig::default();
        cfg.node.id = "   ".to_string();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validation_empty_db_url() {
        let mut cfg = NodeConfig::default();
        cfg.database.url = "".to_string();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_pool_sizes() {
        let mut cfg = NodeConfig::default();
        cfg.database.min_pool_size = 50;
        cfg.database.max_pool_size = 10;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validation_port_zero() {
        let mut cfg = NodeConfig::default();
        cfg.network.port = 0;
        assert!(cfg.validate().is_err());
    }
}
