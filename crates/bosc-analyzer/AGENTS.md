# bosc-analyzer - agent notes

Scope: Semantic analysis: AST -> IR (`SemanticAnalyzer`), page lowering/merging/resolution, page source outline + printer for the designer, client page expressions, TS stripping (+ Oxc parse), script optimizer, AOT QuickJS bytecode compiler.
Owner: rust-developer. Specs: docs/01, docs/02, docs/03_compiler_and_ir_architecture.md, docs/17_page_compiler_and_designer.md, docs/16 (AOT bytecode).

## Gotchas
- Depends on bosc-syntax, bosc-ir, bosc-codegen-db. Printer round-trip test covers every page in the repo.

## Verify
`cargo test -p bosc-analyzer` then `cargo clippy -p bosc-analyzer`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

