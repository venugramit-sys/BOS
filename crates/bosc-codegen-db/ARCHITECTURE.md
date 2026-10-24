# bosc-codegen-db architecture

## Purpose
PostgreSQL code generation from IR: DDL (`PostgresGenerator`), schema diff/migration planner (safe vs destructive steps), relations -> real FKs, codeunit -> PL/pgSQL pushdown (AMDP-style).

## Map
`src/lib.rs` PostgresGenerator; `src/migration.rs` diff planner; `src/relations.rs`; `src/pushdown.rs`.

## Contracts / constraints
Destructive migration steps must stay explicitly classified. Generated SQL must respect the tenancy strategy (StorageStrategyIR).

## Specs
docs/04_database_engine_and_storage_strategies.md, docs/05_schema_sync_and_declarative_migrations.md, docs/01

_Keep current: update when modules, public types or dependencies change._

