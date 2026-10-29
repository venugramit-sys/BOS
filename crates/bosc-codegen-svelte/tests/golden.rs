//! Golden-file tests: every UI page type in `tests/fixtures/pages.bos` is generated and compared with
//! `tests/golden/<Page>.svelte`. Run with `BOS_UPDATE_GOLDEN=1` to (re)write the golden files after an
//! intentional generator change, then review the diff.

use bosc_analyzer::{compile_page, sort_extensions_for_merge, ModelCatalog, SemanticAnalyzer};
use bosc_codegen_svelte::{generate, CodegenError, GenerateOptions, GeneratedPage};
use bosc_ir::{CompiledPageIR, ModelIR, PageExtensionIR};
use bosc_syntax::{Lexer, Parser};
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_source(src: &str) -> ModelIR {
    let tokens = Lexer::new(src).tokenize().expect("lex");
    let program = Parser::with_source(tokens, src.to_string()).parse_program().expect("parse");
    SemanticAnalyzer::new("fixture.pages").analyze_and_lower(&program).expect("lower")
}

fn fixture_model() -> ModelIR {
    let src = std::fs::read_to_string(crate_dir().join("tests/fixtures/pages.bos")).expect("fixture");
    let model = compile_source(&src);
    let errors: Vec<_> = model.diagnostics.iter().filter(|d| d.severity == "error").collect();
    assert!(errors.is_empty(), "fixture must compile cleanly: {errors:#?}");
    model
}

fn compiled(model: &ModelIR, name: &str) -> CompiledPageIR {
    let page = model.pages.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("page {name}"));
    let mut exts: Vec<PageExtensionIR> = model.page_extensions.iter().filter(|e| e.target_page == name).cloned().collect();
    sort_extensions_for_merge(&mut exts, |_| 0);
    let result = compile_page(page, &exts, &ModelCatalog::new(vec![model]).closed());
    let errors: Vec<_> = result.diagnostics.iter().filter(|d| d.severity == bosc_ir::PageDiagnosticSeverity::Error).collect();
    assert!(errors.is_empty(), "{name}: {errors:#?}");
    result.page
}

fn check_golden(file: &str, generated: &GeneratedPage) {
    let path = crate_dir().join("tests/golden").join(file);
    if std::env::var_os("BOS_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &generated.source).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden file {}; run with BOS_UPDATE_GOLDEN=1", path.display()))
        .replace("\r\n", "\n");
    assert_eq!(generated.source, expected, "generated output for {file} changed; review and run with BOS_UPDATE_GOLDEN=1");
}

pub const UI_PAGES: &[&str] = &[
    "CustomerCard",
    "CustomerList",
    "SalesLineSubform",
    "CustomerStatsFactBox",
    "SalesOrder",
    "ItemJournal",
    "SignIn",
    "PostConfirm",
    "SetupWizard",
    "SalesHome",
    "OrderBoard",
    "ActivityTimeline",
    "CustomerWorkspace",
    "CustomDashboard",
];

#[test]
fn every_page_type_matches_its_golden_file() {
    let model = fixture_model();
    for name in UI_PAGES {
        let page = compiled(&model, name);
        let out = generate(&page, GenerateOptions::default()).expect("generate");
        check_golden(&format!("{name}.svelte"), &out);
    }
    let design = generate(&compiled(&model, "CustomerCard"), GenerateOptions { design: true }).unwrap();
    check_golden("CustomerCard.design.svelte", &design);
}

#[test]
fn fixture_covers_every_ui_page_type() {
    let model = fixture_model();
    let mut seen: Vec<bosc_ir::PageType> = UI_PAGES.iter().map(|n| compiled(&model, n).page_type).collect();
    seen.sort();
    seen.dedup();
    let expected: Vec<_> = bosc_ir::PageType::ALL.into_iter().filter(|t| t.has_ui()).collect();
    assert_eq!(seen, expected);
}

#[test]
fn api_pages_have_no_ui() {
    let model = fixture_model();
    let err = generate(&compiled(&model, "CustomerApi"), GenerateOptions::default()).unwrap_err();
    assert_eq!(err, CodegenError::NoUserInterface("CustomerApi".into()));
}

#[test]
fn output_is_deterministic_and_markup_carries_no_authored_text() {
    let model = fixture_model();
    for name in UI_PAGES {
        let page = compiled(&model, name);
        let a = generate(&page, GenerateOptions::default()).unwrap();
        let b = generate(&page, GenerateOptions::default()).unwrap();
        assert_eq!(a, b, "{name} is deterministic");
        let markup = a.source.split("</script>").nth(1).expect("markup");
        assert!(!markup.contains('"'), "{name}: markup must only reference META/C/A, found a literal:\n{markup}");
        assert!(!a.source.contains("class="), "{name}: generated pages carry no CSS classes");
        assert!(!a.source.contains("<style"), "{name}: generated pages carry no style block");
        assert!(!a.source.contains("eval(") && !a.source.contains("Function("), "{name}: no dynamic code");
    }
}

#[test]
fn hostile_captions_cannot_break_out_of_the_script() {
    let model = compile_source(
        r#"
        table T { no: String(20) { notNull = true; }; primaryKey(no); }
        page Evil for T {
            pageType = Card;
            caption = "</script><script>alert(1)</script>{boom}";
            layout { area(Content) { group(G) { caption = "{#if true}<img src=x onerror=alert(1)>{/if}"; field(no) { caption = " </SCRIPT>"; } } } }
            actions { action(A) { caption = "</script>"; } }
        }
        "#,
    );
    let page = compiled(&model, "Evil");
    let out = generate(&page, GenerateOptions::default()).unwrap();
    assert_eq!(out.source.matches("</script>").count(), 1, "only the real closing tag:\n{}", out.source);
    assert!(out.source.contains("<\\/script>"));
    let markup = out.source.split("</script>").nth(1).unwrap();
    assert!(!markup.contains("onerror") && !markup.contains("{#if true}") && !markup.contains("{boom}"));
}

#[test]
fn hidden_controls_are_dropped_except_in_design_mode() {
    let model = fixture_model();
    let page = compiled(&model, "CustomerCard");
    let runtime = generate(&page, GenerateOptions::default()).unwrap();
    let design = generate(&page, GenerateOptions { design: true }).unwrap();
    assert!(!runtime.source.contains("\"name\":\"internalRating\""));
    assert!(design.source.contains("\"name\":\"internalRating\""));
    assert!(runtime.source.contains("{#if v"), "conditional group is gated at runtime");
    assert!(!design.source.contains("{#if v"), "design mode keeps every node selectable");
    assert!(design.source.contains("visible={v"));
    assert!(!runtime.source.contains("auto_filled") && design.source.contains("auto_filled"));
}

#[test]
fn bos_core_pages_generate() {
    let src = std::fs::read_to_string(Path::new(&crate_dir()).join("../../packages/bos.core/03_user_auth.bos")).unwrap();
    let model = compile_source(&src);
    for page in &model.pages {
        let out = generate(&compiled(&model, &page.name), GenerateOptions::default()).expect("generate");
        assert!(out.source.contains("createPage(() => ctx, META, C, X)"), "{}", page.name);
    }
}

#[cfg(feature = "svelte-compile")]
mod svelte_compile {
    //! Compiles every golden page with the real Svelte 5 compiler inside QuickJS (the same engine the
    //! BOS node uses), proving the generated source is valid runes-mode Svelte.
    use super::*;
    use rquickjs::{CatchResultExt, Context, Runtime};

    const PRELUDE: &str = "globalThis.self = globalThis; globalThis.btoa = (s) => { throw new Error('btoa: sourcemaps are disabled'); };";
    const COMPILE: &str = r#"(() => {
        const r = svelte.compile(globalThis.__SRC, { generate: 'client', css: 'external', runes: true,
            filename: globalThis.__NAME + '.svelte', sourcemap: false, discloseVersion: false });
        return JSON.stringify({ code: r.js.code, warnings: r.warnings.map((w) => w.code + ': ' + w.message) });
    })()"#;

    /// The Svelte compiler recurses deeply; QuickJS' stack limit must stay below the native thread
    /// stack, so the compile runs on a thread with an explicit 64 MB stack (default test threads have ~2 MB).
    #[test]
    fn generated_pages_compile_with_svelte() {
        std::thread::Builder::new()
            .name("svelte-compile".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(compile_all)
            .expect("spawn compiler thread")
            .join()
            .expect("svelte compile thread panicked");
    }

    fn compile_all() {
        let compiler = std::fs::read_to_string(crate_dir().join("../../ui/node_modules/svelte/compiler/index.js"))
            .expect("ui/node_modules/svelte/compiler/index.js missing: run `npm ci` in ui/");
        let rt = Runtime::new().unwrap();
        rt.set_memory_limit(512 * 1024 * 1024);
        rt.set_max_stack_size(8 * 1024 * 1024);
        let ctx = Context::full(&rt).unwrap();
        let model = fixture_model();
        ctx.with(|ctx| {
            ctx.eval::<(), _>(PRELUDE).catch(&ctx).unwrap();
            ctx.eval::<(), _>(compiler.as_str()).catch(&ctx).expect("load compiler");
            let mut pages: Vec<(String, GeneratedPage)> = UI_PAGES
                .iter()
                .map(|n| (n.to_string(), generate(&compiled(&model, n), GenerateOptions::default()).unwrap()))
                .collect();
            pages.push(("CustomerCardDesign".into(), generate(&compiled(&model, "CustomerCard"), GenerateOptions { design: true }).unwrap()));
            for (name, page) in pages {
                ctx.globals().set("__SRC", page.source.as_str()).unwrap();
                ctx.globals().set("__NAME", name.as_str()).unwrap();
                let out: String = ctx
                    .eval(COMPILE)
                    .catch(&ctx)
                    .unwrap_or_else(|e| panic!("{name} failed to compile: {e:?}\n{}", page.source));
                let v: serde_json::Value = serde_json::from_str(&out).unwrap();
                let warnings = v["warnings"].as_array().unwrap();
                assert!(warnings.is_empty(), "{name} compiled with warnings: {warnings:?}");
                let code = v["code"].as_str().unwrap();
                assert!(code.contains("svelte/internal/client") && code.contains("@bos/page-kit"), "{name}");
            }
        });
    }
}
