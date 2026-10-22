//! Tests of the page designer source outline, span edits and customization printer.

use crate::page_source::*;
use crate::SemanticAnalyzer;
use bosc_syntax::{Lexer, Parser};

const SRC: &str = r#"table Customer {
    no: String(20) { notNull = true; };
    name: String(100);
    email: String(80);
    blocked: Boolean;
    primaryKey(no);
}

// Customer card used by the sales team.
page CustomerCard for Customer {
    pageType = Card;
    caption = "Customer";
    layout {
        area(Content) {
            group(General) {
                field(no);
                // the display name
                field(name) { importance = Promoted; }
                field(email);
            }
            group(Status) {
                field(blocked);
            }
        }
    }
    actions {
        area(Processing) {
            action(Block) { caption = "Block"; }
        }
    }
}
"#;

fn lower(src: &str) -> bosc_ir::ModelIR {
    let tokens = Lexer::new(src).tokenize().unwrap();
    let program = Parser::with_source(tokens, src.to_string()).parse_program().unwrap();
    SemanticAnalyzer::new("test").analyze_and_lower(&program).unwrap()
}

fn field_order(src: &str) -> Vec<String> {
    fn walk(c: &[bosc_ir::PageControlIR], out: &mut Vec<String>) {
        for x in c {
            if x.kind == "field" {
                out.push(x.name.clone());
            }
            walk(&x.children, out);
        }
    }
    let m = lower(src);
    let mut out = Vec::new();
    walk(&m.pages[0].controls, &mut out);
    out
}

fn set(path: &str, name: &str, value: &str, block: Option<&str>) -> PageEditOp {
    PageEditOp::SetProperty { path: path.into(), name: name.into(), value: value.into(), block: block.map(str::to_string) }
}

#[test]
fn outline_uses_compiled_node_paths() {
    let o = outline(SRC).unwrap();
    let page = o.object("CustomerCard").unwrap();
    assert_eq!(page.kind, "page");
    assert_eq!(page.target.as_deref(), Some("Customer"));
    assert!(page.props.iter().any(|p| p.name == "caption" && p.value == "\"Customer\""));
    let name = page.find("area:Content/group:General/field:name").expect("field path");
    assert_eq!(name.props[0].name, "importance");
    assert_eq!(&SRC[name.start..name.end], "field(name) { importance = Promoted; }");
    assert!(page.find("actions/area:Processing/action:Block").is_some());
}

#[test]
fn set_property_creates_a_body_and_keeps_comments() {
    let email = "area:Content/group:General/field:email";
    let r = apply_edits(SRC, "CustomerCard", &[set(email, "caption", "\"E-mail\"", None)]).unwrap();
    assert!(r.source.contains("field(email) { caption = \"E-mail\"; }"), "{}", r.source);
    assert!(r.source.contains("// the display name"));
    assert!(r.source.contains("// Customer card used by the sales team."));
    let r2 = apply_edits(&r.source, "CustomerCard", &[set(email, "caption", "\"Mail\"", None)]).unwrap();
    assert!(r2.source.contains("field(email) { caption = \"Mail\"; }"));
    assert_eq!(r2.source.matches("caption = ").count(), 3);
}

#[test]
fn style_blocks_are_created_updated_and_removed() {
    let path = "area:Content/group:General/field:name";
    let r = apply_edits(
        SRC,
        "CustomerCard",
        &[
            set(path, "background", "attention", Some("style")),
            set(path, "icon", "\"user\"", Some("style")),
            set(path, "background", "\"#1e40af\"", Some("style")),
        ],
    )
    .unwrap();
    assert!(r.source.contains("style { background = \"#1e40af\"; icon = \"user\"; }"), "{}", r.source);
    let m = lower(&r.source);
    let general = &m.pages[0].controls[0].children[0];
    let name = general.children.iter().find(|c| c.name == "name").unwrap();
    assert_eq!(name.style.as_ref().unwrap().icon.as_deref(), Some("user"));
    let r2 = apply_edits(
        &r.source,
        "CustomerCard",
        &[
            PageEditOp::RemoveProperty { path: path.into(), name: "icon".into(), block: Some("style".into()) },
            PageEditOp::RemoveProperty { path: path.into(), name: "background".into(), block: Some("style".into()) },
        ],
    )
    .unwrap();
    assert!(!r2.source.contains("style"), "{}", r2.source);
    assert!(r2.source.contains("field(name) { importance = Promoted; }"), "{}", r2.source);
}

#[test]
fn move_insert_and_delete_reorder_the_page() {
    let r = apply_edits(
        SRC,
        "CustomerCard",
        &[PageEditOp::Move {
            path: "area:Content/group:General/field:email".into(),
            parent: "area:Content/group:Status".into(),
            before: Some("area:Content/group:Status/field:blocked".into()),
        }],
    )
    .unwrap();
    assert_eq!(field_order(&r.source), vec!["no", "name", "email", "blocked"]);
    assert_eq!(r.selected.as_deref(), Some("area:Content/group:Status/field:email"));
    assert!(r.source.contains("            group(Status) {\n                field(email);\n                field(blocked);"), "{}", r.source);

    let r2 = apply_edits(
        &r.source,
        "CustomerCard",
        &[PageEditOp::Insert {
            parent: "area:Content/group:General".into(),
            before: Some("area:Content/group:General/field:no".into()),
            source: "field(blocked) { caption = \"On hold\"; }".into(),
        }],
    )
    .unwrap();
    assert!(r2.source.contains("                field(blocked) { caption = \"On hold\"; }\n                field(no);"), "{}", r2.source);

    let r3 = apply_edits(&r2.source, "CustomerCard", &[PageEditOp::Delete { path: "area:Content/group:Status/field:blocked".into() }]).unwrap();
    assert_eq!(field_order(&r3.source), vec!["blocked", "no", "name", "email"]);
    assert!(r3.source.contains("// the display name"));

    // appending into an empty-bodied container creates a properly indented body
    let r4 = apply_edits(
        SRC,
        "CustomerCard",
        &[PageEditOp::Insert { parent: "area:Content".into(), before: None, source: "group(Notes) { field(email); }".into() }],
    )
    .unwrap();
    assert!(r4.source.contains("            group(Notes) { field(email); }\n        }"), "{}", r4.source);
}

#[test]
fn inserting_into_a_page_without_layout_creates_the_section() {
    let src = "page Empty for Customer {\n    pageType = Card;\n}\n";
    let r = apply_edits(
        src,
        "Empty",
        &[PageEditOp::Insert { parent: "layout".into(), before: None, source: "area(Content) {\n    field(no);\n}".into() }],
    )
    .unwrap();
    assert_eq!(r.source, "page Empty for Customer {\n    pageType = Card;\n    layout {\n        area(Content) {\n            field(no);\n        }\n    }\n}\n");
    assert_eq!(r.selected.as_deref(), Some("area:Content"));
    let one_line = apply_edits("page E for Customer { pageType = Card; }", "E", &[PageEditOp::Insert { parent: "actions".into(), before: None, source: "action(Go) { }".into() }]).unwrap();
    assert!(lower(&format!("table Customer {{ no: String(20); primaryKey(no); }}\n{}", one_line.source)).pages[0].actions.iter().any(|a| a.name == "Go"), "{}", one_line.source);
}

#[test]
fn edits_that_break_the_source_are_rejected() {
    assert!(apply_edits(SRC, "CustomerCard", &[set("page", "caption", "\"x\"; table", None)]).is_err());
    let missing = apply_edits(SRC, "CustomerCard", &[PageEditOp::Delete { path: "area:Content/field:nope".into() }]);
    assert!(missing.unwrap_err().contains("does not exist"));
    let into_self = apply_edits(
        SRC,
        "CustomerCard",
        &[PageEditOp::Move { path: "area:Content/group:General".into(), parent: "area:Content/group:General".into(), before: None }],
    );
    assert!(into_self.is_err());
    assert!(apply_edits(SRC, "Nope", &[]).is_err());
}

#[test]
fn page_level_properties_are_edited_in_place() {
    let r = apply_edits(SRC, "CustomerCard", &[set("page", "caption", "\"Client\"", None), set("page", "radius", "lg", Some("style"))]).unwrap();
    assert!(r.source.contains("    caption = \"Client\";\n"), "{}", r.source);
    let m = lower(&r.source);
    assert_eq!(m.pages[0].caption.as_deref(), Some("Client"));
    assert!(m.pages[0].style.is_some());
}

#[test]
fn customization_prints_parses_and_round_trips() {
    let changes = CustomizationChanges {
        page: PageChange { caption: Some("Client \"VIP\"".into()), style: None },
        controls: vec![ControlChange {
            name: "email".into(),
            caption: Some("Mail".into()),
            visible: Some(false),
            importance: Some(bosc_ir::Importance::Additional),
            style: Some(bosc_ir::PageStyleIR {
                background: Some(bosc_ir::ColorValue::parse("#ff0000").unwrap()),
                icon: Some("mail".into()),
                ..Default::default()
            }),
            ..Default::default()
        }],
        moves: vec![MoveChange { name: "email".into(), anchor: "no".into(), position: Placement::After }],
        adds: vec![AddChange { field: "blocked".into(), anchor: "General".into(), position: Placement::Last, caption: Some("On hold".into()) }],
        actions: vec![ActionChange { name: "Block".into(), visible: Some(false), caption: None }],
    };
    let text = print_customization("CustomerCard", &changes).unwrap();
    let m = lower(&format!("{SRC}\n{text}"));
    let ext = m.page_extensions.iter().find(|e| e.name == "CustomerCard_Customization").expect("extension");
    let (clean, diags) = crate::page_resolver::enforce_customization_whitelist(ext);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(clean.controls.len(), ext.controls.len());
    let back = changes_from_extension(ext);
    assert_eq!(back.page.caption.as_deref(), Some("Client \"VIP\""));
    assert_eq!(back.controls[0].caption.as_deref(), Some("Mail"));
    assert_eq!(back.controls[0].visible, Some(false));
    assert_eq!(back.controls[0].importance, Some(bosc_ir::Importance::Additional));
    assert_eq!(back.controls[0].style.as_ref().unwrap().icon.as_deref(), Some("mail"));
    assert_eq!(back.moves, changes.moves);
    assert_eq!(back.adds, changes.adds);
    assert_eq!(back.actions[0].visible, Some(false));
    assert!(print_customization("Bad Name", &changes).is_err());
}
