//! Page extension merging (NAV/BC `pageextension` semantics).
//!
//! Extensions are applied in a deterministic order (see [`sort_extensions_for_merge`]). Every directive
//! is anchor based; a directive whose anchor does not exist is **skipped** and reported as a
//! [`MergeOrphan`] instead of being silently appended somewhere else. The resolver turns orphans into
//! BOS3007 diagnostics: errors for developer extensions, warnings for the super-user customization layer
//! (so a base upgrade that removes a field never breaks a customized page).

use bosc_ir::{ActionArea, ActionGroupIR, PageActionIR, PageControlIR, PageControlKind, PageExtensionIR, PageIR, TriggerIR};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOrphan {
    pub extension: String,
    /// Directive kind (`addAfter`, `modify`, ... or `action:modify`).
    pub directive: String,
    pub anchor: String,
    pub customization: bool,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeOutcome {
    pub page: PageIR,
    /// Extension names in the order they were applied.
    pub applied: Vec<String>,
    pub orphans: Vec<MergeOrphan>,
}

/// Merges a base page with extensions (in the given order). Orphaned directives are skipped.
pub fn merge_page_with_extensions(base: &PageIR, extensions: &[PageExtensionIR]) -> PageIR {
    merge_page_detailed(base, extensions).page
}

/// Deterministic merge order: developer extensions by package rank then name, the customization
/// layer always last.
pub fn sort_extensions_for_merge<F>(extensions: &mut [PageExtensionIR], package_rank: F)
where
    F: Fn(&PageExtensionIR) -> usize,
{
    extensions.sort_by(|a, b| {
        (a.is_customization, package_rank(a), a.name.to_ascii_lowercase()).cmp(&(
            b.is_customization,
            package_rank(b),
            b.name.to_ascii_lowercase(),
        ))
    });
}

pub fn merge_page_detailed(base: &PageIR, extensions: &[PageExtensionIR]) -> MergeOutcome {
    let mut page = base.clone();
    let mut applied = Vec::new();
    let mut orphans = Vec::new();

    for ext in extensions {
        let mut orphan = |directive: &str, anchor: &str, reason: &str| {
            orphans.push(MergeOrphan {
                extension: ext.name.clone(),
                directive: directive.to_string(),
                anchor: anchor.to_string(),
                customization: ext.is_customization,
                reason: reason.to_string(),
            })
        };

        if let Some(caption) = &ext.caption {
            page.caption = Some(caption.clone());
        }
        if let Some(style) = &ext.style {
            page.style.get_or_insert_with(Default::default).overlay(style);
        }
        for var in &ext.variables {
            if !page.variables.iter().any(|v| v.name.eq_ignore_ascii_case(&var.name)) {
                page.variables.push(var.clone());
            }
        }
        for (k, v) in &ext.properties {
            page.properties.insert(k.clone(), v.clone());
        }

        for delta in &ext.controls {
            if let Err(reason) = apply_control_delta(&mut page.controls, delta) {
                orphan(&delta.kind, &delta.name, &reason);
            }
        }

        for group in &ext.action_groups {
            merge_action_group(&mut page.action_groups, group);
        }
        for delta in &ext.actions {
            if let Err(reason) = apply_action_delta(&mut page.actions, &page.action_groups, delta) {
                let directive = format!("action:{}", delta.placement.as_deref().unwrap_or("override"));
                let anchor = delta.anchor.clone().unwrap_or_else(|| delta.name.clone());
                orphan(&directive, &anchor, &reason);
            }
        }

        for trigger in &ext.triggers {
            apply_trigger_delta(&mut page.triggers, trigger);
        }
        applied.push(ext.name.clone());
    }

    MergeOutcome { page, applied, orphans }
}

// ----------------------------------------------------
// CONTROL TREE HELPERS
// ----------------------------------------------------

fn find_mut<'a>(controls: &'a mut [PageControlIR], name: &str) -> Option<&'a mut PageControlIR> {
    for ctrl in controls.iter_mut() {
        if ctrl.name.eq_ignore_ascii_case(name) {
            return Some(ctrl);
        }
        if let Some(found) = find_mut(&mut ctrl.children, name) {
            return Some(found);
        }
    }
    None
}

fn contains(controls: &[PageControlIR], name: &str) -> bool {
    controls.iter().any(|c| c.name.eq_ignore_ascii_case(name) || contains(&c.children, name))
}

/// Removes and returns the named control from anywhere in the tree.
fn take(controls: &mut Vec<PageControlIR>, name: &str) -> Option<PageControlIR> {
    if let Some(pos) = controls.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
        return Some(controls.remove(pos));
    }
    controls.iter_mut().find_map(|c| take(&mut c.children, name))
}

/// Inserts `items` next to the sibling named `anchor` (anywhere in the tree).
fn insert_adjacent(controls: &mut Vec<PageControlIR>, anchor: &str, items: Vec<PageControlIR>, before: bool) -> Result<(), Vec<PageControlIR>> {
    if let Some(pos) = controls.iter().position(|c| c.name.eq_ignore_ascii_case(anchor)) {
        let at = if before { pos } else { pos + 1 };
        for (i, item) in items.into_iter().enumerate() {
            controls.insert(at + i, item);
        }
        return Ok(());
    }
    let mut items = items;
    for ctrl in controls.iter_mut() {
        match insert_adjacent(&mut ctrl.children, anchor, items, before) {
            Ok(()) => return Ok(()),
            Err(back) => items = back,
        }
    }
    Err(items)
}

fn insert_into(controls: &mut [PageControlIR], container: &str, items: Vec<PageControlIR>, first: bool) -> bool {
    let Some(target) = find_mut(controls, container) else {
        return false;
    };
    if first {
        for (i, item) in items.into_iter().enumerate() {
            target.children.insert(i, item);
        }
    } else {
        target.children.extend(items);
    }
    true
}

/// Names of the controls a move directive relocates: `moveAfter(anchor; ctrl)` or `moveAfter(anchor) { a; b; }`.
fn moved_names(delta: &PageControlIR) -> Vec<String> {
    delta.source_expr.iter().cloned().chain(delta.children.iter().map(|c| c.name.clone())).collect()
}

fn apply_control_delta(root: &mut Vec<PageControlIR>, delta: &PageControlIR) -> Result<(), String> {
    use PageControlKind as K;
    let target = delta.name.as_str();
    let missing = || format!("control '{target}' does not exist on the page");

    match delta.kind_typed() {
        K::AddFirst | K::AddLast => {
            if insert_into(root, target, delta.children.clone(), delta.kind_typed() == K::AddFirst) {
                Ok(())
            } else {
                Err(missing())
            }
        }
        K::AddBefore | K::AddAfter => {
            insert_adjacent(root, target, delta.children.clone(), delta.kind_typed() == K::AddBefore).map_err(|_| missing())
        }
        K::Modify => {
            let ctrl = find_mut(root, target).ok_or_else(missing)?;
            if let Some(cap) = &delta.caption {
                ctrl.caption = Some(cap.clone());
            }
            for (k, v) in &delta.properties {
                if k != "caption" && k != "label" {
                    ctrl.properties.insert(k.clone(), v.clone());
                }
            }
            if let Some(style) = &delta.style {
                ctrl.style.get_or_insert_with(Default::default).overlay(style);
            }
            if let Some(resp) = &delta.responsive {
                ctrl.responsive.get_or_insert_with(Default::default).overlay(resp);
            }
            for trg in &delta.triggers {
                apply_trigger_delta(&mut ctrl.triggers, trg);
            }
            Ok(())
        }
        K::Hide => {
            let ctrl = find_mut(root, target).ok_or_else(missing)?;
            ctrl.properties.insert("visible".to_string(), "false".to_string());
            Ok(())
        }
        K::Wrap => {
            // Wrapping with a single container child uses that container; otherwise a generic container.
            let mut wrapper = match delta.children.as_slice() {
                [only] if only.kind_typed().is_container() => only.clone(),
                _ => {
                    let mut w = PageControlIR::new("container", format!("{target}_wrapper"));
                    w.caption = delta.caption.clone();
                    w.properties = delta.properties.clone();
                    w.style = delta.style.clone();
                    w.responsive = delta.responsive.clone();
                    w.children = delta.children.clone();
                    w
                }
            };
            let mut build = Some(move |original: PageControlIR| {
                wrapper.children.insert(0, original);
                wrapper
            });
            if replace_in_place(root, target, &mut build) { Ok(()) } else { Err(missing()) }
        }
        K::Replace => {
            if splice_in_place(root, target, delta.children.clone()) { Ok(()) } else { Err(missing()) }
        }
        K::MoveBefore | K::MoveAfter | K::MoveFirst | K::MoveLast => {
            let names = moved_names(delta);
            if names.is_empty() {
                return Err("move directive lists no controls to move".to_string());
            }
            if !contains(root, target) {
                return Err(missing());
            }
            for name in &names {
                if !contains(root, name) {
                    return Err(format!("control '{name}' to move does not exist on the page"));
                }
                if name.eq_ignore_ascii_case(target) {
                    return Err(format!("control '{name}' cannot be moved relative to itself"));
                }
                if let Some(moved) = find_mut(root, name)
                    && contains(&moved.children, target)
                {
                    return Err(format!("control '{name}' contains the anchor '{target}'"));
                }
            }
            let items: Vec<PageControlIR> = names.iter().filter_map(|n| take(root, n)).collect();
            match delta.kind_typed() {
                K::MoveBefore | K::MoveAfter => {
                    insert_adjacent(root, target, items, delta.kind_typed() == K::MoveBefore)
                        .map_err(|_| missing())
                }
                _ => {
                    if insert_into(root, target, items, delta.kind_typed() == K::MoveFirst) {
                        Ok(())
                    } else {
                        Err(missing())
                    }
                }
            }
        }
        _ => {
            add_top_level(root, delta.clone());
            Ok(())
        }
    }
}

/// A non-directive control in an extension's `layout`: an `area` with an existing name merges its children
/// into that area; anything else is appended to the first `area(Content)` (or the root when there is none).
fn add_top_level(root: &mut Vec<PageControlIR>, ctrl: PageControlIR) {
    if ctrl.kind_typed() == PageControlKind::Area {
        if let Some(existing) = root
            .iter_mut()
            .find(|c| c.kind_typed() == PageControlKind::Area && c.name.eq_ignore_ascii_case(&ctrl.name))
        {
            existing.children.extend(ctrl.children);
            return;
        }
        root.push(ctrl);
        return;
    }
    let content = root
        .iter_mut()
        .find(|c| c.kind_typed() == PageControlKind::Area && c.name.eq_ignore_ascii_case("Content"));
    match content {
        Some(area) => area.children.push(ctrl),
        None => root.push(ctrl),
    }
}

/// Replaces the named control (at any depth) with `items`, keeping its position.
fn splice_in_place(controls: &mut Vec<PageControlIR>, name: &str, items: Vec<PageControlIR>) -> bool {
    if let Some(pos) = controls.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
        controls.splice(pos..=pos, items);
        return true;
    }
    let mut items = Some(items);
    for c in controls.iter_mut() {
        if contains(std::slice::from_ref(c), name) {
            return splice_in_place(&mut c.children, name, items.take().unwrap_or_default());
        }
    }
    false
}

/// Replaces the named control (at any depth) with `build(original)`, keeping its position.
fn replace_in_place<F>(controls: &mut Vec<PageControlIR>, name: &str, build: &mut Option<F>) -> bool
where
    F: FnOnce(PageControlIR) -> PageControlIR,
{
    if let Some(pos) = controls.iter().position(|c| c.name.eq_ignore_ascii_case(name)) {
        let Some(f) = build.take() else {
            return false;
        };
        let original = controls.remove(pos);
        controls.insert(pos, f(original));
        return true;
    }
    controls.iter_mut().any(|c| replace_in_place(&mut c.children, name, build))
}

// ----------------------------------------------------
// ACTIONS
// ----------------------------------------------------

fn merge_action_group(groups: &mut Vec<ActionGroupIR>, delta: &ActionGroupIR) {
    if let Some(existing) = groups.iter_mut().find(|g| g.name.eq_ignore_ascii_case(&delta.name)) {
        if delta.caption.is_some() {
            existing.caption = delta.caption.clone();
        }
        if delta.icon.is_some() {
            existing.icon = delta.icon.clone();
        }
        if delta.area.is_some() {
            existing.area = delta.area;
        }
        existing.visible = delta.visible;
    } else {
        groups.push(delta.clone());
    }
}

/// Copies every property set on `delta` onto `existing`.
fn overlay_action(existing: &mut PageActionIR, delta: &PageActionIR) {
    macro_rules! take_some {
        ($($field:ident),+) => { $( if delta.$field.is_some() { existing.$field = delta.$field.clone(); } )+ };
    }
    take_some!(
        caption, icon, shortcut, confirm, show_when, enable_when, run_codeunit, open_page, selection, script, tooltip,
        promoted_category, run_page_mode, target
    );
    if delta.primary {
        existing.primary = true;
    }
    if delta.promoted_only {
        existing.promoted_only = true;
    }
    if delta.is_default {
        existing.is_default = true;
    }
    if delta.in_footer_bar {
        existing.in_footer_bar = true;
    }
    if delta.ellipsis {
        existing.ellipsis = true;
    }
    if !delta.visible {
        existing.visible = false;
    }
    if let Some(style) = &delta.style {
        existing.style.get_or_insert_with(Default::default).overlay(style);
    }
    for (k, v) in &delta.run_page_link {
        existing.run_page_link.insert(k.clone(), v.clone());
    }
    for (k, v) in &delta.properties {
        existing.properties.insert(k.clone(), v.clone());
    }
}

fn find_action(actions: &[PageActionIR], name: &str) -> Option<usize> {
    actions.iter().position(|a| a.name.eq_ignore_ascii_case(name))
}

fn clean(delta: &PageActionIR) -> PageActionIR {
    let mut a = delta.clone();
    a.placement = None;
    a.anchor = None;
    a
}

fn apply_action_delta(actions: &mut Vec<PageActionIR>, groups: &[ActionGroupIR], delta: &PageActionIR) -> Result<(), String> {
    let anchor = delta.anchor.as_deref().unwrap_or(&delta.name);
    let missing_action = |n: &str| format!("action '{n}' does not exist on the page");

    match delta.placement.as_deref() {
        None => {
            match find_action(actions, &delta.name) {
                Some(i) => overlay_action(&mut actions[i], delta),
                None => actions.push(clean(delta)),
            }
            Ok(())
        }
        Some("modify") => {
            let i = find_action(actions, anchor).ok_or_else(|| missing_action(anchor))?;
            overlay_action(&mut actions[i], delta);
            Ok(())
        }
        Some("hide") => {
            let i = find_action(actions, anchor).ok_or_else(|| missing_action(anchor))?;
            actions[i].visible = false;
            Ok(())
        }
        Some(kind @ ("addBefore" | "addAfter")) => {
            let i = find_action(actions, anchor).ok_or_else(|| missing_action(anchor))?;
            let mut a = clean(delta);
            if a.area.is_none() && a.group.is_none() {
                a.area = actions[i].area;
                a.group = actions[i].group.clone();
            }
            actions.insert(if kind == "addBefore" { i } else { i + 1 }, a);
            Ok(())
        }
        Some(kind @ ("addFirst" | "addLast")) => {
            let is_group = groups.iter().any(|g| g.name.eq_ignore_ascii_case(anchor));
            let area = ActionArea::parse(anchor);
            if !is_group && area.is_none() {
                return Err(format!("action group or area '{anchor}' does not exist on the page"));
            }
            let mut a = clean(delta);
            if is_group {
                a.group = Some(anchor.to_string());
                a.area = groups.iter().find(|g| g.name.eq_ignore_ascii_case(anchor)).and_then(|g| g.area).or(a.area);
            } else {
                a.area = area;
                a.group = None;
            }
            let same_place = |x: &PageActionIR| {
                if is_group {
                    x.group.as_deref().is_some_and(|g| g.eq_ignore_ascii_case(anchor))
                } else {
                    x.group.is_none() && x.area == area
                }
            };
            let pos = if kind == "addFirst" {
                actions.iter().position(same_place).unwrap_or(actions.len())
            } else {
                actions.iter().rposition(same_place).map(|p| p + 1).unwrap_or(actions.len())
            };
            actions.insert(pos, a);
            Ok(())
        }
        Some(kind @ ("moveBefore" | "moveAfter")) => {
            if delta.name.eq_ignore_ascii_case(anchor) {
                return Err(format!("action '{anchor}' cannot be moved relative to itself"));
            }
            find_action(actions, anchor).ok_or_else(|| missing_action(anchor))?;
            let from = find_action(actions, &delta.name).ok_or_else(|| missing_action(&delta.name))?;
            let mut moved = actions.remove(from);
            let to = find_action(actions, anchor).ok_or_else(|| missing_action(anchor))?;
            moved.area = actions[to].area;
            moved.group = actions[to].group.clone();
            actions.insert(if kind == "moveBefore" { to } else { to + 1 }, moved);
            Ok(())
        }
        Some(other) => Err(format!("unknown action directive '{other}'")),
    }
}

fn apply_trigger_delta(triggers: &mut Vec<TriggerIR>, delta: &TriggerIR) {
    if let Some(existing) = triggers.iter_mut().find(|t| t.name.eq_ignore_ascii_case(&delta.name)) {
        *existing = delta.clone();
    } else {
        triggers.push(delta.clone());
    }
}
