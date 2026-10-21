//! Lowering of `page` / `pageextension` AST into the authored page IR (v2).
//!
//! Lowering is total: it never fails on bad property *values*. Invalid styles, responsive settings and
//! expressions are reported with source spans by [`crate::page_check`]; here they are simply not
//! carried into typed slots.

use bosc_ir::*;
use bosc_syntax::ast::*;
use std::collections::BTreeMap;

use crate::page_expr;
use crate::{format_expr, lower_statements, property_text, strip_typescript};

/// Properties that hold client expressions, with their canonical IR key.
fn expression_key(name: &str) -> Option<&'static str> {
    match name {
        "showWhen" | "visibleWhen" => Some("showWhen"),
        "enableWhen" | "enabledWhen" => Some("enableWhen"),
        "editableWhen" => Some("editableWhen"),
        "styleExpr" | "styleWhen" => Some("styleExpr"),
        _ => None,
    }
}

/// Text form of a property value: strings and identifiers verbatim, other expressions formatted.
pub(crate) fn value_text(value: &Expr) -> String {
    match value {
        Expr::IntLit(n) => n.to_string(),
        Expr::BoolLit(b) => b.to_string(),
        other => property_text(other).unwrap_or_else(|| format_expr(other)),
    }
}

/// Canonical expression text, or the raw text when the expression is invalid (re-validated later).
fn expression_text(value: &Expr) -> String {
    page_expr::canonical_from_property(value).unwrap_or_else(|_| match value {
        Expr::StringLit(s) => s.clone(),
        other => format_expr(other),
    })
}

fn bool_value(value: &Expr) -> Option<bool> {
    match value {
        Expr::BoolLit(b) => Some(*b),
        Expr::Ident(s) | Expr::StringLit(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "yes" => Some(true),
            "false" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn int_value(value: &Expr) -> Option<i64> {
    match value {
        Expr::IntLit(n) => Some(*n),
        Expr::StringLit(s) | Expr::Ident(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn lower_triggers(triggers: &[TriggerDef]) -> Vec<TriggerIR> {
    triggers
        .iter()
        .map(|trg| TriggerIR {
            name: trg.name.clone(),
            params: trg.params.clone(),
            actions: lower_statements(&trg.body.statements),
            script: trg.body.raw_source.as_ref().map(|s| strip_typescript(s)),
        })
        .collect()
}

/// Lowers a `style { ... }` block. Unknown keys and invalid values are skipped (see `page_check`).
pub(crate) fn lower_style(props: &[Property]) -> Option<PageStyleIR> {
    let mut style = PageStyleIR::default();
    for p in props {
        let text = value_text(&p.value);
        match p.name.as_str() {
            "background" | "backgroundColor" | "bg" => style.background = ColorValue::parse(&text).ok(),
            "textColor" | "color" | "foreground" => style.text_color = ColorValue::parse(&text).ok(),
            "borderColor" | "border" => style.border_color = ColorValue::parse(&text).ok(),
            "borderWidth" => {
                style.border_width = int_value(&p.value)
                    .filter(|w| (0..=PageStyleIR::MAX_BORDER_WIDTH as i64).contains(w))
                    .map(|w| w as u8)
            }
            "radius" | "borderRadius" | "rounded" => style.radius = Radius::parse(&text),
            "shadow" | "elevation" => style.shadow = Shadow::parse(&text),
            "density" | "padding" => style.density = Density::parse(&text),
            "emphasis" | "style" => style.emphasis = Emphasis::parse(&text),
            "icon" | "image" => style.icon = normalize_icon_name(&text),
            _ => {}
        }
    }
    (!style.is_empty()).then_some(style)
}

/// Lowers a `responsive { ... }` block plus flat responsive properties found on the control.
pub(crate) fn lower_responsive(block: &[Property], flat: &[Property]) -> Option<ResponsiveIR> {
    let mut r = ResponsiveIR::default();
    for p in block.iter().chain(flat.iter()) {
        let text = value_text(&p.value);
        match p.name.as_str() {
            "hideBelow" => r.hide_below = Breakpoint::parse(&text),
            "importance" => r.importance = Importance::parse(&text),
            "colSpan" | "columnSpan" | "span" => {
                if let Some(n) = int_value(&p.value).filter(|n| (1..=BreakpointSpans::COLUMNS as i64).contains(n)) {
                    r.span.set(Breakpoint::Md, n as u8);
                }
            }
            name => {
                if let (Some(bp), Some(n)) = (
                    Breakpoint::parse(name),
                    int_value(&p.value).filter(|n| (1..=BreakpointSpans::COLUMNS as i64).contains(n)),
                ) {
                    r.span.set(bp, n as u8);
                }
            }
        }
    }
    (!r.is_empty()).then_some(r)
}

pub(crate) fn lower_variables(vars: &[PageVariableDef]) -> Vec<PageVariableIR> {
    vars.iter()
        .map(|v| {
            let mut caption = None;
            let mut default_value = None;
            for p in &v.properties {
                match p.name.as_str() {
                    "caption" | "label" => caption = property_text(&p.value),
                    "default" => default_value = Some(value_text(&p.value)),
                    _ => {}
                }
            }
            PageVariableIR { name: v.name.clone(), data_type: v.field_type.to_string(), caption, default_value }
        })
        .collect()
}

pub(crate) fn lower_page(p: &PageDef) -> PageIR {
    let mut page = PageIR {
        ir_version: PAGE_IR_VERSION,
        name: p.name.clone(),
        caption: None,
        page_type: PageType::parse(&p.page_type).map(|t| t.as_str().to_string()).unwrap_or_else(|| p.page_type.clone()),
        source_table: p.source_table.clone(),
        source_kind: Some("table".to_string()),
        editable: None,
        insert_allowed: None,
        modify_allowed: None,
        delete_allowed: None,
        filters: Vec::new(),
        views: lower_views(&p.views),
        controls: p.controls.iter().map(lower_control).collect(),
        actions: Vec::new(),
        triggers: lower_triggers(&p.triggers),
        card_page: None,
        data_caption_fields: Vec::new(),
        refresh_on_activate: false,
        delayed_insert: false,
        multiple_new_lines: false,
        auto_split_key: false,
        show_filter: None,
        usage_category: None,
        instructional_text: None,
        style: lower_style(&p.style),
        action_groups: Vec::new(),
        variables: lower_variables(&p.variables),
        properties: BTreeMap::new(),
    };

    for prop in &p.properties {
        let v = &prop.value;
        match prop.name.as_str() {
            "pageType" | "page_type" | "layout" | "sourceTable" | "source_table" | "source" => {}
            "caption" | "label" => page.caption = property_text(v),
            "sourceKind" => page.source_kind = property_text(v),
            "editable" => page.editable = bool_value(v),
            "insertAllowed" => page.insert_allowed = bool_value(v),
            "modifyAllowed" => page.modify_allowed = bool_value(v),
            "deleteAllowed" => page.delete_allowed = bool_value(v),
            "filters" => match v {
                Expr::ArrayLit(items) => {
                    page.filters.extend(items.iter().map(|i| property_text(i).unwrap_or_else(|| format_expr(i))))
                }
                other => page.filters.extend(property_text(other)),
            },
            "cardPage" | "cardPageId" | "cardPageID" => page.card_page = property_text(v),
            "dataCaptionFields" => {
                page.data_caption_fields = match v {
                    Expr::ArrayLit(items) => items.iter().filter_map(property_text).collect(),
                    other => value_text(other).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
                }
            }
            "refreshOnActivate" => page.refresh_on_activate = bool_value(v).unwrap_or(false),
            "delayedInsert" => page.delayed_insert = bool_value(v).unwrap_or(false),
            "multipleNewLines" => page.multiple_new_lines = bool_value(v).unwrap_or(false),
            "autoSplitKey" => page.auto_split_key = bool_value(v).unwrap_or(false),
            "showFilter" => page.show_filter = bool_value(v),
            "usageCategory" => page.usage_category = property_text(v),
            "instructionalText" => page.instructional_text = property_text(v),
            other => {
                page.properties.insert(other.to_string(), value_text(v));
            }
        }
    }

    let mut ctx = ActionLowering::default();
    ctx.lower(&p.actions, None, None, None);
    page.actions = ctx.actions;
    page.action_groups = ctx.groups;
    page
}

fn lower_views(views: &[PageViewDef]) -> Vec<PageViewIR> {
    views
        .iter()
        .map(|v| {
            let mut caption = None;
            let mut filter = None;
            let mut sort = None;
            let mut is_default = false;
            for prop in &v.properties {
                match prop.name.as_str() {
                    "caption" | "label" => caption = property_text(&prop.value),
                    "where" | "filter" => filter = Some(format_expr(&prop.value)),
                    "sort" | "orderBy" => sort = property_text(&prop.value).or_else(|| Some(format_expr(&prop.value))),
                    "default" => is_default = bool_value(&prop.value).unwrap_or(false),
                    _ => {}
                }
            }
            PageViewIR { name: v.name.clone(), caption, filter, sort, is_default }
        })
        .collect()
}

pub(crate) fn lower_page_extension(pe: &PageExtDef) -> PageExtensionIR {
    let mut ext = PageExtensionIR {
        ir_version: PAGE_IR_VERSION,
        name: pe.name.clone(),
        target_page: pe.target_page.clone(),
        controls: pe.controls.iter().map(lower_control).collect(),
        actions: Vec::new(),
        triggers: lower_triggers(&pe.triggers),
        caption: None,
        is_customization: false,
        scope: None,
        style: lower_style(&pe.style),
        action_groups: Vec::new(),
        variables: lower_variables(&pe.variables),
        properties: BTreeMap::new(),
    };
    for prop in &pe.properties {
        match prop.name.as_str() {
            "caption" | "label" => ext.caption = property_text(&prop.value),
            "isCustomization" | "customization" => ext.is_customization = bool_value(&prop.value).unwrap_or(false),
            "scope" => ext.scope = property_text(&prop.value),
            other => {
                ext.properties.insert(other.to_string(), value_text(&prop.value));
            }
        }
    }
    let mut ctx = ActionLowering::default();
    ctx.lower(&pe.actions, None, None, None);
    ext.actions = ctx.actions;
    ext.action_groups = ctx.groups;
    ext
}

/// Legacy `style = "password"` / `"switch"` string hints from the v1 designer.
fn apply_legacy_style_hint(hint: &str, props: &mut BTreeMap<String, String>, style: &mut Option<PageStyleIR>) -> bool {
    if let Some(emphasis) = Emphasis::parse(hint) {
        style.get_or_insert_with(PageStyleIR::default).emphasis = Some(emphasis);
        return true;
    }
    if let Some(ext) = ExtendedDatatype::parse(hint) {
        props.insert("extendedDatatype".into(), ext.as_str().into());
        return true;
    }
    if let Some(control) = FieldControl::parse(hint) {
        props.insert("control".into(), control.as_str().into());
        return true;
    }
    false
}

pub(crate) fn lower_control(c: &PageControlDef) -> PageControlIR {
    let mut ctrl = PageControlIR::new(c.kind.clone(), c.name.clone());
    ctrl.source_expr = c.source_expr.clone();
    ctrl.style = lower_style(&c.style);
    ctrl.triggers = lower_triggers(&c.triggers);

    let mut flat_responsive = Vec::new();
    for prop in &c.properties {
        let v = &prop.value;
        let name = prop.name.as_str();
        if let Some(key) = expression_key(name) {
            ctrl.properties.insert(key.to_string(), expression_text(v));
            continue;
        }
        if let Some(field) = name.strip_prefix("link.") {
            ctrl.properties.insert(format!("link.{field}"), expression_text(v));
            continue;
        }
        match name {
            "caption" | "label" => {
                if let Some(s) = property_text(v) {
                    ctrl.caption = Some(s);
                }
            }
            "importance" | "hideBelow" | "colSpan" | "columnSpan" | "span" => flat_responsive.push(prop.clone()),
            // NAV: `Visible/Editable/Enabled = <expression>` are dynamic; literals stay static.
            "visible" | "editable" | "enabled" if bool_value(v).is_none() => {
                let key = match name {
                    "visible" => "showWhen",
                    "editable" => "editableWhen",
                    _ => "enableWhen",
                };
                ctrl.properties.insert(key.to_string(), expression_text(v));
            }
            "readonly" | "readOnly" => {
                if bool_value(v).unwrap_or(true) {
                    ctrl.properties.insert("editable".into(), "false".into());
                }
            }
            "hidden" => {
                if bool_value(v).unwrap_or(true) {
                    ctrl.properties.insert("visible".into(), "false".into());
                }
            }
            "style" => {
                let hint = value_text(v);
                if !apply_legacy_style_hint(&hint, &mut ctrl.properties, &mut ctrl.style) {
                    ctrl.properties.insert("style".into(), hint);
                }
            }
            "icon" => {
                ctrl.style.get_or_insert_with(PageStyleIR::default).icon = normalize_icon_name(&value_text(v));
            }
            _ => {
                ctrl.properties.insert(name.to_string(), value_text(v));
            }
        }
    }
    ctrl.responsive = lower_responsive(&c.responsive, &flat_responsive);
    ctrl.children = c.children.iter().map(lower_control).collect();
    ctrl
}

#[derive(Default)]
struct ActionLowering {
    actions: Vec<PageActionIR>,
    groups: Vec<ActionGroupIR>,
    separator_pending: bool,
}

impl ActionLowering {
    fn lower(&mut self, defs: &[PageActionDef], area: Option<ActionArea>, group: Option<&str>, placement: Option<(&str, &str)>) {
        for def in defs {
            match def.kind.as_str() {
                "area" => {
                    let area = ActionArea::parse(&def.name).or(area);
                    self.lower(&def.children, area, None, placement);
                }
                "group" => {
                    let mut g = ActionGroupIR {
                        name: def.name.clone(),
                        area,
                        parent: group.map(str::to_string),
                        caption: None,
                        icon: lower_style(&def.style).and_then(|s| s.icon),
                        visible: true,
                    };
                    for p in &def.properties {
                        match p.name.as_str() {
                            "caption" | "label" => g.caption = property_text(&p.value),
                            "icon" | "image" => g.icon = normalize_icon_name(&value_text(&p.value)),
                            "visible" => g.visible = bool_value(&p.value).unwrap_or(true),
                            _ => {}
                        }
                    }
                    self.groups.push(g);
                    self.lower(&def.children, area, Some(&def.name), placement);
                }
                "separator" => self.separator_pending = true,
                "addFirst" | "addLast" | "addBefore" | "addAfter" => {
                    // The anchor of addFirst/addLast is a group or area; children inherit it as their location.
                    let (child_area, child_group) = match def.kind.as_str() {
                        "addFirst" | "addLast" => match ActionArea::parse(&def.name) {
                            Some(a) => (Some(a), None),
                            None => (area, Some(def.name.as_str())),
                        },
                        _ => (area, group),
                    };
                    self.lower(&def.children, child_area, child_group, Some((def.kind.as_str(), def.name.as_str())));
                }
                "modify" | "hide" => {
                    let mut a = self.lower_action(def, area, group);
                    a.placement = Some(def.kind.clone());
                    a.anchor = Some(def.name.clone());
                    self.actions.push(a);
                }
                "moveBefore" | "moveAfter" => {
                    // `moveAfter(Anchor; Moved)` or `moveAfter(Anchor) { Moved1; Moved2; }`.
                    let moved: Vec<String> = def
                        .target
                        .iter()
                        .cloned()
                        .chain(def.properties.iter().filter(|p| matches!(p.value, Expr::BoolLit(true))).map(|p| p.name.clone()))
                        .collect();
                    for name in moved {
                        let mut a = PageActionIR::new(name);
                        a.placement = Some(def.kind.clone());
                        a.anchor = Some(def.name.clone());
                        self.actions.push(a);
                    }
                }
                _ => {
                    // `action` / `actionref`. A legacy action with nested actions acts as a group.
                    let mut a = self.lower_action(def, area, group);
                    if let Some((kind, anchor)) = placement {
                        a.placement = Some(kind.to_string());
                        a.anchor = Some(anchor.to_string());
                    }
                    if std::mem::take(&mut self.separator_pending) {
                        a.properties.insert("separatorBefore".into(), "true".into());
                    }
                    let nested = !def.children.is_empty();
                    let group_area = a.area;
                    self.actions.push(a);
                    if nested {
                        self.groups.push(ActionGroupIR {
                            name: def.name.clone(),
                            area: group_area,
                            parent: group.map(str::to_string),
                            caption: None,
                            icon: None,
                            visible: true,
                        });
                        self.lower(&def.children, group_area, Some(&def.name), placement);
                    }
                }
            }
        }
    }

    fn lower_action(&self, def: &PageActionDef, area: Option<ActionArea>, group: Option<&str>) -> PageActionIR {
        let mut a = PageActionIR::new(def.name.clone());
        a.area = area;
        a.group = group.map(str::to_string);
        a.style = lower_style(&def.style);
        if def.kind == "actionref" {
            a.target = def.target.clone().or_else(|| Some(def.name.clone()));
            if a.area.is_none() {
                a.area = Some(ActionArea::Promoted);
            }
        }
        for prop in &def.properties {
            let v = &prop.value;
            let name = prop.name.as_str();
            if let Some(field) = name.strip_prefix("link.") {
                a.run_page_link.insert(field.to_string(), expression_text(v));
                continue;
            }
            match name {
                "caption" | "label" => a.caption = property_text(v),
                "icon" | "image" => a.icon = Some(value_text(v)),
                "primary" | "promoted" | "promotedIsBig" => a.primary = bool_value(v).unwrap_or(true),
                "promotedOnly" => a.promoted_only = bool_value(v).unwrap_or(true),
                "promotedCategory" => a.promoted_category = property_text(v),
                "shortcut" | "shortcutKey" => a.shortcut = property_text(v),
                "confirm" | "confirmationMessage" => a.confirm = property_text(v),
                "tooltip" | "toolTip" => a.tooltip = property_text(v),
                "isDefault" => a.is_default = bool_value(v).unwrap_or(true),
                "inFooterBar" => a.in_footer_bar = bool_value(v).unwrap_or(true),
                "ellipsis" => a.ellipsis = bool_value(v).unwrap_or(true),
                "visible" => match bool_value(v) {
                    Some(b) => a.visible = b,
                    None => a.show_when = Some(expression_text(v)),
                },
                "enabled" => match bool_value(v) {
                    Some(true) => {}
                    Some(false) => a.enable_when = Some("false".to_string()),
                    None => a.enable_when = Some(expression_text(v)),
                },
                "showWhen" | "visibleWhen" => a.show_when = Some(expression_text(v)),
                "enableWhen" | "enabledWhen" => a.enable_when = Some(expression_text(v)),
                "run" | "runObject" | "runCodeunit" => {
                    a.run_codeunit = property_text(v).or_else(|| Some(format_expr(v)));
                }
                "openPage" | "runPage" | "runPageId" => {
                    a.open_page = property_text(v).or_else(|| Some(format_expr(v)));
                }
                "runPageMode" | "openMode" => {
                    a.run_page_mode = PageOpenMode::parse(&value_text(v));
                    if a.run_page_mode.is_none() {
                        a.properties.insert(name.to_string(), value_text(v));
                    }
                }
                "selection" => a.selection = property_text(v),
                "area" => {
                    a.area = ActionArea::parse(&value_text(v));
                    if a.area.is_none() {
                        a.properties.insert("area".into(), value_text(v));
                    }
                }
                "group" => a.group = property_text(v),
                _ => {
                    a.properties.insert(name.to_string(), value_text(v));
                }
            }
        }
        a.script = def.on_action.as_ref().and_then(|b| b.raw_source.as_ref().map(|s| strip_typescript(s)));
        a
    }
}
