//! Svelte 5 page code generator.
//!
//! Turns a [`CompiledPageIR`] into a single Svelte 5 component in runes mode. The generated component
//! is deliberately thin:
//!
//! - All metadata lives in frozen JS literals (`META`, the control table `C`, the action table `A`).
//!   The markup only references them by index, so **no authored text is ever written into markup** and
//!   captions cannot inject template syntax or script.
//! - Client expressions (`showWhen`, `enableWhen`, ...) become arrow functions in the expression table
//!   `X`. Top-level visibility conditions become `$derived` values gating `{#if}` blocks; everything else
//!   is evaluated inside the page-kit components, so only the controls that read a field re-render.
//! - There are no CSS classes and no `<style>` block. Styling comes from page-kit components reading
//!   the design tokens in each control's `style`.
//! - Statically hidden controls are left out of the bundle, except in design mode.
//!
//! # Page-kit contract
//!
//! The component imports from [`PAGE_KIT_MODULE`] only:
//!
//! - `createPage(() => ctx, META, C, X)` returns the page controller `p`. It exposes `p.rec` (the reactive
//!   current record), `p.vars` (page variables), `p.design` and `p.x` (= `X`).
//! - Layout components all take `{p}` and `c` (one entry of `C`; `c.i` is its index in `C`):
//!   `PageFrame` (with `actions`), `Area`, `FactBoxes`, `Group`, `Grid`, `Card`, `Flex`, `Container`,
//!   `CueGroup`, `Cue`, `Field`, `Label`, `Image`, `Spacer`, `Divider`, `Headline`, `UserControl`,
//!   `Part`, `SystemPart`, `Repeater` (with `columns`), `Tabs` (with `tabs`), `Split` (with `panes`)
//!   and `WizardSteps` (with `steps`). `tabs`, `steps` and `panes` are `{ c, body, visible? }` entries where
//!   `body` is a snippet; `visible` is present (reactive) only for conditionally shown entries.
//! - `X[i].style` follows NAV `StyleExpr`: a boolean switches the control's static `style.emphasis` on
//!   and off, a string names the emphasis to use.
//! - `fx` is the expression helper object (`truthy`, `eq`, `cmp`, `num`, `isEmpty`, ...).

use bosc_ir::{ClientExprIR, CompiledPageIR, PageControlKind, ResolvedActionIR, ResolvedControlIR};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt::Write as _;

/// Bumped whenever generated output changes for the same input. It is part of the artifact hash.
pub const CODEGEN_VERSION: &str = "1";

/// Module specifier of the page runtime kit, resolved through the import map.
pub const PAGE_KIT_MODULE: &str = "@bos/page-kit";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GenerateOptions {
    /// Designer preview build: keeps statically hidden controls (rendered dimmed by the kit) and never
    /// removes conditionally hidden ones, so every node can be selected on the canvas.
    pub design: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedPage {
    pub name: String,
    /// Complete `.svelte` source.
    pub source: String,
    /// Page-kit exports the component imports (sorted).
    pub imports: Vec<&'static str>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CodegenError {
    #[error("page '{0}' is an API page and has no user interface")]
    NoUserInterface(String),
    #[error("page '{page}' could not be serialized: {reason}")]
    Serialize { page: String, reason: String },
}

/// Serializes a JSON value as a JS literal that is safe inside a Svelte `<script>` block.
fn js_literal(value: &Value) -> String {
    let text = value.to_string();
    text.replace("</", "<\\/").replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029")
}

fn expr_fn(e: &ClientExprIR) -> String {
    format!("(rec, page) => {}", e.js)
}

struct Emitter {
    design: bool,
    /// JSON literal of each registered control, by index.
    controls: Vec<String>,
    /// Expression table entries: control index -> JS object literal.
    exprs: Vec<(usize, String)>,
    /// `$derived` declarations for top-level visibility gates.
    derived: Vec<String>,
    used: BTreeSet<&'static str>,
    markup: String,
}

impl Emitter {
    fn line(&mut self, depth: usize, text: &str) {
        for _ in 0..depth {
            self.markup.push_str("  ");
        }
        self.markup.push_str(text);
        self.markup.push('\n');
    }

    fn use_component(&mut self, name: &'static str) -> &'static str {
        self.used.insert(name);
        name
    }

    /// Registers a control in `C` (without children and expressions) and its expressions in `X`.
    fn register(&mut self, c: &ResolvedControlIR) -> Result<usize, String> {
        let idx = self.controls.len();
        let mut value = serde_json::to_value(c).map_err(|e| e.to_string())?;
        if let Value::Object(map) = &mut value {
            for key in ["children", "show_when", "enable_when", "editable_when", "style_expr"] {
                map.remove(key);
            }
            if !self.design
                && let Some(Value::Object(field)) = map.get_mut("field")
            {
                field.remove("auto_filled");
            }
            if let Some(Value::Object(part)) = map.get_mut("part") {
                part.remove("link");
            }
            map.insert("i".to_string(), Value::from(idx));
        }
        self.controls.push(js_literal(&value));

        let mut entries = Vec::new();
        for (key, e) in [("show", &c.show_when), ("enable", &c.enable_when), ("editable", &c.editable_when), ("style", &c.style_expr)] {
            if let Some(e) = e {
                entries.push(format!("{key}: {}", expr_fn(e)));
            }
        }
        if let Some(part) = &c.part
            && !part.link.is_empty()
        {
            let links: Vec<String> = part
                .link
                .iter()
                .map(|l| format!("{{ field: {}, value: {} }}", js_literal(&Value::from(l.field.clone())), expr_fn(&l.value)))
                .collect();
            entries.push(format!("links: [{}]", links.join(", ")));
        }
        if !entries.is_empty() {
            self.exprs.push((idx, format!("{{ {} }}", entries.join(", "))));
        }
        Ok(idx)
    }

    fn component_for(&mut self, c: &ResolvedControlIR) -> &'static str {
        use PageControlKind as K;
        let name = match PageControlKind::parse(&c.kind) {
            K::Area if c.name.eq_ignore_ascii_case("FactBoxes") => "FactBoxes",
            K::Area => "Area",
            K::Group | K::Tab => "Group",
            K::Grid | K::Fixed => "Grid",
            K::Card => "Card",
            K::Flex => "Flex",
            K::CueGroup => "CueGroup",
            K::Cue => "Cue",
            K::Field => "Field",
            K::Label => "Label",
            K::Image => "Image",
            K::Spacer => "Spacer",
            K::Divider => "Divider",
            K::Headline => "Headline",
            K::UserControl => "UserControl",
            K::Part => "Part",
            K::SystemPart => "SystemPart",
            K::Repeater => "Repeater",
            K::Tabs => "Tabs",
            K::Split => "Split",
            K::WizardStep => "WizardSteps",
            _ => "Container",
        };
        self.use_component(name)
    }

    fn emit_children(&mut self, children: &[ResolvedControlIR], depth: usize) -> Result<(), String> {
        let mut i = 0;
        while i < children.len() {
            // Consecutive wizard steps form one step navigator.
            if PageControlKind::parse(&children[i].kind) == PageControlKind::WizardStep {
                let start = i;
                while i < children.len() && PageControlKind::parse(&children[i].kind) == PageControlKind::WizardStep {
                    i += 1;
                }
                self.emit_wizard(&children[start..i], depth)?;
                continue;
            }
            self.emit_control(&children[i], depth)?;
            i += 1;
        }
        Ok(())
    }

    fn visible_children<'c>(&self, children: &'c [ResolvedControlIR]) -> Vec<&'c ResolvedControlIR> {
        children.iter().filter(|c| self.design || c.visible).collect()
    }

    /// `{ c, body }` entry of a tab / wizard step / split pane; conditional ones carry `visible`.
    fn snippet_entry(&mut self, c: &ResolvedControlIR, idx: usize, snippet: &str) -> String {
        if c.show_when.is_some() {
            self.derived.push(format!("const v{idx} = $derived(fx.truthy(X[{idx}].show(p.rec, p.vars)));"));
            format!("{{ c: C[{idx}], body: {snippet}, visible: v{idx} }}")
        } else {
            format!("{{ c: C[{idx}], body: {snippet} }}")
        }
    }

    /// Opens a visibility gate for top-level conditional controls. Returns whether a block was opened.
    fn open_gate(&mut self, c: &ResolvedControlIR, idx: usize, depth: usize) -> bool {
        if c.show_when.is_none() {
            return false;
        }
        self.derived.push(format!("const v{idx} = $derived(fx.truthy(X[{idx}].show(p.rec, p.vars)));"));
        if self.design {
            // The designer keeps every node on the canvas; the kit dims it when `visible` is false.
            return false;
        }
        self.line(depth, &format!("{{#if v{idx}}}"));
        true
    }

    fn visible_attr(&self, c: &ResolvedControlIR, idx: usize) -> String {
        if self.design && c.show_when.is_some() { format!(" visible={{v{idx}}}") } else { String::new() }
    }

    fn emit_snippet(&mut self, name: &str, body: &[ResolvedControlIR], depth: usize) -> Result<(), String> {
        self.line(depth, &format!("{{#snippet {name}()}}"));
        self.emit_children(body, depth + 1)?;
        self.line(depth, "{/snippet}");
        Ok(())
    }

    fn emit_wizard(&mut self, steps: &[ResolvedControlIR], depth: usize) -> Result<(), String> {
        let comp = self.use_component("WizardSteps");
        let mut entries = Vec::new();
        for step in self.visible_children(steps) {
            let idx = self.register(step)?;
            let snippet = format!("s{idx}");
            self.emit_snippet(&snippet, &step.children, depth)?;
            let entry = self.snippet_entry(step, idx, &snippet);
            entries.push(entry);
        }
        if !entries.is_empty() {
            self.line(depth, &format!("<{comp} {{p}} steps={{[{}]}} />", entries.join(", ")));
        }
        Ok(())
    }

    fn collect_columns(&mut self, controls: &[ResolvedControlIR], out: &mut Vec<usize>) -> Result<(), String> {
        for c in controls {
            if !self.design && !c.visible {
                continue;
            }
            match PageControlKind::parse(&c.kind) {
                PageControlKind::Field => out.push(self.register(c)?),
                _ => self.collect_columns(&c.children, out)?,
            }
        }
        Ok(())
    }

    fn emit_control(&mut self, c: &ResolvedControlIR, depth: usize) -> Result<(), String> {
        if !self.design && !c.visible {
            return Ok(());
        }
        let idx = self.register(c)?;
        let gated = self.open_gate(c, idx, depth);
        let d = if gated { depth + 1 } else { depth };
        let vis = self.visible_attr(c, idx);
        let kind = PageControlKind::parse(&c.kind);

        match kind {
            PageControlKind::Repeater => {
                let comp = self.use_component("Repeater");
                let mut cols = Vec::new();
                self.collect_columns(&c.children, &mut cols)?;
                let columns: Vec<String> = cols.iter().map(|i| format!("C[{i}]")).collect();
                self.line(d, &format!("<{comp} {{p}} c={{C[{idx}]}}{vis} columns={{[{}]}} />", columns.join(", ")));
            }
            PageControlKind::Tabs => {
                let comp = self.use_component("Tabs");
                let mut entries = Vec::new();
                for tab in self.visible_children(&c.children) {
                    let tab_idx = self.register(tab)?;
                    let snippet = format!("s{tab_idx}");
                    self.emit_snippet(&snippet, &tab.children, d)?;
                    let entry = self.snippet_entry(tab, tab_idx, &snippet);
                    entries.push(entry);
                }
                self.line(d, &format!("<{comp} {{p}} c={{C[{idx}]}}{vis} tabs={{[{}]}} />", entries.join(", ")));
            }
            PageControlKind::Split => {
                let comp = self.use_component("Split");
                let mut panes = Vec::new();
                for pane in self.visible_children(&c.children) {
                    let pane_idx = self.register(pane)?;
                    let snippet = format!("s{pane_idx}");
                    // A pane renders as its own control (keeps its caption, style and children).
                    self.line(d, &format!("{{#snippet {snippet}()}}"));
                    self.emit_pane(pane, pane_idx, d + 1)?;
                    self.line(d, "{/snippet}");
                    let entry = self.snippet_entry(pane, pane_idx, &snippet);
                    panes.push(entry);
                }
                self.line(d, &format!("<{comp} {{p}} c={{C[{idx}]}}{vis} panes={{[{}]}} />", panes.join(", ")));
            }
            _ if c.children.is_empty() || !kind_has_children(&kind) => {
                let comp = self.component_for(c);
                self.line(d, &format!("<{comp} {{p}} c={{C[{idx}]}}{vis} />"));
            }
            _ => {
                let comp = self.component_for(c);
                self.line(d, &format!("<{comp} {{p}} c={{C[{idx}]}}{vis}>"));
                self.emit_children(&c.children, d + 1)?;
                self.line(d, &format!("</{comp}>"));
            }
        }

        if gated {
            self.line(depth, "{/if}");
        }
        Ok(())
    }

    /// Renders an already registered control as the body of a split pane.
    fn emit_pane(&mut self, pane: &ResolvedControlIR, idx: usize, depth: usize) -> Result<(), String> {
        let kind = PageControlKind::parse(&pane.kind);
        if kind_has_children(&kind) && !pane.children.is_empty() {
            let comp = self.component_for(pane);
            self.line(depth, &format!("<{comp} {{p}} c={{C[{idx}]}}>"));
            self.emit_children(&pane.children, depth + 1)?;
            self.line(depth, &format!("</{comp}>"));
        } else {
            let comp = self.component_for(pane);
            self.line(depth, &format!("<{comp} {{p}} c={{C[{idx}]}} />"));
        }
        Ok(())
    }
}

/// Kinds rendered as a component wrapping its children (unknown kinds render as a generic container).
fn kind_has_children(kind: &PageControlKind) -> bool {
    kind.is_container() || matches!(kind, PageControlKind::Other(_))
}

fn action_literal(a: &ResolvedActionIR) -> Result<String, String> {
    let mut value = serde_json::to_value(a).map_err(|e| e.to_string())?;
    let mut fns = Vec::new();
    if let Value::Object(map) = &mut value {
        map.remove("show_when");
        map.remove("enable_when");
        map.remove("run_page_link");
    }
    if let Some(e) = &a.show_when {
        fns.push(format!("show: {}", expr_fn(e)));
    }
    if let Some(e) = &a.enable_when {
        fns.push(format!("enable: {}", expr_fn(e)));
    }
    if !a.run_page_link.is_empty() {
        let links: Vec<String> = a
            .run_page_link
            .iter()
            .map(|l| format!("{{ field: {}, value: {} }}", js_literal(&Value::from(l.field.clone())), expr_fn(&l.value)))
            .collect();
        fns.push(format!("links: [{}]", links.join(", ")));
    }
    let base = js_literal(&value);
    Ok(if fns.is_empty() { base } else { format!("{{ ...{base}, {} }}", fns.join(", ")) })
}

/// Generates the Svelte component for a compiled page.
pub fn generate(page: &CompiledPageIR, opts: GenerateOptions) -> Result<GeneratedPage, CodegenError> {
    if !page.page_type.has_ui() {
        return Err(CodegenError::NoUserInterface(page.name.clone()));
    }
    let ser = |reason: String| CodegenError::Serialize { page: page.name.clone(), reason };

    let mut em = Emitter {
        design: opts.design,
        controls: Vec::new(),
        exprs: Vec::new(),
        derived: Vec::new(),
        used: BTreeSet::new(),
        markup: String::new(),
    };
    em.used.insert("createPage");
    let frame = em.use_component("PageFrame");
    em.line(0, &format!("<{frame} {{p}} actions={{A}}>"));
    em.emit_children(&page.controls, 1).map_err(ser)?;
    em.line(0, &format!("</{frame}>"));

    let actions = page.actions.iter().map(action_literal).collect::<Result<Vec<_>, _>>().map_err(ser)?;
    let uses_fx = em.exprs.iter().any(|(_, e)| e.contains("fx."))
        || actions.iter().any(|a| a.contains("fx."))
        || !em.derived.is_empty();
    if uses_fx {
        em.used.insert("fx");
    }

    let mut meta = serde_json::to_value(page).map_err(|e| ser(e.to_string()))?;
    if let Value::Object(map) = &mut meta {
        map.remove("controls");
        map.remove("actions");
        map.insert("codegen".to_string(), Value::from(CODEGEN_VERSION));
        map.insert("design".to_string(), Value::from(opts.design));
    }

    let imports: Vec<&'static str> = em.used.iter().copied().collect();
    let mut src = String::with_capacity(4096 + em.markup.len());
    src.push_str("<svelte:options runes={true} />\n\n<script>\n");
    let _ = writeln!(src, "  // Generated by bosc-codegen-svelte v{CODEGEN_VERSION} from page {}. Do not edit.", page.name);
    let _ = writeln!(src, "  import {{ {} }} from {};", imports.join(", "), js_literal(&Value::from(PAGE_KIT_MODULE)));
    src.push_str("\n  let { ctx } = $props();\n\n");
    let _ = writeln!(src, "  const META = Object.freeze({});", js_literal(&meta));
    src.push_str("  const C = Object.freeze([\n");
    for c in &em.controls {
        let _ = writeln!(src, "    {c},");
    }
    src.push_str("  ]);\n");
    if em.exprs.is_empty() {
        src.push_str("  const X = Object.freeze({});\n");
    } else {
        src.push_str("  const X = Object.freeze({\n");
        for (idx, entry) in &em.exprs {
            let _ = writeln!(src, "    {idx}: Object.freeze({entry}),");
        }
        src.push_str("  });\n");
    }
    if actions.is_empty() {
        src.push_str("  const A = Object.freeze([]);\n");
    } else {
        src.push_str("  const A = Object.freeze([\n");
        for a in &actions {
            let _ = writeln!(src, "    {a},");
        }
        src.push_str("  ]);\n");
    }
    src.push_str("\n  const p = createPage(() => ctx, META, C, X);\n");
    for d in &em.derived {
        let _ = writeln!(src, "  {d}");
    }
    src.push_str("</script>\n\n");
    src.push_str(&em.markup);


    Ok(GeneratedPage { name: page.name.clone(), source: src, imports })
}
