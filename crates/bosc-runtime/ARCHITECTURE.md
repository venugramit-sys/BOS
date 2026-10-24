# bosc-runtime architecture

## Purpose
BosEngine and Axum server: DB manager and sessions, schema sync, meta manager, package manager + vault, QuickJS engine (sandbox), WASM engine, event engine, execution engine, page artifacts/compiler/designer/routes, node config, notifications, sidecar host, NAS worker, bytecode and prepared-statement caches.

## Map
`src/lib.rs` (~170 KB) BosEngine, SessionContext, password hashing; `server.rs` routes (/api/health, /api/auth/*, /api/setup/*, /api/meta/*, /api/pages/*, /api/v1/pages/*); `db_manager.rs`, `db_session.rs` (one tx per unit of work); `schema_sync.rs`; `meta_manager.rs` (~130 KB); `package_manager.rs`, `package_vault.rs`; `quickjs_engine.rs` (~195 KB); `page_*.rs`.

## Contracts / constraints
Security-sensitive: auth, tenancy/RLS, CORS live here (see reports/cto_proposals_d1_d2_d3_d9.md). Integration tests in `tests/` need a real PostgreSQL.

## Specs
docs/04-08 (DB, schema sync, QuickJS, events/WASM/sidecars, API server), docs/10, docs/13 (node/NAS), docs/15 (notifications), docs/16, docs/17

_Keep current: update when modules, public types or dependencies change._

