file_lib = "crates/bosc-runtime/src/lib.rs"
with open(file_lib, "r", encoding="utf-8") as f:
    content = f.read()

# 1. Update insert_batch
target_batch = """        for p in &mut payloads {
            let id = p
                .get("_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::new_v4);

            generated_ids.push(id);

            let mut val_tokens = vec![
                format!("'{}'::uuid", id),
                format!("'{}'::uuid", scope.ctx.tenant_id),
            ];

            if data_per_company {
                val_tokens.push(format!("'{}'::uuid", scope.ctx.company_id));
            }"""

replacement_batch = """        for p in &mut payloads {
            let id = p
                .get("_id")
                .and_then(|v| v.as_str())
                .and_then(|s| Uuid::parse_str(s).ok())
                .unwrap_or_else(Uuid::now_v7);

            generated_ids.push(id);

            let tenant_token = if scope.ctx.tenant_id.is_nil() {
                "NULL".to_string()
            } else {
                format!("'{}'::uuid", scope.ctx.tenant_id)
            };

            let mut val_tokens = vec![
                format!("'{}'::uuid", id),
                tenant_token,
            ];

            if data_per_company {
                let comp_token = if scope.ctx.company_id.is_nil() {
                    "NULL".to_string()
                } else {
                    format!("'{}'::uuid", scope.ctx.company_id)
                };
                val_tokens.push(comp_token);
            }"""

assert target_batch in content, "target_batch not found"
content = content.replace(target_batch, replacement_batch, 1)
print("Updated insert_batch")

# 2. Update fast path
target_fp = """                    for p in &stmt.param_names {
                        if p == "_tenant_id" {
                            q = q.bind(ctx.tenant_id);
                        } else if p == "_company_id" {
                            q = q.bind(ctx.company_id);
                        } else {
                            q = q.bind(id.to_string());
                        }
                    }"""

replacement_fp = """                    for p in &stmt.param_names {
                        if p == "_tenant_id" {
                            if ctx.tenant_id.is_nil() {
                                q = q.bind(Option::<Uuid>::None);
                            } else {
                                q = q.bind(Some(ctx.tenant_id));
                            }
                        } else if p == "_company_id" {
                            if ctx.company_id.is_nil() {
                                q = q.bind(Option::<Uuid>::None);
                            } else {
                                q = q.bind(Some(ctx.company_id));
                            }
                        } else {
                            q = q.bind(id.to_string());
                        }
                    }"""

assert target_fp in content, "target_fp not found"
content = content.replace(target_fp, replacement_fp, 1)
print("Updated fast path")

with open(file_lib, "w", encoding="utf-8") as f:
    f.write(content)

print("Finished updating lib.rs successfully!")
