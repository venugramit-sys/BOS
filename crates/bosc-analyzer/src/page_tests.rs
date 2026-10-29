//! End-to-end tests for page lowering, merging, resolution and diagnostics.

use crate::page_merger::{merge_page_detailed, sort_extensions_for_merge};
use crate::page_resolver::{compile_page, prettify_caption, ModelCatalog};
use crate::SemanticAnalyzer;
use bosc_ir::*;
use bosc_syntax::{Lexer, Parser};

fn compile(src: &str) -> ModelIR {
    let tokens = Lexer::new(src).tokenize().expect("lex");
    let program = Parser::with_source(tokens, src.to_string()).parse_program().expect("parse");
    SemanticAnalyzer::new("test.pages").analyze_and_lower(&program).expect("lower")
}

fn page_codes(model: &ModelIR) -> Vec<(String, String)> {
    model
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("BOS30"))
        .map(|d| (d.code.clone(), d.severity.clone()))
        .collect()
}

fn compiled(model: &ModelIR, page: &str) -> CompiledPageIR {
    let base = model.pages.iter().find(|p| p.name == page).expect("page");
    let mut exts: Vec<PageExtensionIR> = model.page_extensions.iter().filter(|e| e.target_page == page).cloned().collect();
    sort_extensions_for_merge(&mut exts, |_| 0);
    compile_page(base, &exts, &ModelCatalog::new(vec![model])).page
}

fn find<'a>(controls: &'a [ResolvedControlIR], name: &str) -> &'a ResolvedControlIR {
    fn walk<'a>(cs: &'a [ResolvedControlIR], name: &str) -> Option<&'a ResolvedControlIR> {
        cs.iter().find_map(|c| if c.name == name { Some(c) } else { walk(&c.children, name) })
    }
    walk(controls, name).unwrap_or_else(|| panic!("control {name} not found"))
}

fn field<'a>(page: &'a CompiledPageIR, name: &str) -> &'a ResolvedFieldIR {
    find(&page.controls, name).field.as_ref().unwrap_or_else(|| panic!("{name} is not a resolved field"))
}

const SALES: &str = r##"
enum CustomerStatus { Open; OnHold; Blocked; }

table Country {
    code: String(10) { notNull = true; };
    name: String(50);
    primaryKey(code);
}

table CustomerLedgerEntry {
    id: UUID { default = auto(); };
    customerNo: String(20) { notNull = true; };
    amount: Decimal(18, 2);
    primaryKey(id);
}

table Customer {
    caption = "Customer";
    no: String(20) { notNull = true; };
    name: String(100) { caption = "Customer Name"; };
    email: String(80);
    phoneNo: String(30);
    website: String(200);
    notes: String(1000);
    creditLimit: Decimal(18, 2) { default = 0; };
    visitCount: Integer;
    blocked: Boolean { default = false; };
    createdOn: Date;
    lastSeenAt: DateTime;
    status: Enum(CustomerStatus) { default = "Open"; };
    countryCode: String(10) { relation = Country.code; };
    balance: Decimal(18, 2) {
        computed = sum(CustomerLedgerEntry.amount where { customerNo == no });
    };
    primaryKey(no);
}

page CountryList for Country {
    pageType = List;
    layout { area(Content) { repeater(Lines) { field(code); field(name); } } }
}

page CustomerLedgerEntries for CustomerLedgerEntry {
    pageType = List;
    layout { area(Content) { repeater(Lines) { field(customerNo); field(amount); } } }
}

page CustomerList for Customer {
    pageType = List;
    layout { area(Content) { repeater(Lines) { field(no); field(name); field(blocked); field(balance); } } }
}

page CustomerCard for Customer {
    pageType = Card;
    layout {
        area(Content) {
            group(General) {
                field(no);
                field(name);
                field(email);
                field(phoneNo);
                field(website);
                field(notes);
                field(creditLimit) { caption = "Credit Limit (INR)"; showWhen = rec.blocked == false; }
                field(visitCount);
                field(blocked);
                field(createdOn);
                field(lastSeenAt);
                field(status);
                field(countryCode);
                field(balance);
            }
        }
    }
    actions {
        area(Processing) {
            action(Ledger) { openPage = CustomerLedgerEntries; link { customerNo = rec.no; } }
        }
    }
}
"##;

#[test]
fn fields_are_auto_filled_from_the_table_nav_style() {
    let model = compile(SALES);
    let errors: Vec<_> = model.diagnostics.iter().filter(|d| d.severity == "error" && d.code.starts_with("BOS30")).collect();
    assert!(errors.is_empty(), "unexpected page errors: {errors:#?}");

    let card = compiled(&model, "CustomerCard");
    assert_eq!(card.page_type, PageType::Card);
    assert_eq!(card.caption, "Customer Card", "no page caption and table caption == table name -> prettified page name");

    let no = field(&card, "no");
    assert_eq!(find(&card.controls, "no").caption.as_deref(), Some("No"));
    assert!(no.required, "notNull without default is mandatory");
    assert!(no.is_primary_key);
    assert_eq!(no.max_length, Some(20));
    assert_eq!(no.control, FieldControl::Text);

    assert_eq!(find(&card.controls, "name").caption.as_deref(), Some("Customer Name"), "explicit table caption");
    assert_eq!(field(&card, "email").control, FieldControl::Email);
    assert_eq!(field(&card, "phoneNo").control, FieldControl::Phone);
    assert_eq!(find(&card.controls, "phoneNo").caption.as_deref(), Some("Phone No"));
    assert_eq!(field(&card, "website").control, FieldControl::Url);
    assert_eq!(field(&card, "notes").control, FieldControl::TextArea);

    let credit = field(&card, "creditLimit");
    assert_eq!(find(&card.controls, "creditLimit").caption.as_deref(), Some("Credit Limit (INR)"), "authored caption wins");
    assert!(!credit.auto_filled.contains(&"caption".to_string()));
    assert_eq!(credit.control, FieldControl::Number);
    assert_eq!(credit.decimals, Some((2, 2)));
    assert_eq!(credit.key, "credit_limit");
    assert!(!credit.required, "has a default");
    let show = find(&card.controls, "creditLimit").show_when.as_ref().expect("showWhen");
    assert_eq!(show.js, "fx.eq(rec[\"blocked\"], false)");
    assert_eq!(show.fields, vec!["blocked"]);

    assert_eq!(field(&card, "visitCount").decimals, Some((0, 0)));
    assert_eq!(field(&card, "blocked").control, FieldControl::Switch, "boolean on a card");
    assert_eq!(field(&card, "createdOn").control, FieldControl::Date);
    assert_eq!(field(&card, "lastSeenAt").control, FieldControl::DateTime);

    let status = field(&card, "status");
    assert_eq!(status.control, FieldControl::Select);
    let opts: Vec<(&str, &str)> = status.options.iter().map(|o| (o.value.as_str(), o.caption.as_str())).collect();
    assert_eq!(opts, vec![("Open", "Open"), ("OnHold", "On Hold"), ("Blocked", "Blocked")]);
    assert_eq!(status.default_value.as_deref(), Some("Open"));

    let country = field(&card, "countryCode");
    assert_eq!(country.control, FieldControl::Lookup);
    let lookup = country.lookup.as_ref().expect("lookup");
    assert_eq!(lookup.table, "Country");
    assert_eq!(lookup.sql_field, "code");
    assert_eq!(lookup.page.as_deref(), Some("CountryList"));
    assert_eq!(lookup.display_fields, vec!["code", "name"]);

    let balance = field(&card, "balance");
    assert_eq!(balance.binding, FieldBinding::FlowField);
    assert!(!balance.editable, "FlowFields are calculated");
    assert_eq!(balance.drill_down.as_ref().and_then(|d| d.page.as_deref()), Some("CustomerLedgerEntries"));

    let ledger = card.actions.iter().find(|a| a.name == "Ledger").unwrap();
    assert_eq!(ledger.area, ActionArea::Processing);
    assert_eq!(ledger.caption, "Ledger");
    assert_eq!(ledger.run_page_link[0].field, "customer_no");
    assert_eq!(ledger.run_page_link[0].value.js, "rec[\"no\"]");

    let list = compiled(&model, "CustomerList");
    assert_eq!(list.card_page.as_deref(), Some("CustomerCard"), "list opens the card of the same table");
    assert_eq!(field(&list, "blocked").control, FieldControl::Checkbox, "boolean in a grid");
    assert!(list.show_filter);
}

#[test]
fn page_diagnostics_cover_every_code() {
    let model = compile(
        r##"
        table Item { no: String(20) { notNull = true; }; kind: Enum(MissingEnum); primaryKey(no); }

        page Ghost for NoSuchTable { pageType = Card; }

        page ItemCard for Item {
            pageType = Card;
            style { background = "#12"; }
            layout {
                area(Content) {
                    group(General) {
                        field(no) { responsive { md = 40; } }
                        field(nope);
                        field(kind);
                        field(no);
                        field(no2) { showWhen = "rec.no =="; }
                    }
                    repeater(Lines) { }
                }
            }
            actions {
                area(Elsewhere) { }
                action(Post) { run = MissingCodeunit; }
            }
        }

        page Weird for Item { pageType = Spaceship; }

        pageextension ItemCardExt extends ItemCard {
            layout { addAfter(doesNotExist) { field(no3); } }
        }

        pageextension ItemCardCustomization extends ItemCard {
            isCustomization = true;
            actions { action(Sneaky) { onAction(rec, ctx) { ctx.call("X", "Y"); } } }
        }
        "##,
    );
    let codes = page_codes(&model);
    let has = |code: &str, sev: &str| codes.iter().any(|(c, s)| c == code && s == sev);
    assert!(has("BOS3001", "warning"), "unknown table in an open catalog is a warning: {codes:?}");
    assert!(has("BOS3002", "error"), "{codes:?}");
    assert!(has("BOS3003", "error"), "{codes:?}");
    assert!(has("BOS3004", "warning"), "{codes:?}");
    assert!(has("BOS3005", "error"), "{codes:?}");
    assert!(has("BOS3006", "error"), "{codes:?}");
    assert!(has("BOS3007", "error"), "{codes:?}");
    assert!(has("BOS3008", "error"), "{codes:?}");
    assert!(has("BOS3009", "error"), "{codes:?}");
    assert!(has("BOS3010", "error"), "{codes:?}");
    assert!(has("BOS3011", "error"), "{codes:?}");
    assert!(has("BOS3012", "warning"), "{codes:?}");

    let dup = model.diagnostics.iter().find(|d| d.code == "BOS3006").unwrap();
    assert!(dup.line > 1, "resolver diagnostics are mapped back to source lines");
}

#[test]
fn closed_catalog_turns_missing_references_into_errors() {
    let model = compile("page P for Missing { pageType = List; }");
    let catalog = ModelCatalog::new(vec![&model]).closed();
    let result = compile_page(&model.pages[0], &[], &catalog);
    assert!(result.diagnostics.iter().any(|d| d.code == "BOS3001" && d.severity == PageDiagnosticSeverity::Error));
}

#[test]
fn page_variables_bind_fields_that_are_not_on_the_table() {
    let model = compile(
        r##"
        table AuthConfiguration { id: String(20) { notNull = true; }; welcomeMessage: String(255); primaryKey(id); }
        page SignIn for AuthConfiguration {
            pageType = StandardDialog;
            variables { username: String(50); password: String(128); rememberMe: Boolean = false; }
            layout {
                area(Content) {
                    group(Credentials) {
                        field(welcomeMessage) { editable = false; }
                        field(username) { required = true; }
                        field(password);
                        field(rememberMe) { enableWhen = isNotEmpty(page.username); }
                    }
                }
            }
            actions { action(SignIn) { caption = "Sign In"; primary = true; } }
        }
        "##,
    );
    assert!(page_codes(&model).iter().all(|(_, s)| s != "error"), "{:?}", model.diagnostics);
    let page = compiled(&model, "SignIn");
    let pw = field(&page, "password");
    assert_eq!(pw.binding, FieldBinding::Variable);
    assert_eq!(pw.control, FieldControl::Masked, "password name hint");
    assert!(pw.editable);
    assert!(field(&page, "username").required);
    assert_eq!(field(&page, "rememberMe").default_value.as_deref(), Some("false"));
    assert!(!field(&page, "welcomeMessage").editable);
    let remember = find(&page.controls, "rememberMe");
    assert_eq!(remember.enable_when.as_ref().unwrap().js, "fx.isNotEmpty(page[\"username\"])");
    let sign_in = &page.actions[0];
    assert!(sign_in.in_footer_bar, "dialog actions without an area go to the footer");
    assert!(sign_in.primary);
}

// ----------------------------------------------------
// MERGER
// ----------------------------------------------------

fn field_ctrl(name: &str) -> PageControlIR {
    let mut c = PageControlIR::new("field", name);
    c.source_expr = Some(name.to_string());
    c
}

fn container(kind: &str, name: &str, children: Vec<PageControlIR>) -> PageControlIR {
    let mut c = PageControlIR::new(kind, name);
    c.children = children;
    c
}

fn directive(kind: &str, anchor: &str, children: Vec<PageControlIR>) -> PageControlIR {
    container(kind, anchor, children)
}

fn base_page() -> PageIR {
    let model = compile("page OrderCard for SalesHeader { pageType = Card; }");
    let mut page = model.pages[0].clone();
    page.controls = vec![container(
        "area",
        "Content",
        vec![
            container("group", "General", vec![field_ctrl("orderNo"), field_ctrl("orderDate"), field_ctrl("customer")]),
            container("group", "Shipping", vec![field_ctrl("shipTo")]),
        ],
    )];
    let mut post = PageActionIR::new("Post");
    post.area = Some(ActionArea::Processing);
    post.group = Some("Posting".into());
    page.actions = vec![post, PageActionIR::new("Print")];
    page.action_groups = vec![ActionGroupIR { name: "Posting".into(), area: Some(ActionArea::Processing), parent: None, caption: None, icon: None, visible: true }];
    page
}

fn ext(name: &str, controls: Vec<PageControlIR>, actions: Vec<PageActionIR>) -> PageExtensionIR {
    let model = compile(&format!("pageextension {name} extends OrderCard {{ }}"));
    let mut e = model.page_extensions[0].clone();
    e.controls = controls;
    e.actions = actions;
    e
}

fn names(controls: &[PageControlIR]) -> Vec<&str> {
    controls.iter().map(|c| c.name.as_str()).collect()
}

fn general(page: &PageIR) -> &PageControlIR {
    &page.controls[0].children[0]
}

#[test]
fn merger_applies_add_directives() {
    let e = ext(
        "Adds",
        vec![
            directive("addFirst", "General", vec![field_ctrl("priority")]),
            directive("addLast", "General", vec![field_ctrl("notes")]),
            directive("addBefore", "orderDate", vec![field_ctrl("postingDate")]),
            directive("addAfter", "customer", vec![field_ctrl("contact")]),
        ],
        vec![],
    );
    let out = merge_page_detailed(&base_page(), &[e]);
    assert!(out.orphans.is_empty(), "{:?}", out.orphans);
    assert_eq!(names(&general(&out.page).children), vec!["priority", "orderNo", "postingDate", "orderDate", "customer", "contact", "notes"]);
    assert_eq!(out.applied, vec!["Adds"]);
}

#[test]
fn merger_modify_hide_wrap_replace_keep_positions() {
    let mut modify = directive("modify", "customer", vec![]);
    modify.caption = Some("Client".into());
    modify.style = Some(PageStyleIR { background: Some(ColorValue::parse("#fff7ed").unwrap()), ..Default::default() });
    let mut base = base_page();
    general_mut(&mut base).children[2].style = Some(PageStyleIR { radius: Some(Radius::Lg), ..Default::default() });

    let e = ext(
        "Edits",
        vec![
            modify,
            directive("hide", "shipTo", vec![]),
            directive("wrap", "orderDate", vec![container("flex", "DateRow", vec![field_ctrl("dueDate")])]),
            directive("replace", "orderNo", vec![field_ctrl("orderNo"), field_ctrl("externalNo")]),
        ],
        vec![],
    );
    let out = merge_page_detailed(&base, &[e]);
    assert!(out.orphans.is_empty(), "{:?}", out.orphans);
    let g = general(&out.page);
    assert_eq!(names(&g.children), vec!["orderNo", "externalNo", "DateRow", "customer"]);
    assert_eq!(names(&g.children[2].children), vec!["orderDate", "dueDate"], "wrap keeps the original first, in place");
    let customer = &g.children[3];
    assert_eq!(customer.caption.as_deref(), Some("Client"));
    let style = customer.style.as_ref().unwrap();
    assert_eq!(style.radius, Some(Radius::Lg), "modify overlays style");
    assert_eq!(style.background, Some(ColorValue::Hex("#fff7ed".into())));
    assert_eq!(out.page.controls[0].children[1].children[0].properties.get("visible").map(String::as_str), Some("false"));
}

fn general_mut(page: &mut PageIR) -> &mut PageControlIR {
    &mut page.controls[0].children[0]
}

#[test]
fn merger_moves_controls_and_reports_orphans() {
    let mut move_after = directive("moveAfter", "customer", vec![]);
    move_after.source_expr = Some("orderNo".into());
    let e = ext(
        "Moves",
        vec![
            move_after,
            directive("moveFirst", "Shipping", vec![field_ctrl("orderDate")]),
            directive("moveBefore", "missingAnchor", vec![field_ctrl("customer")]),
            directive("addAfter", "ghost", vec![field_ctrl("x")]),
            directive("moveLast", "General", vec![field_ctrl("General")]),
        ],
        vec![],
    );
    let out = merge_page_detailed(&base_page(), &[e]);
    assert_eq!(names(&general(&out.page).children), vec!["customer", "orderNo"]);
    assert_eq!(names(&out.page.controls[0].children[1].children), vec!["orderDate", "shipTo"]);
    let anchors: Vec<&str> = out.orphans.iter().map(|o| o.anchor.as_str()).collect();
    assert_eq!(anchors, vec!["missingAnchor", "ghost", "General"], "orphans are skipped, nothing is appended elsewhere");
    assert!(!out.page.controls[0].children.iter().any(|c| c.name == "x"));
}

#[test]
fn merger_adds_top_level_controls_into_content() {
    let e = ext("Loyalty", vec![container("group", "Loyalty", vec![field_ctrl("tier")])], vec![]);
    let out = merge_page_detailed(&base_page(), &[e]);
    assert_eq!(names(&out.page.controls[0].children), vec!["General", "Shipping", "Loyalty"]);
}

#[test]
fn merger_applies_action_placements() {
    let mut add_after = PageActionIR::new("PostAndPrint");
    add_after.placement = Some("addAfter".into());
    add_after.anchor = Some("Post".into());
    let mut add_first = PageActionIR::new("Preview");
    add_first.placement = Some("addFirst".into());
    add_first.anchor = Some("Posting".into());
    let mut modify = PageActionIR::new("Post");
    modify.placement = Some("modify".into());
    modify.anchor = Some("Post".into());
    modify.caption = Some("Post Invoice".into());
    let mut hide = PageActionIR::new("Print");
    hide.placement = Some("hide".into());
    hide.anchor = Some("Print".into());
    let mut orphan = PageActionIR::new("X");
    orphan.placement = Some("modify".into());
    orphan.anchor = Some("Nope".into());

    let out = merge_page_detailed(&base_page(), &[ext("Acts", vec![], vec![add_after, add_first, modify, hide, orphan])]);
    let order: Vec<&str> = out.page.actions.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(order, vec!["Preview", "Post", "PostAndPrint", "Print"]);
    assert_eq!(out.page.actions[0].group.as_deref(), Some("Posting"));
    assert_eq!(out.page.actions[2].group.as_deref(), Some("Posting"), "addAfter inherits the anchor's group");
    assert_eq!(out.page.actions[1].caption.as_deref(), Some("Post Invoice"));
    assert!(!out.page.actions[3].visible);
    assert_eq!(out.orphans.len(), 1);
    assert_eq!(out.orphans[0].directive, "action:modify");
}

#[test]
fn customization_layer_is_sorted_last_and_tolerant() {
    let mut custom = ext("AAA_Customization", vec![directive("hide", "removedInUpgrade", vec![])], vec![]);
    custom.is_customization = true;
    let dev = ext("ZZZ_Dev", vec![], vec![]);
    let mut exts = vec![custom, dev];
    sort_extensions_for_merge(&mut exts, |_| 0);
    assert_eq!(exts[1].name, "AAA_Customization");

    let model = compile("table SalesHeader { no: String(20); primaryKey(no); }");
    let mut catalog_model = model.clone();
    catalog_model.pages.push(base_page());
    let result = compile_page(&base_page(), &exts, &ModelCatalog::new(vec![&catalog_model]));
    let orphan = result.diagnostics.iter().find(|d| d.code == "BOS3007").expect("orphan reported");
    assert_eq!(orphan.severity, PageDiagnosticSeverity::Warning, "customization orphans never fail a page");
    assert_eq!(result.page.applied_extensions, vec!["ZZZ_Dev", "AAA_Customization"]);
}

#[test]
fn customization_whitelist_strips_behaviour() {
    let mut custom = ext(
        "C",
        vec![
            {
                let mut m = directive("modify", "customer", vec![]);
                m.properties.insert("editable".into(), "false".into());
                m
            },
            directive("hide", "shipTo", vec![]),
        ],
        vec![PageActionIR::new("NewAction")],
    );
    custom.is_customization = true;
    let (clean, diags) = crate::page_resolver::enforce_customization_whitelist(&custom);
    assert_eq!(names(&clean.controls), vec!["shipTo"]);
    assert!(clean.actions.is_empty());
    assert_eq!(diags.iter().filter(|d| d.code == "BOS3009").count(), 2);
}

#[test]
fn captions_are_prettified() {
    assert_eq!(prettify_caption("fullName"), "Full Name");
    assert_eq!(prettify_caption("ipAddress"), "IP Address");
    assert_eq!(prettify_caption("sso_login_url"), "SSO Login URL");
    assert_eq!(prettify_caption("UserCard"), "User Card");
    assert_eq!(prettify_caption("HTTPStatus"), "Http Status");
    assert_eq!(prettify_caption("id"), "ID");
}

#[test]
fn lowering_keeps_v2_properties_and_legacy_hints() {
    let model = compile(
        r##"
        page P for T {
            pageType = Dialog;
            cardPageId = TCard;
            refreshOnActivate = true;
            customThing = "kept";
            layout {
                area(Content) {
                    group(G) {
                        field(secret) { style = "password"; icon = "UserMinus"; importance = Additional; visible = rec.kind == "A"; }
                        field(flag) { style = "switch"; readonly; }
                    }
                }
            }
            actions {
                area(Processing) {
                    group(Posting) { caption = "Posting"; separator; action(Post) { image = "Receipt"; promotedOnly = true; selection = multiple; } }
                }
                area(Promoted) { actionref(Post_Promoted; Post) {} }
            }
        }
        "##,
    );
    let p = &model.pages[0];
    assert_eq!(p.ir_version, PAGE_IR_VERSION);
    assert_eq!(p.page_type, "StandardDialog");
    assert_eq!(p.card_page.as_deref(), Some("TCard"));
    assert!(p.refresh_on_activate);
    assert_eq!(p.properties.get("customThing").map(String::as_str), Some("kept"));

    let secret = &p.controls[0].children[0].children[0];
    assert_eq!(secret.properties.get("extendedDatatype").map(String::as_str), Some("Masked"));
    assert_eq!(secret.style.as_ref().unwrap().icon.as_deref(), Some("user-minus"));
    assert_eq!(secret.responsive.as_ref().unwrap().importance, Some(Importance::Additional));
    assert_eq!(secret.properties.get("showWhen").map(String::as_str), Some("rec.kind == \"A\""));
    let flag = &p.controls[0].children[0].children[1];
    assert_eq!(flag.properties.get("control").map(String::as_str), Some("switch"));
    assert_eq!(flag.properties.get("editable").map(String::as_str), Some("false"));

    assert_eq!(p.action_groups.len(), 1);
    assert_eq!(p.action_groups[0].caption.as_deref(), Some("Posting"));
    let post = &p.actions[0];
    assert_eq!((post.area, post.group.as_deref()), (Some(ActionArea::Processing), Some("Posting")));
    assert_eq!(post.properties.get("separatorBefore").map(String::as_str), Some("true"));
    assert!(post.promoted_only);
    let promoted = &p.actions[1];
    assert_eq!(promoted.area, Some(ActionArea::Promoted));
    assert_eq!(promoted.target.as_deref(), Some("Post"));

    let json = serde_json::to_value(p).unwrap();
    let back: PageIR = serde_json::from_value(json).unwrap();
    assert_eq!(&back, p, "page IR round-trips through the catalog JSON");
}

/// Keeps docs/17 honest: its authoring examples must compile without page errors.
#[test]
fn documentation_examples_compile() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let doc = std::fs::read_to_string(root.join("docs/17_page_compiler_and_designer.md"))
        .or_else(|_| std::fs::read_to_string(root.join("stale-docs-2/17_page_compiler_and_designer.md")))
        .expect("docs/17");
    let blocks: Vec<&str> = doc.split("```typescript").skip(1).filter_map(|b| b.split("```").next()).collect();
    assert!(blocks.len() >= 2, "page and pageextension examples present");
    let tables = r#"
        enum CustomerStatus { Open; Released; }
        table Customer {
            no: String(20) { notNull = true; };
            name: String(100); creditLimit: Decimal(18, 2); blocked: Boolean; status: Enum(CustomerStatus);
            loyaltyTier: String(20); points: Integer; email: String(80); phone: String(30); mobile: String(30);
            balance: Decimal(18, 2); website: String(200); notes: String(500); remarks: String(500); memberSince: Date;
            primaryKey(no);
        }
    "#;
    let src = format!("{tables}\n{}\n{}", blocks[0], blocks[1]);
    let model = compile(&src);
    let errors: Vec<_> = model.diagnostics.iter().filter(|d| d.code.starts_with("BOS30") && d.severity == "error").collect();
    assert!(errors.is_empty(), "doc example errors: {errors:#?}");
    let page = compiled(&model, "CustomerCard");
    assert_eq!(page.caption, "Client", "extension caption override applied");
    assert!(find(&page.controls, "loyaltyTier").field.is_some());
    assert_eq!(page.applied_extensions, vec!["CustomerCardLoyalty"]);
}

#[test]
fn bos_core_package_pages_compile_without_errors() {
    let pkg_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages/bos.core");
    let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(pkg_dir)
        .expect("read bos.core dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().and_then(|s| s.to_str()) == Some("bos"))
        .collect();
    entries.sort();
    let mut src = String::new();
    for p in entries {
        src.push_str(&std::fs::read_to_string(&p).expect("read .bos file"));
        src.push('\n');
    }
    let model = compile(&src);
    let errors: Vec<_> = model.diagnostics.iter().filter(|d| d.code.starts_with("BOS30") && d.severity == "error").collect();
    assert!(errors.is_empty(), "bos.core page errors: {errors:#?}");
    assert_eq!(model.pages.len(), 50);
    let catalog = ModelCatalog::new(vec![&model]);
    for page in &model.pages {
        let result = compile_page(page, &[], &catalog);
        assert!(!result.page.controls.is_empty(), "{} has controls", page.name);
    }
}
