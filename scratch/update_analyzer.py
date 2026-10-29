with open('crates/bosc-analyzer/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

old_block = """        let mut select_fields = Vec::new();
        for sf in &v.select_fields {
            select_fields.push(ViewSelectFieldIR {
                expr: format_expr(&sf.field_expr),
                alias: sf.alias.clone(),
            });
        }

        let mut joins = Vec::new();
        for j in &v.joins {
            joins.push(ViewJoinIR {
                join_type: match j.join_type {
                    JoinType::Left => "LEFT".to_string(),
                    JoinType::Inner => "INNER".to_string(),
                },
                target_table: to_snake_case(&j.target_table),
                alias: j.alias.clone(),
                condition: format_expr(&j.condition),
            });
        }

        let where_clause = v.where_filter.as_ref().map(|f| format_expr(f));"""

new_block = """        let mut select_fields = Vec::new();
        for sf in &v.select_fields {
            let expr_str = match &sf.field_expr {
                Expr::Ident(s) => format!("\\"{}\\"", to_snake_case(s)),
                Expr::MemberAccess { object, member } => {
                    format!("\\"{}\\".\\"{}\\"", format_expr(object), to_snake_case(member))
                }
                _ => format_expr(&sf.field_expr),
            };
            select_fields.push(ViewSelectFieldIR {
                expr: expr_str,
                alias: sf.alias.as_ref().map(|a| to_snake_case(a)),
            });
        }

        let src_table = to_snake_case(&v.source_table);
        let mut joins = Vec::new();
        for j in &v.joins {
            joins.push(ViewJoinIR {
                join_type: match j.join_type {
                    JoinType::Left => "LEFT".to_string(),
                    JoinType::Inner => "INNER".to_string(),
                },
                target_table: to_snake_case(&j.target_table),
                alias: j.alias.as_ref().map(|a| to_snake_case(a)),
                condition: lower_expr_to_sql_filter(&j.condition, &to_snake_case(&j.target_table)),
            });
        }

        let where_clause = v.where_filter.as_ref().map(|f| lower_expr_to_sql_filter(f, &src_table));"""

content_norm = content.replace("\r\n", "\n")
old_norm = old_block.replace("\r\n", "\n")
assert old_norm in content_norm, "old_block not found in bosc-analyzer/src/lib.rs"

content_norm = content_norm.replace(old_norm, new_block.replace("\r\n", "\n"))

with open('crates/bosc-analyzer/src/lib.rs', 'w', encoding='utf-8', newline='\r\n') as f:
    f.write(content_norm)

print("Updated bosc-analyzer/src/lib.rs successfully!")
