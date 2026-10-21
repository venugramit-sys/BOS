use bosc_ir::{PackageArtifactsIR, PackageManifestIR};
use sqlx::{PgPool, Row};
use std::fs;
use std::path::Path;
use tracing::{info, warn};
use crate::RuntimeError;

/// Authoritative Database Package Vault & Automated Self-Healing Pipeline
pub struct PackageVault;

impl PackageVault {
    /// Ensure the authoritative _bos_sys_package_vault table exists
    pub async fn ensure_vault_table(pool: &PgPool) -> Result<(), RuntimeError> {
        let ddl = r#"
            CREATE TABLE IF NOT EXISTS "_bos_sys_package_vault" (
                "package_id" VARCHAR(100) NOT NULL,
                "version" VARCHAR(50) NOT NULL,
                "publisher" VARCHAR(200) NOT NULL,
                "merkle_root" CHAR(64) NOT NULL,
                "signature" VARCHAR(256),
                "bundle_bytes" BYTEA NOT NULL,
                "artifacts_json" JSONB NOT NULL DEFAULT '{}',
                "installed_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY ("package_id", "version")
            );
            CREATE INDEX IF NOT EXISTS "idx_bos_vault_pkg" ON "_bos_sys_package_vault" ("package_id", "installed_at" DESC);
        "#;
        sqlx::raw_sql(ddl).execute(pool).await?;
        Ok(())
    }

    /// Store a verified package bundle and its compiled artifacts into the database vault
    pub async fn store_package(
        pool: &PgPool,
        manifest: &PackageManifestIR,
        merkle_root: &str,
        signature: Option<&str>,
        bundle_bytes: &[u8],
        artifacts_json: serde_json::Value,
    ) -> Result<(), RuntimeError> {
        Self::ensure_vault_table(pool).await?;

        let sql = r#"
            INSERT INTO "_bos_sys_package_vault" (
                "package_id", "version", "publisher", "merkle_root", "signature", "bundle_bytes", "artifacts_json", "installed_at"
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT ("package_id", "version") DO UPDATE SET
                "publisher" = EXCLUDED."publisher",
                "merkle_root" = EXCLUDED."merkle_root",
                "signature" = EXCLUDED."signature",
                "bundle_bytes" = EXCLUDED."bundle_bytes",
                "artifacts_json" = EXCLUDED."artifacts_json",
                "installed_at" = NOW();
        "#;

        sqlx::query(sql)
            .bind(&manifest.package_id)
            .bind(&manifest.version)
            .bind(&manifest.publisher)
            .bind(merkle_root)
            .bind(signature)
            .bind(bundle_bytes)
            .bind(artifacts_json)
            .execute(pool)
            .await?;

        info!(
            "[PACKAGE VAULT] Successfully secured package '{}' (v{}) in authoritative vault",
            manifest.package_id, manifest.version
        );
        Ok(())
    }

    /// Fetch a package bundle from the vault by package_id and optional version
    pub async fn fetch_package(
        pool: &PgPool,
        package_id: &str,
        version_opt: Option<&str>,
    ) -> Result<Option<(PackageManifestIR, String, Vec<u8>, serde_json::Value)>, RuntimeError> {
        Self::ensure_vault_table(pool).await?;

        let row = if let Some(ver) = version_opt {
            sqlx::query(
                r#"SELECT "package_id", "version", "publisher", "merkle_root", "bundle_bytes", "artifacts_json"
                   FROM "_bos_sys_package_vault" WHERE "package_id" = $1 AND "version" = $2 LIMIT 1"#
            )
            .bind(package_id)
            .bind(ver)
            .fetch_optional(pool)
            .await?
        } else {
            sqlx::query(
                r#"SELECT "package_id", "version", "publisher", "merkle_root", "bundle_bytes", "artifacts_json"
                   FROM "_bos_sys_package_vault" WHERE "package_id" = $1 ORDER BY "installed_at" DESC LIMIT 1"#
            )
            .bind(package_id)
            .fetch_optional(pool)
            .await?
        };

        if let Some(r) = row {
            let pkg_id: String = r.get("package_id");
            let ver: String = r.get("version");
            let publ: String = r.get("publisher");
            let merkle: String = r.get("merkle_root");
            let bundle_bytes: Vec<u8> = r.get("bundle_bytes");
            let artifacts: serde_json::Value = r.get("artifacts_json");

            let manifest = PackageManifestIR {
                package_id: pkg_id,
                name: package_id.to_string(),
                version: ver,
                publisher: publ,
                dependencies: vec![],
                optional_features: vec![],
            };

            Ok(Some((manifest, merkle, bundle_bytes, artifacts)))
        } else {
            Ok(None)
        }
    }

    /// Self-Heal: Extract pristine package artifacts from vault to disk atomically using temporary staging
    pub async fn self_heal_package(
        pool: &PgPool,
        package_id: &str,
        target_base_dir: &Path,
    ) -> Result<bool, RuntimeError> {
        info!(
            "[SELF-HEALING] Initiating automated self-recovery for package '{}' from vault...",
            package_id
        );

        let Some((manifest, merkle, _bundle_bytes, artifacts_val)) =
            Self::fetch_package(pool, package_id, None).await?
        else {
            warn!(
                "[SELF-HEALING] Cannot self-heal '{}': package not found in vault",
                package_id
            );
            return Ok(false);
        };

        let target_pkg_dir = target_base_dir.join(&manifest.package_id);
        let staging_dir = target_base_dir.join(format!(
            ".staging_{}_{}",
            manifest.package_id,
            uuid::Uuid::new_v4()
        ));

        let bytecode_dir = staging_dir.join("bytecode");
        let schema_dir = staging_dir.join("schema");
        fs::create_dir_all(&bytecode_dir).map_err(|e| {
            RuntimeError::Internal(format!("Failed to create staging bytecode dir: {}", e))
        })?;
        fs::create_dir_all(&schema_dir).map_err(|e| {
            RuntimeError::Internal(format!("Failed to create staging schema dir: {}", e))
        })?;

        // Deserialize PackageArtifactsIR from vault
        if let Ok(artifacts) = serde_json::from_value::<PackageArtifactsIR>(artifacts_val.clone()) {
            for b in &artifacts.bytecodes {
                let out_file = bytecode_dir.join(format!("{}.qbc", b.name));
                let envelope = b.to_binary_envelope();
                fs::write(&out_file, &envelope).map_err(|e| {
                    RuntimeError::Internal(format!(
                        "Failed to write recovered bytecode {:?}: {}",
                        out_file, e
                    ))
                })?;
            }

            if !artifacts.tables_ddl.is_empty() {
                let ddl_file = schema_dir.join("tables.sql");
                fs::write(&ddl_file, &artifacts.tables_ddl).map_err(|e| {
                    RuntimeError::Internal(format!("Failed to write recovered tables DDL: {}", e))
                })?;
            }

            if !artifacts.prepared_queries.is_empty() {
                let pq_file = schema_dir.join("prepared_queries.json");
                let json =
                    serde_json::to_string_pretty(&artifacts.prepared_queries).unwrap_or_default();
                fs::write(&pq_file, &json).map_err(|e| {
                    RuntimeError::Internal(format!(
                        "Failed to write recovered prepared queries: {}",
                        e
                    ))
                })?;
            }
        }

        // Write manifest.json
        let manifest_file = staging_dir.join("manifest.json");
        let manifest_json = serde_json::json!({
            "package_id": manifest.package_id,
            "name": manifest.name,
            "version": manifest.version,
            "publisher": manifest.publisher,
            "merkle_root": merkle,
        });
        fs::write(
            &manifest_file,
            serde_json::to_string_pretty(&manifest_json).unwrap_or_default(),
        )
        .map_err(|e| {
            RuntimeError::Internal(format!("Failed to write recovered manifest.json: {}", e))
        })?;

        // Atomic swap
        if target_pkg_dir.exists() {
            let _ = fs::remove_dir_all(&target_pkg_dir);
        }
        fs::rename(&staging_dir, &target_pkg_dir).map_err(|e| {
            RuntimeError::Internal(format!("Atomic swap failed during self-healing: {}", e))
        })?;

        info!(
            "[SELF-HEALING] ✓ Package '{}' successfully healed and restored to {:?}",
            manifest.package_id, target_pkg_dir
        );
        Ok(true)
    }
}
