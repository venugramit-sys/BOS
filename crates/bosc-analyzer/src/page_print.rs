//! Printer: authored page IR (v2) → BOS source.
//!
//! Used where a page or page extension has no stored source text (objects installed from a package
//! bundle, rows written by older nodes, the designer's customization layer). The output is the exact
//! inverse of [`crate::page_lower`]: parsing and lowering the printed text gives back the same IR
//! (verified for every page in the repository by `page_print_tests`). Formatting is canonical
//! (4-space indent, one property per line); developer-authored source is always preferred when it exists.

use bosc_ir::*;
use std::collections::BTreeMap;

const INDENT: &str = "    ";

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !RESERVED.contains(&s)
}

/// Words the lexer turns into keywords; they must be quoted where an identifier is expected.
const RESERVED: &[&str] = &[
    "table", "tableextension", "tablelogic", "tablelogicextension", "statemachine", "statemachineextension", "codeunit",
    "interface", "enum", "enumextension", "page", "pageextension", "view", "query", "report", "custommetadata",
    "permissionset", "elements", "dataitem", "select", "from", "leftjoin", "innerjoin", "groupby", "having", "orderby",
    "extends", "implements", "for", "state", "transition", "guard", "approvalmatrix", "tier", "params", "timeout",
    "escalateto", "primaryKey", "key", "relation", "computed", "flowfilter", "where", "in", "true", "false", "null",
    "public", "private", "static", "function", "return", "if", "else",
];

/// A string literal with BOS escapes.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Name position (`group(General)` / `group("Order Lines")`).
fn name(s: &str) -> String {
    if is_ident(s) { s.to_string() } else { quote(s) }
}

/// Free-form value from a property map: numbers, booleans and identifiers bare, everything else quoted.
fn raw_value(v: &str) -> String {
    let t = v.trim();
    if t == "true" || t == "false" || (t == v && is_ident(t)) || (!t.is_empty() && t.parse::<i64>().is_ok() && t == v) {
        v.to_string()
    } else {
        quote(v)
    }
}

fn line(out: &mut String, depth: usize, text: &str) {
    for _ in 0..depth {
        out.push_str(INDENT);
    }
    out.push_str(text);
    out.push('\n');
}

fn color(c: &ColorValue) -> String {
    let s: String = c.clone().into();
    if s.starts_with('#') { quote(&s) } else { s }
}

fn style_entries(style: &PageStyleIR) -> Vec<String> {
    let mut v = Vec::new();
    if let Some(c) = &style.background {
        v.push(format!("background = {};", color(c)));
    }
    if let Some(c) = &style.text_color {
        v.push(format!("textColor = {};", color(c)));
    }
    if let Some(c) = &style.border_color {
        v.push(format!("borderColor = {};", color(c)));
    }
    if let Some(w) = style.border_width {
        v.push(format!("borderWidth = {w};"));
    }
    if let Some(r) = style.radius {
        v.push(format!("radius = {};", r.as_str()));
    }
    if let Some(s) = style.shadow {
        v.push(format!("shadow = {};", s.as_str()));
    }
    if let Some(d) = style.density {
        v.push(format!("density = {};", d.as_str()));
    }
    if let Some(e) = style.emphasis {
        v.push(format!("emphasis = {};", e.as_str()));
    }
    if let Some(i) = &style.icon {
        v.push(format!("icon = {};", quote(i)));
    }
    if let Some(s) = &style.icon_size {
        v.push(format!("iconSize = {};", quote(s)));
    }
    if let Some(c) = &style.icon_color {
        v.push(format!("iconColor = {};", color(c)));
    }
    if let Some(c) = &style.icon_background {
        v.push(format!("iconBackground = {};", color(c)));
    }
    if let Some(r) = style.icon_radius {
        v.push(format!("iconRadius = {};", r.as_str()));
    }
    if let Some(s) = &style.font_size {
        v.push(format!("fontSize = {};", quote(s)));
    }
    if let Some(w) = &style.font_weight {
        v.push(format!("fontWeight = {};", quote(w)));
    }
    if let Some(a) = &style.text_align {
        v.push(format!("textAlign = {};", quote(a)));
    }
    if let Some(ls) = &style.letter_spacing {
        v.push(format!("letterSpacing = {};", quote(ls)));
    }
    if let Some(lh) = &style.line_height {
        v.push(format!("lineHeight = {};", quote(lh)));
    }
    if let Some(tt) = &style.text_transform {
        v.push(format!("textTransform = {};", quote(tt)));
    }
    if let Some(td) = &style.text_decoration {
        v.push(format!("textDecoration = {};", quote(td)));
    }
    if let Some(bs) = &style.border_style {
        v.push(format!("borderStyle = {};", quote(bs)));
    }
    if let Some(p) = &style.padding {
        v.push(format!("padding = {};", quote(p)));
    }
    if let Some(m) = &style.margin {
        v.push(format!("margin = {};", quote(m)));
    }
    if let Some(g) = &style.gap {
        v.push(format!("gap = {};", quote(g)));
    }
    if let Some(w) = &style.width {
        v.push(format!("width = {};", quote(w)));
    }
    if let Some(mw) = &style.max_width {
        v.push(format!("maxWidth = {};", quote(mw)));
    }
    if let Some(minw) = &style.min_width {
        v.push(format!("minWidth = {};", quote(minw)));
    }
    if let Some(h) = &style.height {
        v.push(format!("height = {};", quote(h)));
    }
    if let Some(minh) = &style.min_height {
        v.push(format!("minHeight = {};", quote(minh)));
    }
    if let Some(maxh) = &style.max_height {
        v.push(format!("maxHeight = {};", quote(maxh)));
    }
    if let Some(op) = &style.opacity {
        v.push(format!("opacity = {};", quote(op)));
    }
    if let Some(ov) = &style.overflow {
        v.push(format!("overflow = {};", quote(ov)));
    }
    if let Some(disp) = &style.display {
        v.push(format!("display = {};", quote(disp)));
    }
    if let Some(jc) = &style.justify_content {
        v.push(format!("justifyContent = {};", quote(jc)));
    }
    if let Some(ai) = &style.align_items {
        v.push(format!("alignItems = {};", quote(ai)));
    }
    if let Some(fd) = &style.flex_direction {
        v.push(format!("flexDirection = {};", quote(fd)));
    }
    if let Some(fw) = &style.flex_wrap {
        v.push(format!("flexWrap = {};", quote(fw)));
    }
    v
}

fn style_block(out: &mut String, depth: usize, style: &Option<PageStyleIR>) {
    if let Some(s) = style {
        let entries = style_entries(s);
        if !entries.is_empty() {
            line(out, depth, &format!("style {{ {} }}", entries.join(" ")));
        }
    }
}

fn responsive_block(out: &mut String, depth: usize, r: &Option<ResponsiveIR>) {
    let Some(r) = r else { return };
    let mut v = Vec::new();
    for (bp, n) in [("xs", r.span.xs), ("sm", r.span.sm), ("md", r.span.md), ("lg", r.span.lg), ("xl", r.span.xl)] {
        if let Some(n) = n {
            v.push(format!("{bp} = {n};"));
        }
    }
    if let Some(h) = r.hide_below {
        v.push(format!("hideBelow = {};", h.as_str()));
    }
    if let Some(i) = r.importance {
        v.push(format!("importance = {};", i.as_str()));
    }
    if !v.is_empty() {
        line(out, depth, &format!("responsive {{ {} }}", v.join(" ")));
    }
}

/// `onX(params) { script }`; the parser dedents the body, so the script is re-indented one level.
fn script_block(out: &mut String, depth: usize, header: &str, script: &str) {
    if script.trim().is_empty() {
        line(out, depth, &format!("{header} {{ }}"));
        return;
    }
    line(out, depth, &format!("{header} {{"));
    for l in script.lines() {
        if l.trim().is_empty() {
            out.push('\n');
        } else {
            line(out, depth + 1, l);
        }
    }
    line(out, depth, "}");
}

fn triggers(out: &mut String, depth: usize, list: &[TriggerIR]) {
    for t in list {
        let header = format!("{}({})", t.name, t.params.join(", "));
        script_block(out, depth, &header, t.script.as_deref().unwrap_or(""));
    }
}

fn link_block(out: &mut String, depth: usize, links: &BTreeMap<String, String>) {
    if links.is_empty() {
        return;
    }
    let entries: Vec<String> = links.iter().map(|(f, e)| format!("{f} = {};", quote(e))).collect();
    line(out, depth, &format!("link {{ {} }}", entries.join(" ")));
}

// ----------------------------------------------------
// CONTROLS
// ----------------------------------------------------

fn print_control(out: &mut String, depth: usize, c: &PageControlIR) {
    let header = match (&c.source_expr, c.kind.as_str()) {
        (Some(src), "field") if src == &c.name => format!("field({})", name(&c.name)),
        (Some(src), _) => format!("{}({}; {})", c.kind, name(&c.name), name(src)),
        (None, _) => format!("{}({})", c.kind, name(&c.name)),
    };
    let mut body = String::new();
    if let Some(cap) = &c.caption {
        line(&mut body, depth + 1, &format!("caption = {};", quote(cap)));
    }
    let mut links = BTreeMap::new();
    for (k, v) in &c.properties {
        if let Some(field) = k.strip_prefix("link.") {
            links.insert(field.to_string(), v.clone());
        } else if matches!(k.as_str(), "showWhen" | "enableWhen" | "editableWhen" | "styleExpr") {
            line(&mut body, depth + 1, &format!("{k} = {};", quote(v)));
        } else {
            line(&mut body, depth + 1, &format!("{k} = {};", raw_value(v)));
        }
    }
    link_block(&mut body, depth + 1, &links);
    style_block(&mut body, depth + 1, &c.style);
    responsive_block(&mut body, depth + 1, &c.responsive);
    triggers(&mut body, depth + 1, &c.triggers);
    for child in &c.children {
        print_control(&mut body, depth + 1, child);
    }
    if body.is_empty() {
        line(out, depth, &format!("{header};"));
    } else {
        line(out, depth, &format!("{header} {{"));
        out.push_str(&body);
        line(out, depth, "}");
    }
}

// ----------------------------------------------------
// ACTIONS
// ----------------------------------------------------

/// Body of an `action` / `actionref` (everything but its location, which nesting expresses).
fn action_body(a: &PageActionIR, depth: usize, skip_area: bool) -> String {
    let mut b = String::new();
    let mut p = |text: String| line(&mut b, depth, &text);
    if let Some(v) = &a.caption {
        p(format!("caption = {};", quote(v)));
    }
    if let Some(v) = &a.icon {
        p(format!("icon = {};", quote(v)));
    }
    if a.primary {
        p("primary = true;".into());
    }
    if a.promoted_only {
        p("promotedOnly = true;".into());
    }
    if let Some(v) = &a.promoted_category {
        p(format!("promotedCategory = {};", quote(v)));
    }
    if let Some(v) = &a.shortcut {
        p(format!("shortcut = {};", quote(v)));
    }
    if let Some(v) = &a.confirm {
        p(format!("confirm = {};", quote(v)));
    }
    if let Some(v) = &a.tooltip {
        p(format!("tooltip = {};", quote(v)));
    }
    if a.is_default {
        p("isDefault = true;".into());
    }
    if a.in_footer_bar {
        p("inFooterBar = true;".into());
    }
    if a.ellipsis {
        p("ellipsis = true;".into());
    }
    if !a.visible {
        p("visible = false;".into());
    }
    if let Some(v) = &a.show_when {
        p(format!("showWhen = {};", quote(v)));
    }
    if let Some(v) = &a.enable_when {
        p(format!("enableWhen = {};", quote(v)));
    }
    if let Some(v) = &a.run_codeunit {
        p(format!("runCodeunit = {};", quote(v)));
    }
    if let Some(v) = &a.open_page {
        p(format!("openPage = {};", quote(v)));
    }
    if let Some(v) = a.run_page_mode {
        p(format!("runPageMode = {};", v.as_str()));
    }
    if let Some(v) = &a.selection {
        p(format!("selection = {};", quote(v)));
    }
    if !skip_area && let Some(area) = a.area {
        p(format!("area = {};", area.as_str()));
    }
    for (k, v) in &a.properties {
        if k != "separatorBefore" {
            p(format!("{k} = {};", raw_value(v)));
        }
    }
    link_block(&mut b, depth, &a.run_page_link);
    style_block(&mut b, depth, &a.style);
    if let Some(script) = &a.script {
        script_block(&mut b, depth, "onAction(rec, ctx)", script);
    }
    b
}

fn print_action(out: &mut String, depth: usize, a: &PageActionIR, skip_area: bool) {
    if a.properties.get("separatorBefore").map(String::as_str) == Some("true") {
        line(out, depth, "separator;");
    }
    let header = match &a.target {
        Some(t) if t != &a.name => format!("actionref({}; {})", name(&a.name), name(t)),
        Some(_) => format!("actionref({})", name(&a.name)),
        None => format!("action({})", name(&a.name)),
    };
    // actionrefs default to the Promoted area; anything else is implied by the surrounding area.
    let body = action_body(a, depth + 1, skip_area || (a.target.is_some() && a.area == Some(ActionArea::Promoted)));
    if body.is_empty() {
        line(out, depth, &format!("{header} {{ }}"));
    } else {
        line(out, depth, &format!("{header} {{"));
        out.push_str(&body);
        line(out, depth, "}");
    }
}

fn print_group(out: &mut String, depth: usize, g: &ActionGroupIR, actions: &[&PageActionIR], groups: &[ActionGroupIR]) {
    line(out, depth, &format!("group({}) {{", name(&g.name)));
    if let Some(c) = &g.caption {
        line(out, depth + 1, &format!("caption = {};", quote(c)));
    }
    if let Some(i) = &g.icon {
        line(out, depth + 1, &format!("icon = {};", quote(i)));
    }
    if !g.visible {
        line(out, depth + 1, "visible = false;");
    }
    print_level(out, depth + 1, g.area, Some(&g.name), actions, groups);
    line(out, depth, "}");
}

/// Prints the actions and groups located at (`area`, `group`), in their original order.
fn print_level(out: &mut String, depth: usize, area: Option<ActionArea>, group: Option<&str>, actions: &[&PageActionIR], groups: &[ActionGroupIR]) {
    let here_groups: Vec<&ActionGroupIR> = groups.iter().filter(|g| g.area == area && g.parent.as_deref() == group).collect();
    // Position of a group = position of its first action anywhere below it.
    let first_member = |g: &ActionGroupIR| actions.iter().position(|x| is_inside(x, &g.name, groups));
    let mut printed_groups = std::collections::HashSet::new();
    for (index, a) in actions.iter().enumerate().filter(|(_, a)| a.area == area && a.group.as_deref() == group) {
        for g in &here_groups {
            let is_legacy_of_this = g.name == a.name;
            if !printed_groups.contains(&g.name) && !is_legacy_of_this && first_member(g).is_some_and(|f| f < index) {
                printed_groups.insert(g.name.clone());
                print_group(out, depth, g, actions, groups);
            }
        }
        // A legacy action with nested actions was lowered into an action plus a group of the same name.
        if let Some(g) = here_groups.iter().find(|g| g.name == a.name && g.caption.is_none() && g.icon.is_none()) {
            printed_groups.insert(g.name.clone());
            let body = action_body(a, depth + 1, true);
            line(out, depth, &format!("action({}) {{", name(&a.name)));
            out.push_str(&body);
            print_level(out, depth + 1, g.area, Some(&g.name), actions, groups);
            line(out, depth, "}");
            continue;
        }
        print_action(out, depth, a, true);
    }
    for g in here_groups {
        if !printed_groups.contains(&g.name) {
            print_group(out, depth, g, actions, groups);
        }
    }
}

fn is_inside(a: &PageActionIR, group: &str, groups: &[ActionGroupIR]) -> bool {
    let mut current = a.group.clone();
    while let Some(g) = current {
        if g == group {
            return true;
        }
        current = groups.iter().find(|x| x.name == g).and_then(|x| x.parent.clone());
    }
    false
}

fn print_base_actions(out: &mut String, depth: usize, actions: &[PageActionIR], groups: &[ActionGroupIR]) {
    if actions.is_empty() && groups.is_empty() {
        return;
    }
    let all: Vec<&PageActionIR> = actions.iter().collect();
    line(out, depth, "actions {");
    // Actions without an area first, then one block per area in first-use order.
    print_level(out, depth + 1, None, None, &all, groups);
    let mut areas: Vec<ActionArea> = Vec::new();
    for a in actions.iter().filter_map(|a| a.area).chain(groups.iter().filter_map(|g| g.area)) {
        if !areas.contains(&a) {
            areas.push(a);
        }
    }
    for area in areas {
        line(out, depth + 1, &format!("area({}) {{", area.as_str()));
        print_level(out, depth + 2, Some(area), None, &all, groups);
        line(out, depth + 1, "}");
    }
    line(out, depth, "}");
}

fn print_extension_actions(out: &mut String, depth: usize, ext: &PageExtensionIR) {
    if ext.actions.is_empty() && ext.action_groups.is_empty() {
        return;
    }
    line(out, depth, "actions {");
    let mut i = 0;
    let acts = &ext.actions;
    while i < acts.len() {
        let a = &acts[i];
        match a.placement.as_deref() {
            Some(kind @ ("addFirst" | "addLast" | "addBefore" | "addAfter")) => {
                let anchor = a.anchor.clone().unwrap_or_default();
                let mut j = i;
                line(out, depth + 1, &format!("{kind}({}) {{", name(&anchor)));
                while j < acts.len() && acts[j].placement.as_deref() == Some(kind) && acts[j].anchor.as_deref() == Some(anchor.as_str()) {
                    print_action(out, depth + 2, &acts[j], true);
                    j += 1;
                }
                line(out, depth + 1, "}");
                i = j;
            }
            Some("hide") => {
                line(out, depth + 1, &format!("hide({});", name(a.anchor.as_deref().unwrap_or(&a.name))));
                i += 1;
            }
            Some("modify") => {
                let body = action_body(a, depth + 2, true);
                let anchor = name(a.anchor.as_deref().unwrap_or(&a.name));
                if body.is_empty() {
                    line(out, depth + 1, &format!("modify({anchor}) {{ }}"));
                } else {
                    line(out, depth + 1, &format!("modify({anchor}) {{"));
                    out.push_str(&body);
                    line(out, depth + 1, "}");
                }
                i += 1;
            }
            Some(kind @ ("moveBefore" | "moveAfter")) => {
                let anchor = a.anchor.clone().unwrap_or_default();
                let mut names = Vec::new();
                let mut j = i;
                while j < acts.len() && acts[j].placement.as_deref() == Some(kind) && acts[j].anchor.as_deref() == Some(anchor.as_str()) {
                    names.push(format!("{};", name(&acts[j].name)));
                    j += 1;
                }
                line(out, depth + 1, &format!("{kind}({}) {{ {} }}", name(&anchor), names.join(" ")));
                i = j;
            }
            _ => {
                // Plain added action: located by its own area.
                match a.area {
                    Some(area) => {
                        line(out, depth + 1, &format!("area({}) {{", area.as_str()));
                        print_action(out, depth + 2, a, true);
                        line(out, depth + 1, "}");
                    }
                    None => print_action(out, depth + 1, a, true),
                }
                i += 1;
            }
        }
    }
    line(out, depth, "}");
}

// ----------------------------------------------------
// OBJECTS
// ----------------------------------------------------

fn variables(out: &mut String, depth: usize, vars: &[PageVariableIR]) {
    if vars.is_empty() {
        return;
    }
    line(out, depth, "variables {");
    for v in vars {
        let mut props = Vec::new();
        if let Some(c) = &v.caption {
            props.push(format!("caption = {};", quote(c)));
        }
        if let Some(d) = &v.default_value {
            props.push(format!("default = {};", raw_value(d)));
        }
        if props.is_empty() {
            line(out, depth + 1, &format!("{}: {};", v.name, v.data_type));
        } else {
            line(out, depth + 1, &format!("{}: {} {{ {} }};", v.name, v.data_type, props.join(" ")));
        }
    }
    line(out, depth, "}");
}

fn layout(out: &mut String, depth: usize, controls: &[PageControlIR]) {
    if controls.is_empty() {
        return;
    }
    line(out, depth, "layout {");
    for c in controls {
        print_control(out, depth + 1, c);
    }
    line(out, depth, "}");
}

fn extra_properties(out: &mut String, depth: usize, props: &BTreeMap<String, String>) {
    for (k, v) in props {
        line(out, depth, &format!("{k} = {};", raw_value(v)));
    }
}

/// BOS source of a page.
pub fn print_page(p: &PageIR) -> String {
    let mut out = String::new();
    let header = match &p.source_table {
        Some(t) => format!("page {} for {} {{", p.name, t),
        None => format!("page {} {{", p.name),
    };
    line(&mut out, 0, &header);
    line(&mut out, 1, &format!("pageType = {};", p.page_type));
    if let Some(c) = &p.caption {
        line(&mut out, 1, &format!("caption = {};", quote(c)));
    }
    if let Some(k) = p.source_kind.as_deref().filter(|k| *k != "table") {
        line(&mut out, 1, &format!("sourceKind = {};", quote(k)));
    }
    for (key, v) in [("editable", p.editable), ("insertAllowed", p.insert_allowed), ("modifyAllowed", p.modify_allowed), ("deleteAllowed", p.delete_allowed), ("showFilter", p.show_filter)] {
        if let Some(b) = v {
            line(&mut out, 1, &format!("{key} = {b};"));
        }
    }
    if !p.filters.is_empty() {
        let items: Vec<String> = p.filters.iter().map(|f| quote(f)).collect();
        line(&mut out, 1, &format!("filters = [{}];", items.join(", ")));
    }
    if let Some(c) = &p.card_page {
        line(&mut out, 1, &format!("cardPageId = {};", name(c)));
    }
    if !p.data_caption_fields.is_empty() {
        let items: Vec<String> = p.data_caption_fields.iter().map(|f| name(f)).collect();
        line(&mut out, 1, &format!("dataCaptionFields = [{}];", items.join(", ")));
    }
    for (key, on) in [("refreshOnActivate", p.refresh_on_activate), ("delayedInsert", p.delayed_insert), ("multipleNewLines", p.multiple_new_lines), ("autoSplitKey", p.auto_split_key)] {
        if on {
            line(&mut out, 1, &format!("{key} = true;"));
        }
    }
    if let Some(u) = &p.usage_category {
        line(&mut out, 1, &format!("usageCategory = {};", quote(u)));
    }
    if let Some(t) = &p.instructional_text {
        line(&mut out, 1, &format!("instructionalText = {};", quote(t)));
    }
    extra_properties(&mut out, 1, &p.properties);
    style_block(&mut out, 1, &p.style);
    variables(&mut out, 1, &p.variables);
    if !p.views.is_empty() {
        line(&mut out, 1, "views {");
        for v in &p.views {
            let mut props = Vec::new();
            if let Some(c) = &v.caption {
                props.push(format!("caption = {};", quote(c)));
            }
            if let Some(f) = &v.filter {
                props.push(format!("filter = {f};"));
            }
            if let Some(s) = &v.sort {
                props.push(format!("sort = {};", quote(s)));
            }
            if v.is_default {
                props.push("default = true;".into());
            }
            line(&mut out, 2, &format!("view({}) {{ {} }}", name(&v.name), props.join(" ")));
        }
        line(&mut out, 1, "}");
    }
    triggers(&mut out, 1, &p.triggers);
    layout(&mut out, 1, &p.controls);
    print_base_actions(&mut out, 1, &p.actions, &p.action_groups);
    line(&mut out, 0, "}");
    out
}

/// BOS source of a page extension.
pub fn print_page_extension(e: &PageExtensionIR) -> String {
    let mut out = String::new();
    line(&mut out, 0, &format!("pageextension {} extends {} {{", e.name, e.target_page));
    if let Some(c) = &e.caption {
        line(&mut out, 1, &format!("caption = {};", quote(c)));
    }
    if e.is_customization {
        line(&mut out, 1, "isCustomization = true;");
    }
    if let Some(s) = &e.scope {
        line(&mut out, 1, &format!("scope = {};", quote(s)));
    }
    extra_properties(&mut out, 1, &e.properties);
    style_block(&mut out, 1, &e.style);
    variables(&mut out, 1, &e.variables);
    triggers(&mut out, 1, &e.triggers);
    layout(&mut out, 1, &e.controls);
    print_extension_actions(&mut out, 1, e);
    line(&mut out, 0, "}");
    out
}

#[cfg(test)]
#[path = "page_print_tests.rs"]
mod tests;
