//! Client-side page expression language (`showWhen`, `enableWhen`, `editableWhen`, `styleExpr`,
//! `subPageLink` / `runPageLink` values).
//!
//! The language is deliberately small and side-effect free: literals, record/page-variable references,
//! comparison, boolean logic, arithmetic and a fixed whitelist of helper functions. Expressions are
//! compiled to JavaScript that only touches `rec[...]`, `page[...]` and the page-kit helper object `fx`,
//! so generated pages never need `eval` or `Function`.

use bosc_ir::ClientExprIR;
use bosc_syntax::ast::{BinaryOp, Expr, UnaryOp};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum PExpr {
    Null,
    Bool(bool),
    /// Decimal literal kept as validated source text (exact; never converted to f64 here).
    Num(String),
    Str(String),
    List(Vec<PExpr>),
    /// Dotted reference: `rec.status`, `page.password`, or a bare `status`.
    Ref(Vec<String>),
    Call { name: String, args: Vec<PExpr> },
    Unary { op: PUnOp, expr: Box<PExpr> },
    Binary { op: PBinOp, left: Box<PExpr>, right: Box<PExpr> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PUnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PBinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    Add,
    Sub,
    Mul,
    Div,
}

impl PBinOp {
    fn precedence(self) -> u8 {
        match self {
            PBinOp::Or => 1,
            PBinOp::And => 2,
            PBinOp::Eq | PBinOp::Ne | PBinOp::Lt | PBinOp::Le | PBinOp::Gt | PBinOp::Ge | PBinOp::In => 3,
            PBinOp::Add | PBinOp::Sub => 4,
            PBinOp::Mul | PBinOp::Div => 5,
        }
    }

    fn symbol(self) -> &'static str {
        match self {
            PBinOp::Or => "||",
            PBinOp::And => "&&",
            PBinOp::Eq => "==",
            PBinOp::Ne => "!=",
            PBinOp::Lt => "<",
            PBinOp::Le => "<=",
            PBinOp::Gt => ">",
            PBinOp::Ge => ">=",
            PBinOp::In => "in",
            PBinOp::Add => "+",
            PBinOp::Sub => "-",
            PBinOp::Mul => "*",
            PBinOp::Div => "/",
        }
    }
}

/// Whitelisted helper functions: `(name, min args, max args)`. Implemented by page-kit `fx`.
pub const FUNCTIONS: &[(&str, usize, usize)] = &[
    ("isEmpty", 1, 1),
    ("isNotEmpty", 1, 1),
    ("today", 0, 0),
    ("now", 0, 0),
    ("workDate", 0, 0),
    ("len", 1, 1),
    ("lower", 1, 1),
    ("upper", 1, 1),
    ("trim", 1, 1),
    ("contains", 2, 2),
    ("startsWith", 2, 2),
    ("endsWith", 2, 2),
    ("round", 1, 2),
    ("abs", 1, 1),
    ("min", 2, 8),
    ("max", 2, 8),
    ("coalesce", 1, 8),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprError {
    pub message: String,
    /// Byte offset in the source text where the problem was found.
    pub offset: usize,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at offset {})", self.message, self.offset)
    }
}

impl std::error::Error for ExprError {}

// ----------------------------------------------------
// LEXER & PARSER
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(String),
    Str(String),
    Ident(String),
    Op(&'static str),
    Eof,
}

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, ExprError> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() {
            while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i + 1 < bytes.len() && bytes[i] == b'.' && (bytes[i + 1] as char).is_ascii_digit() {
                i += 1;
                while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            out.push((Tok::Num(src[start..i].to_string()), start));
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            loop {
                let Some(ch) = src[i..].chars().next() else {
                    return Err(ExprError { message: "unterminated string literal".into(), offset: start });
                };
                i += ch.len_utf8();
                if ch == quote {
                    break;
                }
                if ch == '\\' {
                    let Some(esc) = src[i..].chars().next() else {
                        return Err(ExprError { message: "unterminated escape".into(), offset: i });
                    };
                    i += esc.len_utf8();
                    s.push(match esc {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    });
                } else {
                    s.push(ch);
                }
            }
            out.push((Tok::Str(s), start));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            while i < bytes.len() && ((bytes[i] as char).is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &src[start..i];
            match word {
                "and" => out.push((Tok::Op("&&"), start)),
                "or" => out.push((Tok::Op("||"), start)),
                "not" => out.push((Tok::Op("!"), start)),
                "in" => out.push((Tok::Op("in"), start)),
                _ => out.push((Tok::Ident(word.to_string()), start)),
            }
            continue;
        }
        let two = src.get(i..i + 2).unwrap_or("");
        let op: Option<&'static str> = match two {
            "||" => Some("||"),
            "&&" => Some("&&"),
            "==" => Some("=="),
            "!=" => Some("!="),
            "<>" => Some("!="),
            "<=" => Some("<="),
            ">=" => Some(">="),
            _ => None,
        };
        if let Some(op) = op {
            out.push((Tok::Op(op), start));
            i += 2;
            continue;
        }
        let one: &'static str = match c {
            '=' => "==",
            '<' => "<",
            '>' => ">",
            '!' => "!",
            '+' => "+",
            '-' => "-",
            '*' => "*",
            '/' => "/",
            '(' => "(",
            ')' => ")",
            '[' => "[",
            ']' => "]",
            ',' => ",",
            '.' => ".",
            _ => {
                return Err(ExprError { message: format!("unexpected character '{c}'"), offset: start });
            }
        };
        out.push((Tok::Op(one), start));
        i += 1;
    }
    out.push((Tok::Eof, src.len()));
    Ok(out)
}

struct ExprParser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

impl ExprParser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }

    fn offset(&self) -> usize {
        self.toks[self.pos].1
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Tok::Op(o) if *o == op) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: &str) -> Result<(), ExprError> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(ExprError { message: format!("expected '{op}'"), offset: self.offset() })
        }
    }

    fn binary_op(&self) -> Option<PBinOp> {
        match self.peek() {
            Tok::Op("||") => Some(PBinOp::Or),
            Tok::Op("&&") => Some(PBinOp::And),
            Tok::Op("==") => Some(PBinOp::Eq),
            Tok::Op("!=") => Some(PBinOp::Ne),
            Tok::Op("<") => Some(PBinOp::Lt),
            Tok::Op("<=") => Some(PBinOp::Le),
            Tok::Op(">") => Some(PBinOp::Gt),
            Tok::Op(">=") => Some(PBinOp::Ge),
            Tok::Op("in") => Some(PBinOp::In),
            Tok::Op("+") => Some(PBinOp::Add),
            Tok::Op("-") => Some(PBinOp::Sub),
            Tok::Op("*") => Some(PBinOp::Mul),
            Tok::Op("/") => Some(PBinOp::Div),
            _ => None,
        }
    }

    /// Precedence climbing. Comparison operators are non-associative (`a == b == c` is rejected).
    fn parse_binary(&mut self, min_prec: u8) -> Result<PExpr, ExprError> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.binary_op() {
            let prec = op.precedence();
            if prec < min_prec {
                break;
            }
            self.bump();
            let right = self.parse_binary(prec + 1)?;
            if prec == 3 && self.binary_op().is_some_and(|next| next.precedence() == 3) {
                return Err(ExprError {
                    message: "comparisons cannot be chained; combine them with && or ||".into(),
                    offset: self.offset(),
                });
            }
            left = PExpr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<PExpr, ExprError> {
        if self.eat("!") {
            return Ok(PExpr::Unary { op: PUnOp::Not, expr: Box::new(self.parse_unary()?) });
        }
        if self.eat("-") {
            let inner = self.parse_unary()?;
            if let PExpr::Num(n) = &inner {
                return Ok(PExpr::Num(format!("-{n}")));
            }
            return Ok(PExpr::Unary { op: PUnOp::Neg, expr: Box::new(inner) });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<PExpr, ExprError> {
        let offset = self.offset();
        match self.bump() {
            Tok::Num(n) => Ok(PExpr::Num(n)),
            Tok::Str(s) => Ok(PExpr::Str(s)),
            Tok::Op("(") => {
                let e = self.parse_binary(1)?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Op("[") => {
                let mut items = Vec::new();
                if !self.eat("]") {
                    loop {
                        items.push(self.parse_binary(1)?);
                        if self.eat("]") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(PExpr::List(items))
            }
            Tok::Ident(word) => match word.as_str() {
                "true" => Ok(PExpr::Bool(true)),
                "false" => Ok(PExpr::Bool(false)),
                "null" => Ok(PExpr::Null),
                _ => {
                    if self.eat("(") {
                        let mut args = Vec::new();
                        if !self.eat(")") {
                            loop {
                                args.push(self.parse_binary(1)?);
                                if self.eat(")") {
                                    break;
                                }
                                self.expect(",")?;
                            }
                        }
                        return Ok(PExpr::Call { name: word, args });
                    }
                    let mut path = vec![word];
                    while self.eat(".") {
                        match self.bump() {
                            Tok::Ident(seg) => path.push(seg),
                            _ => {
                                return Err(ExprError { message: "expected a name after '.'".into(), offset: self.offset() });
                            }
                        }
                    }
                    Ok(PExpr::Ref(path))
                }
            },
            Tok::Eof => Err(ExprError { message: "unexpected end of expression".into(), offset }),
            Tok::Op(op) => Err(ExprError { message: format!("unexpected '{op}'"), offset }),
        }
    }
}

/// Parses expression source text.
pub fn parse(src: &str) -> Result<PExpr, ExprError> {
    if src.trim().is_empty() {
        return Err(ExprError { message: "expression is empty".into(), offset: 0 });
    }
    let mut p = ExprParser { toks: lex(src)?, pos: 0 };
    let e = p.parse_binary(1)?;
    if !matches!(p.peek(), Tok::Eof) {
        return Err(ExprError { message: "unexpected trailing input".into(), offset: p.offset() });
    }
    validate_calls(&e)?;
    Ok(e)
}

fn validate_calls(e: &PExpr) -> Result<(), ExprError> {
    match e {
        PExpr::Call { name, args } => {
            let Some((_, min, max)) = FUNCTIONS.iter().find(|(n, _, _)| n == name) else {
                let names: Vec<&str> = FUNCTIONS.iter().map(|(n, _, _)| *n).collect();
                return Err(ExprError {
                    message: format!("function '{name}' is not allowed; available: {}", names.join(", ")),
                    offset: 0,
                });
            };
            if args.len() < *min || args.len() > *max {
                return Err(ExprError {
                    message: format!("function '{name}' takes {min}..={max} arguments, got {}", args.len()),
                    offset: 0,
                });
            }
            args.iter().try_for_each(validate_calls)
        }
        PExpr::List(items) => items.iter().try_for_each(validate_calls),
        PExpr::Unary { expr, .. } => validate_calls(expr),
        PExpr::Binary { left, right, .. } => {
            validate_calls(left)?;
            validate_calls(right)
        }
        _ => Ok(()),
    }
}

/// Converts a BOS-Script expression (as parsed from a property value) into a page expression.
pub fn from_bos(expr: &Expr) -> Result<PExpr, String> {
    let e = match expr {
        Expr::NullLit => PExpr::Null,
        Expr::BoolLit(b) => PExpr::Bool(*b),
        Expr::IntLit(n) => PExpr::Num(n.to_string()),
        Expr::FloatLit(f) => {
            if !f.is_finite() {
                return Err("numeric literal is not finite".into());
            }
            PExpr::Num(f.to_string())
        }
        Expr::StringLit(s) => PExpr::Str(s.clone()),
        Expr::Ident(s) => match s.as_str() {
            "true" => PExpr::Bool(true),
            "false" => PExpr::Bool(false),
            "null" => PExpr::Null,
            _ => PExpr::Ref(vec![s.clone()]),
        },
        Expr::ArrayLit(items) => PExpr::List(items.iter().map(from_bos).collect::<Result<_, _>>()?),
        Expr::MemberAccess { .. } => PExpr::Ref(member_path(expr)?),
        Expr::Call { callee, args } => {
            let Expr::Ident(name) = callee.as_ref() else {
                return Err("only plain function calls such as isEmpty(rec.name) are allowed".into());
            };
            PExpr::Call { name: name.clone(), args: args.iter().map(from_bos).collect::<Result<_, _>>()? }
        }
        Expr::Unary { op, operand } => PExpr::Unary {
            op: match op {
                UnaryOp::Not => PUnOp::Not,
                UnaryOp::Neg => PUnOp::Neg,
            },
            expr: Box::new(from_bos(operand)?),
        },
        Expr::Binary { left, op, right } => PExpr::Binary {
            op: match op {
                BinaryOp::Or => PBinOp::Or,
                BinaryOp::And => PBinOp::And,
                BinaryOp::Eq => PBinOp::Eq,
                BinaryOp::NotEq => PBinOp::Ne,
                BinaryOp::Lt => PBinOp::Lt,
                BinaryOp::Lte => PBinOp::Le,
                BinaryOp::Gt => PBinOp::Gt,
                BinaryOp::Gte => PBinOp::Ge,
                BinaryOp::In => PBinOp::In,
                BinaryOp::Add => PBinOp::Add,
                BinaryOp::Sub => PBinOp::Sub,
                BinaryOp::Mul => PBinOp::Mul,
                BinaryOp::Div => PBinOp::Div,
            },
            left: Box::new(from_bos(left)?),
            right: Box::new(from_bos(right)?),
        },
        Expr::Range { .. } => return Err("ranges are not supported in page expressions".into()),
    };
    validate_calls(&e).map_err(|err| err.message)?;
    Ok(e)
}

fn member_path(expr: &Expr) -> Result<Vec<String>, String> {
    match expr {
        Expr::Ident(s) => Ok(vec![s.clone()]),
        Expr::MemberAccess { object, member } => {
            let mut p = member_path(object)?;
            p.push(member.clone());
            Ok(p)
        }
        _ => Err("only field references such as rec.status are allowed before '.'".into()),
    }
}

// ----------------------------------------------------
// CANONICAL SOURCE
// ----------------------------------------------------

fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

impl PExpr {
    /// Canonical source text; `parse(e.to_source()) == e`.
    pub fn to_source(&self) -> String {
        self.source_with(0)
    }

    fn source_with(&self, parent_prec: u8) -> String {
        match self {
            PExpr::Null => "null".into(),
            PExpr::Bool(b) => b.to_string(),
            PExpr::Num(n) => n.clone(),
            PExpr::Str(s) => quote(s),
            PExpr::List(items) => format!("[{}]", items.iter().map(|i| i.source_with(0)).collect::<Vec<_>>().join(", ")),
            PExpr::Ref(path) => path.join("."),
            PExpr::Call { name, args } => {
                format!("{name}({})", args.iter().map(|a| a.source_with(0)).collect::<Vec<_>>().join(", "))
            }
            PExpr::Unary { op, expr } => {
                let sym = match op {
                    PUnOp::Not => "!",
                    PUnOp::Neg => "-",
                };
                format!("{sym}{}", expr.source_with(6))
            }
            PExpr::Binary { op, left, right } => {
                let prec = op.precedence();
                let text = format!("{} {} {}", left.source_with(prec), op.symbol(), right.source_with(prec + 1));
                if prec < parent_prec { format!("({text})") } else { text }
            }
        }
    }

    /// Visits every reference path in the expression.
    pub fn refs(&self) -> Vec<&[String]> {
        let mut out = Vec::new();
        self.collect_refs(&mut out);
        out
    }

    fn collect_refs<'a>(&'a self, out: &mut Vec<&'a [String]>) {
        match self {
            PExpr::Ref(p) => out.push(p),
            PExpr::List(items) => items.iter().for_each(|i| i.collect_refs(out)),
            PExpr::Call { args, .. } => args.iter().for_each(|a| a.collect_refs(out)),
            PExpr::Unary { expr, .. } => expr.collect_refs(out),
            PExpr::Binary { left, right, .. } => {
                left.collect_refs(out);
                right.collect_refs(out);
            }
            _ => {}
        }
    }
}

// ----------------------------------------------------
// JAVASCRIPT EMISSION
// ----------------------------------------------------

/// What a reference path resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefTarget {
    /// A record field; the key is its JSON / SQL name.
    Field(String),
    /// A page variable.
    Variable(String),
}

/// JSON string literal that is also safe inside an inline `<script>` block.
pub fn js_string(s: &str) -> String {
    serde_json::to_string(s)
        .unwrap_or_else(|_| "\"\"".to_string())
        .replace("</", "<\\/")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn emit(e: &PExpr, resolve: &mut dyn FnMut(&[String]) -> Result<RefTarget, String>, deps: &mut ClientExprIR) -> Result<String, String> {
    Ok(match e {
        PExpr::Null => "null".into(),
        PExpr::Bool(b) => b.to_string(),
        PExpr::Num(n) => {
            if !n.trim_start_matches('-').chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Err(format!("invalid number '{n}'"));
            }
            format!("fx.num({})", js_string(n))
        }
        PExpr::Str(s) => js_string(s),
        PExpr::List(items) => {
            let parts = items.iter().map(|i| emit(i, resolve, deps)).collect::<Result<Vec<_>, _>>()?;
            format!("[{}]", parts.join(", "))
        }
        PExpr::Ref(path) => match resolve(path)? {
            RefTarget::Field(key) => {
                if !deps.fields.contains(&key) {
                    deps.fields.push(key.clone());
                }
                format!("rec[{}]", js_string(&key))
            }
            RefTarget::Variable(name) => {
                if !deps.variables.contains(&name) {
                    deps.variables.push(name.clone());
                }
                format!("page[{}]", js_string(&name))
            }
        },
        PExpr::Call { name, args } => {
            let parts = args.iter().map(|a| emit(a, resolve, deps)).collect::<Result<Vec<_>, _>>()?;
            format!("fx.{name}({})", parts.join(", "))
        }
        PExpr::Unary { op, expr } => {
            let inner = emit(expr, resolve, deps)?;
            match op {
                PUnOp::Not => format!("!fx.truthy({inner})"),
                PUnOp::Neg => format!("fx.neg({inner})"),
            }
        }
        PExpr::Binary { op, left, right } => {
            let l = emit(left, resolve, deps)?;
            let r = emit(right, resolve, deps)?;
            match op {
                PBinOp::Or => format!("(fx.truthy({l}) || fx.truthy({r}))"),
                PBinOp::And => format!("(fx.truthy({l}) && fx.truthy({r}))"),
                PBinOp::Eq => format!("fx.eq({l}, {r})"),
                PBinOp::Ne => format!("!fx.eq({l}, {r})"),
                PBinOp::Lt => format!("(fx.cmp({l}, {r}) < 0)"),
                PBinOp::Le => format!("(fx.cmp({l}, {r}) <= 0)"),
                PBinOp::Gt => format!("(fx.cmp({l}, {r}) > 0)"),
                PBinOp::Ge => format!("(fx.cmp({l}, {r}) >= 0)"),
                PBinOp::In => format!("fx.in({l}, {r})"),
                PBinOp::Add => format!("fx.add({l}, {r})"),
                PBinOp::Sub => format!("fx.sub({l}, {r})"),
                PBinOp::Mul => format!("fx.mul({l}, {r})"),
                PBinOp::Div => format!("fx.div({l}, {r})"),
            }
        }
    })
}

/// Compiles an expression to JS. `resolve` maps each reference path to a record field or page variable
/// and returns an error message for unknown names.
pub fn compile(e: &PExpr, resolve: &mut dyn FnMut(&[String]) -> Result<RefTarget, String>) -> Result<ClientExprIR, String> {
    let mut out = ClientExprIR { source: e.to_source(), js: String::new(), fields: Vec::new(), variables: Vec::new() };
    out.js = emit(e, resolve, &mut out)?;
    Ok(out)
}

/// Parses and compiles source text in one step.
pub fn compile_source(src: &str, resolve: &mut dyn FnMut(&[String]) -> Result<RefTarget, String>) -> Result<ClientExprIR, String> {
    let e = parse(src).map_err(|err| err.to_string())?;
    compile(&e, resolve)
}

/// Canonical text for a BOS property value that holds a page expression. String literals are parsed as
/// expression text; anything else is converted from the BOS AST. Returns the raw text on failure so the
/// resolver can report BOS3005 with the original input.
pub fn canonical_from_property(value: &Expr) -> Result<String, String> {
    match value {
        Expr::StringLit(s) => parse(s).map(|e| e.to_source()).map_err(|err| err.to_string()),
        other => from_bos(other).map(|e| e.to_source()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver(path: &[String]) -> Result<RefTarget, String> {
        let names: Vec<&str> = path.iter().map(String::as_str).collect();
        match names.as_slice() {
            ["rec", f] | [f] if ["status", "amount", "blocked", "creditLimit"].contains(f) => {
                Ok(RefTarget::Field(f.chars().fold(String::new(), |mut s, c| {
                    if c.is_ascii_uppercase() {
                        s.push('_');
                        s.push(c.to_ascii_lowercase());
                    } else {
                        s.push(c);
                    }
                    s
                })))
            }
            ["page", v] => Ok(RefTarget::Variable((*v).to_string())),
            _ => Err(format!("unknown field '{}'", path.join("."))),
        }
    }

    #[test]
    fn parses_precedence_and_round_trips() {
        let src = "rec.status == 'Open' && (rec.amount > 1000 || !blocked) and creditLimit in [0, 1.5]";
        let e = parse(src).unwrap();
        let canonical = e.to_source();
        assert_eq!(
            canonical,
            "rec.status == \"Open\" && (rec.amount > 1000 || !blocked) && creditLimit in [0, 1.5]"
        );
        assert_eq!(parse(&canonical).unwrap(), e);
    }

    #[test]
    fn emits_safe_js_with_dependencies() {
        let out = compile_source("rec.status <> \"Released\" && isEmpty(page.note) == false", &mut resolver).unwrap();
        assert_eq!(
            out.js,
            "(fx.truthy(!fx.eq(rec[\"status\"], \"Released\")) && fx.truthy(fx.eq(fx.isEmpty(page[\"note\"]), false)))"
        );
        assert_eq!(out.fields, vec!["status"]);
        assert_eq!(out.variables, vec!["note"]);
    }

    #[test]
    fn strings_cannot_break_out_of_script() {
        let out = compile_source("rec.status == \"</script><script>alert(1)</script>\"", &mut resolver).unwrap();
        assert!(!out.js.contains("</script>"));
        assert!(out.js.contains("<\\/script>"));
    }

    #[test]
    fn rejects_unknown_functions_names_and_chained_comparisons() {
        assert!(parse("eval('x')").unwrap_err().message.contains("not allowed"));
        assert!(parse("round()").is_err());
        assert!(parse("a == b == c").unwrap_err().message.contains("chained"));
        assert!(parse("rec.status ==").is_err());
        assert!(parse("rec.status; alert(1)").is_err());
        assert!(parse("").is_err());
        assert!(compile_source("rec.unknownField", &mut resolver).unwrap_err().contains("unknown field"));
    }

    #[test]
    fn converts_bos_ast() {
        let e = Expr::Binary {
            left: Box::new(Expr::MemberAccess { object: Box::new(Expr::Ident("rec".into())), member: "amount".into() }),
            op: BinaryOp::Gte,
            right: Box::new(Expr::IntLit(5)),
        };
        assert_eq!(canonical_from_property(&e).unwrap(), "rec.amount >= 5");
        assert_eq!(canonical_from_property(&Expr::StringLit("not blocked".into())).unwrap(), "!blocked");
        assert_eq!(canonical_from_property(&Expr::BoolLit(false)).unwrap(), "false");
    }

    #[test]
    fn numbers_stay_exact() {
        let out = compile_source("rec.amount > 0.1 - -2", &mut resolver).unwrap();
        assert_eq!(out.js, "(fx.cmp(rec[\"amount\"], fx.sub(fx.num(\"0.1\"), fx.num(\"-2\"))) > 0)");
    }
}
