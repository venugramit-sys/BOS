# bosc-codegen-db - agent notes

Scope: PostgreSQL code generation from IR: DDL (`PostgresGenerator`), schema diff/migration planner (safe vs destructive steps), relations -> real FKs, codeunit -> PL/pgSQL pushdown (AMDP-style).
Owner: rust-developer. Specs: docs/04_database_engine_and_storage_strategies.md, docs/05_schema_sync_and_declarative_migrations.md, docs/01.

## Gotchas
- Destructive migration steps must stay explicitly classified. Generated SQL must respect the tenancy strategy (StorageStrategyIR).

## Verify
`cargo test -p bosc-codegen-db` then `cargo clippy -p bosc-codegen-db`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

