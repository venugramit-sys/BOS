//! Round-trip tests of the page printer: lower(parse(print(ir))) == ir for every page in the repository.

use crate::page_print::{print_page, print_page_extension};
use crate::SemanticAnalyzer;
use bosc_ir::ModelIR;
use bosc_syntax::{Lexer, Parser};

fn lower(src: &str) -> ModelIR {
    let tokens = Lexer::new(src).tokenize().unwrap_or_else(|e| panic!("lex: {e}\n{src}"));
    let program = Parser::with_source(tokens, src.to_string()).parse_program().unwrap_or_else(|e| panic!("parse: {e}\n{src}"));
    SemanticAnalyzer::new("test").analyze_and_lower(&program).unwrap_or_else(|e| panic!("lower: {e}\n{src}"))
}

fn assert_round_trip(src: &str) -> usize {
    let model = lower(src);
    for page in &model.pages {
        let printed = print_page(page);
        let again = lower(&printed);
        assert_eq!(again.pages.len(), 1, "{printed}");
        assert_eq!(&again.pages[0], page, "page {} does not round-trip:\n{printed}", page.name);
    }
    for ext in &model.page_extensions {
        let printed = print_page_extension(ext);
        let again = lower(&printed);
        assert_eq!(again.page_extensions.len(), 1, "{printed}");
        assert_eq!(&again.page_extensions[0], ext, "extension {} does not round-trip:\n{printed}", ext.name);
    }
    model.pages.len() + model.page_extensions.len()
}

fn repo_file(rel: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join(rel);
    std::fs::read_to_string(&path)
        .or_else(|_| {
            let alt = rel.replace("docs/", "stale-docs-2/");
            std::fs::read_to_string(root.join(alt))
        })
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn bos_core_pages_round_trip() {
    assert_eq!(assert_round_trip(&repo_file("packages/bos.core/03_user_auth.bos")), 5);
}

#[test]
fn every_page_type_fixture_round_trips() {
    assert!(assert_round_trip(&repo_file("crates/bosc-codegen-svelte/tests/fixtures/pages.bos")) >= 15);
}

#[test]
fn documentation_examples_round_trip() {
    let doc = repo_file("docs/17_page_compiler_and_designer.md");
    let blocks: Vec<&str> = doc.split("```typescript").skip(1).filter_map(|b| b.split("```").next()).collect();
    assert_eq!(assert_round_trip(&format!("{}\n{}", blocks[0], blocks[1])), 2);
}

#[test]
fn every_construct_round_trips() {
    let src = r##"
page Everything for Customer {
    pageType = List;
    caption = "All \"quoted\" things";
    editable = false;
    insertAllowed = true;
    showFilter = true;
    cardPageId = CustomerCard;
    dataCaptionFields = [no, name];
    delayedInsert = true;
    usageCategory = "Lists";
    instructionalText = "Pick one.";
    customFlag = yes;
    style { background = "#1e40af"; radius = md; icon = "users"; }
    variables {
        note: String(250) { caption = "Note"; };
        confirmed: Boolean = false;
        count: Integer;
    }
    views {
        view(Open) { caption = "Open"; filter = status == "Open"; sort = "name asc"; default = true; }
    }
    onOpenPage(rec, ctx) {
        ctx.page.variables.count = 1;
        if (rec.blocked) {
            ctx.ui.toast("warning", "Blocked", "x");
        }
    }
    layout {
        area(Content) {
            group("Order Lines") {
                caption = "Order Lines";
                field(no) { importance = Promoted; showWhen = rec.blocked == false; }
                field(display; name) { caption = "Display"; editable = false; style { emphasis = Strong; } }
                field(email) { responsive { md = 6; hideBelow = sm; } }
                field(phone) {
                    onValidate(rec, ctx) {
                        if (!rec.phone) throw new Error("phone");
                    }
                }
            }
            grid(Contact) { cols = 2; field(note); }
            part(Lines; LineList) { link { documentNo = rec.no; } updatePropagation = true; }
            label(Hint) { caption = "Hint"; }
            spacer(Gap);
        }
    }
    actions {
        action(TopLevel) { caption = "Top"; }
        area(Processing) {
            group(Posting) {
                caption = "Posting";
                icon = "receipt";
                action(Post) {
                    caption = "Post";
                    icon = "receipt";
                    confirm = "Post it?";
                    enableWhen = rec.status == "Released";
                    runCodeunit = "SalesPost.Post";
                    onAction(rec, ctx) {
                        ctx.call("SalesPost", "Post", { no: rec.no });
                    }
                }
                separator;
                action(Ledger) { openPage = LedgerList; runPageMode = View; link { customerNo = rec.no; } }
            }
            action(Delete) { confirm = "Delete?"; visible = false; selection = "single"; }
        }
        area(Navigation) { action(Open) { openPage = CustomerCard; } }
        area(Promoted) { actionref(Post_Promoted; Post) { } }
    }
}

pageextension EverythingExt extends Everything {
    caption = "Client";
    layout {
        addAfter(no) { field(points) { caption = "Points"; } }
        modify(email) { caption = "Mail"; visible = false; style { background = primary; } }
        moveAfter(no) { email; phone; }
        hide(phone);
        group(Extra) { field(rating); }
    }
    actions {
        addAfter(Post) { action(PostPrint) { caption = "Post and Print"; } }
        addLast(Posting) { action(Preview) { caption = "Preview"; } }
        modify(Post) { caption = "Post Invoice"; }
        hide(Delete);
        moveBefore(Post) { Ledger; }
    }
}
"##;
    assert_eq!(assert_round_trip(src), 2);
}

#[test]
fn printed_source_is_valid_when_names_need_quoting() {
    // Keyword-like names must be quoted in name positions.
    let mut p = lower("page P for T { layout { area(Content) { field(name); } } }").pages.remove(0);
    p.controls[0].children[0].name = "key".into();
    p.controls[0].children[0].source_expr = Some("key".into());
    let printed = print_page(&p);
    assert!(printed.contains("field(\"key\")"), "{printed}");
    assert_eq!(lower(&printed).pages[0].controls[0].children[0].name, "key");
}
