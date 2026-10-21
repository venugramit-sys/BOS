//! Phase 0 feasibility gate: run the real Svelte 5 compiler inside QuickJS (no Node.js)
//! and measure cold/warm compile time of a generated, page-kit style component.
//!
//! Run with: cargo test -p bosc-runtime --release --test svelte_compile_bench -- --nocapture

use rquickjs::{CatchResultExt, Context, Runtime};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const WARM_BUDGET: Duration = Duration::from_millis(1500);

fn compiler_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../ui/node_modules/svelte/compiler/index.js")
}

/// Host shims the compiler bundle probes for. `btoa` is only reached for sourcemaps,
/// which are disabled, but it is defined so any unexpected path fails loudly instead of silently.
const PRELUDE: &str = r#"
globalThis.self = globalThis;
globalThis.btoa = function (s) {
  const c = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  let o = '';
  for (let i = 0; i < s.length; i += 3) {
    const a = s.charCodeAt(i), b = s.charCodeAt(i + 1), d = s.charCodeAt(i + 2);
    const n = (a << 16) | ((b || 0) << 8) | (d || 0);
    o += c[(n >> 18) & 63] + c[(n >> 12) & 63] + (isNaN(b) ? '=' : c[(n >> 6) & 63]) + (isNaN(d) ? '=' : c[n & 63]);
  }
  return o;
};
"#;

const COMPILE_CALL: &str = r#"
(() => {
  const r = svelte.compile(globalThis.__SRC, {
    generate: 'client', css: 'external', dev: false, runes: true,
    filename: 'CustomerCard.svelte', sourcemap: false, discloseVersion: false,
  });
  return r.js.code;
})()
"#;

/// Builds a ~300 line component in the shape `bosc-codegen-svelte` will emit:
/// thin markup of page-kit components with literal props, `$derived` only for expressions.
fn generated_page(fields: usize) -> String {
    let mut s = String::from(
        "<script>\n  import { PageFrame, FastTab, FieldGrid, FieldControl, ActionBar, createRecordStore } from '@bos/page-kit';\n  let { ctx } = $props();\n  const META = Object.freeze({ name: \"CustomerCard\", sourceTable: \"Customer\" });\n  const rec = createRecordStore(ctx, META);\n",
    );
    for i in 0..fields / 4 {
        s.push_str(&format!("  const show_{i} = $derived(rec.data.blocked !== true && rec.data.amount_{i} > 0);\n"));
    }
    s.push_str("</script>\n\n<PageFrame {ctx} {rec} caption={\"Customer Card\"}>\n  <ActionBar {ctx} {rec} actions={[{ name: \"Post\", caption: \"Post\", icon: \"receipt\", primary: true }]} />\n");
    for g in 0..fields / 10 {
        s.push_str(&format!("  <FastTab node={{\"area:Content/group:G{g}\"}} caption={{\"General {g}\"}}>\n    <FieldGrid cols={{2}}>\n"));
        for f in 0..10 {
            let i = g * 10 + f;
            let vis = if i % 4 == 0 { format!(" visible={{show_{}}}", i / 4) } else { String::new() };
            s.push_str(&format!(
                "      <FieldControl {{rec}} node={{\"field:f{i}\"}} field={{\"f{i}\"}} control={{\"number\"}} caption={{\"Field {i}\"}} decimals={{2}} required={{false}}{vis} style={{{{ background: \"primary\", radius: \"md\" }}}} />\n"
            ));
        }
        s.push_str("    </FieldGrid>\n  </FastTab>\n");
    }
    s.push_str("</PageFrame>\n");
    s
}

#[test]
fn svelte_compiler_runs_in_quickjs_within_budget() {
    // The compiler recurses deeply: QuickJS' 8 MB stack limit needs a larger native thread stack.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(bench)
        .expect("spawn bench thread")
        .join()
        .expect("bench thread panicked");
}

fn bench() {
    let compiler = std::fs::read_to_string(compiler_path())
        .expect("ui/node_modules/svelte/compiler/index.js missing: run `npm ci` in ui/");
    let page = generated_page(120);
    let lines = page.lines().count();

    let rt = Runtime::new().expect("quickjs runtime");
    rt.set_memory_limit(512 * 1024 * 1024);
    rt.set_max_stack_size(8 * 1024 * 1024);
    let ctx = Context::full(&rt).expect("quickjs context");

    ctx.with(|ctx| {
        let t0 = Instant::now();
        ctx.eval::<(), _>(PRELUDE).catch(&ctx).expect("prelude");
        ctx.eval::<(), _>(compiler.as_str()).catch(&ctx).expect("load svelte compiler");
        let load = t0.elapsed();

        ctx.globals().set("__SRC", page.as_str()).expect("set source");
        let t1 = Instant::now();
        let first: String = ctx.eval(COMPILE_CALL).catch(&ctx).expect("first compile");
        let cold = t1.elapsed();

        let mut warm = Duration::MAX;
        for _ in 0..3 {
            let t = Instant::now();
            let out: String = ctx.eval(COMPILE_CALL).catch(&ctx).expect("warm compile");
            warm = warm.min(t.elapsed());
            assert_eq!(out, first, "compiler output must be deterministic");
        }

        println!("svelte compiler load: {load:?}");
        println!("page: {lines} lines, {} bytes", page.len());
        println!("compile cold: {cold:?}, warm (best of 3): {warm:?}");
        println!("output: {} bytes JS", first.len());

        assert!(first.contains("svelte/internal/client"), "client runtime import expected");
        assert!(first.contains("@bos/page-kit"), "page-kit import must be preserved");
        assert!(!first.contains("eval("), "generated code must not use eval");
        // The performance gate applies to release builds (debug QuickJS is several times slower).
        if !cfg!(debug_assertions) {
            assert!(warm < WARM_BUDGET, "warm compile {warm:?} exceeds gate {WARM_BUDGET:?}");
        }
    });
}
