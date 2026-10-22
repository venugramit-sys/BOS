//! Page designer support: a structural outline of `page` / `pageextension` source and span-based edits.
//!
//! The WYSIWYG designer never regenerates a developer's page from the IR. It edits the original text:
//! every operation replaces, inserts or removes the exact byte range of one control or property, with
//! indentation taken from the surrounding lines, so comments, blank lines and formatting elsewhere in the
//! file survive untouched. The outline is built from the real lexer's tokens (comments are skipped by the
//! lexer, byte offsets stay exact) and mirrors the parser's rules for control headers and bodies. Node paths
//! use the same `kind:name/kind:name` form as the compiled page (`ResolvedControlIR::path`), so the designer
//! maps a clicked `data-bos-node` straight to its source range.
//!
//! After all operations are applied the result is parsed again with the full parser; an edit that would
//! produce invalid source is rejected instead of written.

use bosc_ir::PageControlKind;
use bosc_syntax::{Lexer, Parser, Token, TokenKind};
use serde::{Deserialize, Serialize};

// ----------------------------------------------------
// OUTLINE
// ----------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineProp {
    pub name: String,
    /// Source text of the value (`"Customer"`, `primary`, `rec.blocked == false`). Flags (`readonly;`) are `true`.
    pub value: String,
    pub start: usize,
    /// Exclusive end, including the terminating `;` when present.
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineBlock {
    /// `style`, `responsive`, `link` / `subPageLink`.
    pub name: String,
    pub start: usize,
    pub end: usize,
    /// Byte offsets of `{` and `}`.
    pub open: usize,
    pub close: usize,
    pub props: Vec<OutlineProp>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineNode {
    /// Canonical control kind (`PageControlKind::as_str`) or action kind (`action`, `area`, `group`, ...).
    pub kind: String,
    /// Keyword as written (`field`, `section`, `addAfter`, compact field name, ...).
    pub raw_kind: String,
    pub name: String,
    /// Second positional argument (`part(Lines; SalesLineSubform)`, `actionref(A; B)`).
    pub source: Option<String>,
    pub path: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    /// End of the header (`field(no)`, `group(General)`), before the body or the `;`.
    pub header_end: usize,
    /// `{` and `}` of the body when there is one.
    pub body: Option<(usize, usize)>,
    pub props: Vec<OutlineProp>,
    pub blocks: Vec<OutlineBlock>,
    /// Trigger names in the body (`onValidate`, `onAction`, ...).
    pub triggers: Vec<String>,
    pub children: Vec<OutlineNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineSection {
    pub open: usize,
    pub close: usize,
    pub nodes: Vec<OutlineNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OutlineObject {
    /// `page` or `pageextension`.
    pub kind: String,
    pub name: String,
    /// Source table of a page, target page of an extension.
    pub target: Option<String>,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub open: usize,
    pub close: usize,
    pub props: Vec<OutlineProp>,
    pub blocks: Vec<OutlineBlock>,
    pub layout: Option<OutlineSection>,
    pub actions: Option<OutlineSection>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SourceOutline {
    pub objects: Vec<OutlineObject>,
}

impl SourceOutline {
    pub fn object(&self, name: &str) -> Option<&OutlineObject> {
        self.objects.iter().find(|o| o.name.eq_ignore_ascii_case(name))
    }
}

impl OutlineObject {
    /// Finds a node by path in the layout (`area:Content/...`) or actions (`actions/...`) tree.
    pub fn find(&self, path: &str) -> Option<&OutlineNode> {
        fn walk<'a>(nodes: &'a [OutlineNode], path: &str) -> Option<&'a OutlineNode> {
            for n in nodes {
                if n.path.eq_ignore_ascii_case(path) {
                    return Some(n);
                }
                if let Some(found) = walk(&n.children, path) {
                    return Some(found);
                }
            }
            None
        }
        self.layout
            .as_ref()
            .and_then(|s| walk(&s.nodes, path))
            .or_else(|| self.actions.as_ref().and_then(|s| walk(&s.nodes, path)))
    }

    /// Finds a node by its control name (first match, layout before actions).
    pub fn find_by_name(&self, name: &str) -> Option<&OutlineNode> {
        fn walk<'a>(nodes: &'a [OutlineNode], name: &str) -> Option<&'a OutlineNode> {
            for n in nodes {
                if n.name.eq_ignore_ascii_case(name) {
                    return Some(n);
                }
                if let Some(found) = walk(&n.children, name) {
                    return Some(found);
                }
            }
            None
        }
        self.layout
            .as_ref()
            .and_then(|s| walk(&s.nodes, name))
            .or_else(|| self.actions.as_ref().and_then(|s| walk(&s.nodes, name)))
    }
}

/// Keywords the parser treats as containers (a bare label may follow them).
const CONTAINER_KEYWORDS: &[&str] = &[
    "area", "group", "section", "repeater", "lines", "part", "factbox", "tabs", "tab", "columns", "column", "card", "step",
    "steps", "tile", "tiles", "region", "regions", "main", "side", "header", "footer", "content", "split", "grid", "flex",
    "bento", "kanban", "accordion", "panel", "drawer", "modal", "stepper", "fixed", "cuegroup", "cue", "headline",
    "wizardstep", "container", "systempart", "usercontrol", "label", "image", "spacer", "divider", "wrap", "replace",
    "addAfter", "addBefore", "addFirst", "addLast", "modify", "moveAfter", "moveBefore", "moveFirst", "moveLast", "hide",
];
/// Bodies in which `name;` is a flag property rather than a compact field.
const FIELD_LIKE: &[&str] = &["field", "modify", "tile", "cue", "part", "factbox"];
const FLAGS: &[&str] = &["readonly", "required", "hidden", "promoted", "multiline", "drillDown", "editable", "visible"];
const ACTION_KEYWORDS: &[&str] = &[
    "action", "command", "area", "group", "actionref", "separator", "addFirst", "addLast", "addBefore", "addAfter", "modify",
    "hide", "moveBefore", "moveAfter",
];

struct Scanner<'a> {
    src: &'a str,
    toks: Vec<Token>,
}

impl<'a> Scanner<'a> {
    fn new(src: &'a str) -> Result<Self, String> {
        let toks = Lexer::new(src).tokenize().map_err(|e| e.to_string())?;
        Ok(Scanner { src, toks })
    }

    fn kind(&self, i: usize) -> &TokenKind {
        self.toks.get(i).map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn is(&self, i: usize, k: &TokenKind) -> bool {
        std::mem::discriminant(self.kind(i)) == std::mem::discriminant(k)
    }

    fn start(&self, i: usize) -> usize {
        self.toks.get(i).map(|t| t.span.start).unwrap_or(self.src.len())
    }

    fn end(&self, i: usize) -> usize {
        self.toks.get(i).map(|t| t.span.end).unwrap_or(self.src.len())
    }

    fn line(&self, i: usize) -> usize {
        self.toks.get(i).map(|t| t.span.line).unwrap_or(0)
    }

    /// Identifier text of a token; keywords count too (`field(state)`, `key`).
    fn ident(&self, i: usize) -> Option<String> {
        match self.kind(i) {
            TokenKind::Ident(s) => Some(s.clone()),
            TokenKind::StringLit(_) | TokenKind::IntLit(_) | TokenKind::FloatLit(_) | TokenKind::Eof => None,
            _ => {
                let text = self.src.get(self.start(i)..self.end(i))?;
                (!text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')).then(|| text.to_string())
            }
        }
    }

    fn text(&self, i: usize) -> String {
        match self.kind(i) {
            TokenKind::StringLit(s) => s.clone(),
            _ => self.ident(i).unwrap_or_default(),
        }
    }

    /// Index of the token closing the bracket opened at `open`.
    fn matching(&self, open: usize) -> Result<usize, String> {
        let (o, c) = match self.kind(open) {
            TokenKind::OpenBrace => (TokenKind::OpenBrace, TokenKind::CloseBrace),
            TokenKind::OpenParen => (TokenKind::OpenParen, TokenKind::CloseParen),
            TokenKind::OpenBracket => (TokenKind::OpenBracket, TokenKind::CloseBracket),
            _ => return Err(format!("expected a bracket at offset {}", self.start(open))),
        };
        let mut depth = 0usize;
        let mut i = open;
        while i < self.toks.len() {
            if self.is(i, &o) {
                depth += 1;
            } else if self.is(i, &c) {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            } else if self.is(i, &TokenKind::Eof) {
                break;
            }
            i += 1;
        }
        Err(format!("unclosed bracket at line {}", self.line(open)))
    }

    /// Skips a value up to the `;` (inclusive) at bracket depth 0, or up to (not including) `stop`.
    fn skip_value(&self, mut i: usize, stop: usize) -> Result<usize, String> {
        while i < stop {
            match self.kind(i) {
                TokenKind::OpenBrace | TokenKind::OpenParen | TokenKind::OpenBracket => i = self.matching(i)? + 1,
                TokenKind::Semicolon => return Ok(i + 1),
                _ => i += 1,
            }
        }
        Ok(stop)
    }

    fn prop(&self, i: usize, stop: usize) -> Result<(OutlineProp, usize), String> {
        let name = self.ident(i).unwrap_or_default();
        if self.is(i + 1, &TokenKind::Assign) || self.is(i + 1, &TokenKind::Colon) {
            let next = self.skip_value(i + 2, stop)?;
            let value_end = if next > 0 && self.is(next - 1, &TokenKind::Semicolon) { self.start(next - 1) } else { self.end(next - 1) };
            let value = self.src[self.start(i + 2)..value_end.max(self.start(i + 2))].trim().to_string();
            Ok((OutlineProp { name, value, start: self.start(i), end: self.end(next - 1) }, next))
        } else {
            // flag: `readonly;`
            let next = if self.is(i + 1, &TokenKind::Semicolon) { i + 2 } else { i + 1 };
            Ok((OutlineProp { name, value: "true".into(), start: self.start(i), end: self.end(next - 1) }, next))
        }
    }

    fn block(&self, i: usize) -> Result<(OutlineBlock, usize), String> {
        let name = self.ident(i).unwrap_or_default();
        let open = i + 1;
        let close = self.matching(open)?;
        let mut props = Vec::new();
        let mut k = open + 1;
        while k < close {
            if self.ident(k).is_some() {
                let (p, next) = self.prop(k, close)?;
                props.push(p);
                k = next;
            } else {
                k += 1;
            }
        }
        let mut next = close + 1;
        if self.is(next, &TokenKind::Semicolon) {
            next += 1;
        }
        Ok((OutlineBlock { name, start: self.start(i), end: self.end(next - 1), open: self.start(open), close: self.start(close), props }, next))
    }

    /// Skips a trigger (`onValidate(rec, ctx) { ... }`).
    fn skip_trigger(&self, mut i: usize) -> Result<usize, String> {
        i += 1;
        if self.is(i, &TokenKind::OpenParen) {
            i = self.matching(i)? + 1;
        }
        if self.is(i, &TokenKind::OpenBrace) {
            i = self.matching(i)? + 1;
        }
        if self.is(i, &TokenKind::Semicolon) {
            i += 1;
        }
        Ok(i)
    }

    fn control(&self, i: usize, parent: &str, actions: bool) -> Result<(OutlineNode, usize), String> {
        let raw = self.ident(i).ok_or_else(|| format!("expected a control at line {}", self.line(i)))?;
        let mut kind_word = raw.clone();
        let mut name = raw.clone();
        let mut source = None;
        let mut j = i + 1;
        let is_container = CONTAINER_KEYWORDS.contains(&raw.as_str()) || (actions && ACTION_KEYWORDS.contains(&raw.as_str()));
        let mut header_end = self.end(i);
        if self.is(j, &TokenKind::OpenParen) {
            let close = self.matching(j)?;
            let mut positional = Vec::new();
            let mut k = j + 1;
            while k < close {
                if self.ident(k).is_some() && (self.is(k + 1, &TokenKind::Assign) || self.is(k + 1, &TokenKind::Colon)) {
                    k = self.skip_value(k + 2, close)?;
                    continue;
                }
                match self.kind(k) {
                    TokenKind::Semicolon | TokenKind::Comma => k += 1,
                    _ => {
                        positional.push(self.text(k));
                        k += 1;
                    }
                }
            }
            if let Some(first) = positional.first() {
                name = first.clone();
            }
            if positional.len() > 1 {
                source = Some(positional[1].clone());
            } else if raw == "field" {
                source = positional.first().cloned();
            }
            header_end = self.end(close);
            j = close + 1;
        } else if is_container {
            if matches!(self.kind(j), TokenKind::StringLit(_)) || (self.ident(j).is_some() && !self.is(j, &TokenKind::OpenBrace)) {
                name = self.text(j);
                header_end = self.end(j);
                j += 1;
            }
        } else if raw != "field" {
            // compact field `email;` / `email { readonly; }`
            kind_word = "field".to_string();
            source = Some(raw.clone());
        }

        let kind = if actions {
            match kind_word.as_str() {
                "command" => "action".to_string(),
                k if ACTION_KEYWORDS.contains(&k) => k.to_string(),
                _ => "action".to_string(),
            }
        } else {
            PageControlKind::parse(&kind_word).as_str().to_string()
        };
        let segment = format!("{kind}:{name}");
        let path = if parent.is_empty() { segment } else { format!("{parent}/{segment}") };

        let mut node = OutlineNode {
            kind,
            raw_kind: raw.clone(),
            name,
            source,
            path: path.clone(),
            line: self.line(i),
            start: self.start(i),
            end: header_end,
            header_end,
            body: None,
            props: Vec::new(),
            blocks: Vec::new(),
            triggers: Vec::new(),
            children: Vec::new(),
        };

        if self.is(j, &TokenKind::OpenBrace) {
            let close = self.matching(j)?;
            node.body = Some((self.start(j), self.start(close)));
            let field_like = FIELD_LIKE.contains(&kind_word.as_str());
            let mut k = j + 1;
            while k < close {
                let Some(s) = self.ident(k) else {
                    k += 1;
                    continue;
                };
                let second_brace = self.is(k + 1, &TokenKind::OpenBrace);
                if matches!(s.as_str(), "style" | "responsive" | "link" | "subPageLink") && second_brace {
                    let (b, next) = self.block(k)?;
                    node.blocks.push(b);
                    k = next;
                } else if s.starts_with("on") && s.len() > 2 && (self.is(k + 1, &TokenKind::OpenParen) || second_brace) {
                    node.triggers.push(s);
                    k = self.skip_trigger(k)?;
                } else if self.is(k + 1, &TokenKind::Assign)
                    || self.is(k + 1, &TokenKind::Colon)
                    || (self.is(k + 1, &TokenKind::Semicolon) && !actions && (field_like || FLAGS.contains(&s.as_str())))
                    // action bodies list moved names as flags: `moveAfter(A) { B; C; }`
                    || (self.is(k + 1, &TokenKind::Semicolon) && actions && !ACTION_KEYWORDS.contains(&s.as_str()))
                {
                    let (p, next) = self.prop(k, close)?;
                    node.props.push(p);
                    k = next;
                } else {
                    let (child, next) = self.control(k, &path, actions)?;
                    node.children.push(child);
                    k = next;
                }
            }
            j = close + 1;
            node.end = self.end(close);
        }
        if self.is(j, &TokenKind::Semicolon) {
            node.end = self.end(j);
            j += 1;
        }
        Ok((node, j))
    }

    fn section(&self, i: usize, prefix: &str, actions: bool) -> Result<(OutlineSection, usize), String> {
        let open = i + 1;
        let close = self.matching(open)?;
        let mut nodes = Vec::new();
        let mut k = open + 1;
        while k < close {
            if self.ident(k).is_some() {
                let (n, next) = self.control(k, prefix, actions)?;
                nodes.push(n);
                k = next;
            } else {
                k += 1;
            }
        }
        Ok((OutlineSection { open: self.start(open), close: self.start(close), nodes }, close + 1))
    }

    fn object(&self, i: usize) -> Result<(OutlineObject, usize), String> {
        let kind = if self.is(i, &TokenKind::Page) { "page" } else { "pageextension" };
        let name = self.ident(i + 1).ok_or_else(|| format!("expected a page name at line {}", self.line(i)))?;
        let mut j = i + 2;
        let mut target = None;
        if self.is(j, &TokenKind::For) || self.is(j, &TokenKind::Extends) {
            target = self.ident(j + 1);
            j += 2;
        }
        while !self.is(j, &TokenKind::OpenBrace) && !self.is(j, &TokenKind::Eof) {
            j += 1;
        }
        let close = self.matching(j)?;
        let mut obj = OutlineObject {
            kind: kind.to_string(),
            name,
            target,
            line: self.line(i),
            start: self.start(i),
            end: self.end(close),
            open: self.start(j),
            close: self.start(close),
            props: Vec::new(),
            blocks: Vec::new(),
            layout: None,
            actions: None,
        };
        let mut k = j + 1;
        while k < close {
            let Some(s) = self.ident(k) else {
                k += 1;
                continue;
            };
            let brace = self.is(k + 1, &TokenKind::OpenBrace);
            if s == "layout" && brace {
                let (sec, next) = self.section(k, "", false)?;
                obj.layout = Some(sec);
                k = next;
            } else if s == "actions" && brace {
                let (sec, next) = self.section(k, "actions", true)?;
                obj.actions = Some(sec);
                k = next;
            } else if s == "style" && brace {
                let (b, next) = self.block(k)?;
                obj.blocks.push(b);
                k = next;
            } else if brace {
                // variables { }, views { }, ...
                k = self.matching(k + 1)? + 1;
            } else if s.starts_with("on") && self.is(k + 1, &TokenKind::OpenParen) {
                k = self.skip_trigger(k)?;
            } else if self.is(k + 1, &TokenKind::Assign) || self.is(k + 1, &TokenKind::Colon) {
                let (p, next) = self.prop(k, close)?;
                obj.props.push(p);
                k = next;
            } else {
                k += 1;
            }
        }
        Ok((obj, close + 1))
    }
}

/// Outline of every `page` / `pageextension` in `source`.
pub fn outline(source: &str) -> Result<SourceOutline, String> {
    let sc = Scanner::new(source)?;
    let mut objects = Vec::new();
    let mut i = 0;
    while i < sc.toks.len() {
        if sc.is(i, &TokenKind::Page) || sc.is(i, &TokenKind::PageExtension) {
            let (obj, next) = sc.object(i)?;
            objects.push(obj);
            i = next;
        } else {
            i += 1;
        }
    }
    Ok(SourceOutline { objects })
}

// ----------------------------------------------------
// EDIT OPERATIONS
// ----------------------------------------------------

/// One designer operation. `path` is a node path, or `page` for the object itself.
/// `value` / `source` are BOS source text (`"Customer"`, `primary`, `field(email);`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum PageEditOp {
    #[serde(rename_all = "camelCase")]
    SetProperty { path: String, name: String, value: String, #[serde(default)] block: Option<String> },
    #[serde(rename_all = "camelCase")]
    RemoveProperty { path: String, name: String, #[serde(default)] block: Option<String> },
    /// Inserts control source into `parent` (a node path, or `layout`), before the sibling `before` or last.
    #[serde(rename_all = "camelCase")]
    Insert { parent: String, #[serde(default)] before: Option<String>, source: String },
    #[serde(rename_all = "camelCase")]
    Move { path: String, parent: String, #[serde(default)] before: Option<String> },
    #[serde(rename_all = "camelCase")]
    Delete { path: String },
}

const INDENT_UNIT: &str = "    ";

fn line_start(src: &str, pos: usize) -> usize {
    src[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

fn line_end(src: &str, pos: usize) -> usize {
    src[pos..].find('\n').map(|i| pos + i).unwrap_or(src.len())
}

fn indent_at(src: &str, pos: usize) -> String {
    let ls = line_start(src, pos);
    src[ls..].chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// Range of `start..end` widened to whole lines when nothing else shares those lines.
fn whole_lines(src: &str, start: usize, end: usize) -> (usize, usize) {
    let ls = line_start(src, start);
    let le = line_end(src, end);
    let before_blank = src[ls..start].trim().is_empty();
    let after_blank = src[end..le].trim().is_empty();
    if before_blank && after_blank {
        let le_incl = if le < src.len() { le + 1 } else { le };
        (ls, le_incl)
    } else {
        (start, end)
    }
}

fn reindent(text: &str, from: &str, to: &str) -> String {
    text.lines()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else if let Some(rest) = l.strip_prefix(from) {
                format!("{to}{rest}")
            } else {
                format!("{to}{}", l.trim_start())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Removes an inline `start..end` (`{ a = 1; b = 2; }`) together with one of its surrounding spaces.
fn remove_inline(src: &str, start: usize, end: usize) -> String {
    let (s, e) = whole_lines(src, start, end);
    if s != start {
        return splice(src, s, e, "");
    }
    let e = if start > 0 && src[..start].ends_with(' ') && src[end..].starts_with(' ') { end + 1 } else { end };
    splice(src, start, e, "")
}

fn splice(src: &str, start: usize, end: usize, with: &str) -> String {
    let mut out = String::with_capacity(src.len() + with.len());
    out.push_str(&src[..start]);
    out.push_str(with);
    out.push_str(&src[end..]);
    out
}

struct Target {
    /// Offset of `{` and `}` of the container body, when it has one.
    body: Option<(usize, usize)>,
    header_end: usize,
    /// End of the node (to turn `field(x);` into `field(x) { ... }`).
    end: usize,
    start: usize,
    props: Vec<OutlineProp>,
    blocks: Vec<OutlineBlock>,
    children: Vec<OutlineNode>,
}

fn target_of(obj: &OutlineObject, path: &str) -> Result<Target, String> {
    if path.eq_ignore_ascii_case("page") {
        return Ok(Target {
            body: Some((obj.open, obj.close)),
            header_end: obj.open,
            end: obj.end,
            start: obj.start,
            props: obj.props.clone(),
            blocks: obj.blocks.clone(),
            children: Vec::new(),
        });
    }
    if path.eq_ignore_ascii_case("layout") || path.eq_ignore_ascii_case("actions") {
        let sec = if path.eq_ignore_ascii_case("layout") { obj.layout.as_ref() } else { obj.actions.as_ref() }
            .ok_or_else(|| format!("page '{}' has no {path} section", obj.name))?;
        return Ok(Target {
            body: Some((sec.open, sec.close)),
            header_end: sec.open,
            end: sec.close + 1,
            start: sec.open,
            props: Vec::new(),
            blocks: Vec::new(),
            children: sec.nodes.clone(),
        });
    }
    let n = obj.find(path).ok_or_else(|| format!("control '{path}' does not exist in the source"))?;
    Ok(Target {
        body: n.body,
        header_end: n.header_end,
        end: n.end,
        start: n.start,
        props: n.props.clone(),
        blocks: n.blocks.clone(),
        children: n.children.clone(),
    })
}

/// Inserts `entry` (one line of body text without indentation) into a node body, creating the body if needed.
fn insert_entry(src: &str, t: &Target, entry: &str, at_end: bool) -> String {
    let indent = indent_at(src, t.start);
    match t.body {
        Some((open, close)) => {
            let single_line = !src[open..close].contains('\n');
            if single_line {
                let inner = src[open + 1..close].trim();
                let mut parts: Vec<String> = Vec::new();
                if !inner.is_empty() && at_end {
                    parts.push(inner.to_string());
                }
                parts.push(entry.to_string());
                if !inner.is_empty() && !at_end {
                    parts.push(inner.to_string());
                }
                splice(src, open, close + 1, &format!("{{ {} }}", parts.join(" ")))
            } else {
                let child_indent = format!("{indent}{INDENT_UNIT}");
                if at_end {
                    let ls = line_start(src, close);
                    let pos = if src[ls..close].trim().is_empty() { ls } else { close };
                    let text = if pos == ls { format!("{child_indent}{entry}\n") } else { format!("\n{child_indent}{entry}\n{indent}") };
                    splice(src, pos, pos, &text)
                } else {
                    splice(src, open + 1, open + 1, &format!("\n{child_indent}{entry}"))
                }
            }
        }
        None => {
            // `field(x);` → `field(x) { entry }`
            let tail_end = if src[t.header_end..t.end].trim() == ";" { t.end } else { t.header_end };
            splice(src, t.header_end, tail_end, &format!(" {{ {entry} }}"))
        }
    }
}

fn set_property(src: &str, obj: &OutlineObject, path: &str, name: &str, value: &str, block: Option<&str>) -> Result<String, String> {
    let t = target_of(obj, path)?;
    let entry = format!("{name} = {value};");
    match block {
        None => {
            if let Some(p) = t.props.iter().find(|p| p.name == name) {
                return Ok(splice(src, p.start, p.end, &entry));
            }
            Ok(insert_entry(src, &t, &entry, false))
        }
        Some(b) => {
            if let Some(blk) = t.blocks.iter().find(|x| x.name == b) {
                if let Some(p) = blk.props.iter().find(|p| p.name == name) {
                    return Ok(splice(src, p.start, p.end, &entry));
                }
                let bt = Target {
                    body: Some((blk.open, blk.close)),
                    header_end: blk.open,
                    end: blk.end,
                    start: blk.start,
                    props: Vec::new(),
                    blocks: Vec::new(),
                    children: Vec::new(),
                };
                return Ok(insert_entry(src, &bt, &entry, true));
            }
            Ok(insert_entry(src, &t, &format!("{b} {{ {entry} }}"), true))
        }
    }
}

fn remove_property(src: &str, obj: &OutlineObject, path: &str, name: &str, block: Option<&str>) -> Result<String, String> {
    let t = target_of(obj, path)?;
    match block {
        None => match t.props.iter().find(|p| p.name == name) {
            Some(p) => Ok(remove_inline(src, p.start, p.end)),
            None => Ok(src.to_string()),
        },
        Some(b) => {
            let Some(blk) = t.blocks.iter().find(|x| x.name == b) else { return Ok(src.to_string()) };
            let Some(p) = blk.props.iter().find(|p| p.name == name) else { return Ok(src.to_string()) };
            if blk.props.len() == 1 {
                return Ok(remove_inline(src, blk.start, blk.end));
            }
            Ok(remove_inline(src, p.start, p.end))
        }
    }
}

fn delete_node(src: &str, obj: &OutlineObject, path: &str) -> Result<String, String> {
    let n = obj.find(path).ok_or_else(|| format!("control '{path}' does not exist in the source"))?;
    let (s, e) = whole_lines(src, n.start, n.end);
    Ok(splice(src, s, e, ""))
}

fn insert_node(src: &str, obj: &OutlineObject, parent: &str, before: Option<&str>, text: &str) -> Result<String, String> {
    let t = target_of(obj, parent)?;
    let parent_indent = indent_at(src, t.start);
    let child_indent = match t.children.first() {
        Some(c) => indent_at(src, c.start),
        None => format!("{parent_indent}{INDENT_UNIT}"),
    };
    let first_indent = text.lines().find(|l| !l.trim().is_empty()).map(|l| l.chars().take_while(|c| c.is_whitespace()).collect::<String>()).unwrap_or_default();
    let body = reindent(text.trim_end(), &first_indent, &child_indent);

    if let Some(b) = before {
        let sib = t
            .children
            .iter()
            .find(|c| c.path.eq_ignore_ascii_case(b))
            .ok_or_else(|| format!("'{b}' is not a child of '{parent}'"))?;
        let ls = line_start(src, sib.start);
        if src[ls..sib.start].trim().is_empty() {
            return Ok(splice(src, ls, ls, &format!("{body}\n")));
        }
        return Ok(splice(src, sib.start, sib.start, &format!("{} ", body.trim())));
    }
    match t.body {
        Some((open, close)) => {
            let ls = line_start(src, close);
            if src[open..close].contains('\n') && src[ls..close].trim().is_empty() {
                Ok(splice(src, ls, ls, &format!("{body}\n")))
            } else {
                let inner = src[open + 1..close].trim();
                let mut lines = Vec::new();
                if !inner.is_empty() {
                    lines.push(format!("{child_indent}{inner}"));
                }
                lines.push(body);
                Ok(splice(src, open, close + 1, &format!("{{\n{}\n{parent_indent}}}", lines.join("\n"))))
            }
        }
        None => {
            let tail_end = if src[t.header_end..t.end].trim() == ";" { t.end } else { t.header_end };
            Ok(splice(src, t.header_end, tail_end, &format!(" {{\n{body}\n{parent_indent}}}")))
        }
    }
}

fn move_node(src: &str, obj: &OutlineObject, path: &str, parent: &str, before: Option<&str>) -> Result<(String, String), String> {
    let n = obj.find(path).ok_or_else(|| format!("control '{path}' does not exist in the source"))?;
    if parent.to_ascii_lowercase().starts_with(&format!("{}/", n.path.to_ascii_lowercase())) || parent.eq_ignore_ascii_case(&n.path) {
        return Err(format!("'{path}' cannot be moved into itself"));
    }
    if before.is_some_and(|b| b.eq_ignore_ascii_case(path)) {
        return Ok((src.to_string(), path.to_string()));
    }
    let (s, e) = whole_lines(src, n.start, n.end);
    let indent = indent_at(src, n.start);
    let text: String = if s == n.start { src[n.start..n.end].to_string() } else { format!("{indent}{}", &src[n.start..n.end]) };
    let removed = splice(src, s, e, "");
    let out2 = outline(&removed)?;
    let obj2 = out2.object(&obj.name).ok_or("page vanished while moving")?;
    let inserted = insert_node(&removed, obj2, parent, before, &text)?;
    let segment = n.path.rsplit('/').next().unwrap_or(&n.path).to_string();
    let new_path = if parent.eq_ignore_ascii_case("layout") { segment } else { format!("{parent}/{segment}") };
    Ok((inserted, new_path))
}

#[derive(Debug, Clone, Serialize)]
pub struct EditResult {
    pub source: String,
    pub outline: SourceOutline,
    /// Path of the last inserted or moved node (the designer selects it).
    pub selected: Option<String>,
}

/// Applies `ops` to page `page` in `source` in order. Fails without partial output when any op does not
/// apply or the result does not parse.
pub fn apply_edits(source: &str, page: &str, ops: &[PageEditOp]) -> Result<EditResult, String> {
    if outline(source)?.object(page).is_none() {
        return Err(format!("page '{page}' is not defined in this source"));
    }
    let mut src = source.to_string();
    let mut selected = None;
    for op in ops {
        let out = outline(&src)?;
        let obj = out.object(page).ok_or_else(|| format!("page '{page}' is not defined in this source"))?;
        src = match op {
            PageEditOp::SetProperty { path, name, value, block } => {
                check_name(name)?;
                check_value(value)?;
                set_property(&src, obj, path, name, value, block.as_deref())?
            }
            PageEditOp::RemoveProperty { path, name, block } => remove_property(&src, obj, path, name, block.as_deref())?,
            PageEditOp::Delete { path } => delete_node(&src, obj, path)?,
            PageEditOp::Insert { parent, before, source } => {
                // A new page may not have its `layout { }` / `actions { }` section yet.
                let (with_section, outline_now);
                let obj = match ensure_section(&src, obj, parent) {
                    Some(created) => {
                        with_section = created;
                        outline_now = outline(&with_section)?;
                        src = with_section.clone();
                        outline_now.object(page).ok_or("page vanished while adding a section")?
                    }
                    None => obj,
                };
                let next = insert_node(&src, obj, parent, before.as_deref(), source)?;
                selected = outline(&next).ok().and_then(|o| {
                    // The inserted root is the first new path in document (pre-)order.
                    let before_paths: std::collections::HashSet<String> = all_paths(obj).into_iter().collect();
                    o.object(page).and_then(|ob| all_paths(ob).into_iter().find(|p| !before_paths.contains(p)))
                });
                next
            }
            PageEditOp::Move { path, parent, before } => {
                let (next, new_path) = move_node(&src, obj, path, parent, before.as_deref())?;
                selected = Some(new_path);
                next
            }
        };
    }
    // The result must be valid BOS.
    let tokens = Lexer::new(&src).tokenize().map_err(|e| format!("edit produced invalid source: {e}"))?;
    Parser::with_source(tokens, src.clone()).parse_program().map_err(|e| format!("edit produced invalid source: {e}"))?;
    let outline = outline(&src)?;
    Ok(EditResult { source: src, outline, selected })
}

/// Source with an empty `layout { }` or `actions { }` section added before the object's closing brace,
/// when `parent` names that section and it does not exist yet.
fn ensure_section(src: &str, obj: &OutlineObject, parent: &str) -> Option<String> {
    let section = if parent.eq_ignore_ascii_case("layout") && obj.layout.is_none() {
        "layout"
    } else if parent.eq_ignore_ascii_case("actions") && obj.actions.is_none() {
        "actions"
    } else {
        return None;
    };
    let indent = indent_at(src, obj.start);
    let ls = line_start(src, obj.close);
    let text = format!("{indent}{INDENT_UNIT}{section} {{\n{indent}{INDENT_UNIT}}}\n");
    if src[ls..obj.close].trim().is_empty() {
        Some(splice(src, ls, ls, &text))
    } else {
        Some(splice(src, obj.close, obj.close, &format!("\n{text}{indent}")))
    }
}

/// Every node path in document (pre-)order.
fn all_paths(obj: &OutlineObject) -> Vec<String> {
    fn walk(nodes: &[OutlineNode], out: &mut Vec<String>) {
        for n in nodes {
            out.push(n.path.clone());
            walk(&n.children, out);
        }
    }
    let mut out = Vec::new();
    if let Some(l) = &obj.layout {
        walk(&l.nodes, &mut out);
    }
    if let Some(a) = &obj.actions {
        walk(&a.nodes, &mut out);
    }
    out
}

fn check_name(name: &str) -> Result<(), String> {
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
        Ok(())
    } else {
        Err(format!("'{name}' is not a valid property name"))
    }
}

/// A property value must be one expression: no `;`, `{` or `}` outside string literals.
fn check_value(value: &str) -> Result<(), String> {
    let mut in_str = false;
    let mut prev = '\0';
    for c in value.chars() {
        if c == '"' && prev != '\\' {
            in_str = !in_str;
        } else if !in_str && matches!(c, ';' | '{' | '}') {
            return Err(format!("'{value}' is not a single property value"));
        }
        prev = c;
    }
    if in_str || value.trim().is_empty() {
        return Err(format!("'{value}' is not a valid property value"));
    }
    Ok(())
}

// ----------------------------------------------------
// CUSTOMIZATION LAYER PRINTER
// ----------------------------------------------------

/// Super-user changes to one page, as edited by the designer in Customize mode.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomizationChanges {
    pub page: PageChange,
    pub controls: Vec<ControlChange>,
    pub moves: Vec<MoveChange>,
    pub adds: Vec<AddChange>,
    pub actions: Vec<ActionChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PageChange {
    pub caption: Option<String>,
    pub style: Option<bosc_ir::PageStyleIR>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ControlChange {
    pub name: String,
    pub caption: Option<String>,
    pub tooltip: Option<String>,
    pub visible: Option<bool>,
    pub importance: Option<bosc_ir::Importance>,
    pub style: Option<bosc_ir::PageStyleIR>,
    pub responsive: Option<bosc_ir::ResponsiveIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Placement {
    Before,
    After,
    First,
    Last,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveChange {
    pub name: String,
    pub anchor: String,
    pub position: Placement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddChange {
    /// Table field to add (authored field name).
    pub field: String,
    pub anchor: String,
    pub position: Placement,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ActionChange {
    pub name: String,
    pub caption: Option<String>,
    pub visible: Option<bool>,
}

fn ident_ok(s: &str) -> Result<&str, String> {
    if !s.is_empty() && s.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(s)
    } else {
        Err(format!("'{s}' is not a valid control name"))
    }
}

fn quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
    format!("\"{escaped}\"")
}

fn style_lines(style: &bosc_ir::PageStyleIR) -> Vec<String> {
    let mut out = Vec::new();
    let color = |c: &bosc_ir::ColorValue| {
        let s: String = c.clone().into();
        if s.starts_with('#') { quote(&s) } else { s }
    };
    if let Some(c) = &style.background {
        out.push(format!("background = {};", color(c)));
    }
    if let Some(c) = &style.text_color {
        out.push(format!("textColor = {};", color(c)));
    }
    if let Some(c) = &style.border_color {
        out.push(format!("borderColor = {};", color(c)));
    }
    if let Some(w) = style.border_width {
        out.push(format!("borderWidth = {w};"));
    }
    if let Some(r) = style.radius {
        out.push(format!("radius = {};", r.as_str()));
    }
    if let Some(s) = style.shadow {
        out.push(format!("shadow = {};", s.as_str()));
    }
    if let Some(d) = style.density {
        out.push(format!("density = {};", d.as_str()));
    }
    if let Some(e) = style.emphasis {
        out.push(format!("emphasis = {};", e.as_str()));
    }
    if let Some(i) = &style.icon {
        out.push(format!("icon = {};", quote(i)));
    }
    out
}

fn responsive_lines(r: &bosc_ir::ResponsiveIR) -> Vec<String> {
    let mut out = Vec::new();
    let spans = [("xs", r.span.xs), ("sm", r.span.sm), ("md", r.span.md), ("lg", r.span.lg), ("xl", r.span.xl)];
    for (bp, n) in spans {
        if let Some(n) = n {
            out.push(format!("{bp} = {n};"));
        }
    }
    if let Some(h) = r.hide_below {
        out.push(format!("hideBelow = {};", h.as_str()));
    }
    out
}

/// Prints the machine-owned customization extension of `page` (`<Page>_Customization`).
pub fn print_customization(page: &str, changes: &CustomizationChanges) -> Result<String, String> {
    ident_ok(page)?;
    let mut out = String::new();
    out.push_str("// Generated by the BOS page designer (Customize mode). Changes here are overwritten by the designer.\n");
    out.push_str(&format!("pageextension {page}_Customization extends {page} {{\n"));
    if let Some(c) = &changes.page.caption {
        out.push_str(&format!("    caption = {};\n", quote(c)));
    }
    if let Some(s) = &changes.page.style {
        let lines = style_lines(s);
        if !lines.is_empty() {
            out.push_str(&format!("    style {{ {} }}\n", lines.join(" ")));
        }
    }
    let mut layout = Vec::new();
    for c in &changes.controls {
        let name = ident_ok(&c.name)?;
        let mut body = Vec::new();
        if let Some(v) = &c.caption {
            body.push(format!("caption = {};", quote(v)));
        }
        if let Some(v) = &c.tooltip {
            body.push(format!("tooltip = {};", quote(v)));
        }
        if let Some(v) = c.visible {
            body.push(format!("visible = {v};"));
        }
        if let Some(v) = c.importance {
            body.push(format!("importance = {};", v.as_str()));
        }
        if let Some(s) = &c.style {
            let lines = style_lines(s);
            if !lines.is_empty() {
                body.push(format!("style {{ {} }}", lines.join(" ")));
            }
        }
        if let Some(r) = &c.responsive {
            let lines = responsive_lines(r);
            if !lines.is_empty() {
                body.push(format!("responsive {{ {} }}", lines.join(" ")));
            }
        }
        if !body.is_empty() {
            layout.push(format!("modify({name}) {{ {} }}", body.join(" ")));
        }
    }
    for a in &changes.adds {
        let field = ident_ok(&a.field)?;
        let anchor = ident_ok(&a.anchor)?;
        let directive = match a.position {
            Placement::Before => "addBefore",
            Placement::After => "addAfter",
            Placement::First => "addFirst",
            Placement::Last => "addLast",
        };
        let inner = match &a.caption {
            Some(c) => format!("field({field}) {{ caption = {}; }}", quote(c)),
            None => format!("field({field});"),
        };
        layout.push(format!("{directive}({anchor}) {{ {inner} }}"));
    }
    for m in &changes.moves {
        let name = ident_ok(&m.name)?;
        let anchor = ident_ok(&m.anchor)?;
        let directive = match m.position {
            Placement::Before => "moveBefore",
            Placement::After => "moveAfter",
            Placement::First => "moveFirst",
            Placement::Last => "moveLast",
        };
        layout.push(format!("{directive}({anchor}) {{ {name}; }}"));
    }
    if !layout.is_empty() {
        out.push_str("    layout {\n");
        for l in &layout {
            out.push_str(&format!("        {l}\n"));
        }
        out.push_str("    }\n");
    }
    let mut actions = Vec::new();
    for a in &changes.actions {
        let name = ident_ok(&a.name)?;
        if a.visible == Some(false) {
            actions.push(format!("hide({name});"));
            continue;
        }
        if let Some(c) = &a.caption {
            actions.push(format!("modify({name}) {{ caption = {}; }}", quote(c)));
        }
    }
    if !actions.is_empty() {
        out.push_str("    actions {\n");
        for l in &actions {
            out.push_str(&format!("        {l}\n"));
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n");
    Ok(out)
}

/// Reads the designer model back from a stored customization extension.
pub fn changes_from_extension(ext: &bosc_ir::PageExtensionIR) -> CustomizationChanges {
    use bosc_ir::PageControlKind as K;
    let mut ch = CustomizationChanges { page: PageChange { caption: ext.caption.clone(), style: ext.style.clone() }, ..Default::default() };
    for c in &ext.controls {
        let position = match c.kind_typed() {
            K::AddBefore | K::MoveBefore => Placement::Before,
            K::AddAfter | K::MoveAfter => Placement::After,
            K::AddFirst | K::MoveFirst => Placement::First,
            K::AddLast | K::MoveLast => Placement::Last,
            _ => Placement::Last,
        };
        match c.kind_typed() {
            K::Modify => ch.controls.push(ControlChange {
                name: c.name.clone(),
                caption: c.caption.clone(),
                tooltip: c.properties.get("tooltip").or_else(|| c.properties.get("toolTip")).cloned(),
                visible: c.properties.get("visible").map(|v| v != "false"),
                importance: c.responsive.as_ref().and_then(|r| r.importance),
                style: c.style.clone(),
                responsive: c.responsive.clone().map(|mut r| {
                    r.importance = None;
                    r
                }),
            }),
            K::Hide => ch.controls.push(ControlChange { name: c.name.clone(), visible: Some(false), ..Default::default() }),
            K::AddBefore | K::AddAfter | K::AddFirst | K::AddLast => {
                for f in &c.children {
                    ch.adds.push(AddChange {
                        field: f.source_expr.clone().unwrap_or_else(|| f.name.clone()),
                        anchor: c.name.clone(),
                        position: position.clone(),
                        caption: f.caption.clone(),
                    });
                }
            }
            K::MoveBefore | K::MoveAfter | K::MoveFirst | K::MoveLast => {
                for name in c.source_expr.iter().cloned().chain(c.children.iter().map(|x| x.name.clone())) {
                    ch.moves.push(MoveChange { name, anchor: c.name.clone(), position: position.clone() });
                }
            }
            _ => {}
        }
    }
    for a in &ext.actions {
        match a.placement.as_deref() {
            Some("hide") => ch.actions.push(ActionChange { name: a.name.clone(), visible: Some(false), ..Default::default() }),
            Some("modify") => ch.actions.push(ActionChange { name: a.name.clone(), caption: a.caption.clone(), visible: None }),
            _ => {}
        }
    }
    ch
}

#[cfg(test)]
#[path = "page_source_tests.rs"]
mod tests;
