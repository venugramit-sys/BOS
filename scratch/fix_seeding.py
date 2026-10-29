with open('crates/bosc-runtime/src/package_manager.rs', 'r', encoding='utf-8') as f:
    text = f.read()

old_block = """        // 1. Ensure Roles (SUPER, ADMIN) exist in "role"
        let insert_roles = r#"
            INSERT INTO "role" ("_id", "_tenant_id", "code", "name", "description", "is_system_role", "is_active", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, 'SUPER', 'Super Administrator', 'Full unrestricted platform access', true, true, NOW(), NOW()
            WHERE NOT EXISTS (SELECT 1 FROM "role" WHERE "code" = 'SUPER');

            INSERT INTO "role" ("_id", "_tenant_id", "code", "name", "description", "is_system_role", "is_active", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, 'ADMIN', 'System Administrator', 'Administrative system management', true, true, NOW(), NOW()
            WHERE NOT EXISTS (SELECT 1 FROM "role" WHERE "code" = 'ADMIN');
        "#;
        if let Err(e) = sqlx::query(insert_roles).bind(tenant_id).execute(pool).await {
            tracing::error!("Failed to seed roles into role table: {e}");
        }"""

new_block = """        // 1. Ensure Roles (SUPER, ADMIN) exist in "role"
        let insert_roles = r#"
            INSERT INTO "role" ("_id", "_tenant_id", "code", "name", "description", "is_system_role", "is_active", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, v.code, v.name, v.description, true, true, NOW(), NOW()
            FROM (
                VALUES 
                    ('SUPER', 'Super Administrator', 'Full unrestricted platform access'),
                    ('ADMIN', 'System Administrator', 'Administrative system management')
            ) AS v(code, name, description)
            WHERE NOT EXISTS (SELECT 1 FROM "role" WHERE "code" = v.code);
        "#;
        if let Err(e) = sqlx::query(insert_roles).bind(tenant_id).execute(pool).await {
            tracing::error!("Failed to seed roles into role table: {e}");
        }"""

assert old_block in text, "old_block not found"
text = text.replace(old_block, new_block)

old_ur = """        // 3. Ensure UserRole mappings exist
        let insert_user_roles = r#"
            INSERT INTO "user_role" ("_id", "_tenant_id", "username", "role_code", "assigned_by", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, 'admin', 'SUPER', 'SYSTEM', NOW(), NOW()
            WHERE NOT EXISTS (SELECT 1 FROM "user_role" WHERE "username" = 'admin' AND "role_code" = 'SUPER');

            INSERT INTO "user_role" ("_id", "_tenant_id", "username", "role_code", "assigned_by", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, 'admin', 'ADMIN', 'SYSTEM', NOW(), NOW()
            WHERE NOT EXISTS (SELECT 1 FROM "user_role" WHERE "username" = 'admin' AND "role_code" = 'ADMIN');
        "#;
        if let Err(e) = sqlx::query(insert_user_roles).bind(tenant_id).execute(pool).await {
            tracing::error!("Failed to seed user_role mappings: {e}");
        }"""

new_ur = """        // 3. Ensure UserRole mappings exist
        let insert_user_roles = r#"
            INSERT INTO "user_role" ("_id", "_tenant_id", "username", "role_code", "assigned_by", "_created_at", "_updated_at")
            SELECT gen_random_uuid(), $1, 'admin', v.role_code, 'SYSTEM', NOW(), NOW()
            FROM (
                VALUES ('SUPER'), ('ADMIN')
            ) AS v(role_code)
            WHERE NOT EXISTS (SELECT 1 FROM "user_role" WHERE "username" = 'admin' AND "role_code" = v.role_code);
        "#;
        if let Err(e) = sqlx::query(insert_user_roles).bind(tenant_id).execute(pool).await {
            tracing::error!("Failed to seed user_role mappings: {e}");
        }"""

assert old_ur in text, "old_ur not found"
text = text.replace(old_ur, new_ur)

with open('crates/bosc-runtime/src/package_manager.rs', 'w', encoding='utf-8', newline='\n') as f:
    f.write(text)
print("UPDATED SINGLE QUERY SEEDS")
