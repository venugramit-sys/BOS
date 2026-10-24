# bosc-codegen-svelte architecture

## Purpose
Generates one Svelte 5 (runes) component per CompiledPageIR. Metadata lives in frozen JS tables (META, C, A, X); no authored text in markup, no CSS classes; imports only from the page-kit module.

## Map
`src/lib.rs` generate/GenerateOptions/GeneratedPage; page-kit contract documented in the module header; `tests/golden.rs` golden output tests.

## Contracts / constraints
Contract with `ui/src/lib/page-kit` (createPage, layout components). Change both sides together and re-run golden tests + `npm run test:page-kit`.

## Specs
docs/11_svelte5_ui_studio_and_component_library.md, docs/17_page_compiler_and_designer.md

_Keep current: update when modules, public types or dependencies change._

