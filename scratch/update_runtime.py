# 2. Update lib.rs
file_lib = "crates/bosc-runtime/src/lib.rs"
with open(file_lib, "r", encoding="utf-8") as f:
    lib_content = f.read()

# Check CRLF
is_crlf = "\r\n" in lib_content
lines = lib_content.splitlines()

# 1. Update insert_tuple
start_idx = None
for i, line in enumerate(lines):
    if 'let mut columns = vec!["\\"_id\\"".to_string(), "\\"_tenant_id\\"".to_string()];' in line:
        start_idx = i
        break

if start_idx is not None:
    print(f"Found insert_tuple at line {start_idx+1}")
    end_idx = None
    for j in range(start_idx, start_idx + 25):
        if 'added.insert("_company_id".to_string());' in lines[j]:
            end_idx = j
            break
    if end_idx is not None:
        replacement_lines = [
            '        let mut columns = vec!["\\"_id\\"".to_string(), "\\"_tenant_id\\"".to_string()];',
            '',
            '        let tenant_val = if ctx.tenant_id.is_nil() {',
            '            "NULL".to_string()',
            '        } else {',
            '            format!("\'{}\'::uuid", ctx.tenant_id)',
            '        };',
            '',
            '        let mut values = vec![',
            '            format!("\'{}\'::uuid", id),',
            '            tenant_val,',
            '        ];',
            '',
            '        added.insert("_id".to_string());',
            '',
            '        added.insert("_tenant_id".to_string());',
            '',
            '        if data_per_company {',
            '            columns.push("\\"_company_id\\"".to_string());',
            '',
            '            let comp_val = if ctx.company_id.is_nil() {',
            '                "NULL".to_string()',
            '            } else {',
            '                format!("\'{}\'::uuid", ctx.company_id)',
            '            };',
            '',
            '            values.push(comp_val);',
            '',
            '            added.insert("_company_id".to_string());'
        ]
        lines = lines[:start_idx] + replacement_lines + lines[end_idx+1:]
        print("Replaced insert_tuple")

# 2. Update insert_batch
batch_id_idx = None
for i, line in enumerate(lines):
    if '.unwrap_or_else(Uuid::new_v4);' in line and 'for p in &mut payloads {' in lines[max(0, i-6):i]:
        batch_id_idx = i
        break

if batch_id_idx is not None:
    print(f"Found batch_id at line {batch_id_idx+1}")
    end_batch_idx = None
    for j in range(batch_id_idx, batch_id_idx + 18):
        if 'val_tokens.push(format!("\'{}\'::uuid", scope.ctx.company_id));' in lines[j]:
            end_batch_idx = j
            break
    if end_batch_idx is not None:
        replacement_batch = [
            '                .unwrap_or_else(Uuid::now_v7);',
            '',
            '            generated_ids.push(id);',
            '',
            '            let tenant_token = if scope.ctx.tenant_id.is_nil() {',
            '                "NULL".to_string()',
            '            } else {',
            '                format!("\'{}\'::uuid", scope.ctx.tenant_id)',
            '            };',
            '',
            '            let mut val_tokens = vec![',
            '                format!("\'{}\'::uuid", id),',
            '                tenant_token,',
            '            ];',
            '',
            '            if data_per_company {',
            '                let comp_token = if scope.ctx.company_id.is_nil() {',
            '                    "NULL".to_string()',
            '                } else {',
            '                    format!("\'{}\'::uuid", scope.ctx.company_id)',
            '                };',
            '                val_tokens.push(comp_token);',
            '            }'
        ]
        brace_idx = end_batch_idx + 1
        while brace_idx < len(lines) and '}' not in lines[brace_idx]:
            brace_idx += 1
        lines = lines[:batch_id_idx] + replacement_batch + lines[brace_idx+1:]
        print("Replaced insert_batch")

# 3. job_id Uuid::new_v4 -> now_v7
for i, line in enumerate(lines):
    if "let job_id = Uuid::new_v4();" in line:
        lines[i] = line.replace("let job_id = Uuid::new_v4();", "let job_id = Uuid::now_v7();")
        print("Replaced job_id with now_v7")
    if "object_id: Uuid::new_v4()," in line:
        lines[i] = line.replace("object_id: Uuid::new_v4(),", "object_id: Uuid::now_v7(),")
        print("Replaced object_id with now_v7")

# 4. query_records_paginated
paginated_idx = None
for i, line in enumerate(lines):
    if 'let mut where_clauses = vec![format!("t.\\"_tenant_id\\" = \'{}\'", ctx.tenant_id)];' in line:
        paginated_idx = i
        break

if paginated_idx is not None:
    print(f"Found paginated where clauses at line {paginated_idx+1}")
    end_p_idx = paginated_idx + 1
    while end_p_idx < len(lines) and 'where_clauses.push(format!("t.\\"_company_id\\"' not in lines[end_p_idx]:
        end_p_idx += 1
    while end_p_idx < len(lines) and '}' not in lines[end_p_idx]:
        end_p_idx += 1
    
    replacement_paginated = [
        '        // Multi-tenant and company isolation WHERE conditions',
        '        let mut where_clauses = Vec::new();',
        '        if !ctx.tenant_id.is_nil() {',
        '            where_clauses.push(format!("t.\\"_tenant_id\\" = \'{}\'", ctx.tenant_id));',
        '        } else {',
        '            where_clauses.push("t.\\"_tenant_id\\" IS NULL".to_string());',
        '        }',
        '',
        '        if data_per_company {',
        '            if !ctx.company_id.is_nil() {',
        '                where_clauses.push(format!("t.\\"_company_id\\" = \'{}\'", ctx.company_id));',
        '            } else {',
        '                where_clauses.push("t.\\"_company_id\\" IS NULL".to_string());',
        '            }',
        '        }'
    ]
    start_p_idx = paginated_idx
    if 'Multi-tenant isolation WHERE conditions' in lines[start_p_idx - 2]:
        start_p_idx = start_p_idx - 2
    elif 'Multi-tenant isolation WHERE conditions' in lines[start_p_idx - 1]:
        start_p_idx = start_p_idx - 1
    lines = lines[:start_p_idx] + replacement_paginated + lines[end_p_idx+1:]
    print("Replaced query_records_paginated where clauses")

# 5. fast path get_record_by_id parameters
fast_path_idx = None
for i, line in enumerate(lines):
    if 'if p == "_tenant_id" {' in line and 'stmt.param_names' in lines[max(0, i-2)]:
        fast_path_idx = i
        break

if fast_path_idx is not None:
    print(f"Found fast path get_record_by_id at line {fast_path_idx+1}")
    end_fp_idx = fast_path_idx
    while end_fp_idx < len(lines) and 'q = q.bind(id.to_string());' not in lines[end_fp_idx]:
        end_fp_idx += 1
    while end_fp_idx < len(lines) and '}' not in lines[end_fp_idx]:
        end_fp_idx += 1
    
    replacement_fp = [
        '                        if p == "_tenant_id" {',
        '                            if ctx.tenant_id.is_nil() {',
        '                                q = q.bind(Option::<Uuid>::None);',
        '                            } else {',
        '                                q = q.bind(Some(ctx.tenant_id));',
        '                            }',
        '                        } else if p == "_company_id" {',
        '                            if ctx.company_id.is_nil() {',
        '                                q = q.bind(Option::<Uuid>::None);',
        '                            } else {',
        '                                q = q.bind(Some(ctx.company_id));',
        '                            }',
        '                        } else {',
        '                            q = q.bind(id.to_string());',
        '                        }'
    ]
    lines = lines[:fast_path_idx] + replacement_fp + lines[end_fp_idx+1:]
    print("Replaced fast path param bindings")

# 6. calculate_flowfield: exact decimal
calc_idx = None
for i, line in enumerate(lines):
    if 'pub async fn calculate_flowfield(' in line:
        calc_idx = i
        break

if calc_idx is not None:
    print(f"Found calculate_flowfield at line {calc_idx+1}")
    end_calc_idx = calc_idx
    while end_calc_idx < len(lines) and 'Ok(val.unwrap_or(0.0))' not in lines[end_calc_idx]:
        end_calc_idx += 1
    while end_calc_idx < len(lines) and '}' not in lines[end_calc_idx]:
        end_calc_idx += 1
    
    replacement_calc = [
        '    /// Compute FlowField live via generated subquery with exact decimal arithmetic',
        '    pub async fn calculate_flowfield(',
        '        &self,',
        '        table: &str,',
        '        record_id: Uuid,',
        '        computed_subquery: &str,',
        '    ) -> Result<rust_decimal::Decimal, RuntimeError> {',
        '        let pool = self.pool().await;',
        '        let safe_table = sanitize_identifier(table)?;',
        '        let query = format!(',
        '            "SELECT COALESCE(({}), 0) as val FROM \\"{}\\" WHERE \\"_id\\" = $1;",',
        '            computed_subquery, safe_table',
        '        );',
        '        let row = sqlx::query(&query)',
        '            .bind(record_id)',
        '            .fetch_one(&pool)',
        '            .await?;',
        '        let val: Option<rust_decimal::Decimal> = row.try_get("val").ok();',
        '        Ok(val.unwrap_or(rust_decimal::Decimal::ZERO))',
        '    }'
    ]
    start_c_idx = calc_idx
    if 'Compute FlowField live via generated subquery' in lines[start_c_idx-2]:
        start_c_idx = start_c_idx - 2
    elif 'Compute FlowField live via generated subquery' in lines[start_c_idx-1]:
        start_c_idx = start_c_idx - 1
    lines = lines[:start_c_idx] + replacement_calc + lines[end_calc_idx+1:]
    print("Replaced calculate_flowfield")

sep = "\r\n" if is_crlf else "\n"
output = sep.join(lines) + sep
with open(file_lib, "w", encoding="utf-8") as f:
    f.write(output)

print("Saved lib.rs successfully!")
