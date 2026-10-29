with open('crates/bosc-analyzer/src/page_source.rs', 'r', encoding='utf-8') as f:
    c = f.read()

target = """fn target_of(obj: &OutlineObject, path: &str) -> Result<Target, String> {
    if path.eq_ignore_ascii_case("page") {
        return Ok(Target {"""

repl = """fn target_of(obj: &OutlineObject, path: &str) -> Result<Target, String> {
    if path.eq_ignore_ascii_case("page")
        || path.eq_ignore_ascii_case("header")
        || path.eq_ignore_ascii_case("header:title")
        || path.eq_ignore_ascii_case("header:instruction")
    {
        return Ok(Target {"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('crates/bosc-analyzer/src/page_source.rs', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated page_source.rs successfully")
