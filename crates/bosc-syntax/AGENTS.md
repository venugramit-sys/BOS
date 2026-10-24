# bosc-syntax - agent notes

Scope: Lexer, tokens, AST and recursive-descent parser for BOS-Script (`.bos`, `.bl.bos`). No dependencies besides serde.
Owner: rust-developer. Specs: docs/01_bos_script_language_reference.md, docs/02_meta_objects_specification.md.

## Gotchas
- Grammar changes ripple to analyzer, IR and page printer round-trip tests (`bosc-analyzer/src/page_print_tests.rs`).

## Verify
`cargo test -p bosc-syntax` then `cargo clippy -p bosc-syntax`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

