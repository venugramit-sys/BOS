-- Insert shared master into public.tenant_master
INSERT INTO public.tenant_master ("_id", "_tenant_id", "code", "name", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), 'cf89bc0d-c954-407d-860e-136c4f3b75c9'::uuid, 'SHARED_PAYTERM_30D', 'Net 30 Days', NOW(), NOW())
ON CONFLICT ("code") DO NOTHING;

-- Insert into main_company$company_transaction
INSERT INTO public."main_company$company_transaction" ("_id", "_tenant_id", "_company_id", "entry_no", "document_no", "amount", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), 'cf89bc0d-c954-407d-860e-136c4f3b75c9'::uuid, '33eb729f-7d68-435d-9a83-15e6808f426b'::uuid, 301, 'BC-MAIN-001', 75000.00, NOW(), NOW())
ON CONFLICT ("entry_no") DO NOTHING;

-- Insert into default$company_transaction
INSERT INTO public."default$company_transaction" ("_id", "_tenant_id", "_company_id", "entry_no", "document_no", "amount", "_created_at", "_updated_at")
VALUES (gen_random_uuid(), 'cf89bc0d-c954-407d-860e-136c4f3b75c9'::uuid, '00000000-0000-0000-0000-000000000000'::uuid, 302, 'BC-DEF-001', 12000.00, NOW(), NOW())
ON CONFLICT ("entry_no") DO NOTHING;

-- Session 1: MAIN_COMPANY table queries
SELECT '=== [1] TABLE_PER_COMPANY: MAIN_COMPANY Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [1] TABLE_PER_COMPANY: MAIN_COMPANY Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM "main_company$company_transaction";

-- Session 2: DEFAULT table queries
SELECT '=== [2] TABLE_PER_COMPANY: DEFAULT Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [2] TABLE_PER_COMPANY: DEFAULT Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM "default$company_transaction";
