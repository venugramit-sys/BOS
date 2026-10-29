# BOS code map

One-page orientation for agents. Detail: each crate's ARCHITECTURE.md and docs/00_architecture_overview.md.
Keep current: update this file and the crate ARCHITECTURE.md in the same commit whenever a crate, module, public type or dependency edge changes.

## Pipeline
`.bos` source -> **bosc-syntax** (lexer, parser, AST) -> **bosc-analyzer** (semantic analysis, page lowering/merge/resolve, TS strip, bytecode) -> **bosc-ir** (ModelIR, PackageBundleIR, PageIR/CompiledPageIR)
-> **bosc-codegen-db** (PostgreSQL DDL, migrations, FKs, PL/pgSQL pushdown) and **bosc-codegen-svelte** (Svelte 5 page components)
-> **bosc-runtime** (BosEngine, Axum API, DB manager, schema sync, packages + vault, QuickJS/WASM, events, page service/designer)
-> **bosc-cli** (`bosc` binary). UI: **ui/** (Studio, bos-ui, page-kit runtime for generated pages). Packages: **packages/** (e.g. bos.core).

## Dependency edges (Cargo)
- syntax: none
- ir: syntax
- codegen-db: ir
- analyzer: syntax, ir, codegen-db
- codegen-svelte: ir, syntax, analyzer
- runtime: syntax, analyzer, ir, codegen-db, codegen-svelte
- cli: syntax, analyzer, ir, codegen-db, runtime

## Where to look
| Need | Place |
|---|---|
| Grammar / parse error | crates/bosc-syntax/src/parser.rs |
| Type checks, meta-object lowering | crates/bosc-analyzer/src/lib.rs |
| Page extensions / resolution | bosc-analyzer page_merger.rs, page_resolver.rs |
| IR data contract | crates/bosc-ir/src/lib.rs, page.rs |
| SQL / migrations | crates/bosc-codegen-db/src/lib.rs, migration.rs |
| HTTP routes | crates/bosc-runtime/src/server.rs, page_routes.rs, page_designer.rs |
| Auth, sessions, DB pools | bosc-runtime lib.rs, db_manager.rs, db_session.rs |
| Packages install/vault | bosc-runtime package_manager.rs, package_vault.rs |
| Script sandbox | bosc-runtime quickjs_engine.rs |
| CLI commands | crates/bosc-cli/src/main.rs |
| Page-kit contract | crates/bosc-codegen-svelte/src/lib.rs header + ui/src/lib/page-kit |

## Ignore
target*/, dist/, node_modules/, stale-docs/ (archive), scratch/, Cargo.lock.
