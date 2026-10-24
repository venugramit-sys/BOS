# bosc-analyzer architecture

## Purpose
Semantic analysis: AST -> IR (`SemanticAnalyzer`), page lowering/merging/resolution, page source outline + printer for the designer, client page expressions, TS stripping (+ Oxc parse), script optimizer, AOT QuickJS bytecode compiler.

## Map
`src/lib.rs` (~138 KB) SemanticAnalyzer/AnalyzerError; `page_lower.rs`, `page_merger.rs` (pageextension), `page_resolver.rs` -> CompiledPageIR; `page_source.rs`/`page_print.rs` designer edits + printer; `page_expr.rs`; `ts_stripper.rs`, `ts_oxc.rs`, `script_opt.rs`, `bytecode_compiler.rs`.

## Contracts / constraints
Depends on bosc-syntax, bosc-ir, bosc-codegen-db. Printer round-trip test covers every page in the repo.

## Specs
docs/01, docs/02, docs/03_compiler_and_ir_architecture.md, docs/17_page_compiler_and_designer.md, docs/16 (AOT bytecode)

_Keep current: update when modules, public types or dependencies change._

