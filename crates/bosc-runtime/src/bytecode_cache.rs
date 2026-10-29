use bosc_ir::BytecodeArtifactIR;
use dashmap::DashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};
use crate::RuntimeError;

/// High-Performance In-Memory AOT Bytecode Cache
#[derive(Clone, Default)]
pub struct BytecodeCache {
    /// In-memory cache of compiled bytecode artifacts keyed by object name (lowercase)
    artifacts: Arc<DashMap<String, BytecodeArtifactIR>>,
}

impl BytecodeCache {
    pub fn new() -> Self {
        Self {
            artifacts: Arc::new(DashMap::new()),
        }
    }

    /// Register a verified bytecode artifact in memory
    pub fn register(&self, artifact: BytecodeArtifactIR) {
        let key = artifact.name.to_ascii_lowercase();
        self.artifacts.insert(key, artifact);
    }

    /// Lookup a bytecode artifact by name
    pub fn get(&self, name: &str) -> Option<BytecodeArtifactIR> {
        let key = name.to_ascii_lowercase();
        self.artifacts.get(&key).map(|v| v.value().clone())
    }

    /// Check if a bytecode artifact exists in cache
    pub fn contains(&self, name: &str) -> bool {
        self.artifacts.contains_key(&name.to_ascii_lowercase())
    }

    /// Number of loaded bytecode artifacts
    pub fn count(&self) -> usize {
        self.artifacts.len()
    }

    /// Scan directory, load all .qbc files, verify BCBOS magic header & CRC32, and register into memory
    pub fn load_from_dir(&self, dir: &Path) -> Result<usize, RuntimeError> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut loaded_count = 0;
        let entries = fs::read_dir(dir).map_err(|e| {
            RuntimeError::Internal(format!("Failed to read bytecode directory {:?}: {}", dir, e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("qbc") {
                match self.load_file(&path) {
                    Ok(artifact) => {
                        info!(
                            "[BYTECODE CACHE] Loaded verified AOT bytecode: {} (type: {}, crc32: 0x{:08x})",
                            artifact.name, artifact.object_type, artifact.crc32
                        );
                        self.register(artifact);
                        loaded_count += 1;
                    }
                    Err(e) => {
                        warn!(
                            "[BYTECODE CACHE] Failed to load bytecode file {:?}: {}",
                            path, e
                        );
                    }
                }
            } else if path.is_dir() {
                // Recursively scan subdirectories (e.g. dist/packages/<pkg>/bytecode/)
                if let Ok(count) = self.load_from_dir(&path) {
                    loaded_count += count;
                }
            }
        }

        Ok(loaded_count)
    }

    /// Load and verify a single .qbc file from disk
    pub fn load_file(&self, path: &Path) -> Result<BytecodeArtifactIR, RuntimeError> {
        let data = fs::read(path).map_err(|e| {
            RuntimeError::Internal(format!("Failed to read file {:?}: {}", path, e))
        })?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let artifact = BytecodeArtifactIR::from_binary_envelope(name, "unknown", "", &data)
            .map_err(|e| {
                RuntimeError::Internal(format!(
                    "Corrupted or invalid bytecode envelope in {:?}: {}",
                    path, e
                ))
            })?;

        Ok(artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_real_bytecode_from_dir() {
        let cache = BytecodeCache::new();
        let path = std::path::Path::new("../../dist/packages/bos.core/bytecode");
        let alt_path = std::path::Path::new("dist/packages/bos.core/bytecode");
        let target = if path.exists() { path } else { alt_path };
        if target.exists() {
            let loaded = cache.load_from_dir(target).expect("load from dir");
            assert!(loaded >= 3, "Must load at least 3 bytecode files");
            assert!(cache.contains("AuthManager"), "Must contain AuthManager");
            assert!(cache.contains("UserLogic"), "Must contain UserLogic");
            assert!(cache.contains("package.bundle"), "Must contain package.bundle");
            let art = cache.get("AuthManager").expect("AuthManager artifact");
            assert!(art.verify_integrity(), "CRC32 integrity must be verified");
        }
    }
}
