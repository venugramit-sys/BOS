-- Insert shared master into public.tenant_master
INSERT INTO public.tenant_master ("_id", "_tenant_id", "code", "name", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), '1723a134-06b7-4441-8992-96cd93ae6571'::uuid, 'SHARED_UOM_KG', 'Kilogram', NOW(), NOW())
ON CONFLICT ("code") DO NOTHING;

-- Insert company transaction for comp_main_company
INSERT INTO comp_main_company.company_transaction ("_id", "_tenant_id", "_company_id", "entry_no", "document_no", "amount", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), '1723a134-06b7-4441-8992-96cd93ae6571'::uuid, '2a7e7014-116a-4144-bbce-35d91c290a68'::uuid, 201, 'TX-MAIN-001', 8900.50, NOW(), NOW())
ON CONFLICT ("entry_no") DO NOTHING;

-- Insert company transaction for comp_default
INSERT INTO comp_default.company_transaction ("_id", "_tenant_id", "_company_id", "entry_no", "document_no", "amount", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), '1723a134-06b7-4441-8992-96cd93ae6571'::uuid, '00000000-0000-0000-0000-000000000000'::uuid, 202, 'TX-DEF-001', 3400.00, NOW(), NOW())
ON CONFLICT ("entry_no") DO NOTHING;

-- Session 1: Search path set to MAIN_COMPANY
SET search_path = "comp_main_company", public;

SELECT '=== [1] SCHEMA_PER_COMPANY: MAIN_COMPANY Session - Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [1] SCHEMA_PER_COMPANY: MAIN_COMPANY Session - Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM company_transaction;

-- Session 2: Search path set to DEFAULT company
SET search_path = "comp_default", public;

SELECT '=== [2] SCHEMA_PER_COMPANY: DEFAULT Session - Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [2] SCHEMA_PER_COMPANY: DEFAULT Session - Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM company_transaction;
