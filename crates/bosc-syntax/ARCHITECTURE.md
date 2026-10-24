# bosc-syntax architecture

## Purpose
Lexer, tokens, AST and recursive-descent parser for BOS-Script (`.bos`, `.bl.bos`). No dependencies besides serde.

## Map
`src/token.rs` tokens; `src/lexer.rs` Lexer; `src/ast.rs` AST types; `src/parser.rs` (~166 KB, read by range) Parser; `src/lib.rs` re-exports + parser tests.

## Contracts / constraints
Grammar changes ripple to analyzer, IR and page printer round-trip tests (`bosc-analyzer/src/page_print_tests.rs`).

## Specs
docs/01_bos_script_language_reference.md, docs/02_meta_objects_specification.md

_Keep current: update when modules, public types or dependencies change._

