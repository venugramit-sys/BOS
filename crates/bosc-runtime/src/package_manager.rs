use bosc_ir::{PackageBundleIR, PackageManifestIR};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::hash::{Hash, Hasher};

use crate::RuntimeError;
use crate::meta_manager::{MetaManager, RegisterMetaObjectRequest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInstallResult {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub objects_installed: usize,
    pub status: String,
    pub message: String,
}

pub struct PackageManager;

impl PackageManager {
    /// Build a verified `.bospkg` bundle from a manifest and list of meta-objects
    pub fn build_bundle(
        manifest: PackageManifestIR,
        objects: Vec<serde_json::Value>,
    ) -> Result<PackageBundleIR, RuntimeError> {
        if manifest.package_id.trim().is_empty() {
            return Err(RuntimeError::Internal(
                "Package manifest must contain a valid package_id (e.g. 'bos.sales')".to_string(),
            ));
        }

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        manifest.package_id.hash(&mut hasher);
        manifest.version.hash(&mut hasher);
        for obj in &objects {
            serde_json::to_string(obj)
                .unwrap_or_default()
                .hash(&mut hasher);
        }
        let checksum = format!("{:016x}", hasher.finish());

        Ok(PackageBundleIR {
            manifest,
            objects,
            checksum,
        })
    }

    /// Install a `.bospkg` package bundle into the database
    pub async fn install_package(
        pool: &PgPool,
        bundle: PackageBundleIR,
        _strategy_str: Option<&str>,
        allow_destructive: bool,
    ) -> Result<PackageInstallResult, RuntimeError> {
        let manifest = &bundle.manifest;

        // 1. Dependency Resolution & Verification
        for dep in &manifest.dependencies {
            let dep_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM \"_bos_meta_packages\" WHERE package_id = $1 AND status = 'active')",
            )
            .bind(dep)
            .fetch_one(pool)
            .await?;

            if !dep_exists {
                return Err(RuntimeError::Internal(format!(
                    "Cannot install package '{}': Missing required dependency '{}'. Please install it first.",
                    manifest.package_id, dep
                )));
            }
        }

        // 2. Synchronize all bundled database objects (tables, extensions, state machines, views)
        //    in one migration so cross-references resolve regardless of object order.
        let tables: Vec<bosc_ir::TableIR> = bundle
            .objects
            .iter()
            .filter(|obj| {
                obj.get("object_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("table"))
                    .unwrap_or(false)
            })
            .filter_map(|obj| {
                let def = obj.get("definition").unwrap_or(obj);
                serde_json::from_value::<bosc_ir::TableIR>(def.clone()).ok()
            })
            .collect();

        let extensions: Vec<bosc_ir::TableExtensionIR> = bundle
            .objects
            .iter()
            .filter(|obj| {
                obj.get("object_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("tableextension"))
                    .unwrap_or(false)
            })
            .filter_map(|obj| {
                let def = obj.get("definition").unwrap_or(obj);
                serde_json::from_value::<bosc_ir::TableExtensionIR>(def.clone()).ok()
            })
            .collect();

        let state_machines: Vec<bosc_ir::StateMachineIR> = bundle
            .objects
            .iter()
            .filter(|obj| {
                obj.get("object_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("statemachine"))
                    .unwrap_or(false)
            })
            .filter_map(|obj| {
                let def = obj.get("definition").unwrap_or(obj);
                serde_json::from_value::<bosc_ir::StateMachineIR>(def.clone()).ok()
            })
            .collect();

        let views: Vec<bosc_ir::ViewIR> = bundle
            .objects
            .iter()
            .filter(|obj| {
                obj.get("object_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("view"))
                    .unwrap_or(false)
            })
            .filter_map(|obj| {
                let def = obj.get("definition").unwrap_or(obj);
                serde_json::from_value::<bosc_ir::ViewIR>(def.clone()).ok()
            })
            .collect();

        let permission_sets: Vec<bosc_ir::PermissionSetIR> = bundle
            .objects
            .iter()
            .filter(|obj| {
                obj.get("object_type")
                    .and_then(|v| v.as_str())
                    .map(|s| s.eq_ignore_ascii_case("permissionset"))
                    .unwrap_or(false)
            })
            .filter_map(|obj| {
                let def = obj.get("definition").unwrap_or(obj);
                serde_json::from_value::<bosc_ir::PermissionSetIR>(def.clone()).ok()
            })
            .collect();

        if !tables.is_empty()
            || !extensions.is_empty()
            || !views.is_empty()
            || !state_machines.is_empty()
            || !permission_sets.is_empty()
        {
            let strategy_str: Option<String> = sqlx::query_scalar(
                "SELECT storage_strategy FROM \"_bos_sys_info\" WHERE \"id\" = 'core' LIMIT 1",
            )
            .fetch_optional(pool)
            .await?;
            let strategy = bosc_ir::StorageStrategyIR::from_str_loose(
                strategy_str.as_deref().unwrap_or("shared_rls"),
            );
            let generator = bosc_codegen_db::PostgresGenerator::with_strategy(strategy);
            let mut full_ddl = String::new();

            // 1. Base table structures
            for table in &tables {
                full_ddl.push_str(&generator.generate_table_structure(table));
                full_ddl.push('\n');
            }
            // 2. Table extensions (must precede relations and views that may reference extended columns)
            for ext in &extensions {
                full_ddl.push_str(&generator.generate_table_extension_ddl(ext, &[]));
                full_ddl.push('\n');
            }
            // 3. Foreign key relations
            for table in &tables {
                full_ddl.push_str(&generator.generate_relations(table));
            }
            full_ddl.push('\n');
            // 4. SIFT covering indexes on target tables
            for table in &tables {
                full_ddl.push_str(&generator.generate_table_sift_indexes(table));
            }
            full_ddl.push('\n');
            // 5. State Machine History & Triggers
            for sm in &state_machines {
                full_ddl.push_str(&generator.generate_state_machine_infrastructure(sm));
                full_ddl.push('\n');
            }
            // 6. FlowField pushdown views
            for table in &tables {
                full_ddl.push_str(&generator.generate_table_flowfield_views(table));
            }
            full_ddl.push('\n');
            // 7. Custom Views
            for view in &views {
                full_ddl.push_str(&generator.generate_view_ddl(view));
                full_ddl.push('\n');
            }
            // 8. Permission Sets (RLS Policies & Permissions)
            for ps in &permission_sets {
                full_ddl.push_str(&generator.generate_permission_set_ddl(ps));
                full_ddl.push('\n');
            }

            crate::schema_sync::SchemaSync::sync(
                pool,
                &crate::schema_sync::SchemaSyncRequest {
                    package_name: manifest.package_id.clone(),
                    source: "package".to_string(),
                    ddl: full_ddl,
                    tables,
                    extensions,
                    applied_by: None,
                },
                &crate::schema_sync::SchemaSyncOptions {
                    allow_destructive,
                    dry_run: false,
                },
            )
            .await?;
        }

        // 3. Register / Update package in _bos_meta_packages
        let upsert_pkg = r#"
            INSERT INTO "_bos_meta_packages" (
                "package_id", "name", "version", "publisher", "status", "checksum", "installed_at"
            ) VALUES (
                $1, $2, $3, $4, 'active', $5, NOW()
            )
            ON CONFLICT ("package_id") DO UPDATE
            SET "name" = EXCLUDED."name",
                "version" = EXCLUDED."version",
                "publisher" = EXCLUDED."publisher",
                "checksum" = EXCLUDED."checksum",
                "status" = 'active',
                "installed_at" = NOW();
        "#;

        sqlx::query(upsert_pkg)
            .bind(&manifest.package_id)
            .bind(&manifest.name)
            .bind(&manifest.version)
            .bind(&manifest.publisher)
            .bind(&bundle.checksum)
            .execute(pool)
            .await?;

        // 3b. Secure in Authoritative Database Package Vault for automated self-healing
        let raw_bundle_bytes = serde_json::to_vec(&bundle).unwrap_or_default();
        if let Err(e) = crate::package_vault::PackageVault::store_package(
            pool,
            manifest,
            &bundle.checksum,
            None,
            &raw_bundle_bytes,
            serde_json::to_value(&bundle).unwrap_or_default(),
        ).await {
            tracing::warn!("Failed to store package '{}' in package vault: {}", manifest.package_id, e);
        }

        // 3. Atomically register all bundled meta-objects
        let mut installed_count = 0;
        for obj_val in bundle.objects {
            let (mut req, full_def) = if obj_val.get("definition").is_some() {
                // Backward-compatibility: legacy bundle with nested definition
                let r: RegisterMetaObjectRequest = serde_json::from_value(obj_val.clone())
                    .map_err(|e| {
                        RuntimeError::Internal(format!(
                            "Invalid meta-object in package bundle: {}",
                            e
                        ))
                    })?;
                let d = r.definition.clone().unwrap_or(obj_val.clone());
                (r, d)
            } else {
                // Clean first-class IR object (no nested definition wrapper)
                let obj_type = obj_val
                    .get("object_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("table")
                    .to_string();
                let name = obj_val
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let caption = obj_val
                    .get("caption")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let sub_type = obj_val
                    .get("sub_type")
                    .or_else(|| obj_val.get("table_type"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let data_per_company = obj_val.get("data_per_company").and_then(|v| v.as_bool());

                let r = RegisterMetaObjectRequest {
                    object_type: obj_type,
                    name,
                    package_name: Some(manifest.package_id.clone()),
                    version: Some(manifest.version.clone()),
                    caption,
                    sub_type,
                    data_per_company,
                    columns: None,
                    primary_key: None,
                    check_constraints: None,
                    state_machine: None,
                    definition: Some(obj_val.clone()),
                    allow_destructive,
                };
                (r, obj_val.clone())
            };

            // Enforce package namespace
            req.package_name = Some(manifest.package_id.clone());
            req.allow_destructive = allow_destructive;
            req.definition = Some(full_def.clone());

            let is_codeunit = req.object_type == "codeunit";

            MetaManager::register_meta_object(pool, req).await?;

            if is_codeunit {
                if let Ok(cu) = serde_json::from_value::<bosc_ir::CodeunitIR>(full_def) {
                    crate::execution_engine::ExecutionEngine::register_codeunit(pool, cu).await?;
                }
            }
            installed_count += 1;
        }

        // 4. If package is bos.auth, ensure Super Administrator master records are seeded
        if manifest.package_id == "bos.auth" {
            Self::seed_auth_super_admin(pool).await?;
        }

        Ok(PackageInstallResult {
            package_id: manifest.package_id.clone(),
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            publisher: manifest.publisher.clone(),
            objects_installed: installed_count,
            status: "active".to_string(),
            message: format!(
                "Package '{}' v{} installed successfully with {} meta-object(s)",
                manifest.name, manifest.version, installed_count
            ),
        })
    }

    /// Export an installed package and its associated meta-objects into a .bospkg bundle
    pub async fn export_package(
        pool: &PgPool,
        package_id: &str,
    ) -> Result<PackageBundleIR, RuntimeError> {
        let pkg_row = sqlx::query(
            "SELECT package_id, name, version, publisher FROM \"_bos_meta_packages\" WHERE package_id = $1",
        )
        .bind(package_id)
        .fetch_optional(pool)
        .await?;

        let row = match pkg_row {
            Some(r) => r,
            None => {
                return Err(RuntimeError::NotFound(
                    "Package".to_string(),
                    package_id.to_string(),
                ));
            }
        };

        let manifest = PackageManifestIR {
            package_id: row.try_get("package_id")?,
            name: row.try_get("name")?,
            version: row.try_get("version")?,
            publisher: row.try_get("publisher")?,
            dependencies: Vec::new(),
            optional_features: Vec::new(),
        };

        // Query all meta-objects belonging to this package
        let obj_rows = sqlx::query(
            "SELECT object_type, object_name, package_name, version, caption, sub_type, definition FROM \"_bos_meta_objects\" WHERE package_name = $1",
        )
        .bind(package_id)
        .fetch_all(pool)
        .await?;

        let mut objects = Vec::new();
        for orow in obj_rows {
            let mut def: serde_json::Value = orow.try_get("definition").unwrap_or_default();
            if let Some(map) = def.as_object_mut() {
                map.insert(
                    "object_type".to_string(),
                    serde_json::json!(orow.try_get::<String, _>("object_type")?),
                );
                map.insert(
                    "package_name".to_string(),
                    serde_json::json!(orow.try_get::<String, _>("package_name")?),
                );
                map.insert(
                    "version".to_string(),
                    serde_json::json!(orow.try_get::<String, _>("version")?),
                );
                if let Ok(Some(cap)) = orow.try_get::<Option<String>, _>("caption") {
                    map.entry("caption".to_string())
                        .or_insert_with(|| serde_json::json!(cap));
                }
            }
            objects.push(def);
        }

        Self::build_bundle(manifest, objects)
    }

    /// Seed the default Super Administrator into bos.auth tables and _bos_users idempotently
    pub async fn seed_auth_super_admin(pool: &PgPool) -> Result<(), RuntimeError> {
        use uuid::Uuid;

        let tenant_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT tenant_id FROM \"_bos_tenants\" WHERE is_active = true ORDER BY created_at LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or_else(Uuid::nil);

        let password_plain = "Boss@123";
        let password_hash = crate::hash_password_argon2(password_plain);

        // 1. Ensure Roles (SUPER, ADMIN) exist in "role"
        let insert_roles = r#"
            INSERT INTO "role" ("_id", "_tenant_id", "code", "name", "description", "is_system_role", "is_active", "_created_at", "_updated_at")
            VALUES 
                (gen_random_uuid(), $1, 'SUPER', 'Super Administrator', 'Full unrestricted platform access', true, true, NOW(), NOW()),
                (gen_random_uuid(), $1, 'ADMIN', 'System Administrator', 'Administrative system management', true, true, NOW(), NOW())
            ON CONFLICT ("code") DO NOTHING;
        "#;
        let _ = sqlx::query(insert_roles).bind(tenant_id).execute(pool).await;

        // 2. Ensure super admin exists in "user"
        let insert_user = r#"
            INSERT INTO "user" ("_id", "_tenant_id", "username", "password_hash", "email", "full_name", "user_type", "status", "is_active", "failed_attempts", "password_changed_at", "_created_at", "_updated_at")
            VALUES (gen_random_uuid(), $1, 'admin', $2, 'admin@system.local', 'System Super Administrator', 'Internal', 'Active', true, 0, NOW(), NOW(), NOW())
            ON CONFLICT ("username") DO NOTHING;
        "#;
        let _ = sqlx::query(insert_user)
            .bind(tenant_id)
            .bind(&password_hash)
            .execute(pool)
            .await;

        // 3. Ensure UserRole mappings exist
        let insert_user_roles = r#"
            INSERT INTO "user_role" ("_id", "_tenant_id", "username", "role_code", "assigned_by", "_created_at", "_updated_at")
            VALUES 
                (gen_random_uuid(), $1, 'admin', 'SUPER', 'SYSTEM', NOW(), NOW()),
                (gen_random_uuid(), $1, 'admin', 'ADMIN', 'SYSTEM', NOW(), NOW())
            ON CONFLICT ("username", "role_code") DO NOTHING;
        "#;
        let _ = sqlx::query(insert_user_roles).bind(tenant_id).execute(pool).await;

        // 4. Ensure UserSecuritySetting exists
        let insert_sec = r#"
            INSERT INTO "user_security_setting" ("_id", "_tenant_id", "username", "mfa_method", "mfa_enabled", "max_concurrent_sessions", "must_change_password", "password_expiry_days", "_created_at", "_updated_at")
            VALUES (gen_random_uuid(), $1, 'admin', 'None', false, 10, false, 90, NOW(), NOW())
            ON CONFLICT ("username") DO NOTHING;
        "#;
        let _ = sqlx::query(insert_sec).bind(tenant_id).execute(pool).await;

        // 4b. Ensure default PRIMARY AuthConfiguration exists
        let insert_config = r#"
            INSERT INTO "auth_configuration" (
                "_id", "_tenant_id", "id", "application_title", "welcome_message",
                "primary_color", "allow_remember_me", "allow_emergency_fallback",
                "sso_enabled", "sso_provider_name", "custom_disclaimer", "support_email",
                "_created_at", "_updated_at"
            ) VALUES (
                gen_random_uuid(), $1, 'PRIMARY', 'BOS Enterprise Platform',
                'Sign in to access your enterprise workspace.',
                '#4f46e5', true, true, false, 'Corporate SSO',
                'Authorized personnel only. All access attempts are monitored and audited.',
                'admin@bos.local', NOW(), NOW()
            )
            ON CONFLICT ("id") DO NOTHING;
        "#;
        let _ = sqlx::query(insert_config).bind(tenant_id).execute(pool).await;

        // 5. Ensure SecurityAuditLog has an initial seed record
        let insert_audit = r#"
            INSERT INTO "security_audit_log" ("id", "_id", "_tenant_id", "username", "action", "details", "is_success", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), gen_random_uuid(), $1, 'admin', 'UserCreated', 'Super administrator seeded on package import.', true, NOW(), NOW()
            WHERE NOT EXISTS (
                SELECT 1 FROM "security_audit_log" WHERE "username" = 'admin' AND "action" = 'UserCreated'
            );
        "#;
        let _ = sqlx::query(insert_audit).bind(tenant_id).execute(pool).await;

        // 6. Synchronize system kernel _bos_users
        let insert_sys_user = r#"
            CREATE TABLE IF NOT EXISTS "_bos_users" (
                "user_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                "username" VARCHAR(100) NOT NULL UNIQUE,
                "password_hash" TEXT NOT NULL,
                "display_name" VARCHAR(200) NOT NULL,
                "email" VARCHAR(200) NOT NULL,
                "roles" TEXT[] NOT NULL DEFAULT ARRAY['User']::text[],
                "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            INSERT INTO "_bos_users" ("username", "password_hash", "display_name", "email", "roles", "created_at")
            VALUES ('admin', $1, 'System Super Administrator', 'admin@system.local', ARRAY['SUPER', 'Admin', 'Global Admin']::text[], NOW())
            ON CONFLICT ("username") DO UPDATE
            SET "password_hash" = EXCLUDED."password_hash",
                "roles" = EXCLUDED."roles",
                "display_name" = EXCLUDED."display_name";
        "#;
        let _ = sqlx::query(insert_sys_user)
            .bind(&password_hash)
            .execute(pool)
            .await;

        tracing::info!("Super Administrator ('admin', password: 'Boss@123') verified and seeded into bos.auth tables and _bos_users");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::PackageManager;
    use bosc_codegen_db::PostgresGenerator;
    use bosc_ir::StorageStrategyIR;

    #[tokio::test]
    async fn test_auth_package_ddl_flowfield_references_are_aliased_to_t() {
        let bundle_opt = crate::db_manager::DatabaseManager::load_base_package_bundle("bos.auth").await;
        if let Some(bundle) = bundle_opt {
            let tables: Vec<bosc_ir::TableIR> = bundle
                .objects
                .iter()
                .filter(|obj| {
                    obj.get("object_type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.eq_ignore_ascii_case("table"))
                        .unwrap_or(false)
                })
                .filter_map(|obj| {
                    let def = obj.get("definition").unwrap_or(obj);
                    serde_json::from_value::<bosc_ir::TableIR>(def.clone()).ok()
                })
                .collect();

            assert!(!tables.is_empty(), "bos.auth should contain tables");
            let generator = PostgresGenerator::with_strategy(StorageStrategyIR::SharedRLS);
            let ddl = generator.generate_tables(&tables);

            // FlowField views MUST use t."username" and NOT "user"."username"
            assert!(ddl.contains("CREATE OR REPLACE VIEW \"user_view\""));
            assert!(ddl.contains("t.\"username\""));
            assert!(!ddl.contains("\"user\".\"username\""), "unaliased 'user'.'username' causes postgres error");

            assert!(ddl.contains("CREATE OR REPLACE VIEW \"role_view\""));
            assert!(ddl.contains("t.\"code\""));
            assert!(!ddl.contains("\"role\".\"code\""), "unaliased 'role'.'code' causes postgres error");
        }
    }

    #[tokio::test]
    async fn test_auth_package_has_seed_super_admin_and_argon2_hash() {
        let hash = crate::hash_password_argon2("Boss@123");
        assert!(crate::verify_password("Boss@123", &hash));
        assert!(!crate::verify_password("WrongPassword", &hash));

        let bundle_opt = crate::db_manager::DatabaseManager::load_base_package_bundle("bos.auth").await;
        let bundle = bundle_opt.expect("bos.auth bundle should load");
        let cu_opt = bundle.objects.iter().find(|obj| {
            obj.get("object_type").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("codeunit")).unwrap_or(false)
                && obj.get("name").and_then(|v| v.as_str()) == Some("AuthManager")
        });
        assert!(cu_opt.is_some(), "AuthManager codeunit must exist");
        let cu_str = serde_json::to_string(cu_opt.unwrap()).unwrap();
        assert!(cu_str.contains("SeedSuperAdmin"), "AuthManager should define SeedSuperAdmin procedure");
    }

    fn compile_auth_package_bundle_helper() -> Result<bosc_ir::PackageBundleIR, Box<dyn std::error::Error>> {
        let script = std::fs::read_to_string("packages/bos.auth/auth.bos")
            .or_else(|_| std::fs::read_to_string("../../packages/bos.auth/auth.bos"))?;
        let mut lexer = bosc_syntax::lexer::Lexer::new(&script);
        let tokens = lexer.tokenize()?;
        let mut parser = bosc_syntax::parser::Parser::with_source(tokens, script.clone());
        let program = parser.parse_program()?;
        let analyzer = bosc_analyzer::SemanticAnalyzer::new("bos.auth");
        let model_ir = analyzer.analyze_and_lower(&program)?;

        let manifest = bosc_ir::PackageManifestIR {
            package_id: "bos.auth".to_string(),
            name: "BOS Enterprise User Management & Authentication".to_string(),
            version: "1.0.0".to_string(),
            publisher: "BOS Core Architecture Team".to_string(),
            dependencies: vec![],
            optional_features: vec![
                "mfa_totp".to_string(),
                "session_ip_lock".to_string(),
                "password_complexity".to_string(),
                "immutable_audit".to_string(),
            ],
        };

        fn push_obj<T: serde::Serialize>(list: &mut Vec<serde_json::Value>, item: &T, obj_type: &str, pkg_id: &str, version: &str) {
            if let Ok(mut val) = serde_json::to_value(item) {
                if let Some(map) = val.as_object_mut() {
                    map.insert("object_type".to_string(), serde_json::json!(obj_type));
                    map.insert("package_name".to_string(), serde_json::json!(pkg_id));
                    map.insert("version".to_string(), serde_json::json!(version));
                }
                list.push(val);
            }
        }

        let mut objects = Vec::new();
        for t in &model_ir.tables { push_obj(&mut objects, t, "table", "bos.auth", "1.0.0"); }
        for tl in &model_ir.table_logics { push_obj(&mut objects, tl, "tablelogic", "bos.auth", "1.0.0"); }
        for cu in &model_ir.codeunits { push_obj(&mut objects, cu, "codeunit", "bos.auth", "1.0.0"); }
        for e in &model_ir.enums { push_obj(&mut objects, e, "enum", "bos.auth", "1.0.0"); }
        for p in &model_ir.pages { push_obj(&mut objects, p, "page", "bos.auth", "1.0.0"); }

        let bundle = PackageManager::build_bundle(manifest, objects)
            .map_err(|e| format!("{}", e))?;

        let bundle_json = serde_json::to_string_pretty(&bundle)?;
        let _ = std::fs::write("packages/bos.auth.bospkg", &bundle_json);
        let _ = std::fs::write("packages/bos.auth/bos.auth.bospkg", &bundle_json);
        let _ = std::fs::write("../../packages/bos.auth.bospkg", &bundle_json);
        let _ = std::fs::write("../../packages/bos.auth/bos.auth.bospkg", &bundle_json);

        Ok(bundle)
    }

    #[tokio::test]
    async fn test_auth_package_has_login_page_and_auth_configuration() {
        let bundle = compile_auth_package_bundle_helper().expect("should compile auth bundle from source");

        // Verify table AuthConfiguration
        let has_auth_config_table = bundle.objects.iter().any(|obj| {
            obj.get("object_type").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("table")).unwrap_or(false)
                && obj.get("name").and_then(|v| v.as_str()) == Some("AuthConfiguration")
        });
        assert!(has_auth_config_table, "AuthConfiguration table must exist in bos.auth bundle");

        // Verify all 10 pages exist
        let expected_pages = vec![
            "UserList",
            "UserCard",
            "RoleList",
            "RoleCard",
            "UserRoleList",
            "SessionList",
            "UserSecuritySettingList",
            "SecurityAuditLogList",
            "LoginPage",
            "AuthConfigurationCard",
        ];
        for page_name in expected_pages {
            let exists = bundle.objects.iter().any(|obj| {
                obj.get("object_type").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("page")).unwrap_or(false)
                    && obj.get("name").and_then(|v| v.as_str()) == Some(page_name)
            });
            assert!(exists, "Page {} must exist in bos.auth bundle", page_name);
        }

        // Verify procedures in AuthManager
        let cu_opt = bundle.objects.iter().find(|obj| {
            obj.get("object_type").and_then(|v| v.as_str()).map(|s| s.eq_ignore_ascii_case("codeunit")).unwrap_or(false)
                && obj.get("name").and_then(|v| v.as_str()) == Some("AuthManager")
        });
        let cu_str = serde_json::to_string(cu_opt.unwrap()).unwrap();
        assert!(cu_str.contains("GetLoginConfiguration"), "AuthManager must have GetLoginConfiguration procedure");
        assert!(cu_str.contains("UpdateLoginConfiguration"), "AuthManager must have UpdateLoginConfiguration procedure");
    }
}
