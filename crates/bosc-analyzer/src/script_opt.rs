//! Script optimizer for BOS-Script procedure bodies.
//!
//! Runs after type erasure. It hoists `Model.get(row.field)` calls out of
//! `for...of` loops into one `getMany`, and rewrites arithmetic on known
//! decimal fields into exact `ctx.decimal` operations.

use bosc_ir::CompilerDiagnosticIR;
use std::collections::HashSet;

pub struct LoopRewrite {
    pub javascript: String,
    pub hoisted: usize,
    pub unhoisted_gets: usize,
}

/// Hoist simple `Model.get(item.field)` calls out of `for (const item of rows)` loops.
pub fn optimize_loops(source: &str) -> LoopRewrite {
    optimize_loops_finished(source)
}

struct ForOf {
    keyword_at: usize,
    header: String,
    var_name: String,
    body: String,
    end: usize,
}

fn find_for_of(source: &str) -> Option<ForOf> {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let keyword_end = i + 3;
        let followed_by_ident = keyword_end < bytes.len()
            && (bytes[keyword_end].is_ascii_alphanumeric() || bytes[keyword_end] == b'_');
        if &bytes[i..keyword_end] != b"for" || !is_ident_boundary(source, i) || followed_by_ident {
            i += 1;
            continue;
        }
        let j = skip_ws(source, keyword_end);
        if j >= bytes.len() || bytes[j] != b'(' {
            i = keyword_end;
            continue;
        }
        let paren_start = j;
        let Some(paren_end) = match_delim(source, paren_start, '(', ')') else {
            i = keyword_end;
            continue;
        };
        let header = source[paren_start + 1..paren_end].trim();
        let Some((var_name, _)) = parse_for_of_header(header) else {
            i = paren_end + 1;
            continue;
        };
        let k = skip_ws(source, paren_end + 1);
        if k >= bytes.len() || bytes[k] != b'{' {
            i = paren_end + 1;
            continue;
        }
        let Some(brace_end) = match_delim(source, k, '{', '}') else {
            i = paren_end + 1;
            continue;
        };
        let body = source[k + 1..brace_end].to_string();
        return Some(ForOf {
            keyword_at: i,
            header: header.to_string(),
            var_name,
            body,
            end: brace_end + 1,
        });
    }
    None
}

fn parse_for_of_header(header: &str) -> Option<(String, String)> {
    let rest = header
        .strip_prefix("const ")
        .or_else(|| header.strip_prefix("let "))
        .or_else(|| header.strip_prefix("var "))?
        .trim_start();
    let (name, after_name) = split_ident(rest)?;
    let mut after_name = after_name.trim_start();
    if let Some(rest_ty) = after_name.strip_prefix(':') {
        let Some(of_at) = rest_ty.find(" of ") else {
            return None;
        };
        after_name = rest_ty[of_at + 1..].trim_start();
    }
    let expr = after_name.strip_prefix("of ")?.trim();
    if name.is_empty() || expr.is_empty() || expr.contains(';') {
        return None;
    }
    Some((name, expr.to_string()))
}

struct GetCall {
    start: usize,
    end: usize,
    model: String,
    field: String,
}

fn rewrite_with_known_expr(var_name: &str, body: &str, counter: usize, calls: &[GetCall]) -> (String, String, usize, usize) {
    // Iterable expression is filled by the wrapper. This function only rewrites the body
    // and returns a preamble template containing `__BOS_OF__`.
    let mut preamble = String::new();
    let mut pieces: Vec<(usize, usize, String)> = Vec::new();
    let mut seen = HashSet::new();
    let mut maps: Vec<(String, String, String)> = Vec::new();
    for call in calls {
        let key = format!("{}:{}", call.model, call.field);
        let map_name = if let Some(existing) = maps.iter().find(|(_, m, f)| m == &call.model && f == &call.field) {
            existing.0.clone()
        } else {
            let map_name = format!("__bosMap{}_{}", counter, maps.len());
            maps.push((map_name.clone(), call.model.clone(), call.field.clone()));
            map_name
        };
        if seen.insert(key) {
            preamble.push_str(&format!(
                "const {map_name} = new Map(({model}.getMany(__bosCollect(__BOS_OF__, (row) => row.{field})) || []).map((r) => [String(r.__bosKey), r]));\n",
                map_name = map_name,
                model = call.model,
                field = call.field,
            ));
        }
        let replacement = format!("{map_name}.get(String({var_name}.{field})) ?? null", field = call.field);
        pieces.push((call.start, call.end, replacement));
    }
    let mut body_out = String::with_capacity(body.len());
    let mut cursor = 0;
    for (start, end, replacement) in pieces {
        body_out.push_str(&body[cursor..start]);
        body_out.push_str(&replacement);
        cursor = end;
    }
    body_out.push_str(&body[cursor..]);
    (body_out, preamble, maps.len(), 0)
}

fn find_gets(body: &str, var_name: &str) -> Vec<GetCall> {
    let mut out = Vec::new();
    let marker = ".get";
    let mut search_from = 0;
    while let Some(rel) = body[search_from..].find(marker) {
        let get_at = search_from + rel;
        let mut args_open = get_at + marker.len();
        if body[args_open..].starts_with('<') {
            let Some(close_ty) = body[args_open..].find('>') else {
                break;
            };
            args_open += close_ty + 1;
        }
        if !body[args_open..].starts_with('(') {
            search_from = get_at + marker.len();
            continue;
        }
        let model_start = ident_start_before(body, get_at);
        let model = body[model_start..get_at].trim();
        let model = model.rsplit('.').next().unwrap_or(model);
        if model.is_empty() || !model.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            search_from = args_open + 1;
            continue;
        }
        let Some(args_close) = match_delim(body, args_open, '(', ')') else {
            break;
        };
        let arg = body[args_open + 1..args_close].trim();
        let prefix = format!("{var_name}.");
        if let Some(field) = arg.strip_prefix(&prefix) {
            let field = field.trim();
            if is_plain_ident(field) && !spans_string(body, get_at) {
                let start = model_prefix_start(body, model_start);
                out.push(GetCall {
                    start,
                    end: args_close + 1,
                    model: model.to_string(),
                    field: field.to_string(),
                });
            }
        }
        search_from = args_close + 1;
    }
    out
}

fn model_prefix_start(body: &str, model_at: usize) -> usize {
    let before = &body[..model_at];
    if let Some(stripped) = before.strip_suffix("models.") {
        return stripped.len();
    }
    if let Some(idx) = before.rfind("ctx.db.") {
        if idx + "ctx.db.".len() == model_at {
            return idx;
        }
    }
    model_at
}

fn spans_string(body: &str, at: usize) -> bool {
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for (i, c) in body.char_indices() {
        if i >= at {
            return in_str.is_some();
        }
        if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                in_str = None;
            }
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            in_str = Some(c);
        }
    }
    false
}

/// Replace `__BOS_OF__` in the preamble with the loop iterable.
pub fn finish_preamble(preamble: &str, iterable: &str) -> String {
    preamble.replace("__BOS_OF__", iterable)
}

/// Diagnostics for the original script body, before erasure.
pub fn diagnose(raw: &str, base_line: usize) -> Vec<CompilerDiagnosticIR> {
    let mut diags = Vec::new();
    let rewritten = optimize_loops(raw);
    if rewritten.hoisted > 0 {
        diags.push(CompilerDiagnosticIR {
            severity: "info".to_string(),
            code: "BOS3001".to_string(),
            message: format!(
                "Hoisted {} Model.get call(s) out of a for...of loop into getMany (one query for the keys).",
                rewritten.hoisted
            ),
            line: base_line,
            column: 1,
            end_line: base_line,
            end_column: 1,
            hint: Some("The stored script reads the keys once, then looks each row up in a Map.".to_string()),
        });
    }
    if raw.contains("for") && raw.contains(".get(") && rewritten.hoisted == 0 && loop_contains_get(raw) {
        diags.push(CompilerDiagnosticIR {
            severity: "warning".to_string(),
            code: "BOS3002".to_string(),
            message: "A database get() sits inside a loop and was not rewritten. Use getMany(keys) or modifyAll / deleteAll.".to_string(),
            line: base_line,
            column: 1,
            end_line: base_line,
            end_column: 1,
            hint: Some("for (const row of rows) { Model.get(row.key) } is rewritten. Other loop shapes are left as written.".to_string()),
        });
    }
    let queries = count_query_calls(raw);
    if queries > 0 || rewritten.hoisted > 0 {
        let estimated = queries.saturating_sub(rewritten.hoisted).max(rewritten.hoisted) as u64 * 10_000;
        diags.push(CompilerDiagnosticIR {
            severity: "info".to_string(),
            code: "BOS3200".to_string(),
            message: format!(
                "Script plan: about {queries} data call(s), {hoisted} hoisted. Fuel estimate {estimated} steps before host SQL. Tier: quickjs for this body; set-based calls stay one SQL statement.",
                hoisted = rewritten.hoisted,
                estimated = estimated
            ),
            line: base_line,
            column: 1,
            end_line: base_line,
            end_column: 1,
            hint: Some("Batch work belongs in getMany, createMany, modifyAll, or a @pushdown procedure.".to_string()),
        });
    }
    diags
}

fn loop_contains_get(raw: &str) -> bool {
    find_for_of(raw)
        .map(|f| f.body.contains(".get(") || find_for_of(&f.body).is_some())
        .unwrap_or(false)
}

fn count_query_calls(raw: &str) -> usize {
    const MARKERS: [&str; 8] = [
        ".get(", ".getMany(", ".findMany(", ".create(", ".createMany(", ".modifyAll(", ".deleteAll(", ".first(",
    ];
    MARKERS.iter().map(|m| raw.matches(m).count()).sum()
}

/// Rewrite `rec.price * rec.qty` and relational comparisons (`<`, `>`, `<=`, `>=`)
/// when decimal columns are involved.
pub fn rewrite_decimal(source: &str, decimal_fields: &HashSet<String>) -> String {
    if decimal_fields.is_empty()
        || !source.contains(['+', '-', '*', '/', '<', '>'])
    {
        return source.to_string();
    }
    let (masked, vault) = mask_strings(source);
    let mut current = masked;
    for _ in 0..32 {
        let next = rewrite_one_decimal_op(&current, decimal_fields);
        if next == current {
            break;
        }
        current = next;
    }
    for _ in 0..32 {
        let next = rewrite_one_decimal_cmp(&current, decimal_fields);
        if next == current {
            break;
        }
        current = next;
    }
    unmask_strings(&current, &vault)
}

fn rewrite_one_decimal_op(source: &str, fields: &HashSet<String>) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let op = chars[i];
        if matches!(op, '+' | '-' | '*' | '/') && !is_operator_prefix(&chars, i) {
            if let (Some((ls, le)), Some((rs, re))) = (atom_before(&chars, i, fields), atom_after(&chars, i, fields)) {
                let left: String = chars[ls..le].iter().collect();
                let right: String = chars[rs..re].iter().collect();
                if !is_number_ident(left.trim()) || !is_number_ident(right.trim()) {
                    let method = match op {
                        '+' => "add",
                        '-' => "sub",
                        '*' => "mul",
                        _ => "div",
                    };
                    let replacement = format!("ctx.decimal({left}).{method}({right}).toString()");
                    let mut out: String = chars[..ls].iter().collect();
                    out.push_str(&replacement);
                    out.extend(chars[re..].iter());
                    return out;
                }
            }
        }
        i += 1;
    }
    source.to_string()
}

fn rewrite_one_decimal_cmp(source: &str, fields: &HashSet<String>) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if matches!(c, '<' | '>') {
            // Skip `=>`, `<<`, `>>`
            if (i > 0 && matches!(chars[i - 1], '=' | '<' | '>'))
                || (i + 1 < chars.len() && chars[i + 1] == c)
            {
                i += 1;
                continue;
            }
            let (method, op_end) = if i + 1 < chars.len() && chars[i + 1] == '=' {
                (if c == '<' { "lte" } else { "gte" }, i + 1)
            } else {
                (if c == '<' { "lt" } else { "gt" }, i)
            };
            if let (Some((ls, le)), Some((rs, re))) =
                (atom_before(&chars, i, fields), atom_after(&chars, op_end, fields))
            {
                let left: String = chars[ls..le].iter().collect();
                let right: String = chars[rs..re].iter().collect();
                if !is_number_ident(left.trim()) || !is_number_ident(right.trim()) {
                    let replacement = format!("ctx.decimal({left}).{method}({right})");
                    let mut out: String = chars[..ls].iter().collect();
                    out.push_str(&replacement);
                    out.extend(chars[re..].iter());
                    return out;
                }
            }
        }
        i += 1;
    }
    source.to_string()
}

fn is_operator_prefix(chars: &[char], i: usize) -> bool {
    // Skip `++`, `--`, `+=`, `-=`, `*=`, `/=`, and the sign in `e-1` style exponents. Also skip `=>`.
    if i > 0 && (chars[i - 1] == chars[i] || chars[i - 1] == '=') {
        return true;
    }
    if i + 1 < chars.len() && (chars[i + 1] == chars[i] || chars[i + 1] == '=' || chars[i + 1] == '>') {
        return true;
    }
    false
}

fn atom_before(chars: &[char], op_at: usize, fields: &HashSet<String>) -> Option<(usize, usize)> {
    let mut end = op_at;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    atom_ending_at(chars, end, fields)
}

fn atom_after(chars: &[char], op_at: usize, fields: &HashSet<String>) -> Option<(usize, usize)> {
    let mut start = op_at + 1;
    while start < chars.len() && chars[start].is_whitespace() {
        start += 1;
    }
    atom_starting_at(chars, start, fields)
}

fn atom_ending_at(chars: &[char], end: usize, fields: &HashSet<String>) -> Option<(usize, usize)> {
    if end == 0 {
        return None;
    }
    if chars[end - 1] == ')' {
        // A previous rewrite ends with `.toString()`. Treat that whole call as an atom
        // so `a * b * c` chains, but only when it is a ctx.decimal call.
        let text: String = chars[..end].iter().collect();
        if let Some(start) = text.rfind("ctx.decimal(") {
            if text[start..].starts_with("ctx.decimal(") && text[start..].ends_with(".toString()") {
                return Some((start, end));
            }
        }
        return None;
    }
    let (start, ident) = read_ident_ending(chars, end)?;
    if ident_is_decimal(&chars[..start], &ident, fields) || is_number_ident(&ident) {
        let member_start = member_start(chars, start);
        return Some((member_start, end));
    }
    None
}

fn atom_starting_at(chars: &[char], start: usize, fields: &HashSet<String>) -> Option<(usize, usize)> {
    if start >= chars.len() {
        return None;
    }
    if chars[start..].starts_with(&['c', 't', 'x', '.', 'd', 'e', 'c', 'i', 'm', 'a', 'l', '(']) {
        let text: String = chars[start..].iter().collect();
        if let Some(rel) = text.find(".toString()") {
            return Some((start, start + rel + ".toString()".len()));
        }
    }
    let (ident, next) = read_ident_starting(chars, start)?;
    if next < chars.len() && chars[next] == '.' && matches!(ident.as_str(), "rec" | "this" | "xRec") {
        let (member, after) = read_ident_starting(chars, next + 1)?;
        if fields.contains(&member) || is_number_ident(&member) {
            return Some((start, after));
        }
        return None;
    }
    if fields.contains(&ident) || is_number_ident(&ident) {
        return Some((start, next));
    }
    None
}

fn ident_is_decimal(before: &[char], ident: &str, fields: &HashSet<String>) -> bool {
    if !fields.contains(ident) {
        return false;
    }
    // `rec.price` — the ident is the field. A bare field name is also accepted.
    let _ = before;
    true
}

fn member_start(chars: &[char], ident_start: usize) -> usize {
    if ident_start >= 1 && chars[ident_start - 1] == '.' {
        let base_end = ident_start - 1;
        if let Some((base_start, _)) = read_ident_ending(chars, base_end) {
            let base: String = chars[base_start..base_end].iter().collect();
            if matches!(base.as_str(), "rec" | "this" | "xRec") {
                return base_start;
            }
        }
    }
    ident_start
}

fn is_number_ident(ident: &str) -> bool {
    let mut seen_dot = false;
    if ident.is_empty() {
        return false;
    }
    for (n, c) in ident.chars().enumerate() {
        if c == '.' {
            if seen_dot || n == 0 {
                return false;
            }
            seen_dot = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return false;
        }
    }
    true
}

fn read_ident_ending(chars: &[char], end: usize) -> Option<(usize, String)> {
    if end == 0 || !is_ident_char(chars[end - 1]) {
        return None;
    }
    let mut start = end;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    Some((start, chars[start..end].iter().collect()))
}

fn read_ident_starting(chars: &[char], start: usize) -> Option<(String, usize)> {
    if start >= chars.len() || !is_ident_start(chars[start]) && !chars[start].is_ascii_digit() {
        return None;
    }
    let mut end = start + 1;
    while end < chars.len() && (is_ident_char(chars[end]) || chars[end] == '.') {
        // Stop before a second dot so `rec.price.foo` is not one ident.
        if chars[end] == '.' {
            break;
        }
        end += 1;
    }
    Some((chars[start..end].iter().collect(), end))
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn mask_strings(source: &str) -> (String, Vec<String>) {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::new();
    let mut vault = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' || c == '\'' || c == '`' {
            let start = i;
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' {
                    i += 2;
                    continue;
                }
                if chars[i] == c {
                    i += 1;
                    break;
                }
                i += 1;
            }
            let lit: String = chars[start..i].iter().collect();
            vault.push(lit);
            out.push_str(&format!("\"__BOS_DEC_{}__\"", vault.len() - 1));
            continue;
        }
        out.push(c);
        i += 1;
    }
    (out, vault)
}

fn unmask_strings(source: &str, vault: &[String]) -> String {
    let mut out = source.to_string();
    for (n, lit) in vault.iter().enumerate() {
        out = out.replace(&format!("\"__BOS_DEC_{n}__\""), lit);
    }
    out
}

fn is_plain_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => chars.all(|c| c.is_ascii_alphanumeric() || c == '_'),
        _ => false,
    }
}

fn ident_start_before(source: &str, at: usize) -> usize {
    let chars: Vec<char> = source.chars().collect();
    // `at` is a byte index. The sources we rewrite are ASCII procedure bodies.
    let mut i = at;
    while i > 0 && is_ident_char(chars[i - 1]) {
        i -= 1;
    }
    i
}

fn split_ident(s: &str) -> Option<(String, &str)> {
    let mut end = 0;
    for (n, c) in s.chars().enumerate() {
        if n == 0 && !(c.is_ascii_alphabetic() || c == '_') {
            return None;
        }
        if n > 0 && !(c.is_ascii_alphanumeric() || c == '_') {
            break;
        }
        end = n + c.len_utf8();
    }
    if end == 0 {
        None
    } else {
        Some((s[..end].to_string(), &s[end..]))
    }
}

fn skip_ws(source: &str, mut i: usize) -> usize {
    while i < source.len() && source.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn is_ident_boundary(source: &str, i: usize) -> bool {
    if i > 0 {
        let prev = source.as_bytes()[i - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            return false;
        }
    }
    true
}

fn match_delim(source: &str, open_at: usize, open: char, close: char) -> Option<usize> {
    let tail = source.get(open_at..)?;
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let mut escaped = false;
    for (rel, c) in tail.char_indices() {
        if let Some(q) = in_str {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                in_str = None;
            }
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            in_str = Some(c);
            continue;
        }
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth == 0 {
                return Some(open_at + rel);
            }
        }
    }
    None
}

fn optimize_loops_finished(source: &str) -> LoopRewrite {
    let (javascript, hoisted, unhoisted) = rewrite_finished(source, 0);
    LoopRewrite { javascript, hoisted, unhoisted_gets: unhoisted }
}

fn rewrite_finished(source: &str, counter: usize) -> (String, usize, usize) {
    let Some(found) = find_for_of(source) else {
        return (source.to_string(), 0, 0);
    };
    let (inner, inner_hoisted, inner_unhoisted) = rewrite_finished(&found.body, counter);
    let calls = find_gets(&inner, &found.var_name);
    let (body, preamble, hoisted_here, still) = if calls.is_empty() {
        (inner, String::new(), 0, 0)
    } else {
        let (body, preamble, n, still) = rewrite_with_known_expr(&found.var_name, &inner, counter + inner_hoisted, &calls);
        let iterable = for_of_expr(&found.header).unwrap_or_default();
        (body, finish_preamble(&preamble, &iterable), n, still)
    };
    let mut out = String::new();
    out.push_str(&source[..found.keyword_at]);
    out.push_str(&preamble);
    out.push_str("for (");
    out.push_str(&found.header);
    out.push_str(") {");
    out.push_str(&body);
    out.push('}');
    let (tail, tail_hoisted, tail_unhoisted) = rewrite_finished(&source[found.end..], counter + inner_hoisted + hoisted_here);
    out.push_str(&tail);
    (out, inner_hoisted + hoisted_here + tail_hoisted, inner_unhoisted + still + tail_unhoisted)
}

fn for_of_expr(header: &str) -> Option<String> {
    let (_, expr) = parse_for_of_header(header)?;
    Some(expr)
}

/// Helper injected beside hoisted loops. Collects keys without a per-row query.
pub const COLLECT_HELPER: &str = "\
function __bosCollect(rows, pick) {\n\
    const keys = [];\n\
    const seen = new Set();\n\
    for (const row of rows || []) {\n\
        const key = pick(row);\n\
        const text = String(key);\n\
        if (key !== undefined && key !== null && !seen.has(text)) { seen.add(text); keys.push(key); }\n\
    }\n\
    return keys;\n\
}\n";

pub fn ensure_collect_helper(source: &str) -> String {
    if !source.contains("__bosCollect(") || source.contains("function __bosCollect") {
        return source.to_string();
    }
    format!("{COLLECT_HELPER}{source}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hoists_get_inside_for_of() {
        let src = "for (const line of lines) {\n  const item = Item.get(line.itemNo);\n  rec.total += item.price;\n}\n";
        let out = optimize_loops_finished(src);
        assert_eq!(out.hoisted, 1, "{}", out.javascript);
        assert!(out.javascript.contains("Item.getMany(__bosCollect(lines"), "{}", out.javascript);
        assert!(out.javascript.contains("__bosMap0_0.get(String(line.itemNo))"), "{}", out.javascript);
        assert!(!out.javascript.contains("Item.get(line.itemNo)"), "{}", out.javascript);
        let with_helper = ensure_collect_helper(&out.javascript);
        assert!(with_helper.contains("function __bosCollect"));
    }

    #[test]
    fn does_not_hoist_a_get_written_inside_a_string() {
        let src = "for (const line of lines) {\n  const note = \"Item.get(line.itemNo)\";\n}\n";
        let out = optimize_loops_finished(src);
        assert_eq!(out.hoisted, 0, "{}", out.javascript);
        assert!(out.javascript.contains("\"Item.get(line.itemNo)\""));
    }

    #[test]
    fn leaves_unrelated_gets() {
        let src = "const item = Item.get(rec.itemNo);\n";
        let out = optimize_loops_finished(src);
        assert_eq!(out.hoisted, 0);
        assert!(out.javascript.contains("Item.get(rec.itemNo)"));
    }

    #[test]
    fn rewrites_decimal_fields_to_exact_math() {
        let mut fields = HashSet::new();
        fields.insert("unitPrice".to_string());
        fields.insert("quantity".to_string());
        let src = "rec.lineAmount = rec.unitPrice * rec.quantity;";
        let out = rewrite_decimal(src, &fields);
        assert_eq!(
            out,
            "rec.lineAmount = ctx.decimal(rec.unitPrice).mul(rec.quantity).toString();"
        );
    }

    #[test]
    fn decimal_rewrite_ignores_strings() {
        let mut fields = HashSet::new();
        fields.insert("quantity".to_string());
        let src = r#"const label = "quantity * quantity"; rec.quantity = rec.quantity + 1;"#;
        let out = rewrite_decimal(src, &fields);
        assert!(out.contains(r#""quantity * quantity""#), "{out}");
        assert!(out.contains("ctx.decimal(rec.quantity).add(1).toString()"), "{out}");
    }
}
