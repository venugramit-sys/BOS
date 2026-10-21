//! High-Speed TypeScript Type Stripper for BOS-Script
//! Strips TypeScript interfaces, types, enums, type annotations, and generics
//! producing 100% compliant ECMAScript executable by QuickJS in microseconds.

use regex::Regex;
use std::sync::LazyLock;

static INTERFACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?ms)\binterface\s+[A-Za-z0-9_]+(?:\s*<[^>]*>)?(?:\s+extends\s+[^{]+)?\s*\{[^}]*\}")
        .expect("Valid regex for INTERFACE_RE")
});

static TYPE_ALIAS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)\btype\s+[A-Za-z0-9_]+(?:\s*<[^>]*>)?\s*=\s*[^;]+;")
        .expect("Valid regex for TYPE_ALIAS_RE")
});

static AS_CAST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+(?:as|satisfies)\s+(?:const|[A-Za-z0-9_]+(?:\s*<[^>]*>)?(?:\s*\[\s*\])?(?:\s*[|&]\s*[A-Za-z0-9_]+(?:\s*<[^>]*>)?(?:\s*\[\s*\])?)*)")
        .expect("Valid regex for AS_CAST_RE")
});

static ANGLE_CAST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([=(\[,:?]\s*)<\s*[A-Za-z_][A-Za-z0-9_]*(?:\s*<[^>]*>)?(?:\s*\[\s*\])?\s*>\s*([A-Za-z0-9_(\[{])")
        .expect("Valid regex for ANGLE_CAST_RE")
});

static NON_NULL_ASSERT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Za-z0-9_)\]])!\s*([.\[(),;\]\}\s])")
        .expect("Valid regex for NON_NULL_ASSERT_RE")
});

static GENERIC_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\.([A-Za-z0-9_]+)\s*<[A-Za-z0-9_,\s<>|\[\]&]+>\s*\(")
        .expect("Valid regex for GENERIC_CALL_RE")
});

static GENERIC_NEW_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bnew\s+([A-Za-z_][A-Za-z0-9_.]*)\s*<[A-Za-z0-9_,\s<>|\[\]&]+>\s*\(")
        .expect("Valid regex for GENERIC_NEW_RE")
});

static CATCH_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bcatch\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*[^)]+\)")
        .expect("Valid regex for CATCH_TYPE_RE")
});

static FOR_VAR_TYPE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bfor\s*\(\s*(const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*[^=;)]+?(\s+(?:of|in)\s+|\s*=\s*)")
        .expect("Valid regex for FOR_VAR_TYPE_RE")
});

static MODIFIERS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*(public|private|protected|readonly|override)\s+")
        .expect("Valid regex for MODIFIERS_RE")
});

static ENUM_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?ms)\benum\s+([A-Za-z0-9_]+)\s*\{([^}]*)\}")
        .expect("Valid regex for ENUM_RE")
});

static ARROW_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\(([^)]*)\)\s*(?::\s*[^=\{]+)?\s*=>")
        .expect("Valid regex for ARROW_FN_RE")
});

static FN_DECL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(async\s+)?function\s+([A-Za-z0-9_]*)\s*(?:<[^>]*>)?\s*\(([^)]*)\)\s*(?::\s*[^=\{]+)?\s*\{")
        .expect("Valid regex for FN_DECL_RE")
});

/// Transform TypeScript `enum Foo { ... }` into standard `const Foo = Object.freeze({ ... });`
fn lower_enums(source: &str) -> String {
    ENUM_RE.replace_all(source, |caps: &regex::Captures| {
        let enum_name = &caps[1];
        let body = &caps[2];
        let mut entries = Vec::new();

        for raw_line in body.split([',', ';', '\n']) {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let val = v.trim();
                if !key.is_empty() {
                    entries.push(format!("{}: {}", key, val));
                }
            } else {
                let key = line.trim();
                if !key.is_empty() {
                    entries.push(format!("{}: \"{}\"", key, key));
                }
            }
        }

        format!("const {} = Object.freeze({{ {} }});", enum_name, entries.join(", "))
    }).to_string()
}

/// Strip parameter and return type annotations from functions and arrow functions:
/// e.g. `(rec: Customer, ctx: BosSessionContext): void =>` -> `(rec, ctx) =>`
/// e.g. `function foo(x: number, y: string): number` -> `function foo(x, y)`
fn strip_function_signatures(source: &str) -> String {
    // 1. Arrow functions: (a: T, b: U): Ret =>
    let res = ARROW_FN_RE.replace_all(source, |caps: &regex::Captures| {
        let params_raw = &caps[1];
        let clean_params = strip_param_types(params_raw);
        format!("({}) =>", clean_params)
    }).to_string();

    // 2. Standard function declarations / methods: [async] function foo<T>(a: T, b: U): Ret {
    FN_DECL_RE.replace_all(&res, |caps: &regex::Captures| {
        let async_prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let name = &caps[2];
        let params_raw = &caps[3];
        let clean_params = strip_param_types(params_raw);
        if name.is_empty() {
            format!("{}function({}) {{", async_prefix, clean_params)
        } else {
            format!("{}function {}({}) {{", async_prefix, name, clean_params)
        }
    }).to_string()
}

/// Split parameter list by top-level commas (ignoring commas inside `<...>`, `{...}`, `(...)`, `[...]`)
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth_angle: i32 = 0;
    let mut depth_brace: i32 = 0;
    let mut depth_paren: i32 = 0;
    let mut depth_bracket: i32 = 0;
    let mut start = 0;

    for (idx, ch) in s.char_indices() {
        match ch {
            '<' => depth_angle += 1,
            '>' => depth_angle = (depth_angle - 1).max(0),
            '{' => depth_brace += 1,
            '}' => depth_brace = (depth_brace - 1).max(0),
            '(' => depth_paren += 1,
            ')' => depth_paren = (depth_paren - 1).max(0),
            '[' => depth_bracket += 1,
            ']' => depth_bracket = (depth_bracket - 1).max(0),
            ',' if depth_angle == 0 && depth_brace == 0 && depth_paren == 0 && depth_bracket == 0 => {
                parts.push(&s[start..idx]);
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

/// Strip types from parameter lists: `a: string, b: number = 10, c?: boolean, { id }: { id: string }` -> `a, b = 10, c, { id }`
fn strip_param_types(params: &str) -> String {
    let mut cleaned = Vec::new();
    for p in split_top_level_commas(params) {
        let trimmed = p.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Find top-level `=` for default value
        let mut eq_pos: Option<usize> = None;
        let mut colon_pos: Option<usize> = None;
        let mut d_angle: i32 = 0;
        let mut d_brace: i32 = 0;
        let mut d_paren: i32 = 0;
        let mut d_bracket: i32 = 0;
        let bytes = trimmed.as_bytes();
        for (i, ch) in trimmed.char_indices() {
            match ch {
                '<' => d_angle += 1,
                '>' => {
                    if i == 0 || bytes[i - 1] != b'=' {
                        d_angle = (d_angle - 1).max(0);
                    }
                }
                '{' => d_brace += 1,
                '}' => d_brace = (d_brace - 1).max(0),
                '(' => d_paren += 1,
                ')' => d_paren = (d_paren - 1).max(0),
                '[' => d_bracket += 1,
                ']' => d_bracket = (d_bracket - 1).max(0),
                ':' if d_angle == 0 && d_brace == 0 && d_paren == 0 && d_bracket == 0 && colon_pos.is_none() && eq_pos.is_none() => {
                    colon_pos = Some(i);
                }
                '=' if d_angle == 0 && d_brace == 0 && d_paren == 0 && d_bracket == 0 && eq_pos.is_none() => {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                        continue;
                    }
                    eq_pos = Some(i);
                }
                _ => {}
            }
        }

        let clean_param_lhs = |lhs: &str| -> String {
            let s = lhs.trim().trim_end_matches('?').trim();
            for prefix in ["public ", "private ", "protected ", "readonly "] {
                if let Some(rest) = s.strip_prefix(prefix) {
                    return rest.trim().to_string();
                }
            }
            s.to_string()
        };

        match (colon_pos, eq_pos) {
            (Some(c_idx), Some(e_idx)) if c_idx < e_idx => {
                let param_name = clean_param_lhs(&trimmed[..c_idx]);
                let default_val = trimmed[e_idx + 1..].trim();
                cleaned.push(format!("{} = {}", param_name, default_val));
            }
            (Some(c_idx), _) => {
                let param_name = clean_param_lhs(&trimmed[..c_idx]);
                cleaned.push(param_name);
            }
            (None, Some(e_idx)) => {
                let param_name = clean_param_lhs(&trimmed[..e_idx]);
                let default_val = trimmed[e_idx + 1..].trim();
                cleaned.push(format!("{} = {}", param_name, default_val));
            }
            (None, None) => {
                cleaned.push(clean_param_lhs(trimmed));
            }
        }
    }
    cleaned.join(", ")
}

/// Strip variable declaration type annotations with balanced delimiter tracking:
/// Supports:
/// - `const x: number = 10;` -> `const x = 10;`
/// - `let test: Date = new Date();` -> `let test = new Date();`
/// - `let note: Note = { code: "N1", content: "Hi" };` -> `let note = { code: "N1", content: "Hi" };`
/// - `let inline: { code: string; qty: number } = { code: "A", qty: 1 };` -> `let inline = { code: "A", qty: 1 };`
/// - `const { code, content }: NoteRecord = rec;` -> `const { code, content } = rec;`
/// - `const [first, second]: [string, number] = ["A", 1];` -> `const [first, second] = ["A", 1];`
/// - `const fn: (a: number) => boolean = (a) => a > 0;` -> `const fn = (a) => a > 0;`
fn strip_var_type_annotations(source: &str) -> String {
    // First strip `for (const x: T of list)` and `catch (e: unknown)`
    let step_for = FOR_VAR_TYPE_RE
        .replace_all(source, "for ($1 $2$3")
        .to_string();
    let step_catch = CATCH_TYPE_RE
        .replace_all(&step_for, "catch ($1)")
        .to_string();

    let mut out = String::with_capacity(step_catch.len());
    for line in step_catch.split_inclusive('\n') {
        out.push_str(&strip_var_type_on_line(line));
    }
    out
}

fn strip_var_type_on_line(line: &str) -> String {
    let trimmed_start = line.trim_start();
    let indent_len = line.len() - trimmed_start.len();
    let indent = &line[..indent_len];

    let kw = if trimmed_start.starts_with("const ") {
        "const"
    } else if trimmed_start.starts_with("let ") {
        "let"
    } else if trimmed_start.starts_with("var ") {
        "var"
    } else {
        return line.to_string();
    };

    let after_kw = &trimmed_start[kw.len()..];
    let after_kw_trimmed = after_kw.trim_start();
    if after_kw_trimmed.is_empty() {
        return line.to_string();
    }

    // Parse variable pattern (identifier, `{ ... }` destructuring, or `[ ... ]` array destructuring)
    let bytes = after_kw_trimmed.as_bytes();
    let mut idx = 0;
    if bytes[0] == b'{' || bytes[0] == b'[' {
        let open = bytes[0];
        let close = if open == b'{' { b'}' } else { b']' };
        let mut depth = 0i32;
        while idx < bytes.len() {
            if bytes[idx] == open {
                depth += 1;
            } else if bytes[idx] == close {
                depth -= 1;
                if depth == 0 {
                    idx += 1;
                    break;
                }
            }
            idx += 1;
        }
    } else {
        while idx < bytes.len() && (bytes[idx].is_ascii_alphanumeric() || bytes[idx] == b'_' || bytes[idx] == b'$') {
            idx += 1;
        }
        // Optional definite assignment assertion `let x!: string;`
        if idx < bytes.len() && bytes[idx] == b'!' {
            idx += 1;
        }
    }

    if idx == 0 {
        return line.to_string();
    }

    let var_pattern = after_kw_trimmed[..idx].trim_end_matches('!');
    let rest = after_kw_trimmed[idx..].trim_start();
    if !rest.starts_with(':') {
        return line.to_string();
    }

    // Scan the type annotation after `:` until top-level `=` (not `=>`) or `;` or end of line
    let type_part = &rest[1..];
    let t_bytes = type_part.as_bytes();
    let mut d_angle = 0i32;
    let mut d_brace = 0i32;
    let mut d_paren = 0i32;
    let mut d_bracket = 0i32;
    let mut split_at: Option<(usize, char)> = None;

    let mut i = 0;
    while i < t_bytes.len() {
        let b = t_bytes[i];
        match b {
            b'<' => d_angle += 1,
            b'>' => {
                if i == 0 || t_bytes[i - 1] != b'=' {
                    d_angle = (d_angle - 1).max(0);
                }
            }
            b'{' => d_brace += 1,
            b'}' => d_brace = (d_brace - 1).max(0),
            b'(' => d_paren += 1,
            b')' => d_paren = (d_paren - 1).max(0),
            b'[' => d_bracket += 1,
            b']' => d_bracket = (d_bracket - 1).max(0),
            b'=' if d_angle == 0 && d_brace == 0 && d_paren == 0 && d_bracket == 0 => {
                if i + 1 < t_bytes.len() && t_bytes[i + 1] == b'>' {
                    i += 2;
                    continue;
                }
                split_at = Some((i, '='));
                break;
            }
            b';' if d_angle == 0 && d_brace == 0 && d_paren == 0 && d_bracket == 0 => {
                split_at = Some((i, ';'));
                break;
            }
            _ => {}
        }
        i += 1;
    }

    match split_at {
        Some((pos, '=')) => {
            let rhs = &type_part[pos + 1..];
            format!("{}{} {} ={}", indent, kw, var_pattern, rhs)
        }
        Some((pos, ';')) => {
            let after_semi = &type_part[pos + 1..];
            format!("{}{} {};{}", indent, kw, var_pattern, after_semi)
        }
        _ => {
            // Uninitialized declaration without semicolon on single line (`let x: number\n`)
            let has_newline = line.ends_with('\n');
            if has_newline {
                format!("{}{} {};\n", indent, kw, var_pattern)
            } else {
                format!("{}{} {}", indent, kw, var_pattern)
            }
        }
    }
}

fn is_regex_keyword(name: &str) -> bool {
    matches!(
        name,
        "return"
            | "throw"
            | "case"
            | "typeof"
            | "void"
            | "delete"
            | "in"
            | "of"
            | "await"
            | "yield"
            | "instanceof"
            | "new"
    )
}

fn flush_template_literal(literal: &mut Vec<char>, masked: &mut String, vaults: &mut Vec<String>) {
    if literal.is_empty() {
        return;
    }
    let text: String = literal.drain(..).collect();
    let newlines = text.chars().filter(|&c| c == '\n').count();
    let id = vaults.len();
    vaults.push(text);
    masked.push_str(&format!("__BOS_TPL_{id}__{}", "\n".repeat(newlines)));
}

/// Mask one `` `...` `` template. Literal text is vaulted; `${ ... }` stays as code.
fn mask_template(
    chars: &[char],
    start: usize,
    masked: &mut String,
    vaults: &mut Vec<String>,
) -> Option<usize> {
    let len = chars.len();
    let mut i = start + 1;
    let mut literal: Vec<char> = Vec::new();
    masked.push('`');
    while i < len {
        let ch = chars[i];
        if ch == '\\' && i + 1 < len {
            literal.push(ch);
            literal.push(chars[i + 1]);
            i += 2;
            continue;
        }
        if ch == '`' {
            flush_template_literal(&mut literal, masked, vaults);
            masked.push('`');
            return Some(i + 1);
        }
        if ch == '$' && i + 1 < len && chars[i + 1] == '{' {
            flush_template_literal(&mut literal, masked, vaults);
            masked.push('$');
            masked.push('{');
            i = copy_template_hole(chars, i + 2, masked, vaults)?;
            continue;
        }
        literal.push(ch);
        i += 1;
    }
    None
}

/// Copy the code inside `${ ... }` until its matching `}`, masking nested strings.
fn copy_template_hole(
    chars: &[char],
    mut i: usize,
    masked: &mut String,
    vaults: &mut Vec<String>,
) -> Option<usize> {
    let mut depth = 1i32;
    let mut regex_ok = true;
    let mut ident_buf = String::new();
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' || ch == '\'' {
            i = mask_quoted(chars, i, masked, vaults);
            ident_buf.clear();
            regex_ok = false;
            continue;
        }
        if ch == '`' {
            let mark = vaults.len();
            let mut nested = String::new();
            if let Some(end) = mask_template(chars, i, &mut nested, vaults) {
                masked.push_str(&nested);
                i = end;
                ident_buf.clear();
                regex_ok = false;
                continue;
            }
            vaults.truncate(mark);
        }
        if ch == '/' && i + 1 < chars.len() && (chars[i + 1] == '/' || chars[i + 1] == '*') {
            i = mask_comment(chars, i, masked, vaults);
            continue;
        }
        if ch == '/' && (regex_ok || is_regex_keyword(&ident_buf)) {
            if let Some(end) = try_scan_regex(chars, i) {
                let literal: String = chars[i..end].iter().collect();
                let id = vaults.len();
                vaults.push(literal);
                masked.push_str(&format!("\"__BOS_LIT_{id}__\""));
                i = end;
                ident_buf.clear();
                regex_ok = false;
                continue;
            }
        }
        if ch == '{' {
            depth += 1;
            masked.push(ch);
            i += 1;
            ident_buf.clear();
            regex_ok = true;
            continue;
        }
        if ch == '}' {
            depth -= 1;
            masked.push('}');
            i += 1;
            ident_buf.clear();
            regex_ok = true;
            if depth == 0 {
                return Some(i);
            }
            continue;
        }
        masked.push(ch);
        i += 1;
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            ident_buf.push(ch);
            regex_ok = false;
        } else if ch.is_whitespace() {
            if is_regex_keyword(&ident_buf) {
                regex_ok = true;
            }
            ident_buf.clear();
        } else {
            let keyword = is_regex_keyword(&ident_buf);
            ident_buf.clear();
            regex_ok = keyword
                || matches!(
                    ch,
                    '(' | '[' | ',' | ';' | ':' | '=' | '!' | '?' | '+' | '-' | '*' | '%' | '&' | '|'
                        | '^' | '~' | '<' | '>'
                );
        }
    }
    None
}

fn mask_quoted(chars: &[char], start: usize, masked: &mut String, vaults: &mut Vec<String>) -> usize {
    let quote = chars[start];
    let mut i = start + 1;
    while i < chars.len() && chars[i] != quote && chars[i] != '\n' {
        if chars[i] == '\\' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        i += 1;
    }
    if i < chars.len() && chars[i] == quote {
        i += 1;
    }
    let literal: String = chars[start..i].iter().collect();
    let id = vaults.len();
    vaults.push(literal);
    masked.push_str(&format!("\"__BOS_LIT_{id}__\""));
    i
}

fn mask_comment(chars: &[char], start: usize, masked: &mut String, vaults: &mut Vec<String>) -> usize {
    let mut i = start;
    let mut newlines = 0usize;
    if i + 1 < chars.len() && chars[i + 1] == '/' {
        while i < chars.len() && chars[i] != '\n' {
            i += 1;
        }
    } else {
        i += 2;
        while i < chars.len() && !(chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/') {
            if chars[i] == '\n' {
                newlines += 1;
            }
            i += 1;
        }
        if i < chars.len() {
            i += 2;
        }
    }
    let comment: String = chars[start..i].iter().collect();
    let id = vaults.len();
    vaults.push(comment);
    masked.push_str(&format!("/*__BOS_LIT_{id}__*/{}", "\n".repeat(newlines)));
    i
}

fn try_scan_regex(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut in_class = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '\n' {
            return None;
        }
        i += 1;
        if ch == '\\' {
            if i < chars.len() {
                i += 1;
            }
            continue;
        }
        if ch == '[' && !in_class {
            in_class = true;
            continue;
        }
        if ch == ']' && in_class {
            in_class = false;
            continue;
        }
        if ch == '/' && !in_class {
            while i < chars.len() && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            return Some(i);
        }
    }
    None
}

/// Remove syntax QuickJS cannot execute. Type-only imports and decorator lines are
/// dropped. `using` becomes `const` because disposal is not available in the sandbox.
fn lower_script_syntax(source: String) -> String {
    let mut out = String::new();
    for line in source.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        if trimmed.starts_with("import type ")
            || trimmed.starts_with("export type ")
            || is_decorator_only_line(trimmed)
        {
            if line.ends_with('\n') {
                out.push('\n');
            }
            continue;
        }
        if trimmed.starts_with("using ") {
            out.push_str(&line[..indent_len]);
            out.push_str("const ");
            out.push_str(&trimmed["using ".len()..]);
            continue;
        }
        out.push_str(line);
    }
    out
}

fn is_decorator_only_line(trimmed: &str) -> bool {
    let body = trimmed.trim_end().trim_end_matches(';');
    let Some(rest) = body.strip_prefix('@') else {
        return false;
    };
    let name_end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    if name_end == 0 {
        return false;
    }
    let after = rest[name_end..].trim();
    after.is_empty() || (after.starts_with('(') && after.ends_with(')'))
}

/// Mask string literals (`"..."`, `'...'`, `` `...` ``) and comments (`//...`, `/*...*/`)
/// into `__BOS_LIT_N__` placeholders while preserving newlines inside multi-line blocks
/// so TypeScript regex/scanner passes never corrupt string contents or shift line numbers.
fn mask_literals_and_comments(source: &str) -> (String, Vec<String>) {
    let mut masked = String::with_capacity(source.len());
    let mut vaults: Vec<String> = Vec::new();
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut regex_ok = true;
    let mut ident_buf = String::new();

    while i < len {
        // 1. Line comment `// ...`
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            let start = i;
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            let comment: String = chars[start..i].iter().collect();
            let id = vaults.len();
            vaults.push(comment);
            masked.push_str(&format!("/*__BOS_LIT_{}__*/", id));
            continue;
        }

        // 2. Block comment `/* ... */`
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            let mut newlines = 0usize;
            while i < len && !(chars[i] == '*' && i + 1 < len && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    newlines += 1;
                }
                i += 1;
            }
            if i < len {
                i += 2;
            }
            let comment: String = chars[start..i].iter().collect();
            let id = vaults.len();
            vaults.push(comment);
            masked.push_str(&format!("/*__BOS_LIT_{}__*/{}", id, "\n".repeat(newlines)));
            continue;
        }

        // 3. Double or single quoted string `"..."` or `'...'`
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote && chars[i] != '\n' {
                if chars[i] == '\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                i += 1;
            }
            if i < len && chars[i] == quote {
                i += 1;
            }
            let literal: String = chars[start..i].iter().collect();
            let id = vaults.len();
            vaults.push(literal);
            masked.push_str(&format!("\"__BOS_LIT_{}__\"", id));
            continue;
        }

        // 4. Template literal, including `${...}` holes. Literal text is vaulted; the hole stays code.
        if chars[i] == '`' {
            let mark = vaults.len();
            let mut local = String::new();
            if let Some(end) = mask_template(&chars, i, &mut local, &mut vaults) {
                masked.push_str(&local);
                i = end;
                ident_buf.clear();
                regex_ok = false;
                continue;
            }
            vaults.truncate(mark);
        }

        // 5. Regex literal. `{` and `}` inside it must not be read as type syntax.
        if chars[i] == '/' && (regex_ok || is_regex_keyword(&ident_buf)) {
            if let Some(end) = try_scan_regex(&chars, i) {
                let literal: String = chars[i..end].iter().collect();
                let newlines = literal.chars().filter(|&c| c == '\n').count();
                let id = vaults.len();
                vaults.push(literal);
                masked.push_str(&format!("\"__BOS_LIT_{id}__\"{}", "\n".repeat(newlines)));
                i = end;
                ident_buf.clear();
                regex_ok = false;
                continue;
            }
        }

        let ch = chars[i];
        masked.push(ch);
        i += 1;
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            ident_buf.push(ch);
            regex_ok = false;
        } else if ch.is_whitespace() {
            if is_regex_keyword(&ident_buf) {
                regex_ok = true;
            }
            ident_buf.clear();
        } else {
            let keyword = is_regex_keyword(&ident_buf);
            ident_buf.clear();
            regex_ok = keyword
                || matches!(
                    ch,
                    '(' | '[' | '{' | ',' | ';' | ':' | '=' | '!' | '?' | '+' | '-' | '*' | '%' | '&'
                        | '|' | '^' | '~' | '<' | '>' | '}'
                );
        }
    }

    (masked, vaults)
}

fn unmask_literals_and_comments(mut code: String, vaults: &[String]) -> String {
    for (id, orig) in vaults.iter().enumerate() {
        let comment_tok = format!("/*__BOS_LIT_{}__*/", id);
        if code.contains(&comment_tok) {
            let newlines = orig.chars().filter(|&c| c == '\n').count();
            let comment_with_nl = format!("{}{}", comment_tok, "\n".repeat(newlines));
            if newlines > 0 && code.contains(&comment_with_nl) {
                code = code.replace(&comment_with_nl, orig);
            } else {
                code = code.replace(&comment_tok, orig);
            }
        }
        let str_tok = format!("\"__BOS_LIT_{}__\"", id);
        if code.contains(&str_tok) {
            code = code.replace(&str_tok, orig);
        }
        let tpl_tok = format!("__BOS_TPL_{id}__");
        if code.contains(&tpl_tok) {
            let newlines = orig.chars().filter(|&c| c == '\n').count();
            let tpl_with_nl = format!("{tpl_tok}{}", "\n".repeat(newlines));
            if newlines > 0 && code.contains(&tpl_with_nl) {
                code = code.replace(&tpl_with_nl, orig);
            } else {
                code = code.replace(&tpl_tok, orig);
            }
        }
    }
    code
}

pub fn syntax_diagnostics(source: &str, base_line: usize) -> Vec<bosc_ir::CompilerDiagnosticIR> {
    let lowered = lower_script_syntax(source.to_string());
    crate::ts_oxc::typescript_syntax_diagnostics(&lowered, base_line)
}

fn finish_script(javascript: String) -> String {
    let rewritten = crate::script_opt::optimize_loops(&javascript);
    crate::script_opt::ensure_collect_helper(&rewritten.javascript)
}

/// Strip all TypeScript types from raw source script, producing pure standard JavaScript.
/// Loop queries are hoisted after erasure.
pub fn strip_typescript(source: &str) -> String {
    if !source.contains(':')
        && !source.contains("interface")
        && !source.contains("enum ")
        && !source.contains("type ")
        && !source.contains(" as ")
        && !source.contains(" satisfies ")
        && !source.contains('<')
        && !source.contains('!')
        && !source.contains("using ")
        && !source.contains('@')
        && !source.contains("for")
    {
        return lower_script_syntax(source.to_string());
    }

    // 0. Mask string literals and comments so TypeScript regexes never alter string contents
    let (masked, vaults) = mask_literals_and_comments(source);

    // 1. Lower enums to Object.freeze (preserving line count)
    let step1 = lower_enums(&masked);

    // 2. Strip interface declarations (preserving line count with newline padding)
    let step2 = INTERFACE_RE
        .replace_all(&step1, |caps: &regex::Captures| {
            let matched = &caps[0];
            let nl_count = matched.chars().filter(|&c| c == '\n').count();
            "\n".repeat(nl_count)
        })
        .to_string();

    // 3. Strip type alias declarations
    let step3 = TYPE_ALIAS_RE
        .replace_all(&step2, |caps: &regex::Captures| {
            let matched = &caps[0];
            let nl_count = matched.chars().filter(|&c| c == '\n').count();
            "\n".repeat(nl_count)
        })
        .to_string();

    // 4. Strip function signature types
    let step4 = strip_function_signatures(&step3);

    // 5. Strip variable type annotations (including destructuring, inline object types, for-of, catch)
    let step5 = strip_var_type_annotations(&step4);

    // 6. Strip generic constructor calls: `new Map<string, number>()` -> `new Map()`
    let step6 = GENERIC_NEW_RE.replace_all(&step5, "new $1(").to_string();

    // 7. Strip generic method invocations: `.getModel<Customer>("Customer")` -> `.getModel("Customer")`
    let step7 = GENERIC_CALL_RE.replace_all(&step6, ".$1(").to_string();

    // 8. Strip 'as Type' and 'satisfies Type' assertions
    let step8 = AS_CAST_RE.replace_all(&step7, "").to_string();

    // 9. Strip angle-bracket casts: `<string>val` -> `val`
    let step9 = ANGLE_CAST_RE.replace_all(&step8, "$1$2").to_string();

    // 10. Strip non-null assertions
    let step10 = NON_NULL_ASSERT_RE.replace_all(&step9, "$1$2").to_string();

    // 11. Strip access modifiers
    let step11 = MODIFIERS_RE.replace_all(&step10, "").to_string();

    // 12. Restore original string literals and comments
    let restored = unmask_literals_and_comments(step11, &vaults);

    // 13. Drop type-only imports and decorator lines. Rewrite `using` to `const`.
    // 14. Hoist Model.get inside for...of into one getMany.
    finish_script(lower_script_syntax(restored))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_interfaces_and_types() {
        let ts = r#"
            interface SalesHeader {
                orderNo: string;
                totalAmount: number;
            }
            type OrderStatus = "Draft" | "Released" | "Posted";

            const status: OrderStatus = "Draft";
            rec.orderNo = "SO-101";
        "#;
        let js = strip_typescript(ts);
        assert!(!js.contains("interface SalesHeader"));
        assert!(!js.contains("type OrderStatus"));
        assert!(js.contains("const status = \"Draft\";"));
        assert!(js.contains("rec.orderNo = \"SO-101\";"));
    }

    #[test]
    fn test_strip_function_annotations_and_generics() {
        let ts = r#"
            function calculateTotal(lines: Array<any>, taxRate: number): number {
                const subtotal: number = lines.reduce((acc, l) => acc + l.amount, 0);
                return subtotal * (1 + taxRate);
            }

            const header = ctx.db.getModel<SalesHeader>("SalesHeader").get("SO-101")!;
            const item = (rec as any).itemCode;
        "#;
        let js = strip_typescript(ts);
        assert!(js.contains("function calculateTotal(lines, taxRate) {"));
        assert!(js.contains("const subtotal = lines.reduce"));
        assert!(js.contains(".getModel(\"SalesHeader\").get(\"SO-101\")"));
        assert!(!js.contains(" as any"));
    }

    #[test]
    fn test_lower_enums() {
        let ts = r#"
            enum ApprovalStatus {
                Pending,
                Approved,
                Rejected
            }
            rec.status = ApprovalStatus.Approved;
        "#;
        let js = strip_typescript(ts);
        assert!(js.contains("const ApprovalStatus = Object.freeze({ Pending: \"Pending\", Approved: \"Approved\", Rejected: \"Rejected\" });"));
        assert!(js.contains("rec.status = ApprovalStatus.Approved;"));
    }

    #[test]
    fn test_strip_all_variable_declaration_and_initialization_patterns() {
        let ts = r#"
            let d: Date = new Date();
            let note: Note = new Note({ code: "N1", content: "Hello" });
            let dec: Decimal = Decimal("10.50");
            const draft: { code: string; qty: number } = { code: "A", qty: 5 };
            const { code, content }: NoteRecord = rec;
            const [first, second]: [string, number] = ["A", 1];
            const cache: Map<string, number> = new Map<string, number>();
            const tags: Set<string> = new Set<string>(["urgent"]);
            const isPositive: (x: number) => boolean = (x: number): boolean => x > 0;
            let uninit: string;
            for (const item: Note of [note]) {
                try {
                    const castVal = <string>item.code;
                } catch (err: unknown) {
                    ctx.log("error", String(err));
                }
            }
        "#;
        let js = strip_typescript(ts);
        assert!(js.contains("let d = new Date();"), "Date var: {}", js);
        assert!(js.contains("let note = new Note({ code: \"N1\", content: \"Hello\" });"), "Note var: {}", js);
        assert!(js.contains("let dec = Decimal(\"10.50\");"), "Decimal var: {}", js);
        assert!(js.contains("const draft = { code: \"A\", qty: 5 };"), "Inline object type: {}", js);
        assert!(js.contains("const { code, content } = rec;"), "Object destructuring: {}", js);
        assert!(js.contains("const [first, second] = [\"A\", 1];"), "Array destructuring: {}", js);
        assert!(js.contains("const cache = new Map();"), "Generic new Map: {}", js);
        assert!(js.contains("const tags = new Set([\"urgent\"]);"), "Generic new Set: {}", js);
        assert!(js.contains("let uninit;"), "Uninitialized let: {}", js);
        assert!(js.contains("for (const item of [note])"), "for-of typed: {}", js);
        assert!(js.contains("catch (err)"), "catch typed: {}", js);
    }

    #[test]
    fn test_strip_typescript_preserves_string_literals_and_line_numbers() {
        let ts = "interface Config {\n    host: string;\n}\nconst msg: string = \"interface Fake { a: number } as string satisfies any\";\n// let commentedOut: number = 10 as any;\nconst ok: boolean = true;";
        let js = strip_typescript(ts);
        // Exact line count must be preserved
        assert_eq!(ts.lines().count(), js.lines().count(), "Line numbers must be 100% preserved");
        // String literal containing TS keywords must be untouched
        assert!(
            js.contains("\"interface Fake { a: number } as string satisfies any\""),
            "String literal must not be corrupted: {}",
            js
        );
        assert!(js.contains("const ok = true;"));
    }

    #[test]
    fn test_strip_preserves_template_regex_and_lowers_type_only_syntax() {
        let ts = r#"
            let dt: Date = new Date();
            const pattern: RegExp = /}/;
            ctx.ui.message(`date ${dt as Date}`, "info");
            import type { Note } from "./note";
            using file = openFile();
            @sealed
        "#;
        let js = strip_typescript(ts);
        assert!(js.contains("let dt = new Date();"), "{js}");
        assert!(js.contains("const pattern = /}/;"), "{js}");
        assert!(js.contains("`date ${dt}`"), "{js}");
        assert!(!js.contains("import type"), "{js}");
        assert!(js.contains("const file = openFile();"), "{js}");
        assert!(!js.contains("@sealed"), "{js}");
    }
}
