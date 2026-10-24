---
name: bos-schema-sync
description: Preview and apply BOS schema changes (deploy / schema sync / migrations) safely. Use when tables, fields, keys or relations change.
---

# bos-schema-sync

1. Read crates/bosc-codegen-db/ARCHITECTURE.md and docs/05_schema_sync_and_declarative_migrations.md (relevant section).
2. Dry run first: `cargo run -p bosc-cli -- deploy -i <schema.bos> --dry-run` and read the planned steps; classify safe vs destructive.
3. Destructive steps (drop/rename/type narrowing) need explicit owner approval; never pass `--allow-destructive` on your own.
4. Apply to a LOCAL dev DB only: `... deploy -i <schema.bos> -d <local dev url>`.
5. Run `cargo test -p bosc-codegen-db` and, if runtime changed, `cargo test -p bosc-runtime`.
6. Update docs/05 or docs/04 if behaviour changed; report steps applied.
