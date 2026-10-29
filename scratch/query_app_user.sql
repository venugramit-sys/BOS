-- Session 1: Context for MAIN_COMPANY
SET bos.current_tenant_id = '61aaf131-dea5-4341-9cb8-8d194fef55b6';
SET bos.current_company_id = '5942be32-9e2b-4c64-b7bd-f8a295e274db';

SELECT '=== [1] MAIN_COMPANY Session - Shared Tenant Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [1] MAIN_COMPANY Session - Company Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM company_transaction;

-- Session 2: Context for SUBSIDIARY_EU
SET bos.current_company_id = 'c2222222-2222-2222-2222-222222222222';

SELECT '=== [2] SUBSIDIARY_EU Session - Shared Tenant Master ===' AS test_scenario;
SELECT "code", "name" FROM tenant_master;

SELECT '=== [2] SUBSIDIARY_EU Session - Company Transactions ===' AS test_scenario;
SELECT "entry_no", "document_no", "amount" FROM company_transaction;
