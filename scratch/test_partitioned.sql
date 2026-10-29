-- 1. Insert Tenant B into _bos_tenants
INSERT INTO _bos_tenants (tenant_id, code, name, is_active, created_at)
VALUES ('b1111111-1111-1111-1111-111111111111'::uuid, 'TENANT_B', 'Tenant B Corp', true, NOW())
ON CONFLICT (code) DO NOTHING;

-- 2. Create partition for Tenant B on tenant_master and company_transaction
SELECT bos_create_table_partition('tenant_master', 'b1111111-1111-1111-1111-111111111111'::uuid, 'tenant_b');
SELECT bos_create_table_partition('company_transaction', 'b1111111-1111-1111-1111-111111111111'::uuid, 'tenant_b');

-- 3. Insert record for Tenant A into root table (PostgreSQL automatically routes to tenant_master_tenant_default_tenant)
INSERT INTO tenant_master ("_id", "_tenant_id", "code", "name", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), '3846f355-2c8f-42a9-afb7-75d5ad612521'::uuid, 'TENANT_A_ITEM', 'Tenant A Exclusive Item', NOW(), NOW())
ON CONFLICT ("_tenant_id", "code") DO NOTHING;

-- 4. Insert record for Tenant B into root table (PostgreSQL automatically routes to tenant_master_tenant_tenant_b)
INSERT INTO tenant_master ("_id", "_tenant_id", "code", "name", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), 'b1111111-1111-1111-1111-111111111111'::uuid, 'TENANT_B_ITEM', 'Tenant B Exclusive Item', NOW(), NOW())
ON CONFLICT ("_tenant_id", "code") DO NOTHING;

-- 5. Query the PHYSICAL partition tables directly to prove PostgreSQL physical partition routing!
SELECT '=== Tenant A Partition (tenant_master_tenant_default_tenant) ===' AS partition_test;
SELECT "code", "name", "_tenant_id" FROM "tenant_master_tenant_default_tenant";

SELECT '=== Tenant B Partition (tenant_master_tenant_tenant_b) ===' AS partition_test;
SELECT "code", "name", "_tenant_id" FROM "tenant_master_tenant_tenant_b";
