# bosc-runtime - agent notes

Scope: BosEngine and Axum server: DB manager and sessions, schema sync, meta manager, package manager + vault, QuickJS engine (sandbox), WASM engine, event engine, execution engine, page artifacts/compiler/designer/routes, node config, notifications, sidecar host, NAS worker, bytecode and prepared-statement caches.
Owner: rust-developer. Specs: docs/04-08 (DB, schema sync, QuickJS, events/WASM/sidecars, API server), docs/10, docs/13 (node/NAS), docs/15 (notifications), docs/16, docs/17.

## Gotchas
- Security-sensitive: auth, tenancy/RLS, CORS live here (see reports/cto_proposals_d1_d2_d3_d9.md). Integration tests in `tests/` need a real PostgreSQL.

## Verify
`cargo test -p bosc-runtime (integration tests need DATABASE_URL / local Postgres)` then `cargo clippy -p bosc-runtime`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

