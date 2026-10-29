with open('crates/bosc-runtime/src/lib.rs', 'r', encoding='utf-8') as f:
    content = f.read()

target = """        if data_per_company {
            if !ctx.company_id.is_nil() {
                where_clauses.push(format!("t.\\"_company_id\\" = '{}'", ctx.company_id));
            } else {
                where_clauses.push("t.\\"_company_id\\" IS NULL".to_string());
            }
        }
        }"""

replacement = """        if data_per_company {
            if !ctx.company_id.is_nil() {
                where_clauses.push(format!("t.\\"_company_id\\" = '{}'", ctx.company_id));
            } else {
                where_clauses.push("t.\\"_company_id\\" IS NULL".to_string());
            }
        }"""

assert target in content, "target not found"
content = content.replace(target, replacement, 1)

with open('crates/bosc-runtime/src/lib.rs', 'w', encoding='utf-8') as f:
    f.write(content)
print("Fixed extra closing brace")
