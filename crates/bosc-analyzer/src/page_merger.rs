use bosc_ir::{PageActionIR, PageControlIR, PageExtensionIR, PageIR, TriggerIR};

/// Merges a base `PageIR` with one or more `PageExtensionIR`s into a unified resolved `PageIR`.
pub fn merge_page_with_extensions(base: &PageIR, extensions: &[PageExtensionIR]) -> PageIR {
    let mut resolved = base.clone();

    for ext in extensions {
        for control in &ext.controls {
            apply_control_delta(&mut resolved.controls, control);
        }

        for action in &ext.actions {
            apply_action_delta(&mut resolved.actions, action);
        }

        for trigger in &ext.triggers {
            apply_trigger_delta(&mut resolved.triggers, trigger);
        }
    }

    resolved
}

fn apply_control_delta(root_controls: &mut Vec<PageControlIR>, delta: &PageControlIR) {
    let kind = delta.kind.as_str();
    let target = &delta.name;

    match kind {
        "addFirst" => {
            if !insert_in_container(root_controls, target, &delta.children, true) {
                // If container not found, append to root
                for child in &delta.children {
                    root_controls.insert(0, child.clone());
                }
            }
        }
        "addLast" => {
            if !insert_in_container(root_controls, target, &delta.children, false) {
                for child in &delta.children {
                    root_controls.push(child.clone());
                }
            }
        }
        "addBefore" => {
            if !insert_adjacent_to_control(root_controls, target, &delta.children, true) {
                for child in &delta.children {
                    root_controls.push(child.clone());
                }
            }
        }
        "addAfter" => {
            if !insert_adjacent_to_control(root_controls, target, &delta.children, false) {
                for child in &delta.children {
                    root_controls.push(child.clone());
                }
            }
        }
        "modify" => {
            modify_control_properties(root_controls, target, delta);
        }
        "hide" => {
            hide_control(root_controls, target);
        }
        "wrap" => {
            wrap_control(root_controls, target, delta);
        }
        _ => {
            // Standalone container or control added at root
            root_controls.push(delta.clone());
        }
    }
}

fn insert_in_container(
    controls: &mut [PageControlIR],
    target_container: &str,
    new_items: &[PageControlIR],
    at_start: bool,
) -> bool {
    for ctrl in controls.iter_mut() {
        if ctrl.name.eq_ignore_ascii_case(target_container) {
            if at_start {
                for item in new_items.iter().rev() {
                    ctrl.children.insert(0, item.clone());
                }
            } else {
                for item in new_items {
                    ctrl.children.push(item.clone());
                }
            }
            return true;
        }
        if insert_in_container(&mut ctrl.children, target_container, new_items, at_start) {
            return true;
        }
    }
    false
}

fn insert_adjacent_to_control(
    controls: &mut Vec<PageControlIR>,
    target_control: &str,
    new_items: &[PageControlIR],
    before: bool,
) -> bool {
    if let Some(pos) = controls
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(target_control))
    {
        let insert_idx = if before { pos } else { pos + 1 };
        for (i, item) in new_items.iter().enumerate() {
            controls.insert(insert_idx + i, item.clone());
        }
        return true;
    }

    for ctrl in controls.iter_mut() {
        if insert_adjacent_to_control(&mut ctrl.children, target_control, new_items, before) {
            return true;
        }
    }
    false
}

fn modify_control_properties(controls: &mut [PageControlIR], target: &str, delta: &PageControlIR) -> bool {
    for ctrl in controls.iter_mut() {
        if ctrl.name.eq_ignore_ascii_case(target) {
            if let Some(ref cap) = delta.caption {
                ctrl.caption = Some(cap.clone());
            }
            for (k, v) in &delta.properties {
                ctrl.properties.insert(k.clone(), v.clone());
            }
            return true;
        }
        if modify_control_properties(&mut ctrl.children, target, delta) {
            return true;
        }
    }
    false
}

fn hide_control(controls: &mut [PageControlIR], target: &str) -> bool {
    for ctrl in controls.iter_mut() {
        if ctrl.name.eq_ignore_ascii_case(target) {
            ctrl.properties.insert("visible".to_string(), "false".to_string());
            return true;
        }
        if hide_control(&mut ctrl.children, target) {
            return true;
        }
    }
    false
}

fn wrap_control(controls: &mut Vec<PageControlIR>, target: &str, delta: &PageControlIR) -> bool {
    if let Some(pos) = controls
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(target))
    {
        let original = controls.remove(pos);
        let mut wrapper = delta.clone();
        // The wrapper's kind becomes "group" or the kind of the delta's first child / container
        wrapper.kind = "container".to_string();
        wrapper.name = format!("{}_wrapper", target);
        wrapper.children.insert(0, original);
        controls.insert(pos, wrapper);
        return true;
    }

    for ctrl in controls.iter_mut() {
        if wrap_control(&mut ctrl.children, target, delta) {
            return true;
        }
    }
    false
}

fn apply_action_delta(actions: &mut Vec<PageActionIR>, delta: &PageActionIR) {
    if let Some(existing) = actions
        .iter_mut()
        .find(|a| a.name.eq_ignore_ascii_case(&delta.name))
    {
        if let Some(ref cap) = delta.caption {
            existing.caption = Some(cap.clone());
        }
        if let Some(ref icon) = delta.icon {
            existing.icon = Some(icon.clone());
        }
        if let Some(ref shortcut) = delta.shortcut {
            existing.shortcut = Some(shortcut.clone());
        }
        if let Some(ref show) = delta.show_when {
            existing.show_when = Some(show.clone());
        }
        if let Some(ref en) = delta.enable_when {
            existing.enable_when = Some(en.clone());
        }
        if let Some(ref cu) = delta.run_codeunit {
            existing.run_codeunit = Some(cu.clone());
        }
        if let Some(ref sc) = delta.script {
            existing.script = Some(sc.clone());
        }
    } else {
        actions.push(delta.clone());
    }
}

fn apply_trigger_delta(triggers: &mut Vec<TriggerIR>, delta: &TriggerIR) {
    if let Some(existing) = triggers
        .iter_mut()
        .find(|t| t.name.eq_ignore_ascii_case(&delta.name))
    {
        *existing = delta.clone();
    } else {
        triggers.push(delta.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_field(name: &str) -> PageControlIR {
        PageControlIR {
            kind: "field".to_string(),
            name: name.to_string(),
            source_expr: Some(name.to_string()),
            caption: Some(name.to_string()),
            properties: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    fn make_container(kind: &str, name: &str, children: Vec<PageControlIR>) -> PageControlIR {
        PageControlIR {
            kind: kind.to_string(),
            name: name.to_string(),
            source_expr: None,
            caption: Some(name.to_string()),
            properties: BTreeMap::new(),
            children,
        }
    }

    #[test]
    fn test_merge_page_with_add_after_and_modify() {
        let base_page = PageIR {
            name: "CustomerCard".to_string(),
            caption: Some("Customer Details".to_string()),
            page_type: "Card".to_string(),
            source_table: Some("Customer".to_string()),
            source_kind: None,
            editable: Some(true),
            insert_allowed: Some(true),
            modify_allowed: Some(true),
            delete_allowed: Some(true),
            filters: Vec::new(),
            views: Vec::new(),
            controls: vec![make_container(
                "group",
                "General",
                vec![make_field("code"), make_field("name"), make_field("creditLimit")],
            )],
            actions: Vec::new(),
            triggers: Vec::new(),
        };

        let mut mod_props = BTreeMap::new();
        mod_props.insert("readonly".to_string(), "true".to_string());
        mod_props.insert("caption".to_string(), "Client Limit".to_string());

        let ext = PageExtensionIR {
            name: "CustomerLoyaltyExt".to_string(),
            target_page: "CustomerCard".to_string(),
            controls: vec![
                PageControlIR {
                    kind: "addAfter".to_string(),
                    name: "name".to_string(),
                    source_expr: None,
                    caption: None,
                    properties: BTreeMap::new(),
                    children: vec![make_field("loyaltyTier")],
                },
                PageControlIR {
                    kind: "modify".to_string(),
                    name: "creditLimit".to_string(),
                    source_expr: None,
                    caption: Some("Client Limit".to_string()),
                    properties: mod_props,
                    children: Vec::new(),
                },
            ],
            actions: Vec::new(),
            triggers: Vec::new(),
        };

        let merged = merge_page_with_extensions(&base_page, &[ext]);
        let general_group = &merged.controls[0];
        assert_eq!(general_group.children.len(), 4);
        assert_eq!(general_group.children[0].name, "code");
        assert_eq!(general_group.children[1].name, "name");
        assert_eq!(general_group.children[2].name, "loyaltyTier"); // Injected via addAfter!
        assert_eq!(general_group.children[3].name, "creditLimit");

        // Verify modify
        assert_eq!(
            general_group.children[3].properties.get("readonly").map(|s| s.as_str()),
            Some("true")
        );
        assert_eq!(general_group.children[3].caption.as_deref(), Some("Client Limit"));
    }

    #[test]
    fn test_merge_page_with_add_before_hide_and_wrap() {
        let base_page = PageIR {
            name: "OrderCard".to_string(),
            caption: Some("Order Details".to_string()),
            page_type: "Card".to_string(),
            source_table: Some("SalesHeader".to_string()),
            source_kind: None,
            editable: Some(true),
            insert_allowed: Some(true),
            modify_allowed: Some(true),
            delete_allowed: Some(true),
            filters: Vec::new(),
            views: Vec::new(),
            controls: vec![make_container(
                "group",
                "General",
                vec![make_field("orderNo"), make_field("orderDate"), make_field("legacyField")],
            )],
            actions: Vec::new(),
            triggers: Vec::new(),
        };

        let ext = PageExtensionIR {
            name: "OrderAuditExt".to_string(),
            target_page: "OrderCard".to_string(),
            controls: vec![
                PageControlIR {
                    kind: "addBefore".to_string(),
                    name: "orderDate".to_string(),
                    source_expr: None,
                    caption: None,
                    properties: BTreeMap::new(),
                    children: vec![make_field("postingDate")],
                },
                PageControlIR {
                    kind: "hide".to_string(),
                    name: "legacyField".to_string(),
                    source_expr: None,
                    caption: None,
                    properties: BTreeMap::new(),
                    children: Vec::new(),
                },
                PageControlIR {
                    kind: "wrap".to_string(),
                    name: "orderNo".to_string(),
                    source_expr: None,
                    caption: None,
                    properties: BTreeMap::new(),
                    children: Vec::new(),
                },
            ],
            actions: Vec::new(),
            triggers: Vec::new(),
        };

        let merged = merge_page_with_extensions(&base_page, &[ext]);
        let general_group = &merged.controls[0];

        // orderNo should now be wrapped in orderNo_wrapper
        assert_eq!(general_group.children[0].name, "orderNo_wrapper");
        assert_eq!(general_group.children[0].children[0].name, "orderNo");

        // postingDate inserted before orderDate
        assert_eq!(general_group.children[1].name, "postingDate");
        assert_eq!(general_group.children[2].name, "orderDate");

        // legacyField has visible: false
        assert_eq!(
            general_group.children[3].properties.get("visible").map(|s| s.as_str()),
            Some("false")
        );
    }
}
