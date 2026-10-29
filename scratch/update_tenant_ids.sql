DELETE FROM _bos_tenants;
DELETE FROM _bos_companies;

INSERT INTO _bos_tenants (tenant_id, code, name, is_active)
VALUES ('61aaf131-dea5-4341-9cb8-8d194fef55b6', 'DEFAULT_TENANT', 'Default Tenant', true);

INSERT INTO _bos_companies (company_id, tenant_id, code, name, is_active)
VALUES ('c1aaf131-dea5-4341-9cb8-8d194fef55b6', '61aaf131-dea5-4341-9cb8-8d194fef55b6', 'MAIN_COMPANY', 'Main Operating Company', true);

UPDATE tenant_master SET _tenant_id = '61aaf131-dea5-4341-9cb8-8d194fef55b6';
UPDATE company_transaction SET _tenant_id = '61aaf131-dea5-4341-9cb8-8d194fef55b6', _company_id = 'c1aaf131-dea5-4341-9cb8-8d194fef55b6';
UPDATE "user" SET _tenant_id = '61aaf131-dea5-4341-9cb8-8d194fef55b6';
