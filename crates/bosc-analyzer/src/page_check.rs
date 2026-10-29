//! Source-level page diagnostics for a compilation unit.
//!
//! Two passes:
//! 1. **Syntax values** on the AST (exact spans): `style { }` keys and values, `responsive { }` values,
//!    client expressions and action areas. Lowering drops invalid values, so these are reported here.
//! 2. **Semantic resolution** through [`crate::page_resolver::compile_page`] for every page of the unit
//!    with the extensions of the same unit. Resolver diagnostics carry a node path that is mapped back to
//!    the authored control's span.

use bosc_ir::*;
use bosc_syntax::ast::*;
use bosc_syntax::Span;
use std::collections::HashMap;

use crate::page_expr;
use crate::page_lower::value_text;
use crate::page_merger::sort_extensions_for_merge;
use crate::page_resolver::{compile_page, ModelCatalog};

fn diag(severity: &str, code: &str, message: String, span: &Span, width: usize, hint: Option<String>) -> CompilerDiagnosticIR {
    CompilerDiagnosticIR {
        severity: severity.to_string(),
        code: code.to_string(),
        message,
        line: span.line,
        column: span.column,
        end_line: span.line,
        end_column: span.column + width.max(1),
        hint,
    }
}

const STYLE_KEYS: &[&str] = &[
    "background", "backgroundColor", "bg", "textColor", "color", "foreground", "borderColor", "border", "borderWidth",
    "radius", "borderRadius", "rounded", "shadow", "elevation", "density", "padding", "emphasis", "style", "icon", "image",
    "fontSize", "textSize", "size", "fontWeight", "weight", "textAlign", "align", "letterSpacing", "charSpace",
    "lineHeight", "textTransform", "transform", "textDecoration", "decoration", "borderStyle", "margin", "gap",
    "width", "w", "maxWidth", "maxW", "minWidth", "minW", "height", "h", "minHeight", "minH", "maxHeight", "maxH",
    "opacity", "overflow", "display", "justifyContent", "justify", "alignItems", "flexDirection", "direction", "flexWrap", "wrap",
    "iconSize", "iconColor", "iconBg", "iconBackground", "iconRadius",
];

fn is_valid_dimension(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s == "0" || s == "auto" {
        return true;
    }
    let stripped = s
        .strip_suffix("px")
        .or_else(|| s.strip_suffix("rem"))
        .or_else(|| s.strip_suffix("em"))
        .or_else(|| s.strip_suffix("%"))
        .or_else(|| s.strip_suffix("pt"))
        .or_else(|| s.strip_suffix("vh"))
        .or_else(|| s.strip_suffix("vw"))
        .unwrap_or(s);
    stripped.parse::<f64>().is_ok()
}

fn is_valid_spacing(s: &str) -> bool {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() || parts.len() > 4 {
        return false;
    }
    parts.iter().all(|p| is_valid_dimension(p))
}

fn check_style_block(props: &[Property], owner: &str, out: &mut Vec<CompilerDiagnosticIR>) {
    for p in props {
        let text = value_text(&p.value);
        let problem: Option<String> = match p.name.as_str() {
            "background" | "backgroundColor" | "bg" | "textColor" | "color" | "foreground" | "borderColor" | "border" => {
                ColorValue::parse(&text).err()
            }
            "borderWidth" => match &p.value {
                Expr::IntLit(n) if (0..=PageStyleIR::MAX_BORDER_WIDTH as i64).contains(n) => None,
                _ => Some(format!("borderWidth must be a whole number from 0 to {}", PageStyleIR::MAX_BORDER_WIDTH)),
            },
            "radius" | "borderRadius" | "rounded" => {
                Radius::parse(&text).is_none().then(|| format!("radius '{text}' must be one of {}", Radius::names().join(", ")))
            }
            "shadow" | "elevation" => {
                Shadow::parse(&text).is_none().then(|| format!("shadow '{text}' must be one of {}", Shadow::names().join(", ")))
            }
            "density" => {
                Density::parse(&text).is_none().then(|| format!("density '{text}' must be one of {}", Density::names().join(", ")))
            }
            "padding" => {
                if Density::parse(&text).is_none() && !is_valid_spacing(&text) {
                    Some(format!("padding '{text}' must be one of {} or a valid spacing value (e.g. 8px, 1rem)", Density::names().join(", ")))
                } else {
                    None
                }
            }
            "margin" => {
                if !is_valid_spacing(&text) {
                    Some(format!("margin '{text}' must be a valid spacing value (e.g. 0, 16px, 8px 16px)"))
                } else {
                    None
                }
            }
            "gap" => {
                if !is_valid_dimension(&text) {
                    Some(format!("gap '{text}' must be a valid dimension (e.g. 8px, 1rem)"))
                } else {
                    None
                }
            }
            "fontSize" | "textSize" | "size" => {
                if is_valid_dimension(&text) || matches!(text.as_str(), "xs" | "sm" | "base" | "md" | "lg" | "xl" | "2xl" | "3xl") {
                    None
                } else {
                    Some(format!("fontSize '{text}' must be a size token or dimension"))
                }
            }
            "fontWeight" | "weight" => {
                if matches!(text.as_str(), "normal" | "medium" | "semibold" | "bold" | "100" | "200" | "300" | "400" | "500" | "600" | "700" | "800" | "900") {
                    None
                } else {
                    Some(format!("fontWeight '{text}' must be normal, medium, semibold, bold, or 100-900"))
                }
            }
            "textAlign" | "align" => {
                if matches!(text.as_str(), "left" | "center" | "right" | "justify") {
                    None
                } else {
                    Some(format!("textAlign '{text}' must be left, center, right, or justify"))
                }
            }
            "letterSpacing" | "charSpace" | "lineHeight" => {
                if is_valid_dimension(&text) || text.parse::<f64>().is_ok() || text == "normal" {
                    None
                } else {
                    Some(format!("'{} = {text}' must be a valid number or dimension", p.name))
                }
            }
            "textTransform" | "transform" => {
                if matches!(text.as_str(), "none" | "uppercase" | "lowercase" | "capitalize") {
                    None
                } else {
                    Some(format!("textTransform '{text}' must be none, uppercase, lowercase, or capitalize"))
                }
            }
            "textDecoration" | "decoration" => {
                if matches!(text.as_str(), "none" | "underline" | "line-through") {
                    None
                } else {
                    Some(format!("textDecoration '{text}' must be none, underline, or line-through"))
                }
            }
            "borderStyle" => {
                if matches!(text.as_str(), "solid" | "dashed" | "dotted" | "double" | "none" | "hidden") {
                    None
                } else {
                    Some(format!("borderStyle '{text}' must be solid, dashed, dotted, or none"))
                }
            }
            "width" | "w" | "maxWidth" | "maxW" | "minWidth" | "minW" | "height" | "h" | "minHeight" | "minH" | "maxHeight" | "maxH" => {
                if is_valid_dimension(&text) || matches!(text.as_str(), "fit-content" | "max-content" | "min-content" | "100%" | "100vw" | "100vh") {
                    None
                } else {
                    Some(format!("'{} = {text}' must be a valid size dimension (e.g. 100%, 320px, auto)", p.name))
                }
            }
            "opacity" => {
                let ok = text.parse::<f64>().map(|v| (0.0..=1.0).contains(&v)).unwrap_or(false);
                if ok {
                    None
                } else {
                    Some(format!("opacity '{text}' must be a number between 0.0 and 1.0"))
                }
            }
            "overflow" => {
                if matches!(text.as_str(), "visible" | "hidden" | "auto" | "scroll" | "clip") {
                    None
                } else {
                    Some(format!("overflow '{text}' must be visible, hidden, auto, or scroll"))
                }
            }
            "display" => {
                if matches!(text.as_str(), "block" | "flex" | "grid" | "inline" | "inline-block" | "inline-flex" | "none") {
                    None
                } else {
                    Some(format!("display '{text}' must be block, flex, grid, inline-flex, or none"))
                }
            }
            "justifyContent" | "justify" => {
                if matches!(text.as_str(), "start" | "flex-start" | "center" | "end" | "flex-end" | "between" | "space-between" | "around" | "space-around" | "evenly") {
                    None
                } else {
                    Some(format!("justifyContent '{text}' must be start, center, end, between, around, or evenly"))
                }
            }
            "alignItems" => {
                if matches!(text.as_str(), "start" | "flex-start" | "center" | "end" | "flex-end" | "stretch" | "baseline") {
                    None
                } else {
                    Some(format!("alignItems '{text}' must be start, center, end, stretch, or baseline"))
                }
            }
            "flexDirection" | "direction" => {
                if matches!(text.as_str(), "row" | "column" | "row-reverse" | "column-reverse") {
                    None
                } else {
                    Some(format!("flexDirection '{text}' must be row, column, row-reverse, or column-reverse"))
                }
            }
            "flexWrap" | "wrap" => {
                if matches!(text.as_str(), "wrap" | "nowrap" | "wrap-reverse") {
                    None
                } else {
                    Some(format!("flexWrap '{text}' must be wrap or nowrap"))
                }
            }
            "iconSize" => {
                if matches!(text.as_str(), "xs" | "sm" | "md" | "lg" | "xl" | "2xl") || is_valid_dimension(&text) {
                    None
                } else {
                    Some(format!("iconSize '{text}' must be xs, sm, md, lg, xl, 2xl or a valid dimension like 16px"))
                }
            }
            "iconColor" | "iconBg" | "iconBackground" => ColorValue::parse(&text)
                .err()
                .map(|e| format!("{} must be a theme token or #rrggbb: {e}", p.name)),
            "iconRadius" => Radius::parse(&text)
                .map(|_| ())
                .ok_or_else(|| format!("iconRadius '{text}' must be none, sm, md, lg, or full"))
                .err(),
            "emphasis" | "style" => {
                Emphasis::parse(&text).is_none().then(|| format!("emphasis '{text}' must be one of {}", Emphasis::names().join(", ")))
            }
            "icon" | "image" => normalize_icon_name(&text).is_none().then(|| format!("'{text}' is not a valid icon name")),
            other => Some(format!("'{other}' is not a style property; use one of {}", STYLE_KEYS.join(", "))),
        };
        if let Some(message) = problem {
            out.push(diag("error", "BOS3003", format!("{owner}: {message}"), &p.span, p.name.len(), Some("Styles accept theme tokens or valid CSS values.".into())));
        }
    }
}

fn check_responsive_block(props: &[Property], owner: &str, out: &mut Vec<CompilerDiagnosticIR>) {
    for p in props {
        let text = value_text(&p.value);
        let ok = match p.name.as_str() {
            "hideBelow" => Breakpoint::parse(&text).is_some(),
            "importance" => Importance::parse(&text).is_some(),
            name if Breakpoint::parse(name).is_some() || matches!(name, "colSpan" | "columnSpan" | "span") => {
                matches!(&p.value, Expr::IntLit(n) if (1..=BreakpointSpans::COLUMNS as i64).contains(n))
            }
            _ => false,
        };
        if !ok {
            out.push(diag(
                "error",
                "BOS3011",
                format!("{owner}: invalid responsive setting '{} = {text}'", p.name),
                &p.span,
                p.name.len(),
                Some("Use xs/sm/md/lg/xl = 1..12, hideBelow = xs|sm|md|lg|xl, importance = Promoted|Standard|Additional.".into()),
            ));
        }
    }
}

fn is_expression_property(name: &str) -> bool {
    matches!(name, "showWhen" | "visibleWhen" | "enableWhen" | "enabledWhen" | "editableWhen" | "styleExpr" | "styleWhen")
        || name.starts_with("link.")
}

fn check_expression(p: &Property, owner: &str, out: &mut Vec<CompilerDiagnosticIR>) {
    if let Err(msg) = page_expr::canonical_from_property(&p.value) {
        out.push(diag("error", "BOS3005", format!("{owner}: invalid expression for '{}': {msg}", p.name), &p.span, p.name.len(), None));
    }
}

fn check_control(c: &PageControlDef, out: &mut Vec<CompilerDiagnosticIR>) {
    let owner = format!("{}({})", c.kind, c.name);
    check_style_block(&c.style, &owner, out);
    check_responsive_block(&c.responsive, &owner, out);
    for p in &c.properties {
        let dynamic = matches!(p.name.as_str(), "visible" | "editable" | "enabled") && !matches!(p.value, Expr::BoolLit(_));
        if is_expression_property(&p.name) || dynamic {
            check_expression(p, &owner, out);
        } else if p.name == "icon" && normalize_icon_name(&value_text(&p.value)).is_none() {
            out.push(diag("error", "BOS3003", format!("{owner}: '{}' is not a valid icon name", value_text(&p.value)), &p.span, 4, None));
        } else if matches!(p.name.as_str(), "importance" | "hideBelow" | "colSpan" | "columnSpan" | "span") {
            check_responsive_block(std::slice::from_ref(p), &owner, out);
        }
    }
    for ch in &c.children {
        check_control(ch, out);
    }
}

fn check_actions(actions: &[PageActionDef], out: &mut Vec<CompilerDiagnosticIR>) {
    for a in actions {
        let owner = format!("{}({})", a.kind, a.name);
        if a.kind == "area" && ActionArea::parse(&a.name).is_none() {
            out.push(diag(
                "error",
                "BOS3011",
                format!("Unknown action area '{}'", a.name),
                &a.span,
                4,
                Some(format!("Use one of {}.", ActionArea::names().join(", "))),
            ));
        }
        check_style_block(&a.style, &owner, out);
        for p in &a.properties {
            let dynamic = matches!(p.name.as_str(), "visible" | "enabled") && !matches!(p.value, Expr::BoolLit(_));
            if is_expression_property(&p.name) || dynamic {
                check_expression(p, &owner, out);
            }
        }
        check_actions(&a.children, out);
    }
}

/// Node path -> span, computed exactly like the resolver computes control paths.
fn collect_spans(controls: &[PageControlDef], parent: &str, out: &mut HashMap<String, Span>) {
    for c in controls {
        let path = if parent.is_empty() {
            format!("{}:{}", PageControlKind::parse(&c.kind).as_str(), c.name)
        } else {
            format!("{parent}/{}:{}", PageControlKind::parse(&c.kind).as_str(), c.name)
        };
        out.insert(path.clone(), c.span.clone());
        collect_spans(&c.children, &path, out);
    }
}

fn collect_action_spans(actions: &[PageActionDef], out: &mut HashMap<String, Span>) {
    for a in actions {
        out.entry(format!("action:{}", a.name)).or_insert_with(|| a.span.clone());
        collect_action_spans(&a.children, out);
    }
}

/// Runs both passes over every page and page extension of the program.
pub(crate) fn check_pages(program: &Program, model: &ModelIR) -> Vec<CompilerDiagnosticIR> {
    let mut out = Vec::new();
    let mut spans: HashMap<String, (Span, HashMap<String, Span>)> = HashMap::new();

    for item in &program.items {
        match item {
            Item::Page(p) => {
                check_style_block(&p.style, &format!("page {}", p.name), &mut out);
                p.controls.iter().for_each(|c| check_control(c, &mut out));
                check_actions(&p.actions, &mut out);
                let mut map = HashMap::new();
                collect_spans(&p.controls, "", &mut map);
                collect_action_spans(&p.actions, &mut map);
                spans.insert(p.name.to_ascii_lowercase(), (p.span.clone(), map));
            }
            Item::PageExtension(pe) => {
                check_style_block(&pe.style, &format!("pageextension {}", pe.name), &mut out);
                pe.controls.iter().for_each(|c| check_control(c, &mut out));
                check_actions(&pe.actions, &mut out);
                let mut map = HashMap::new();
                collect_spans(&pe.controls, "", &mut map);
                collect_action_spans(&pe.actions, &mut map);
                spans.insert(pe.name.to_ascii_lowercase(), (pe.span.clone(), map));
            }
            _ => {}
        }
    }

    let catalog = ModelCatalog::new(vec![model]);
    for page in &model.pages {
        let mut exts: Vec<PageExtensionIR> =
            model.page_extensions.iter().filter(|e| e.target_page.eq_ignore_ascii_case(&page.name)).cloned().collect();
        sort_extensions_for_merge(&mut exts, |_| 0);
        let result = compile_page(page, &exts, &catalog);
        for d in result.diagnostics {
            let (object_span, map) = match spans.get(&d.object.to_ascii_lowercase()).or_else(|| spans.get(&page.name.to_ascii_lowercase())) {
                Some(entry) => entry,
                None => continue,
            };
            // Merged controls keep their base path; fall back to the object (page/extension) span.
            let span = d
                .path
                .as_deref()
                .and_then(|p| map.get(p).or_else(|| spans.get(&page.name.to_ascii_lowercase()).and_then(|(_, m)| m.get(p))))
                .unwrap_or(object_span);
            out.push(diag(d.severity.as_str(), &d.code, d.message, span, d.path.as_deref().map(|p| p.len().min(40)).unwrap_or(4), d.hint));
        }
    }

    // Customization layers whose target lives in another package still get the whitelist check.
    for ext in &model.page_extensions {
        if ext.is_customization && !model.pages.iter().any(|p| p.name.eq_ignore_ascii_case(&ext.target_page)) {
            let (_, ds) = crate::page_resolver::enforce_customization_whitelist(ext);
            if let Some((span, _)) = spans.get(&ext.name.to_ascii_lowercase()) {
                for d in ds {
                    out.push(diag("error", &d.code, d.message, span, ext.name.len(), d.hint));
                }
            }
        }
    }
    out
}
