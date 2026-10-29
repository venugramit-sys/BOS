import sys

file_path = "crates/bosc-codegen-db/src/lib.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 1. Add _bos_object_map before 7. Multi-Tenant Registry
object_map_ddl = """        // 5. Physical Object Mapping Registry (ADR 0003 Multi-Strategy Provisioning)
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

"""
target_reg = "        // 7. Multi-Tenant Registry"
if target_reg in content and "_bos_object_map" not in content:
    content = content.replace(target_reg, object_map_ddl + target_reg, 1)
    print("Added _bos_object_map")

# 2. Update _bos_companies
target_comp = """        // 8. Company Isolation Registry
        sql.push_str(r#"-- 8. Company Isolation Registry
CREATE TABLE IF NOT EXISTS "_bos_companies" (
    "company_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "tenant_id" UUID NOT NULL REFERENCES "_bos_tenants"("tenant_id") ON DELETE CASCADE,
    "code" VARCHAR(50) NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_bos_tenant_company" UNIQUE ("tenant_id", "code")
);

"#);"""

replacement_comp = """        // 8. Company Isolation Registry
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

"#);"""

if target_comp in content:
    content = content.replace(target_comp, replacement_comp, 1)
    print("Updated _bos_companies")

# 3. Update AOT prepared statements GetById, Exists, UpdateOptimistic, Delete, SIFT to IS NOT DISTINCT FROM
target_getbyid_comp = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" = $2 AND \\"_company_id\\" = $3 LIMIT 1;'
replacement_getbyid_comp = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" IS NOT DISTINCT FROM $2 AND \\"_company_id\\" IS NOT DISTINCT FROM $3 LIMIT 1;'
if target_getbyid_comp in content:
    content = content.replace(target_getbyid_comp, replacement_getbyid_comp)
    print("Updated GetById company statement")

target_getbyid_tbl = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" = $2 LIMIT 1;'
replacement_getbyid_tbl = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" IS NOT DISTINCT FROM $2 LIMIT 1;'
if target_getbyid_tbl in content:
    content = content.replace(target_getbyid_tbl, replacement_getbyid_tbl)
    print("Updated GetById table statement")

target_exists = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" = $2);'
replacement_exists = 'WHERE \\"{}\\" = $1 AND \\"_tenant_id\\" IS NOT DISTINCT FROM $2);'
if target_exists in content:
    content = content.replace(target_exists, replacement_exists)
    print("Updated Exists statement")

target_update = 'WHERE \\"{}\\" = ${} AND \\"_system_version\\" = ${} AND \\"_tenant_id\\" = ${} RETURNING *;'
replacement_update = 'WHERE \\"{}\\" = ${} AND \\"_system_version\\" = ${} AND \\"_tenant_id\\" IS NOT DISTINCT FROM ${} RETURNING *;'
if target_update in content:
    content = content.replace(target_update, replacement_update)
    print("Updated UpdateOptimistic statement")

target_delete = 'WHERE \\"{}\\" = $1 AND \\"_system_version\\" = $2 AND \\"_tenant_id\\" = $3 RETURNING *;'
replacement_delete = 'WHERE \\"{}\\" = $1 AND \\"_system_version\\" = $2 AND \\"_tenant_id\\" IS NOT DISTINCT FROM $3 RETURNING *;'
if target_delete in content:
    content = content.replace(target_delete, replacement_delete)
    print("Updated Delete statement")

target_sift = 'WHERE \\"_tenant_id\\" = $1;'
replacement_sift = 'WHERE \\"_tenant_id\\" IS NOT DISTINCT FROM $1;'
if target_sift in content:
    content = content.replace(target_sift, replacement_sift)
    print("Updated SIFT statement")

# 4. In generate_table_structure:
# Replace `    \"_tenant_id\" UUID NOT NULL,\n` with `    \"_tenant_id\" UUID,\n`
content = content.replace('    \"_tenant_id\" UUID NOT NULL,\n', '    \"_tenant_id\" UUID,\n')
# Replace `    \"_company_id\" UUID NOT NULL,\n` with `    \"_company_id\" UUID,\n`
content = content.replace('    \"_company_id\" UUID NOT NULL,\n', '    \"_company_id\" UUID,\n')
print("Updated table column nullability for _tenant_id and _company_id")

# 5. Temporal history table _tenant_id
target_temporal = """CREATE TABLE IF NOT EXISTS "{table_name}_history" (
    "_history_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "_record_id" UUID NOT NULL,
    "_tenant_id" UUID NOT NULL,"""
replacement_temporal = """CREATE TABLE IF NOT EXISTS "{table_name}_history" (
    "_history_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "_record_id" UUID NOT NULL,
    "_tenant_id" UUID,"""
if target_temporal in content:
    content = content.replace(target_temporal, replacement_temporal)
    print("Updated Temporal history table _tenant_id")

# 6. Row-Level Security overhaul
target_rls = """        // Row-Level Security
        if self.strategy == StorageStrategyIR::SharedRLS || self.strategy == StorageStrategyIR::Partitioned {
            ddl.push_str(&format!("ALTER TABLE \\"{}\\" ENABLE ROW LEVEL SECURITY;\\n", table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \\"tenant_isolation_{}\\" ON \\"{}\\";\\n", table.sql_table_name, table.sql_table_name));
            if table.data_per_company {
                ddl.push_str(&format!(
                    "CREATE POLICY \\"tenant_isolation_{}\\" ON \\"{}\\"\\n    FOR ALL\\n    USING (\\n        \\"_tenant_id\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid\\n        AND (\\n            \\"_company_id\\" = COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), ''))::uuid\\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NULL\\n            OR \\"_company_id\\" IS NULL\\n        )\\n    );\\n",
                    table.sql_table_name, table.sql_table_name
                ));
            } else {
                ddl.push_str(&format!(
                    "CREATE POLICY \\"tenant_isolation_{}\\" ON \\"{}\\"\\n    FOR ALL\\n    USING (\\n        \\"_tenant_id\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid\\n    );\\n",
                    table.sql_table_name, table.sql_table_name
                ));
            }
        }"""

replacement_rls = """        // Row-Level Security: Multi-tenant and Company-Only isolation
        if self.strategy == StorageStrategyIR::SharedRLS || self.strategy == StorageStrategyIR::Partitioned {
            ddl.push_str(&format!("ALTER TABLE \\"{}\\" ENABLE ROW LEVEL SECURITY;\\n", table.sql_table_name));
            ddl.push_str(&format!("ALTER TABLE \\"{}\\" FORCE ROW LEVEL SECURITY;\\n", table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \\"tenant_isolation_{}\\" ON \\"{}\\";\\n", table.sql_table_name, table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \\"bos_isolation_{}\\" ON \\"{}\\";\\n", table.sql_table_name, table.sql_table_name));
            ddl.push_str(&format!("DROP POLICY IF EXISTS \\"bos_base_{}\\" ON \\"{}\\";\\n", table.sql_table_name, table.sql_table_name));

            // Base permissive policy allowing operations subjected to restrictive isolation
            ddl.push_str(&format!(
                "CREATE POLICY \\"bos_base_{}\\" ON \\"{}\\"\\n    FOR ALL\\n    USING (true)\\n    WITH CHECK (true);\\n",
                table.sql_table_name, table.sql_table_name
            ));

            if table.data_per_company {
                ddl.push_str(&format!(
                    "CREATE POLICY \\"bos_isolation_{tbl}\\" ON \\"{tbl}\\"\\n    AS RESTRICTIVE\\n    FOR ALL\\n    USING (\\n        (\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \\\"_tenant_id\\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\\n            OR\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \\\"_tenant_id\\\" IS NULL)\\n        )\\n        AND (\\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL AND (\\\"_company_id\\\" = COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), ''))::uuid OR \\\"_company_id\\\" IS NULL))\\n            OR\\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NULL AND (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL OR \\\"_company_id\\\" IS NULL))\\n        )\\n        AND (\\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\\n            OR (\\\"_tenant_id\\\" IS NULL AND \\\"_company_id\\\" IS NULL)\\n        )\\n    )\\n    WITH CHECK (\\n        (\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \\\"_tenant_id\\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\\n            OR\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \\\"_tenant_id\\\" IS NULL)\\n        )\\n        AND (\\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL AND (\\\"_company_id\\\" = COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), ''))::uuid OR \\\"_company_id\\\" IS NULL))\\n            OR\\n            (COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NULL AND (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL OR \\\"_company_id\\\" IS NULL))\\n        )\\n        AND (\\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\\n            OR (\\\"_tenant_id\\\" IS NULL AND \\\"_company_id\\\" IS NULL)\\n        )\\n    );\\n",
                    tbl = table.sql_table_name
                ));
            } else {
                ddl.push_str(&format!(
                    "CREATE POLICY \\"bos_isolation_{tbl}\\" ON \\"{tbl}\\"\\n    AS RESTRICTIVE\\n    FOR ALL\\n    USING (\\n        (\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \\\"_tenant_id\\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\\n            OR\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \\\"_tenant_id\\\" IS NULL)\\n        )\\n        AND (\\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\\n            OR \\\"_tenant_id\\\" IS NULL\\n        )\\n    )\\n    WITH CHECK (\\n        (\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL AND \\\"_tenant_id\\\" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)\\n            OR\\n            (NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \\\"_tenant_id\\\" IS NULL)\\n        )\\n        AND (\\n            NULLIF(current_setting('bos.current_tenant_id', true), '') IS NOT NULL\\n            OR COALESCE(NULLIF(current_setting('bos.current_company_id', true), ''), NULLIF(current_setting('bos.company_id', true), '')) IS NOT NULL\\n            OR \\\"_tenant_id\\\" IS NULL\\n        )\\n    );\\n",
                    tbl = table.sql_table_name
                ));
            }
        }"""

if target_rls in content:
    content = content.replace(target_rls, replacement_rls, 1)
    print("Updated RLS generation")
else:
    print("WARNING: target_rls not found")

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Finished update_codegen_db.py")
