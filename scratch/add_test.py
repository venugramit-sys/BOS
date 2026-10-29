file_path = 'crates/bosc-codegen-db/src/lib.rs'
with open(file_path, 'r', encoding='utf-8') as f:
    content = f.read()

new_test = '''
    #[test]
    fn test_company_only_rls_and_object_map() {
        let generator = PostgresGenerator::new();

        // 1. Kernel DDL includes _bos_object_map and nullable tenant_id on _bos_companies
        let kernel_ddl = generator.generate_system_kernel_ddl();
        assert!(kernel_ddl.contains("CREATE TABLE IF NOT EXISTS \\"_bos_object_map\\""));
        assert!(kernel_ddl.contains("idx_bos_object_map_lookup"));
        assert!(kernel_ddl.contains("uq_bos_company_code_global"));
        assert!(kernel_ddl.contains("ALTER TABLE \\"_bos_companies\\" ALTER COLUMN \\"tenant_id\\" DROP NOT NULL"));

        // 2. Table with data_per_company generates RESTRICTIVE policy and FORCE ROW LEVEL SECURITY
        let mut order_tbl = plain_table("OrderHeader", "order_header", vec![text_column("order_no")]);
        order_tbl.data_per_company = true;
        let ddl = generator.generate_table_structure(&order_tbl);
        assert!(ddl.contains("ALTER TABLE \\"order_header\\" FORCE ROW LEVEL SECURITY;"));
        assert!(ddl.contains("CREATE POLICY \\"bos_base_order_header\\" ON \\"order_header\\""));
        assert!(ddl.contains("CREATE POLICY \\"bos_isolation_order_header\\" ON \\"order_header\\""));
        assert!(ddl.contains("AS RESTRICTIVE"));
        assert!(ddl.contains("NULLIF(current_setting('bos.current_tenant_id', true), '') IS NULL AND \\"_tenant_id\\" IS NULL"));

        // 3. AOT prepared statements use IS NOT DISTINCT FROM
        let aot = generator.generate_aot_prepared_queries(&[order_tbl]);
        let get_by_id = aot.iter().find(|q| q.kind == "get_by_id").expect("get_by_id exists");
        assert!(get_by_id.sql.contains("\\"_tenant_id\\" IS NOT DISTINCT FROM $2"));
        assert!(get_by_id.sql.contains("\\"_company_id\\" IS NOT DISTINCT FROM $3"));
    }
'''

target = "assert!(t_ddl.contains(\"current_setting('bos.current_company_id', true)\"));\n    }"
assert target in content, "target not found"
content = content.replace(target, target + "\n" + new_test, 1)

with open(file_path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Added test_company_only_rls_and_object_map successfully")
