use bosc_ir::*;

pub mod migration;
pub mod pushdown;
pub mod relations;

pub use relations::{relation_constraint_name, SCHEMA_SUPPORT_DDL};

pub struct PostgresGenerator {
    pub strategy: StorageStrategyIR,
}

impl Default for PostgresGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresGenerator {
    pub fn new() -> Self {
        Self {
            strategy: StorageStrategyIR::SharedRLS,
        }
    }

    pub fn with_strategy(strategy: StorageStrategyIR) -> Self {
        Self { strategy }
    }

    /// Formats column DDL supporting PostgreSQL generated stored formula columns
    pub fn format_column_ddl(col: &ColumnIR) -> String {
        Self::format_column_ddl_opt(col, true)
    }

    /// Formats column DDL with option to omit inline UNIQUE (required for partitioned tables)
    pub fn format_column_ddl_opt(col: &ColumnIR, include_inline_unique: bool) -> String {
        let mut line = format!("    \"{}\" {}", col.sql_name, col.sql_type);
        if let Some(form) = &col.formula {
            line.push_str(&format!(" GENERATED ALWAYS AS ({}) STORED", form));
        } else {
            if !col.is_nullable {
                line.push_str(" NOT NULL");
            }
            if let Some(def) = &col.default_val {
                line.push_str(&format!(" DEFAULT {}", def));
            }
            if col.is_unique && include_inline_unique {
                line.push_str(" UNIQUE");
            }
        }
        line
    }

    /// Generates full BOS System Kernel DDL, creating core metadata catalogs,
    /// system dictionaries, and default seeds.
    pub fn generate_system_kernel_ddl(&self) -> String {
        let mut sql = String::new();

        sql.push_str("-- ====================================================\n");
        sql.push_str(&format!(
            "-- BOS Core System Kernel & Metadata Catalog Provisioning (Strategy: {})\n",
            self.strategy.as_str()
        ));
        sql.push_str("-- ====================================================\n\n");

        sql.push_str("CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";\n");
        sql.push_str("CREATE EXTENSION IF NOT EXISTS \"pgcrypto\";\n\n");

        // 1. System Kernel Information Table
        sql.push_str(r#"-- 1. System Kernel Information
CREATE TABLE IF NOT EXISTS "_bos_sys_info" (
    "id" VARCHAR(50) PRIMARY KEY,
    "kernel_version" VARCHAR(50) NOT NULL,
    "schema_version" VARCHAR(50) NOT NULL,
    "status" VARCHAR(50) NOT NULL,
    "storage_strategy" VARCHAR(50) NOT NULL DEFAULT 'shared_rls',
    "provisioned_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "last_upgraded_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
ALTER TABLE IF EXISTS "_bos_sys_info" ADD COLUMN IF NOT EXISTS "storage_strategy" VARCHAR(50) NOT NULL DEFAULT 'shared_rls';

"#);

        // 2. Installed Packages Registry
        sql.push_str(r#"-- 2. Installed Packages Registry
CREATE TABLE IF NOT EXISTS "_bos_meta_packages" (
    "package_id" VARCHAR(100) PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "version" VARCHAR(50) NOT NULL,
    "publisher" VARCHAR(100) NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "checksum" VARCHAR(64) NOT NULL DEFAULT '',
    "installed_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

"#);

        // 3. Core Meta-Objects Catalog
        sql.push_str(r#"-- 3. Core Meta-Objects Catalog
CREATE TABLE IF NOT EXISTS "_bos_meta_objects" (
    "object_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "object_type" VARCHAR(50) NOT NULL,
    "object_name" VARCHAR(100) NOT NULL,
    "package_name" VARCHAR(100) NOT NULL DEFAULT 'bos.core',
    "version" VARCHAR(50) NOT NULL DEFAULT '1.0.0',
    "caption" VARCHAR(255),
    "sub_type" VARCHAR(50),
    "definition" JSONB NOT NULL DEFAULT '{}',
    "checksum" VARCHAR(64) NOT NULL DEFAULT '',
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "created_by" UUID,
    CONSTRAINT "uq_bos_meta_object" UNIQUE ("object_type", "object_name", "package_name")
);

-- Schema Evolution Migration for _bos_meta_objects
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

-- ADR 12: High-performance GIN index on definition JSONB (Single Source of Truth)
DROP INDEX IF EXISTS "idx_bos_meta_objects_ir";
CREATE INDEX IF NOT EXISTS "idx_bos_meta_objects_def" 
ON "_bos_meta_objects" USING gin ("definition" jsonb_path_ops);

-- 4. Authoritative Database Package Vault (Cryptographic Self-Healing Source of Truth)
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

-- ADR 12 Cleanup: Drop deprecated redundant metadata tables
DROP TABLE IF EXISTS "_bos_meta_fields" CASCADE;
DROP TABLE IF EXISTS "_bos_meta_relations" CASCADE;
DROP TABLE IF EXISTS "_bos_meta_state_machines" CASCADE;
"#);

        // 5. Physical Object Mapping Registry (ADR 0003 Multi-Strategy Provisioning)
        sql.push_str(r#"-- 5. Physical Object Mapping Registry
CREATE TABLE IF NOT EXISTS "_bos_object_map" (
    "map_version" INT NOT NULL DEFAULT 1,
    "logical_name" TEXT NOT NULL,
    "tenant_id" UUID,
    "company_id" UUID,
    "layout" VARCHAR(50) NOT NULL DEFAULT 'shared_rls',
    "physical_schema" TEXT NOT NULL DEFAULT 'public',
    "physical_table" TEXT NOT NULL,
    "object_version" VARCHAR(50) NOT NULL DEFAULT '1.0.0',
    PRIMARY KEY ("logical_name", "physical_table")
);
CREATE INDEX IF NOT EXISTS "idx_bos_object_map_lookup" ON "_bos_object_map" ("logical_name", "tenant_id", "company_id");

"#);

        // 7. Multi-Tenant Registry
        sql.push_str(r#"-- 7. Multi-Tenant Registry
CREATE TABLE IF NOT EXISTS "_bos_tenants" (
    "tenant_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(255) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "settings" JSONB NOT NULL DEFAULT '{}',
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

"#);

        // 8. Company Isolation Registry
        sql.push_str(r#"-- 8. Company Isolation Registry
CREATE TABLE IF NOT EXISTS "_bos_companies" (
    "company_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "tenant_id" UUID REFERENCES "_bos_tenants"("tenant_id") ON DELETE CASCADE,
    "code" VARCHAR(50) NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Schema Evolution Migration for _bos_companies (allow company-only provisioning without tenant)
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = '_bos_companies' AND column_name = 'tenant_id' AND is_nullable = 'NO'
    ) THEN
        ALTER TABLE "_bos_companies" ALTER COLUMN "tenant_id" DROP NOT NULL;
    END IF;
END $$;

CREATE UNIQUE INDEX IF NOT EXISTS "uq_bos_tenant_company" ON "_bos_companies" ("tenant_id", "code") WHERE "tenant_id" IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS "uq_bos_company_code_global" ON "_bos_companies" ("code") WHERE "tenant_id" IS NULL;

"#);

        // 9. Privileged Override Statutory Audit Trail
        sql.push_str(r#"-- 9. Privileged Override Statutory Audit Trail
CREATE TABLE IF NOT EXISTS "_system_privileged_audit_log" (
    "id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "table_name" VARCHAR(100) NOT NULL,
    "record_id" VARCHAR(100) NOT NULL,
    "action" VARCHAR(20) NOT NULL,
    "reason" TEXT NOT NULL,
    "ticket_ref" VARCHAR(100) NOT NULL,
    "authorized_by" VARCHAR(100) NOT NULL,
    "old_values" JSONB,
    "new_values" JSONB,
    "actor_id" UUID,
    "timestamp" TIMESTAMPTZ DEFAULT NOW()
);

-- 10. Background Async Job Queue (Scalable Asynchronous Processing)
CREATE TABLE IF NOT EXISTS "_bos_job_queue" (
    "job_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "job_type" VARCHAR(100) NOT NULL,
    "codeunit_name" VARCHAR(100) NOT NULL,
    "method_name" VARCHAR(100) NOT NULL,
    "payload" JSONB NOT NULL DEFAULT '{}',
    "status" VARCHAR(20) NOT NULL DEFAULT 'ready',
    "chunk_size" INT NOT NULL DEFAULT 200,
    "attempts" INT NOT NULL DEFAULT 0,
    "max_attempts" INT NOT NULL DEFAULT 3,
    "error_message" TEXT,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "started_at" TIMESTAMPTZ,
    "completed_at" TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS "idx_bos_job_queue_status" ON "_bos_job_queue" ("status", "created_at");

"#);

        // 12. Helper Function: Touch Updated Timestamp & Version
        sql.push_str(r#"-- 12. Helper Function: Touch Updated Timestamp & Version
CREATE OR REPLACE FUNCTION bos_touch_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW."_updated_at" = NOW();
    NEW."_system_version" = COALESCE(OLD."_system_version", 0) + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

"#);

        // 13. Dynamic Multi-Tenant Helpers
        sql.push_str(r#"-- 13. Dynamic Multi-Tenant Topology Helpers
CREATE OR REPLACE FUNCTION bos_create_table_partition(p_table text, p_tenant_id uuid, p_suffix text)
RETURNS void AS $$
BEGIN
    EXECUTE format('CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES IN (%L)',
        p_table || '_tenant_' || p_suffix, p_table, p_tenant_id);
EXCEPTION WHEN duplicate_table THEN
    NULL;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION bos_provision_company_schema(p_schema_name text)
RETURNS void AS $$
BEGIN
    EXECUTE format('CREATE SCHEMA IF NOT EXISTS %I', p_schema_name);
END;
$$ LANGUAGE plpgsql;

"#);

        // 13b. Schema migration journal and relation (foreign key) installer
        sql.push_str("-- 13b. Schema Migration Journal & Relation Installer\n");
        sql.push_str(SCHEMA_SUPPORT_DDL);
        sql.push('\n');

        // 14. System Kernel Status Record
        sql.push_str(&format!(
            r#"-- 14. System Kernel Status Record
INSERT INTO "_bos_sys_info" ("id", "kernel_version", "schema_version", "status", "storage_strategy")
VALUES ('core', '0.1.0', '2026.10', 'active', '{}')
ON CONFLICT ("id") DO UPDATE 
SET "kernel_version" = EXCLUDED."kernel_version",
    "storage_strategy" = EXCLUDED."storage_strategy",
    "last_upgraded_at" = NOW();
"#,
            self.strategy.as_str()
        ));

        sql
    }

    pub fn generate_ddl(&self, model: &ModelIR) -> String {
        let mut sql = String::new();

        // Include system kernel
        sql.push_str(&self.generate_system_kernel_ddl());
        sql.push('\n');

        sql.push_str("-- ====================================================\n");
        sql.push_str(&format!("-- BOS Domain Objects DDL for Package: {}\n", model.package_name));
        sql.push_str("-- Generated by bosc-codegen-db\n");
        sql.push_str("-- ====================================================\n\n");

        // 1. Base table structures
        for table in &model.tables {
            sql.push_str(&self.generate_table_structure(table));
            sql.push('\n');
        }

        // 2. Table Extensions (must precede relations and views that may reference extended columns)
        for ext in &model.table_extensions {
            sql.push_str(&self.generate_table_extension_ddl(ext, &[]));
            sql.push('\n');
        }

        // 3. Foreign key relations
        for table in &model.tables {
            sql.push_str(&self.generate_relations(table));
        }
        sql.push('\n');

        // 4. SIFT covering indexes on target tables
        for table in &model.tables {
            sql.push_str(&self.generate_table_sift_indexes(table));
        }
        sql.push('\n');

        // 5. State Machine History & Triggers
        for sm in &model.state_machines {
            sql.push_str(&self.generate_state_machine_infrastructure(sm));
            sql.push('\n');
        }

        // 6. FlowField pushdown views
        for table in &model.tables {
            sql.push_str(&self.generate_table_flowfield_views(table));
        }
        sql.push('\n');

        // 7. Views
        for view in &model.views {
            sql.push_str(&self.generate_view_ddl(view));
            sql.push('\n');
        }

        // 8. Permission Sets (RLS Policies & Permissions)
        for ps in &model.permission_sets {
            sql.push_str(&self.generate_permission_set_ddl(ps));
            sql.push('\n');
        }

        // 9. Register Queries in catalog
        for query in &model.queries {
            sql.push_str(&self.generate_query_catalog_ddl(query));
            sql.push('\n');
        }

        sql
    }

    /// DDL for a set of tables: every table structure first, then relations, SIFT indexes, and
    /// FlowField views so all forward cross-table references resolve cleanly regardless of order.
    pub fn generate_tables(&self, tables: &[TableIR]) -> String {
        let mut sql = String::new();
        // 1. All base table structures first (ensures every physical table exists)
        for table in tables {
            sql.push_str(&self.generate_table_structure(table));
            sql.push('\n');
        }
        // 2. Foreign key relations (all target tables now exist)
        for table in tables {
            sql.push_str(&self.generate_relations(table));
        }
        sql.push('\n');
        // 3. SIFT covering indexes on target tables (all target tables now exist)
        for table in tables {
            sql.push_str(&self.generate_table_sift_indexes(table));
        }
        sql.push('\n');
        // 4. FlowField pushdown views (all referenced tables now exist)
        for table in tables {
            sql.push_str(&self.generate_table_flowfield_views(table));
        }
        sql.push('\n');
        sql
    }

    /// FlowField Pushdown Views for `table`
    pub fn generate_table_flowfield_views(&self, table: &TableIR) -> String {
        let mut ddl = String::new();
        if !table.flow_fields.is_empty() {
            let mut ff_selects = Vec::new();
            for ff in &table.flow_fields {
                let sub = if !ff.sql_subquery.trim().is_empty() {
                    let mut rewritten = ff.sql_subquery.clone();
                    rewritten = rewritten.replace(&format!("\"{}\".", table.sql_table_name), "t.");
                    rewritten = rewritten.replace(&format!("\"{}\".", table.name.to_lowercase()), "t.");
                    rewritten = rewritten.replace(&format!("\"{}\".", to_snake_case(&table.name)), "t.");
                    rewritten = rewritten.replace(&format!("\"{}\".", table.name), "t.");
                    rewritten = rewritten.replace(&format!("{}.", table.sql_table_name), "t.");
                    let trimmed = rewritten.trim();
                    if trimmed.starts_with('(') && trimmed.ends_with(')') {
                        format!("{} AS \"{}\"", trimmed, to_snake_case(&ff.name))
                    } else {
                        format!("({}) AS \"{}\"", trimmed, to_snake_case(&ff.name))
                    }
                } else if let Some(target_col) = &ff.target_column {
                    match ff.func {
                        FlowFieldFuncIR::Exist => format!(
                            "(SELECT EXISTS(SELECT 1 FROM \"{}\" WHERE \"_tenant_id\" = t.\"_tenant_id\")) AS \"{}\"",
                            to_snake_case(&ff.target_table),
                            to_snake_case(&ff.name)
                        ),
                        FlowFieldFuncIR::Lookup => format!(
                            "(SELECT \"{}\" FROM \"{}\" WHERE \"_tenant_id\" = t.\"_tenant_id\" LIMIT 1) AS \"{}\"",
                            to_snake_case(target_col),
                            to_snake_case(&ff.target_table),
                            to_snake_case(&ff.name)
                        ),
                        _ => {
                            let func_sql = match ff.func {
                                FlowFieldFuncIR::Sum => "SUM",
                                FlowFieldFuncIR::Average => "AVG",
                                FlowFieldFuncIR::Count => "COUNT",
                                FlowFieldFuncIR::Min => "MIN",
                                FlowFieldFuncIR::Max => "MAX",
                                _ => "SUM",
                            };
                            format!(
                                "(SELECT COALESCE({}(\"{}\"), 0) FROM \"{}\" WHERE \"_tenant_id\" = t.\"_tenant_id\") AS \"{}\"",
                                func_sql,
                                to_snake_case(target_col),
                                to_snake_case(&ff.target_table),
                                to_snake_case(&ff.name)
                            )
                        }
                    }
                } else {
                    format!("NULL AS \"{}\"", to_snake_case(&ff.name))
                };
                ff_selects.push(sub);
            }
            ddl.push_str(&format!(
                "CREATE OR REPLACE VIEW \"{}_view\" AS SELECT t.*, {} FROM \"{}\" t;\n",
                table.sql_table_name, ff_selects.join(", "), table.sql_table_name
            ));
        }
        ddl
    }

    /// SIFT (SumIndexField Technology) covering indexes on target tables for `table`'s FlowFields
    pub fn generate_table_sift_indexes(&self, table: &TableIR) -> String {
        let mut ddl = String::new();
        for ff in &table.flow_fields {
            if ff.target_column.is_some() || !ff.target_table.trim().is_empty() {
                ddl.push_str(&sift_index_ddl(ff));
            }
        }
        ddl
    }

    /// Foreign keys for the `relation` fields of `table` (see [`relations`]).
    pub fn generate_relations(&self, table: &TableIR) -> String {
        if matches!(table.table_type, TableTypeIR::Virtual | TableTypeIR::Temporary) {
            return String::new();
        }
        relations::generate_relations(self.strategy, table)
    }

    /// Structure, relations, sift indexes, and flowfield views of a single table.
    pub fn generate_table(&self, table: &TableIR) -> String {
        let mut ddl = self.generate_table_structure(table);
        ddl.push_str(&self.generate_relations(table));
        ddl.push_str(&self.generate_table_sift_indexes(table));
        ddl.push_str(&self.generate_table_flowfield_views(table));
        ddl
    }

    /// Generate Ahead-of-Time Prepared SQL Statements for instant database connection pipelining
    pub fn generate_aot_prepared_queries(&self, tables: &[TableIR]) -> Vec<AotPreparedStatementIR> {
        let mut queries = Vec::new();

        for table in tables {
            if matches!(table.table_type, TableTypeIR::Virtual | TableTypeIR::Temporary) {
                continue;
            }

            let tbl_name = &table.sql_table_name;
            let primary_key_col = table.primary_key.first().map(|s| s.as_str()).unwrap_or("id");
            let is_pk = |col_name: &str| table.primary_key.iter().any(|pk| pk.eq_ignore_ascii_case(col_name));

            // 1. GetById
            if table.data_per_company {
                queries.push(AotPreparedStatementIR {
                    name: format!("{}_GetById", table.name),
                    table: table.name.clone(),
                    kind: "get_by_id".to_string(),
                    sql: format!(
                        r#"SELECT * FROM "{}" WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2 AND "_company_id" IS NOT DISTINCT FROM $3 LIMIT 1;"#,
                        tbl_name, primary_key_col
                    ),
                    param_names: vec![primary_key_col.to_string(), "_tenant_id".to_string(), "_company_id".to_string()],
                });
            } else {
                queries.push(AotPreparedStatementIR {
                    name: format!("{}_GetById", table.name),
                    table: table.name.clone(),
                    kind: "get_by_id".to_string(),
                    sql: format!(
                        r#"SELECT * FROM "{}" WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2 LIMIT 1;"#,
                        tbl_name, primary_key_col
                    ),
                    param_names: vec![primary_key_col.to_string(), "_tenant_id".to_string()],
                });
            }

            // 2. Exists
            queries.push(AotPreparedStatementIR {
                name: format!("{}_Exists", table.name),
                table: table.name.clone(),
                kind: "exists".to_string(),
                sql: format!(
                    r#"SELECT EXISTS(SELECT 1 FROM "{}" WHERE "{}" = $1 AND "_tenant_id" IS NOT DISTINCT FROM $2);"#,
                    tbl_name, primary_key_col
                ),
                param_names: vec![primary_key_col.to_string(), "_tenant_id".to_string()],
            });

            // 3. Insert (for all writable physical columns + required system columns)
            let mut all_insert_cols = vec!["\"_id\"".to_string(), "\"_tenant_id\"".to_string()];
            let mut all_insert_params = vec!["_id".to_string(), "_tenant_id".to_string()];
            if table.data_per_company {
                all_insert_cols.push("\"_company_id\"".to_string());
                all_insert_params.push("_company_id".to_string());
            }

            let writable_cols: Vec<&ColumnIR> = table.columns.iter()
                .filter(|c| c.formula.is_none())
                .collect();
            
            for c in &writable_cols {
                all_insert_cols.push(format!("\"{}\"", c.sql_name));
                all_insert_params.push(c.sql_name.clone());
            }

            let param_placeholders: Vec<String> = (1..=all_insert_cols.len()).map(|i| format!("${}", i)).collect();

            if !writable_cols.is_empty() {
                queries.push(AotPreparedStatementIR {
                    name: format!("{}_Insert", table.name),
                    table: table.name.clone(),
                    kind: "insert".to_string(),
                    sql: format!(
                        r#"INSERT INTO "{}" ({}) VALUES ({}) RETURNING *;"#,
                        tbl_name,
                        all_insert_cols.join(", "),
                        param_placeholders.join(", ")
                    ),
                    param_names: all_insert_params,
                });
            }

            // 4. UpdateOptimistic
            let non_pk_writable: Vec<&ColumnIR> = writable_cols.iter()
                .filter(|c| !is_pk(&c.name) && !is_pk(&c.sql_name) && c.sql_name != "_system_version")
                .cloned()
                .collect();

            if !non_pk_writable.is_empty() {
                let mut set_clauses = Vec::new();
                let mut update_params = Vec::new();
                let mut param_idx = 1;

                for c in &non_pk_writable {
                    set_clauses.push(format!("\"{}\" = ${}", c.sql_name, param_idx));
                    update_params.push(c.sql_name.clone());
                    param_idx += 1;
                }
                set_clauses.push("\"_system_version\" = \"_system_version\" + 1".to_string());

                let pk_idx = param_idx;
                update_params.push(primary_key_col.to_string());
                param_idx += 1;

                let ver_idx = param_idx;
                update_params.push("_system_version".to_string());
                param_idx += 1;

                let tenant_idx = param_idx;
                update_params.push("_tenant_id".to_string());

                queries.push(AotPreparedStatementIR {
                    name: format!("{}_UpdateOptimistic", table.name),
                    table: table.name.clone(),
                    kind: "update_optimistic".to_string(),
                    sql: format!(
                        r#"UPDATE "{}" SET {} WHERE "{}" = ${} AND "_system_version" = ${} AND "_tenant_id" IS NOT DISTINCT FROM ${} RETURNING *;"#,
                        tbl_name,
                        set_clauses.join(", "),
                        primary_key_col,
                        pk_idx,
                        ver_idx,
                        tenant_idx
                    ),
                    param_names: update_params,
                });
            }

            // 5. Delete
            queries.push(AotPreparedStatementIR {
                name: format!("{}_Delete", table.name),
                table: table.name.clone(),
                kind: "delete".to_string(),
                sql: format!(
                    r#"DELETE FROM "{}" WHERE "{}" = $1 AND "_system_version" = $2 AND "_tenant_id" IS NOT DISTINCT FROM $3 RETURNING *;"#,
                    tbl_name, primary_key_col
                ),
                param_names: vec![primary_key_col.to_string(), "_system_version".to_string(), "_tenant_id".to_string()],
            });

            // 6. SIFT FlowField Covering Aggregation queries
            for ff in &table.flow_fields {
                queries.push(AotPreparedStatementIR {
                    name: format!("{}_SIFT_{}", table.name, ff.name),
                    table: table.name.clone(),
                    kind: "sift_sum".to_string(),
                    sql: format!(
                        r#"SELECT ({}) AS "{}" FROM "{}" WHERE "_tenant_id" IS NOT DISTINCT FROM $1;"#,
                        ff.sql_subquery, ff.name, tbl_name
                    ),
                    param_names: vec!["_tenant_id".to_string()],
                });
            }
        }

        queries
    }



    pub fn generate_table_structure(&self, table: &TableIR) -> String {
        let mut ddl = String::new();
        ddl.push_str(&format!(
            "-- Table: {} ({:?}, Strategy: {})\n",
            table.name,
            table.table_type,
            self.strategy.as_str()
        ));

        if table.table_type == TableTypeIR::Virtual {
            ddl.push_str(&format!(
                "-- Virtual Table {} (in-memory runtime projection; no physical DDL emitted)\n",
                table.name
            ));
            return ddl;
        }

        if table.table_type == TableTypeIR::Temporary {
            ddl.push_str(&format!(
                "CREATE UNLOGGED TABLE IF NOT EXISTS \"{}\" (\n",
                table.sql_table_name
            ));
            ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
            ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
            if table.data_per_company {
                ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
            }
            ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
            ddl.push_str("    \"_created_by\" UUID,\n");
            ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
            ddl.push_str("    \"_updated_by\" UUID,\n");
            ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

            for col in &table.columns {
                ddl.push_str(&Self::format_column_ddl(col));
                ddl.push_str(",\n");
            }
            if ddl.ends_with(",\n") {
                ddl.pop();
                ddl.pop();
                ddl.push('\n');
            }
            ddl.push_str(") ON COMMIT PRESERVE ROWS;\n\n");
            return ddl;
        }

        match self.strategy {
            StorageStrategyIR::Partitioned => {
                // Partitioned table definition
                ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.sql_table_name));
                ddl.push_str("    \"_id\" UUID DEFAULT gen_random_uuid(),\n");
                ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                if table.data_per_company {
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                }
                ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_created_by\" UUID,\n");
                ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_updated_by\" UUID,\n");
                ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                for col in &table.columns {
                    ddl.push_str(&Self::format_column_ddl_opt(col, false));
                    ddl.push_str(",\n");
                }

                for col in &table.columns {
                    if col.is_unique {
                        ddl.push_str(&format!("    CONSTRAINT \"uq_{}_{}\" UNIQUE (\"_tenant_id\", \"{}\"),\n", table.sql_table_name, col.sql_name, col.sql_name));
                    }
                }

                ddl.push_str("    PRIMARY KEY (\"_tenant_id\", \"_id\")\n");
                ddl.push_str(") PARTITION BY LIST (\"_tenant_id\");\n\n");

                // Default partition
                ddl.push_str(&format!(
                    "CREATE TABLE IF NOT EXISTS \"{}_default\" PARTITION OF \"{}\" DEFAULT;\n\n",
                    table.sql_table_name, table.sql_table_name
                ));
            }
            StorageStrategyIR::SchemaPerCompany => {
                if table.data_per_company {
                    ddl.push_str("CREATE SCHEMA IF NOT EXISTS \"comp_default\";\n");
                    ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"comp_default\".\"{}\" (\n", table.sql_table_name));
                    ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
                    ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                    ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                    ddl.push_str("    \"_created_by\" UUID,\n");
                    ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                    ddl.push_str("    \"_updated_by\" UUID,\n");
                    ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                    for col in &table.columns {
                        ddl.push_str(&Self::format_column_ddl(col));
                        ddl.push_str(",\n");
                    }
                    if ddl.ends_with(",\n") {
                        ddl.pop();
                        ddl.pop();
                        ddl.push('\n');
                    }
                    ddl.push_str(");\n\n");
                }

                // Also create public base table for fallback & shared tables
                ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"public\".\"{}\" (\n", table.sql_table_name));
                ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
                ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                if table.data_per_company {
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                }
                ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_created_by\" UUID,\n");
                ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_updated_by\" UUID,\n");
                ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                for col in &table.columns {
                    ddl.push_str(&Self::format_column_ddl(col));
                    ddl.push_str(",\n");
                }
                if ddl.ends_with(",\n") {
                    ddl.pop();
                    ddl.pop();
                    ddl.push('\n');
                }
                ddl.push_str(");\n\n");
            }
            StorageStrategyIR::TablePerCompany => {
                if table.data_per_company {
                    // Physical table per company: default$customer
                    ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"default${}\" (\n", table.sql_table_name));
                    ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
                    ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                    ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                    ddl.push_str("    \"_created_by\" UUID,\n");
                    ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                    ddl.push_str("    \"_updated_by\" UUID,\n");
                    ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                    for col in &table.columns {
                        ddl.push_str(&Self::format_column_ddl(col));
                        ddl.push_str(",\n");
                    }
                    if ddl.ends_with(",\n") {
                        ddl.pop();
                        ddl.pop();
                        ddl.push('\n');
                    }
                    ddl.push_str(");\n\n");
                }

                // Public table / fallback
                ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.sql_table_name));
                ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
                ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                if table.data_per_company {
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                }
                ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_created_by\" UUID,\n");
                ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_updated_by\" UUID,\n");
                ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                for col in &table.columns {
                    ddl.push_str(&Self::format_column_ddl(col));
                    ddl.push_str(",\n");
                }
                if ddl.ends_with(",\n") {
                    ddl.pop();
                    ddl.pop();
                    ddl.push('\n');
                }
                ddl.push_str(");\n\n");
            }
            StorageStrategyIR::SharedRLS => {
                // High density shared table
                ddl.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.sql_table_name));
                ddl.push_str("    \"_id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
                ddl.push_str("    \"_tenant_id\" UUID NOT NULL,\n");
                if table.data_per_company {
                    ddl.push_str("    \"_company_id\" UUID NOT NULL,\n");
                }
                ddl.push_str("    \"_created_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_created_by\" UUID,\n");
                ddl.push_str("    \"_updated_at\" TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n");
                ddl.push_str("    \"_updated_by\" UUID,\n");
                ddl.push_str("    \"_system_version\" BIGINT NOT NULL DEFAULT 1,\n");

                for col in &table.columns {
                    ddl.push_str(&Self::format_column_ddl(col));
                    ddl.push_str(",\n");
                }
                if ddl.ends_with(",\n") {
                    ddl.pop();
                    ddl.pop();
                    ddl.push('\n');
                }
                ddl.push_str(");\n\n");
            }
        }

        // Auto-Evolve Schema: Automatically synchronize newly added columns to existing tables
        for col in &table.columns {
            let default_clause = col
                .default_val
                .as_ref()
                .map(|d| format!(" DEFAULT {}", d))
                .unwrap_or_default();
            let nullable_clause = if col.is_nullable { "" } else { " NOT NULL" };

            match self.strategy {
                StorageStrategyIR::SharedRLS | StorageStrategyIR::Partitioned => {
                    ddl.push_str(&format!(
                        "ALTER TABLE IF EXISTS \"{}\" ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                        table.sql_table_name, col.sql_name, col.sql_type, default_clause, nullable_clause
                    ));
                }
                StorageStrategyIR::SchemaPerCompany => {
                    if table.data_per_company {
                        ddl.push_str(&format!(
                            "ALTER TABLE IF EXISTS \"comp_default\".\"{}\" ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                            table.sql_table_name, col.sql_name, col.sql_type, default_clause, nullable_clause
                        ));
                    }
                    ddl.push_str(&format!(
                        "ALTER TABLE IF EXISTS \"public\".\"{}\" ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                        table.sql_table_name, col.sql_name, col.sql_type, default_clause, nullable_clause
                    ));
                }
                StorageStrategyIR::TablePerCompany => {
                    if table.data_per_company {
                        ddl.push_str(&format!(
                            "ALTER TABLE IF EXISTS \"default${}\" ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                            table.sql_table_name, col.sql_name, col.sql_type, default_clause, nullable_clause
                        ));
                    }
                    ddl.push_str(&format!(
                        "ALTER TABLE IF EXISTS \"{}\" ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                        table.sql_table_name, col.sql_name, col.sql_type, default_clause, nullable_clause
                    ));
                }
            }
        }

        // Indexes
        for idx in &table.indexes {
            let mut cols_vec = idx.columns.clone();
            if self.strategy == StorageStrategyIR::Partitioned && idx.is_unique && !cols_vec.contains(&"_tenant_id".to_string()) {
                cols_vec.insert(0, "_tenant_id".to_string());
            }
            let cols = cols_vec.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ");
            let unique = if idx.is_unique { "UNIQUE " } else { "" };
            let include_clause = if !idx.include_columns.is_empty() {
                let inc = idx.include_columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ");
                format!(" INCLUDE ({})", inc)
            } else {
                String::new()
            };
            ddl.push_str(&format!(
                "CREATE {}INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({}){};\n",
                unique, idx.name, table.sql_table_name, cols, include_clause
            ));
        }

        // Custom Primary Key Unique Index (ensures ON CONFLICT and FK references work on user-declared primaryKey)
        let custom_pk_cols: Vec<String> = table
            .primary_key
            .iter()
            .filter(|pk| *pk != "_id" && table.columns.iter().any(|c| c.sql_name == **pk || c.name.eq_ignore_ascii_case(pk)))
            .cloned()
            .collect();
        if !custom_pk_cols.is_empty() {
            let mut pk_idx_cols = custom_pk_cols.clone();
            if self.strategy == StorageStrategyIR::Partitioned && !pk_idx_cols.contains(&"_tenant_id".to_string()) {
                pk_idx_cols.insert(0, "_tenant_id".to_string());
            }
            let pk_cols_sql = pk_idx_cols.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ");
            let pk_idx_name = format!("uq_pk_{}_{}", table.sql_table_name, custom_pk_cols.join("_"));
            ddl.push_str(&format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});\n",
                pk_idx_name, table.sql_table_name, pk_cols_sql
            ));
        }

        // Row-Level Security: Multi-tenant and Company-Only isolation
        if self.strategy == StorageStrategyIR::SharedRLS || self.strategy == StorageStrategyIR::Partitioned {
            ddl.push_str(&format!("ALTER TABLE \"{}\" ENABLE ROW LEVEL SECURITY;\n", table.sql_table_name));
            ddl.push_str(&format!("ALTER TABLE \"{}\" FORCE ROW LEVEL SECURITY;\n", table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \"tenant_isolation_{}\" ON \"{}\";\n", table.sql_table_name, table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \"bos_isolation_{}\" ON \"{}\";\n", table.sql_table_name, table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \"bos_base_{}\" ON \"{}\";\n", table.sql_table_name, table.sql_table_name));

            // Base permissive policy allowing operations subjected to restrictive isolation
            ddl.push_str(&format!(
                "CREATE POLICY \"bos_base_{}\" ON \"{}\"\n    FOR ALL\n    USING (true)\n    WITH CHECK (true);\n",
                table.sql_table_name, table.sql_table_name
            ));

            if table.data_per_company {
                ddl.push_str(&format!(
                    "CREATE POLICY \"bos_isolation_{tbl}\" ON \"{tbl}\"\n    AS RESTRICTIVE\n    FOR ALL\n    USING (\n        (\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \"_tenant_id\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\n            OR\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \"_tenant_id\" IS NULL)\n        )\n        AND (\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL AND (\"_company_id\" = COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), ''))::uuid OR \"_company_id\" IS NULL))\n            OR\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NULL AND (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL OR \"_company_id\" IS NULL))\n        )\n        AND (\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\n            OR (\"_tenant_id\" IS NULL AND \"_company_id\" IS NULL)\n        )\n    )\n    WITH CHECK (\n        (\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \"_tenant_id\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\n            OR\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \"_tenant_id\" IS NULL)\n        )\n        AND (\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL AND (\"_company_id\" = COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), ''))::uuid OR \"_company_id\" IS NULL))\n            OR\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NULL AND (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL OR \"_company_id\" IS NULL))\n        )\n        AND (\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\n            OR (\"_tenant_id\" IS NULL AND \"_company_id\" IS NULL)\n        )\n    );\n",
                    tbl = table.sql_table_name
                ));
            } else {
                ddl.push_str(&format!(
                    "CREATE POLICY \"bos_isolation_{tbl}\" ON \"{tbl}\"\n    AS RESTRICTIVE\n    FOR ALL\n    USING (\n        (\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \"_tenant_id\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\n            OR\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \"_tenant_id\" IS NULL)\n        )\n        AND (\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\n            OR \"_tenant_id\" IS NULL\n        )\n    )\n    WITH CHECK (\n        (\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \"_tenant_id\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\n            OR\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \"_tenant_id\" IS NULL)\n        )\n        AND (\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\n            OR \"_tenant_id\" IS NULL\n        )\n    );\n",
                    tbl = table.sql_table_name
                ));
            }
        }

        // Temporal History Table & Archival Trigger
        if table.table_type == TableTypeIR::Temporal {
            ddl.push_str(&format!(
                r#"
CREATE TABLE IF NOT EXISTS "{table_name}_history" (
    "_history_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "_record_id" UUID NOT NULL,
    "_tenant_id" UUID,
    "_system_version" BIGINT NOT NULL,
    "_operation" VARCHAR(16) NOT NULL,
    "_valid_from" TIMESTAMPTZ NOT NULL,
    "_valid_to" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "_snapshot" JSONB NOT NULL
);
CREATE INDEX IF NOT EXISTS "idx_{table_name}_history_rec" ON "{table_name}_history" ("_tenant_id", "_record_id", "_valid_to");

CREATE OR REPLACE FUNCTION "archive_temporal_{table_name}"()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO "{table_name}_history" (
        "_record_id", "_tenant_id", "_system_version", "_operation", "_valid_from", "_valid_to", "_snapshot"
    ) VALUES (
        OLD."_id",
        OLD."_tenant_id",
        OLD."_system_version",
        TG_OP,
        OLD."_updated_at",
        NOW(),
        to_jsonb(OLD)
    );
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_temporal_{table_name}" ON "{table_name}";
CREATE TRIGGER "trg_temporal_{table_name}"
    BEFORE UPDATE OR DELETE ON "{table_name}"
    FOR EACH ROW EXECUTE FUNCTION "archive_temporal_{table_name}"();
"#,
                table_name = table.sql_table_name
            ));
        }

        // Ledger Protection Triggers
        if table.table_type == TableTypeIR::Ledger {
            ddl.push_str(&format!(
                r#"
CREATE OR REPLACE FUNCTION "guard_immutable_{table_name}"()
RETURNS TRIGGER AS $$
BEGIN
    IF current_setting('bos.privileged_override', true) = 'true' THEN
        INSERT INTO "_system_privileged_audit_log" (
            "table_name", "record_id", "action", "reason", "ticket_ref", "authorized_by", "old_values", "new_values"
        ) VALUES (
            '{table_name}',
            OLD."_id"::text,
            TG_OP,
            COALESCE(current_setting('bos.privileged_reason', true), 'MANUAL_PRIVILEGED_OVERRIDE'),
            COALESCE(current_setting('bos.privileged_ticket', true), 'N/A'),
            COALESCE(current_setting('bos.privileged_authorizer', true), 'SUPERUSER'),
            to_jsonb(OLD),
            CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE to_jsonb(NEW) END
        );
        RETURN NEW;
    END IF;

    RAISE EXCEPTION 'CANNOT % IMMUTABLE LEDGER TABLE: % (Requires Privileged Override)', TG_OP, '{table_name}';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_immutable_{table_name}" ON "{table_name}";
CREATE TRIGGER "trg_immutable_{table_name}"
    BEFORE UPDATE OR DELETE ON "{table_name}"
    FOR EACH ROW EXECUTE FUNCTION "guard_immutable_{table_name}"();
"#,
                table_name = table.sql_table_name
            ));
        }

        // Referential Deletion Protection Trigger (Dynamics NAV OnDelete / RestrictIfLedgerExists Pattern)
        let should_protect_delete = table.on_delete.as_deref() == Some("RestrictIfLedgerExists")
            || table.on_delete.as_deref() == Some("Restrict")
            || !table.flow_fields.is_empty();

        if should_protect_delete {
            let mut check_statements = Vec::new();
            for ff in &table.flow_fields {
                let target_tbl = to_snake_case(&ff.target_table);
                if let Some(pk_col) = table.primary_key.first() {
                    let fk_candidate = format!("{}_{}", table.sql_table_name, pk_col);
                    check_statements.push(format!(
                        "    IF to_regclass('public.{}') IS NOT NULL THEN\n        IF EXISTS (SELECT 1 FROM \"{}\" WHERE \"{}\" = OLD.\"{}\") THEN\n            RAISE EXCEPTION 'CANNOT DELETE RECORD IN %: Linked ledger entries exist in {} (Referential Integrity Violation)', '{}';\n        END IF;\n    END IF;\n",
                        target_tbl, target_tbl, fk_candidate, pk_col, target_tbl, table.sql_table_name
                    ));
                }
            }

            if !check_statements.is_empty() {
                ddl.push_str(&format!(
                    r#"
CREATE OR REPLACE FUNCTION "guard_delete_{table_name}"()
RETURNS TRIGGER AS $$
BEGIN
{checks}    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS "trg_guard_delete_{table_name}" ON "{table_name}";
CREATE TRIGGER "trg_guard_delete_{table_name}"
    BEFORE DELETE ON "{table_name}"
    FOR EACH ROW EXECUTE FUNCTION "guard_delete_{table_name}"();
"#,
                    table_name = table.sql_table_name,
                    checks = check_statements.join("")
                ));
            }
        }

        // Dual-Tier Defensive Architecture: Native PostgreSQL Check Constraints
        for col in &table.columns {
            if let Some(chk) = &col.check_constraint {
                let chk_name = format!("chk_{}_{}", table.sql_table_name, col.sql_name);
                ddl.push_str(&format!(
                    "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\nALTER TABLE \"{}\" ADD CONSTRAINT \"{}\" CHECK ({});\n",
                    table.sql_table_name, chk_name, table.sql_table_name, chk_name, chk
                ));
            }
        }
        for chk in &table.check_constraints {
            ddl.push_str(&format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";\nALTER TABLE \"{}\" ADD CONSTRAINT \"{}\" CHECK ({});\n",
                table.sql_table_name, chk.name, table.sql_table_name, chk.name, chk.condition
            ));
        }

        ddl
    }

    pub fn generate_state_machine_infrastructure(&self, sm: &StateMachineIR) -> String {
        let mut sql = String::new();
        let history_table = format!("{}_state_history", sm.target_table);

        sql.push_str(&format!("-- State Machine Infrastructure for {}\n", sm.name));
        sql.push_str(&format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", history_table));
        sql.push_str("    \"id\" UUID PRIMARY KEY DEFAULT gen_random_uuid(),\n");
        sql.push_str("    \"entity_id\" UUID NOT NULL,\n");
        sql.push_str("    \"from_state\" VARCHAR(50) NOT NULL,\n");
        sql.push_str("    \"to_state\" VARCHAR(50) NOT NULL,\n");
        sql.push_str("    \"action\" VARCHAR(100) NOT NULL,\n");
        sql.push_str("    \"actor_id\" UUID,\n");
        sql.push_str("    \"comments\" TEXT,\n");
        sql.push_str("    \"params\" JSONB,\n");
        sql.push_str("    \"timestamp\" TIMESTAMPTZ DEFAULT NOW()\n");
        sql.push_str(");\n\n");

        sql.push_str(&format!(
            "CREATE INDEX IF NOT EXISTS \"idx_{}_entity\" ON \"{}\" (\"entity_id\");\n",
            history_table, history_table
        ));

        sql
    }

    /// Generate adaptive DDL for Table Extensions (non-invasive schema evolution)
    pub fn generate_table_extension_ddl(&self, ext: &bosc_ir::TableExtensionIR, companies: &[String]) -> String {
        let mut ddl = String::new();
        ddl.push_str(&format!("-- Table Extension: {} on Table {}\n", ext.name, ext.target_table));

        let lower_table = to_snake_case(&ext.target_table);
        let mut target_tables = vec![format!("\"public\".\"{}\"", lower_table)];
        match self.strategy {
            bosc_ir::StorageStrategyIR::SchemaPerCompany => {
                for comp in companies {
                    let schema = format!("comp_{}", comp.to_lowercase().replace('-', "_"));
                    target_tables.push(format!("\"{}\".\"{}\"", schema, lower_table));
                }
            }
            bosc_ir::StorageStrategyIR::TablePerCompany => {
                for comp in companies {
                    let tbl = format!("{}${}", comp.to_lowercase(), lower_table);
                    target_tables.push(format!("\"public\".\"{}\"", tbl));
                }
            }
            _ => {}
        }

        for target in &target_tables {
            for col in &ext.added_columns {
                let nullable_clause = if col.is_nullable { "" } else { " NOT NULL" };
                let default_clause = col
                    .default_val
                    .as_ref()
                    .map(|d| format!(" DEFAULT {}", d))
                    .unwrap_or_default();
                ddl.push_str(&format!(
                    "ALTER TABLE IF EXISTS {} ADD COLUMN IF NOT EXISTS \"{}\" {}{}{};\n",
                    target, col.sql_name, col.sql_type, default_clause, nullable_clause
                ));
            }

            for idx in &ext.added_indexes {
                let unique_clause = if idx.is_unique { "UNIQUE " } else { "" };
                let cols_joined = idx
                    .columns
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                let idx_name = format!("{}_{}_{}", ext.target_table, ext.name.to_lowercase(), idx.name);
                ddl.push_str(&format!(
                    "CREATE {}INDEX IF NOT EXISTS \"{}\" ON {} ({});\n",
                    unique_clause, idx_name, target, cols_joined
                ));
            }
        }

        let relation_holder = TableIR {
            name: ext.target_table.clone(),
            sql_table_name: lower_table,
            caption: ext.target_table.clone(),
            table_type: TableTypeIR::Normal,
            data_per_company: false,
            columns: ext.added_columns.iter().filter(|c| c.foreign_key.is_some()).cloned().collect(),
            primary_key: Vec::new(),
            indexes: Vec::new(),
            flow_fields: Vec::new(),
            triggers: Vec::new(),
            check_constraints: Vec::new(),
            on_delete: None,
            renamed_from: None,
            field_groups: Vec::new(),
        };
        ddl.push_str(&self.generate_relations(&relation_holder));

        ddl
    }

    /// Compile a Codeunit method into a native PostgreSQL Stored Procedure (BOS-AMDP pushdown)
    pub fn generate_plpgsql_procedure(&self, cu_name: &str, method: &bosc_ir::MethodIR) -> String {
        let proc_name = format!("bos_proc_{}_{}", cu_name.to_lowercase(), method.name.to_lowercase());
        let mut sql = format!(
            "CREATE OR REPLACE FUNCTION \"{}\"(p_params JSONB, p_ctx JSONB)\nRETURNS JSONB LANGUAGE plpgsql AS $$\nDECLARE\n    v_rec JSONB;\nBEGIN\n",
            proc_name
        );

        match crate::pushdown::body(method) {
            Ok(translated) => sql.push_str(&translated),
            Err(reason) => {
                sql.push_str(&format!(
                    "    RAISE EXCEPTION '{}';\n",
                    reason.replace('\'', "''")
                ));
            }
        }

        sql.push_str("    RETURN p_params;\nEND;\n$$;\n");
        sql
    }

    /// Generate PostgreSQL DDL for a BOS View (Normal or Materialized View)
    pub fn generate_view_ddl(&self, view: &bosc_ir::ViewIR) -> String {
        let mut ddl = String::new();
        let _view_type_str = if view.is_materialized { "MATERIALIZED VIEW" } else { "VIEW" };
        let view_name = &view.sql_view_name;

        ddl.push_str(&format!("-- BOS View: {}\n", view.name));

        let mut select_cols = Vec::new();
        if view.select_fields.is_empty() {
            select_cols.push("*".to_string());
        } else {
            for sf in &view.select_fields {
                if let Some(alias) = &sf.alias {
                    select_cols.push(format!("{} AS \"{}\"", sf.expr, alias));
                } else {
                    select_cols.push(sf.expr.clone());
                }
            }
        }

        let mut joins_sql = String::new();
        for j in &view.joins {
            let alias_clause = j.alias.as_ref().map(|a| format!(" AS \"{}\"", a)).unwrap_or_default();
            joins_sql.push_str(&format!(
                "\n    {} JOIN \"{}\"{} ON {}",
                j.join_type, j.target_table, alias_clause, j.condition
            ));
        }

        let where_sql = view.where_clause.as_ref().map(|w| format!("\n    WHERE {}", w)).unwrap_or_default();

        if view.is_materialized {
            ddl.push_str(&format!("DROP MATERIALIZED VIEW IF EXISTS \"{}\" CASCADE;\n", view_name));
            ddl.push_str(&format!(
                "CREATE MATERIALIZED VIEW \"{}\" AS\n    SELECT {}\n    FROM \"{}\"{}{};\n",
                view_name,
                select_cols.join(", "),
                view.source_table,
                joins_sql,
                where_sql
            ));
            ddl.push_str(&format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS \"idx_{}_mat_pk\" ON \"{}\" (\"_id\");\n",
                view_name, view_name
            ));
        } else {
            ddl.push_str(&format!(
                "CREATE OR REPLACE VIEW \"{}\" AS\n    SELECT {}\n    FROM \"{}\"{}{};\n",
                view_name,
                select_cols.join(", "),
                view.source_table,
                joins_sql,
                where_sql
            ));
        }

        ddl
    }

    /// Generate PostgreSQL DDL for a BOS Permission Set (Catalog registration + RLS Policies)
    pub fn generate_permission_set_ddl(&self, ps: &bosc_ir::PermissionSetIR) -> String {
        let mut ddl = String::new();
        ddl.push_str(&format!("-- BOS Permission Set: {}\n", ps.name));

        for perm in &ps.permissions {
            let table_name = &perm.sql_table_name;
            let ps_name_clean = ps.name.to_lowercase().replace(' ', "_");

            if let Some(filter) = &perm.row_filter {
                let policy_name = format!("rls_{}_{}", ps_name_clean, table_name);
                ddl.push_str(&format!("ALTER TABLE IF EXISTS \"{}\" ENABLE ROW LEVEL SECURITY;\n", table_name));
                ddl.push_str(&format!("DROP POLICY IF EXISTS \"{}\" ON \"{}\";\n", policy_name, table_name));

                let mut cmd_list = Vec::new();
                if perm.can_read { cmd_list.push("SELECT"); }
                if perm.can_insert { cmd_list.push("INSERT"); }
                if perm.can_modify { cmd_list.push("UPDATE"); }
                if perm.can_delete { cmd_list.push("DELETE"); }

                let for_clause = if cmd_list.is_empty() || cmd_list.len() == 4 {
                    "FOR ALL".to_string()
                } else {
                    format!("FOR {}", cmd_list.join(", "))
                };

                ddl.push_str(&format!(
                    "CREATE POLICY \"{}\" ON \"{}\"\n    {}\n    USING ({});\n",
                    policy_name, table_name, for_clause, filter
                ));
            }
        }

        // Register Permission Set in _bos_meta_objects catalog
        let perm_json = serde_json::to_string(ps).unwrap_or_else(|_| "{}".to_string());
        ddl.push_str(&format!(
            r#"INSERT INTO "_bos_meta_objects" ("object_type", "object_name", "package_name", "caption", "definition")
VALUES ('permissionset', '{}', 'bos.core', '{}', '{}'::jsonb)
ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
SET "definition" = EXCLUDED."definition", "caption" = EXCLUDED."caption", "updated_at" = NOW();
"#,
            ps.name,
            ps.caption.as_deref().unwrap_or(&ps.name).replace('\'', "''"),
            perm_json.replace('\'', "''")
        ));

        ddl
    }

    /// Register a BOS Query in _bos_meta_objects catalog
    pub fn generate_query_catalog_ddl(&self, query: &bosc_ir::QueryIR) -> String {
        let query_json = serde_json::to_string(query).unwrap_or_else(|_| "{}".to_string());
        format!(
            r#"-- BOS Query Registration: {}
INSERT INTO "_bos_meta_objects" ("object_type", "object_name", "package_name", "caption", "definition")
VALUES ('query', '{}', 'bos.core', '{}', '{}'::jsonb)
ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE
SET "definition" = EXCLUDED."definition", "caption" = EXCLUDED."caption", "updated_at" = NOW();
"#,
            query.name,
            query.name,
            query.caption.as_deref().unwrap_or(&query.name).replace('\'', "''"),
            query_json.replace('\'', "''")
        )
    }
}

fn sift_index_ddl(ff: &bosc_ir::FlowFieldIR) -> String {
    let target = to_snake_case(&ff.target_table);
    if target.is_empty() {
        return String::new();
    }
    let mut keys = vec!["\"_tenant_id\"".to_string()];
    for column in &ff.index_columns {
        let quoted = format!("\"{}\"", column);
        if !keys.contains(&quoted) {
            keys.push(quoted);
        }
    }
    let include = ff
        .target_column
        .as_ref()
        .map(|column| format!(" INCLUDE (\"{}\")", to_snake_case(column)))
        .unwrap_or_default();
    format!(
        "CREATE INDEX IF NOT EXISTS \"idx_sift_{}_{}\" ON \"{}\" ({}){};\n",
        target,
        to_snake_case(&ff.name),
        target,
        keys.join(", "),
        include
    )
}

fn to_snake_case(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amdp_plpgsql_procedure_generation() {
        let generator = PostgresGenerator::new();
        let method = MethodIR {
            name: "PostSalesOrder".to_string(),
            is_public: true,
            is_static: false,
            is_pushdown: true,
            params: vec![("order_id".to_string(), "Guid".to_string())],
            return_type: Some("Void".to_string()),
            trigger_event: None,
            annotations: Vec::new(),
            script: None,
            actions: vec![
                ActionIR::ValidateRequired {
                    field: "order_id".to_string(),
                    location: Some(SourceLocation { file: "test.bos".into(), line: 1, column: 1 }),
                },
                ActionIR::Assert {
                    condition: "amount > 0".to_string(),
                    message: "Order amount must be positive".to_string(),
                    location: Some(SourceLocation { file: "test.bos".into(), line: 2, column: 1 }),
                },
                ActionIR::SetField {
                    field: "status".to_string(),
                    value_expr: "'Posted'".to_string(),
                    location: Some(SourceLocation { file: "test.bos".into(), line: 3, column: 1 }),
                },
                ActionIR::LogAudit {
                    message: "Order posted to ledger".to_string(),
                    location: Some(SourceLocation { file: "test.bos".into(), line: 4, column: 1 }),
                },
            ],
        };

        let plpgsql = generator.generate_plpgsql_procedure("OrderManager", &method);
        assert!(plpgsql.contains("CREATE OR REPLACE FUNCTION \"bos_proc_ordermanager_postsalesorder\""));
        assert!(plpgsql.contains("(p_params->>'order_id') IS NULL"));
        assert!(plpgsql.contains("Order amount must be positive"));
        assert!(plpgsql.contains("jsonb_set(p_params, ARRAY['status']"));
        assert!(plpgsql.contains("INSERT INTO \"_system_privileged_audit_log\""));
    }

    #[test]
    fn test_sift_covering_index_generation() {
        let generator = PostgresGenerator::new();
        let table = TableIR {
            name: "CustLedgerEntry".to_string(),
            sql_table_name: "cust_ledger_entry".to_string(),
            caption: "Customer Ledger Entry".to_string(),
            table_type: TableTypeIR::Ledger,
            data_per_company: true,
            columns: vec![
                ColumnIR {
                    name: "CustomerNo".to_string(),
                    sql_name: "customer_no".to_string(),
                    sql_type: "VARCHAR(20)".to_string(),
                    data_type: Some("String(20)".to_string()),
                    is_nullable: false,
                    is_unique: false,
                    default_val: None,
                    caption: "Customer No.".to_string(),
                    foreign_key: None,
                    check_constraint: None,
                    formula: None,
                    renamed_from: None,
                },
                ColumnIR {
                    name: "Amount".to_string(),
                    sql_name: "amount".to_string(),
                    sql_type: "DECIMAL(18,2)".to_string(),
                    data_type: Some("Decimal(18, 2)".to_string()),
                    is_nullable: false,
                    is_unique: false,
                    default_val: None,
                    caption: "Amount".to_string(),
                    foreign_key: None,
                    check_constraint: None,
                    formula: None,
                    renamed_from: None,
                },
            ],
            primary_key: vec!["customer_no".to_string()],
            indexes: vec![IndexIR {
                name: "idx_cust_ledger_sift".to_string(),
                columns: vec!["customer_no".to_string()],
                include_columns: vec!["amount".to_string()],
                is_unique: false,
            }],
            flow_fields: vec![],
            field_groups: vec![],
            triggers: vec![],
            check_constraints: vec![],
            on_delete: None,
            renamed_from: None,
        };

        let ddl = generator.generate_table(&table);
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS \"idx_cust_ledger_sift\" ON \"cust_ledger_entry\" (\"customer_no\") INCLUDE (\"amount\");"));
    }

    #[test]
    fn test_formula_and_flowfield_views() {
        let generator = PostgresGenerator::new();
        let table = TableIR {
            name: "SalesLine".to_string(),
            sql_table_name: "sales_line".to_string(),
            caption: "Sales Line".to_string(),
            table_type: TableTypeIR::Document,
            data_per_company: true,
            columns: vec![
                ColumnIR {
                    name: "Quantity".to_string(),
                    sql_name: "quantity".to_string(),
                    sql_type: "NUMERIC(18,4)".to_string(),
                    data_type: Some("Decimal(18, 4)".to_string()),
                    is_nullable: false,
                    is_unique: false,
                    default_val: Some("1.0".to_string()),
                    caption: "Quantity".to_string(),
                    foreign_key: None,
                    check_constraint: None,
                    formula: None,
                    renamed_from: None,
                },
                ColumnIR {
                    name: "UnitPrice".to_string(),
                    sql_name: "unit_price".to_string(),
                    sql_type: "NUMERIC(18,4)".to_string(),
                    data_type: Some("Decimal(18, 4)".to_string()),
                    is_nullable: false,
                    is_unique: false,
                    default_val: Some("0.0".to_string()),
                    caption: "Unit Price".to_string(),
                    foreign_key: None,
                    check_constraint: None,
                    formula: None,
                    renamed_from: None,
                },
                ColumnIR {
                    name: "LineAmount".to_string(),
                    sql_name: "line_amount".to_string(),
                    sql_type: "NUMERIC(18,4)".to_string(),
                    data_type: Some("Decimal(18, 4)".to_string()),
                    is_nullable: true,
                    is_unique: false,
                    default_val: None,
                    caption: "Line Amount".to_string(),
                    foreign_key: None,
                    check_constraint: None,
                    formula: Some("\"quantity\" * \"unit_price\"".to_string()),
                    renamed_from: None,
                },
            ],
            primary_key: vec!["_id".to_string()],
            indexes: vec![],
            flow_fields: vec![
                FlowFieldIR {
                    name: "TotalDiscount".to_string(),
                    func: FlowFieldFuncIR::Sum,
                    target_table: "SalesDiscount".to_string(),
                    target_column: Some("Amount".to_string()),
                    sql_subquery: String::new(),
                    drill_down_page: None,
                    index_columns: Vec::new(),
                }
            ],
            field_groups: vec![],
            triggers: vec![],
            check_constraints: vec![],
            on_delete: None,
            renamed_from: None,
        };

        let ddl = generator.generate_table(&table);
        // Assert generated stored column DDL
        assert!(ddl.contains("\"line_amount\" NUMERIC(18,4) GENERATED ALWAYS AS (\"quantity\" * \"unit_price\") STORED"));
        // Assert FlowField pushdown view DDL
        assert!(ddl.contains("CREATE OR REPLACE VIEW \"sales_line_view\" AS SELECT t.*, (SELECT COALESCE(SUM(\"amount\"), 0) FROM \"sales_discount\" WHERE \"_tenant_id\" = t.\"_tenant_id\") AS \"total_discount\" FROM \"sales_line\" t;"));
    }

    fn plain_table(name: &str, sql: &str, columns: Vec<ColumnIR>) -> TableIR {
        TableIR {
            name: name.to_string(),
            sql_table_name: sql.to_string(),
            caption: name.to_string(),
            table_type: TableTypeIR::Normal,
            data_per_company: true,
            columns,
            primary_key: vec!["no".to_string()],
            indexes: vec![],
            flow_fields: vec![],
            field_groups: vec![],
            triggers: vec![],
            check_constraints: vec![],
            on_delete: None,
            renamed_from: None,
        }
    }

    fn text_column(name: &str) -> ColumnIR {
        ColumnIR {
            name: name.to_string(),
            sql_name: name.to_string(),
            sql_type: "VARCHAR(20)".to_string(),
            data_type: Some("String(20)".to_string()),
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

    #[test]
    fn relations_are_emitted_after_all_tables() {
        let generator = PostgresGenerator::new();
        let mut customer_no = text_column("customer_no");
        customer_no.foreign_key = Some(ForeignKeyIR {
            target_table: "customer".to_string(),
            target_column: "no".to_string(),
            on_delete: "CASCADE".to_string(),
        });
        let order = plain_table("SalesOrder", "sales_order", vec![text_column("no"), customer_no]);
        let customer = plain_table("Customer", "customer", vec![text_column("no")]);

        let ddl = generator.generate_tables(&[order, customer]);
        let rel = "SELECT bos_ensure_relation(current_schema(), 'sales_order', 'customer_no', 'customer', 'no', 'CASCADE', '');";
        let rel_pos = ddl.find(rel).expect("relation statement emitted");
        let customer_pos = ddl.find("CREATE TABLE IF NOT EXISTS \"customer\"").expect("customer created");
        assert!(customer_pos < rel_pos, "relation must follow the target table");
    }

    #[test]
    fn flowfield_views_and_sift_indexes_are_emitted_after_all_tables() {
        let generator = PostgresGenerator::new();
        let mut user = plain_table("User", "user", vec![text_column("username")]);
        user.flow_fields = vec![FlowFieldIR {
            name: "ActiveSessionCount".to_string(),
            func: FlowFieldFuncIR::Count,
            target_table: "UserSession".to_string(),
            target_column: Some("Username".to_string()),
            sql_subquery: "(SELECT COUNT(*) FROM \"user_session\" WHERE (\"user_session\".\"username\" = \"user\".\"username\" AND \"user_session\".\"session_state\" = 'Active'))".to_string(),
            drill_down_page: None,
            index_columns: vec!["username".to_string()],
        }];
        let session = plain_table("UserSession", "user_session", vec![text_column("username")]);

        let ddl = generator.generate_tables(&[user, session]);
        let user_tbl_pos = ddl.find("CREATE TABLE IF NOT EXISTS \"user\"").expect("user table created");
        let session_tbl_pos = ddl.find("CREATE TABLE IF NOT EXISTS \"user_session\"").expect("user_session table created");
        let view_pos = ddl.find("CREATE OR REPLACE VIEW \"user_view\"").expect("user_view emitted");
        let sift_pos = ddl.find("CREATE INDEX IF NOT EXISTS \"idx_sift_user_session_active_session_count\"").expect("sift index emitted");

        assert!(user_tbl_pos < session_tbl_pos);
        assert!(session_tbl_pos < view_pos, "flowfield view must be emitted after target table is created");
        assert!(session_tbl_pos < sift_pos, "sift index must be emitted after target table is created");
        assert!(ddl.contains("t.\"username\""), "subquery parent table reference must be rewritten to t.");
        assert!(!ddl.contains("\"user\".\"username\""), "unaliased parent table reference must not leak into view");
    }

    #[test]
    fn relations_cover_every_company_copy() {
        let generator = PostgresGenerator::with_strategy(StorageStrategyIR::TablePerCompany);
        let mut item_no = text_column("item_no");
        item_no.foreign_key = Some(ForeignKeyIR {
            target_table: "item".to_string(),
            target_column: "no".to_string(),
            on_delete: "RESTRICT".to_string(),
        });
        let line = plain_table("SalesLine", "sales_line", vec![item_no]);
        let ddl = generator.generate_relations(&line);
        assert!(ddl.contains("'item_no', 'item', 'no', 'RESTRICT', 'default$');"));
        assert!(ddl.contains("'item_no', 'item', 'no', 'RESTRICT', '');"));
    }

    #[test]
    fn kernel_ddl_installs_relation_support() {
        let ddl = PostgresGenerator::new().generate_system_kernel_ddl();
        assert!(ddl.contains("CREATE OR REPLACE FUNCTION bos_ensure_relation("));
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS \"_bos_schema_migrations\""));
    }

    #[test]
    fn table_extension_uses_sql_names() {
        let generator = PostgresGenerator::new();
        let mut loyalty = text_column("LoyaltyPoints");
        loyalty.sql_name = "loyalty_points".to_string();
        let ext = TableExtensionIR {
            name: "CustomerLoyalty".to_string(),
            package_name: "acme".to_string(),
            target_table: "SalesHeader".to_string(),
            added_columns: vec![loyalty],
            added_indexes: vec![],
        };
        let ddl = generator.generate_table_extension_ddl(&ext, &[]);
        assert!(ddl.contains("ALTER TABLE IF EXISTS \"public\".\"sales_header\" ADD COLUMN IF NOT EXISTS \"loyalty_points\" VARCHAR(20);"));
    }

    #[test]
    fn test_view_and_permissionset_ddl() {
        let generator = PostgresGenerator::new();

        let view = ViewIR {
            name: "CustomerBalanceView".to_string(),
            sql_view_name: "customer_balance_view".to_string(),
            is_materialized: false,
            refresh_interval: None,
            select_fields: vec![
                ViewSelectFieldIR { expr: "c.id".to_string(), alias: Some("customer_id".to_string()) },
                ViewSelectFieldIR { expr: "c.name".to_string(), alias: None },
            ],
            source_table: "customer".to_string(),
            joins: vec![],
            where_clause: Some("c.is_active = TRUE".to_string()),
        };

        let view_ddl = generator.generate_view_ddl(&view);
        assert!(view_ddl.contains("CREATE OR REPLACE VIEW \"customer_balance_view\" AS"));
        assert!(view_ddl.contains("SELECT c.id AS \"customer_id\", c.name"));
        assert!(view_ddl.contains("FROM \"customer\""));
        assert!(view_ddl.contains("WHERE c.is_active = TRUE"));

        let perm_set = PermissionSetIR {
            name: "AuditorRole".to_string(),
            caption: Some("Auditor Role".to_string()),
            permissions: vec![
                TablePermissionIR {
                    table_name: "GeneralLedger".to_string(),
                    sql_table_name: "general_ledger".to_string(),
                    can_read: true,
                    can_insert: false,
                    can_modify: false,
                    can_delete: false,
                    row_filter: Some("\"general_ledger\".\"fiscal_year\" = 2026".to_string()),
                    exclude_fields: vec![],
                }
            ],
            execute: vec![],
        };

        let ps_ddl = generator.generate_permission_set_ddl(&perm_set);
        assert!(ps_ddl.contains("CREATE POLICY \"rls_auditorrole_general_ledger\" ON \"general_ledger\""));
        assert!(ps_ddl.contains("FOR SELECT"));
        assert!(ps_ddl.contains("USING (\"general_ledger\".\"fiscal_year\" = 2026)"));
        assert!(ps_ddl.contains("INSERT INTO \"_bos_meta_objects\""));
    }

    #[test]
    fn test_virtual_temporary_temporal_and_company_rls_ddl() {
        let generator = PostgresGenerator::new();

        let mut virtual_tbl = plain_table("DateBuffer", "date_buffer", vec![text_column("period_no")]);
        virtual_tbl.table_type = TableTypeIR::Virtual;
        let v_ddl = generator.generate_table_structure(&virtual_tbl);
        assert!(v_ddl.contains("Virtual Table DateBuffer (in-memory runtime projection; no physical DDL emitted)"));
        assert!(!v_ddl.contains("CREATE TABLE"));

        let mut temp_tbl = plain_table("PostingBuffer", "posting_buffer", vec![text_column("entry_no")]);
        temp_tbl.table_type = TableTypeIR::Temporary;
        let tmp_ddl = generator.generate_table_structure(&temp_tbl);
        assert!(tmp_ddl.contains("CREATE UNLOGGED TABLE IF NOT EXISTS \"posting_buffer\""));

        let mut temporal_tbl = plain_table("Contract", "contract", vec![text_column("no")]);
        temporal_tbl.table_type = TableTypeIR::Temporal;
        temporal_tbl.data_per_company = true;
        let t_ddl = generator.generate_table_structure(&temporal_tbl);
        assert!(t_ddl.contains("CREATE TABLE IF NOT EXISTS \"contract_history\""));
        assert!(t_ddl.contains("CREATE TRIGGER \"trg_temporal_contract\""));
        assert!(t_ddl.contains("current_setting('bos.current_company_id', true)"));
    }

    #[test]
    fn test_company_only_rls_and_object_map() {
        let generator = PostgresGenerator::new();

        // 1. Kernel DDL includes _bos_object_map and nullable tenant_id on _bos_companies
        let kernel_ddl = generator.generate_system_kernel_ddl();
        assert!(kernel_ddl.contains("CREATE TABLE IF NOT EXISTS \"_bos_object_map\""));
        assert!(kernel_ddl.contains("idx_bos_object_map_lookup"));
        assert!(kernel_ddl.contains("uq_bos_company_code_global"));
        assert!(kernel_ddl.contains("ALTER TABLE \"_bos_companies\" ALTER COLUMN \"tenant_id\" DROP NOT NULL"));

        // 2. Table with data_per_company generates RESTRICTIVE policy and FORCE ROW LEVEL SECURITY
        let mut order_tbl = plain_table("OrderHeader", "order_header", vec![text_column("order_no")]);
        order_tbl.data_per_company = true;
        let ddl = generator.generate_table_structure(&order_tbl);
        assert!(ddl.contains("ALTER TABLE \"order_header\" FORCE ROW LEVEL SECURITY;"));
        assert!(ddl.contains("CREATE POLICY \"bos_base_order_header\" ON \"order_header\""));
        assert!(ddl.contains("CREATE POLICY \"bos_isolation_order_header\" ON \"order_header\""));
        assert!(ddl.contains("AS RESTRICTIVE"));
        assert!(ddl.contains("NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \"_tenant_id\" IS NULL"));

        // 3. AOT prepared statements use IS NOT DISTINCT FROM
        let aot = generator.generate_aot_prepared_queries(&[order_tbl]);
        let get_by_id = aot.iter().find(|q| q.kind == "get_by_id").expect("get_by_id exists");
        assert!(get_by_id.sql.contains("\"_tenant_id\" IS NOT DISTINCT FROM $2"));
        assert!(get_by_id.sql.contains("\"_company_id\" IS NOT DISTINCT FROM $3"));
    }


    #[test]
    fn test_immutable_ledger_trigger_generation() {
        let generator = PostgresGenerator::new();
        let mut ledger_tbl = plain_table("CustLedgerEntry", "cust_ledger_entry", vec![text_column("entry_no")]);
        ledger_tbl.table_type = TableTypeIR::Ledger;

        let ddl = generator.generate_table_structure(&ledger_tbl);
        assert!(ddl.contains("CREATE OR REPLACE FUNCTION \"guard_immutable_cust_ledger_entry\"()"));
        assert!(ddl.contains("CANNOT % IMMUTABLE LEDGER TABLE:"));
        assert!(ddl.contains("CREATE TRIGGER \"trg_immutable_cust_ledger_entry\""));
        assert!(ddl.contains("BEFORE UPDATE OR DELETE ON \"cust_ledger_entry\""));
    }
}
