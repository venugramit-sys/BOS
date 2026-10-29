file_lib = "crates/bosc-runtime/src/lib.rs"
with open(file_lib, "r", encoding="utf-8") as f:
    content = f.read()

# Replace the test
start_marker = "fn test_company_only_mode_and_uuid_v7() {"
end_marker = "fn test_sanitize_identifier() {"

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

correct_test = """fn test_company_only_mode_and_uuid_v7() {
        // 1. UUIDv7 generation
        let id1 = Uuid::now_v7();
        let id2 = Uuid::now_v7();
        assert!(id2 >= id1, "UUIDv7 must be time-ordered");

        // 2. CompanyInfo in company-only mode (tenant_id: None)
        let company = CompanyInfo {
            company_id: id1,
            tenant_id: None,
            code: "CRONUS".to_string(),
            name: "Cronus Enterprise".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
        };
        let serialized = serde_json::to_string(&company).expect("serializes");
        let deserialized: CompanyInfo = serde_json::from_str(&serialized).expect("deserializes");
        assert_eq!(deserialized.company_id, id1);
        assert_eq!(deserialized.tenant_id, None);
        assert_eq!(deserialized.code, "CRONUS");

        // 3. insert_tuple in company-only mode emits NULL for _tenant_id and UUID for _company_id
        let ctx = SessionContext {
            tenant_id: Uuid::nil(),
            company_id: id1,
            user_id: id2,
            roles: vec!["admin".to_string()],
        };
        let mut obj = serde_json::Map::new();
        obj.insert("code".to_string(), serde_json::json!("CUST001"));
        obj.insert("name".to_string(), serde_json::json!("Acme Corp"));

        let (cols, vals) = Runtime::insert_tuple(&obj, id2, &ctx, true).expect("insert_tuple");
        assert!(cols.contains(&"\\"_tenant_id\\"".to_string()));
        assert!(cols.contains(&"\\"_company_id\\"".to_string()));

        let tenant_val_idx = cols.iter().position(|c| c == "\\"_tenant_id\\"").unwrap();
        assert_eq!(vals[tenant_val_idx], "NULL");

        let comp_val_idx = cols.iter().position(|c| c == "\\"_company_id\\"").unwrap();
        assert_eq!(vals[comp_val_idx], format!("'{}'::uuid", id1));
    }

    #[test]
    """

assert start_idx != -1 and end_idx != -1
content = content[:start_idx] + correct_test + content[end_idx:]

with open(file_lib, "w", encoding="utf-8") as f:
    f.write(content)

print("Updated test_company_only_mode_and_uuid_v7")
