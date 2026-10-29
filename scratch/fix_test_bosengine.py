file_lib = "crates/bosc-runtime/src/lib.rs"
with open(file_lib, "r", encoding="utf-8") as f:
    content = f.read()

target = """    #[test]

    
    #[test]
    fn test_company_only_mode_and_uuid_v7() {
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

        let (cols, vals) = Runtime::insert_tuple(&obj, id2, &ctx, true).expect("insert_tuple");"""

replacement = """    #[test]
    fn test_company_only_mode_and_uuid_v7() {
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

        let (cols, vals) = BosEngine::insert_tuple(&obj, id2, &ctx, true).expect("insert_tuple");"""

assert target in content, "target not found"
content = content.replace(target, replacement, 1)

with open(file_lib, "w", encoding="utf-8") as f:
    f.write(content)

print("Fixed BosEngine in test")
