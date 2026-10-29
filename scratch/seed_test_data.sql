INSERT INTO tenant_master ("_id", "_tenant_id", "code", "name", "_created_at", "_updated_at", "_system_version")
VALUES ('17c9d982-38a5-451f-84b5-7971887eded2', '61aaf131-dea5-4341-9cb8-8d194fef55b6', 'SHARED_CURRENCY_USD', 'United States Dollar', NOW(), NOW(), 1)
ON CONFLICT DO NOTHING;

INSERT INTO company_transaction ("_id", "_tenant_id", "_company_id", "entry_no", "document_no", "amount", "_created_at", "_updated_at", "_system_version")
VALUES ('f131c358-5542-4e47-bdc2-92c743fc8bcf', '61aaf131-dea5-4341-9cb8-8d194fef55b6', '00000000-0000-0000-0000-000000000000', 1001, 'DOC-2026-001', 2500.50, NOW(), NOW(), 1)
ON CONFLICT DO NOTHING;
