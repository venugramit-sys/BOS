use bosc_ir::AotPreparedStatementIR;
use dashmap::DashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use crate::RuntimeError;

/// High-Performance In-Memory AOT Prepared SQL Statement Registry
#[derive(Clone, Default)]
pub struct PreparedStatementRegistry {
    /// In-memory cache of AOT prepared SQL statements keyed by query name (lowercase)
    statements: Arc<DashMap<String, AotPreparedStatementIR>>,
}

impl PreparedStatementRegistry {
    pub fn new() -> Self {
        Self {
            statements: Arc::new(DashMap::new()),
        }
    }

    /// Register a verified AOT prepared statement
    pub fn register(&self, stmt: AotPreparedStatementIR) {
        let key = stmt.name.to_ascii_lowercase();
        self.statements.insert(key, stmt);
    }

    /// Lookup a prepared statement by exact name
    pub fn get(&self, name: &str) -> Option<AotPreparedStatementIR> {
        let key = name.to_ascii_lowercase();
        self.statements.get(&key).map(|v| v.value().clone())
    }

    /// Lookup a prepared statement by table name and kind (e.g. "User", "get_by_id" or "insert")
    pub fn get_table_kind(&self, table: &str, kind: &str) -> Option<AotPreparedStatementIR> {
        let direct_key = format!("{}_{}", table, kind).to_ascii_lowercase();
        if let Some(stmt) = self.statements.get(&direct_key) {
            return Some(stmt.value().clone());
        }

        // Try PascalCase / snake_case variants
        let table_clean = table.replace(['"', '_', ' '], "");
        let kind_clean = kind.replace(['"', '_', ' '], "");
        for item in self.statements.iter() {
            let item_tbl = item.table.replace(['"', '_', ' '], "");
            let item_kind = item.kind.replace(['"', '_', ' '], "");
            if item_tbl.eq_ignore_ascii_case(&table_clean) && item_kind.eq_ignore_ascii_case(&kind_clean) {
                return Some(item.value().clone());
            }
        }
        None
    }

    /// Check if a prepared statement exists in registry
    pub fn contains(&self, name: &str) -> bool {
        self.statements.contains_key(&name.to_ascii_lowercase())
    }

    /// Total count of loaded prepared statements
    pub fn count(&self) -> usize {
        self.statements.len()
    }

    /// Scan directory, load all prepared_queries.json files, and register statements into memory
    pub fn load_from_dir(&self, dir: &Path) -> Result<usize, RuntimeError> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut loaded_count = 0;
        let entries = fs::read_dir(dir).map_err(|e| {
            RuntimeError::Internal(format!("Failed to read prepared queries directory {:?}: {}", dir, e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name().and_then(|s| s.to_str()) == Some("prepared_queries.json") {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<Vec<AotPreparedStatementIR>>(&content) {
                        Ok(stmts) => {
                            for stmt in stmts {
                                info!(
                                    "[PREPARED STMTS] Registered AOT query: {} (table: {}, kind: {})",
                                    stmt.name, stmt.table, stmt.kind
                                );
                                self.register(stmt);
                                loaded_count += 1;
                            }
                        }
                        Err(e) => {
                            warn!("[PREPARED STMTS] Failed to parse {:?}: {}", path, e);
                        }
                    },
                    Err(e) => {
                        warn!("[PREPARED STMTS] Failed to read {:?}: {}", path, e);
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories (e.g. dist/packages/<pkg>/schema/)
                if let Ok(count) = self.load_from_dir(&path) {
                    loaded_count += count;
                }
            }
        }

        Ok(loaded_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let registry = PreparedStatementRegistry::new();
        let stmt = AotPreparedStatementIR {
            name: "Customer_GetById".to_string(),
            table: "Customer".to_string(),
            kind: "get_by_id".to_string(),
            sql: "SELECT * FROM \"customer\" WHERE \"id\" = $1;".to_string(),
            param_names: vec!["id".to_string()],
        };
        registry.register(stmt);
        assert_eq!(registry.count(), 1);
        assert!(registry.contains("Customer_GetById"));

        let found = registry.get_table_kind("Customer", "get_by_id").expect("found");
        assert_eq!(found.name, "Customer_GetById");
        assert_eq!(found.sql, "SELECT * FROM \"customer\" WHERE \"id\" = $1;");
    }
}
