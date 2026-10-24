# bosc-codegen-svelte - agent notes

Scope: Generates one Svelte 5 (runes) component per CompiledPageIR. Metadata lives in frozen JS tables (META, C, A, X); no authored text in markup, no CSS classes; imports only from the page-kit module.
Owner: rust-developer. Specs: docs/11_svelte5_ui_studio_and_component_library.md, docs/17_page_compiler_and_designer.md.

## Gotchas
- Contract with `ui/src/lib/page-kit` (createPage, layout components). Change both sides together and re-run golden tests + `npm run test:page-kit`.

## Verify
`cargo test -p bosc-codegen-svelte` then `cargo clippy -p bosc-codegen-svelte`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

