# BOS Platform Readiness Review (CTO)

Date: 2026-10-24 (IST). Author: CTO agent. Scope: read-only review of `D:\Work Studio\Bos` on branch `fix-iter1-security` (working tree has uncommitted changes from the Backend Engineer; see section 0). No code was modified or built.

Evidence method: directory listings, PowerShell `Select-String` scans over `crates/` (57 `.rs` files, ~46.5k lines), targeted reads, `reports/qa_baseline_iter0.md`. `cargo test` was **not** run in this review.

## 0. Branch / in-flight work
- Branches: `fix-iter1-security` (current), `main`, `origin/main`. Only 4 commits total (`eae757a1`, `0a467993`, `151fd6ab`, `11f2266e` pages hardening).
- Uncommitted WIP on `fix-iter1-security` touches `auth_guard`/`server.rs` (session middleware), `db_session.rs`, `quickjs_engine.rs`, `package_manager.rs`, `packages/bos.auth/*`, and UI shell/sign-in. I treat auth + RLS fixes as **owned by the Backend Engineer** and only list residual architectural risks below, not duplicate tasks.
- Also staged: generated files under `data/page-artifacts/` (build outputs should not be committed; see roadmap I0).

## 1. Core area status

| Area | Status | Evidence | Notes |
|---|---|---|---|
| Compiler (`bosc-syntax`, `bosc-analyzer`, `bosc-ir`, codegen) | **Partial (strong)** | syntax 5.3k lines, analyzer 11k, IR 2.4k, codegen-db 2.6k, codegen-svelte 0.6k + `tests/golden.rs`; 0 `todo!`/`unimplemented!` | Page pipeline is the most mature part (docs/17). No `tests/` dirs for syntax/analyzer/ir (unit tests inline only). No formal language version / IR version stamp found. |
| Runtime (Axum, QuickJS, wasmi, events) | **Partial** | `crates/bosc-runtime/src/` (28 files, 23k lines): `server.rs`, `execution_engine.rs`, `quickjs_engine.rs`, `wasm_engine.rs`, `event_engine.rs` (`_bos_event_outbox`), `nas_worker.rs`, `sidecar_host.rs` | 83 `.route(` calls; unversioned `/api/...` mixed with `/api/v1/pages`. QA F2 (trigger ctx lacks `audit`), F4 (aborted-tx masking), F5 (`/api/query` `jsonb || json` bug). 137 `.unwrap()` + 475 `.expect(` across crates (many in tests; needs audit for request paths). |
| DB & tenancy | **Partial** | RLS DDL `bosc-codegen-db/src/lib.rs:896-905`; GUCs `db_session.rs:68` (`set_config(... , true)` tx-local, good); `StorageStrategyIR` in `bosc-ir/src/lib.rs:58` | **RLS is not effective as configured**: `bos-node.toml` connects as `postgres` (superuser, bypasses RLS) and no `FORCE ROW LEVEL SECURITY` exists anywhere, so table owners bypass too. Strategy names disagree (IR: shared_rls/partitioned/schema_per_company/table_per_company vs toml comment: schema_per_tenant/db_per_tenant). |
| Security / auth | **Partial (WIP)** | `auth_guard.rs` (`require_session`, `is_public_route`), argon2 in `package_manager.rs`, `server.rs:440 is_superuser_roles` | Residual design risks in WIP: tenant chosen from client header `x-bos-tenant-id` else "first active tenant" (`auth_guard.rs` `default_tenant`); `resolve_session` returns `user_id: Uuid::nil()` (no user identity flows into RLS/audit); session lookup runs with `roles: ["SUPER"]`; emergency in-memory sessions + `auto_fallback_on_auth_failure = true` default (`node_config.rs:277`); `cors_origins = ["*"]`; DB creds in plaintext toml. QA F3 (all data APIs anonymous) is on the running build. |
| Packaging | **Partial** | `package_manager.rs`, `package_vault.rs`, `packages/bos.auth/` (`bos.json`, `package.bos`, `.bospkg`) | `version` is a free string, upserted on install (`package_manager.rs:239`); no semver parsing, no dependency/compat resolution, no upgrade/migration hooks, no signing. Only one real package. |
| Extensibility | **Partial** | `event_engine.rs`, `wasm_engine.rs`, `sidecar_host.rs`, `tableextension`/`pageextension` in docs/02, docs/17 extension directives | Mechanisms exist; no stable public contract (event names/payload schema versioning, extension ordering, conflict rules across packages). |
| Studio UI (Svelte 5) | **Partial** | `ui/src` 242 files: `bos-ui`, `page-kit`, `api-studio`, `components/studio/designers/*`; 4 Vite entries | QA F1 blocks all post-login screens; F6 null-user crash; F13 no 404. Two icon libs (`@lucide/svelte` + `lucide-svelte`); `pg` in UI devDeps. |
| Testing | **Partial** | 177 `#[test]` + 10 `#[tokio::test]`; `bosc-runtime/tests/` (incl. `transactional_integrity.rs`, `page_service.rs`); 6 UI `*.test.js`, only `test:page-kit` wired in `ui/package.json` | No CI (`.github` absent). Root PS scripts stale (QA F7). No tenant-isolation test suite, no API contract tests, Playwright not installed (F16). docs/18, 19 referenced but missing. |
| Performance | **Missing (as a practice)** | No `benches/` anywhere; `bytecode_cache.rs`, `prepared_statement_registry.rs` exist | QA F9: 2-4 s cold endpoints. No load test, no budgets, no metrics (only `TraceLayer`). |
| Dev experience | **Partial** | `run.ps1`, `run.bat`, `Dockerfile`, `docker-compose.yml`, `Caddyfile`, `.agents/*.md` roles | Repo hygiene: `target`, `target-ledger`, `target-verify`, `scratch/`, `dist/`, `data/` in tree; README links point to `stale-docs` filenames; no CI, no lint/format gates, no ADR folder. |

Zero-mock policy: scan for `mock` found 1 hit; 0 `todo!`/`unimplemented!`/`TODO`. Compliant on the surface.

## 2. Gaps that block module (package) development
1. **No working identity bootstrap** (QA F1/F2): nobody can sign in; role insert fails in `role.onInsert` (missing `audit` in trigger ctx). Package devs cannot exercise permissions. *(Backend Engineer WIP.)*
2. **Tenant isolation is not actually enforced**: superuser DB connection + no `FORCE ROW LEVEL SECURITY` + client-selected tenant header + nil user id. Any module built now would bake in an unsafe assumption.
3. **No package contract**: no semver, dependency resolution, upgrade/data-migration hooks, or compatibility policy between core and packages. Modules written now cannot be safely upgraded.
4. **Unstable, unversioned API & error envelope**: mixed `/api` and `/api/v1`, plaintext 4xx bodies (F10), broken `/api/query` (F5), raw DB errors leaked (F4).
5. **No CI / regression net**: no automated `cargo test` + UI tests + isolation tests on every change; test scripts stale.
6. **Unsettled foundational types**: money/decimal semantics across DB (`NUMERIC(p,s)`), QuickJS (`ctx.decimal`) and JSON; ID strategy (`gen_random_uuid` = v4).

## 3. Decisions to settle early (irreversible or expensive to change)
Sign-off legend: **[SIGN-OFF]** = needs CEO / Md Aziz approval.

### D1. Tenancy default **[SIGN-OFF]**
- Options: (a) shared schema + RLS (`_tenant_id` on every table); (b) schema-per-tenant; (c) DB-per-tenant; (d) hybrid: shared RLS default, dedicated DB for large/regulated tenants.
- Tradeoffs: (a) cheapest, simplest migrations, one pool, but a single bug leaks data and noisy neighbours; (b) better isolation but migrations x N schemas, catalog bloat past ~thousands; (c) strongest isolation/backup/residency, highest cost and ops.
- **Recommendation: (d)** shared-RLS by default with a routing layer (tenant -> connection) so a tenant can be moved to its own DB later. Keep `_tenant_id` on all rows even in dedicated DBs so the move is a copy, not a rewrite. Drop `partitioned` / `table_per_company` from scope for now; keep `schema_per_company` out until a customer needs it. Enforce: runtime connects as a non-superuser, non-owner role; `FORCE ROW LEVEL SECURITY` on all tenant tables; migrations run as a separate owner role.

### D2. Auth model **[SIGN-OFF]**
- Options: (a) opaque server sessions in DB (current direction); (b) stateless JWT; (c) external IdP (OIDC) with local sessions.
- **Recommendation:** (a) opaque, hashed session tokens stored server-side, tenant and user bound to the session (never taken from a client header), plus OIDC/SAML as a later add-on for enterprise SSO. Remove `auto_fallback_on_auth_failure` default=true; emergency access only via CLI on the host, time-boxed and audited. API keys for integrations as a separate, scoped credential.

### D3. Package versioning & compatibility **[SIGN-OFF]**
- Options: free string (today) vs SemVer with `engine` range + dependency ranges vs calendar versions.
- **Recommendation:** SemVer for packages, `bos_engine = ">=x.y <z"` in `bos.json`, dependency ranges resolved at install, upgrade hooks (`onUpgrade(from,to)`), and a rule that table/field IDs and names are stable public contract (renames only via `renamedFrom`, already supported in `schema_sync.rs`). Sign packages later (checksum exists).

### D4. API contract & versioning
- **Recommendation:** all public routes under `/api/v1`; one JSON error envelope (`code`, `message`, `field_errors`, `trace_id`) for every non-2xx, including extractor errors; OpenAPI generated from code and contract-tested; internal/studio routes under `/api/_studio` and excluded from the stability promise.

### D5. Identifiers
- **Recommendation:** UUIDv7 (time-ordered, better index locality) for all system-generated PKs instead of v4 `gen_random_uuid()`; human document numbers via Number Series (docs/13 concept). Cheap now, costly after data exists.

### D6. Decimal / money **[SIGN-OFF on currency rules]**
- **Recommendation:** money never as float anywhere; JSON transports decimals as strings; one fixed scale policy per field type (e.g. amount NUMERIC(19,4), unit price higher scale), rounding mode per currency, currency code stored with amounts. Add round-trip tests DB <-> Rust <-> QuickJS `ctx.decimal` <-> UI.

### D7. Events / outbox
- Present: `_bos_event_outbox` with `enqueue_outbox_in` inside the unit of work (good).
- **Recommendation:** commit to at-least-once delivery with idempotency keys, versioned event schemas (`name@v1`), per-tenant ordering key, retry/dead-letter states. Document as a public contract before modules subscribe.

### D8. Migrations
- Present: declarative `schema_sync.rs` (diff, `renamedFrom`, destructive steps refused unless allowed).
- **Recommendation:** keep declarative for package schemas but record every applied plan in a migration ledger table (hash, package, version, applied_by, time); require expand/contract for destructive changes; dry-run plan surfaced in studio; for multi-tenant, migrations run once per DB (D1 routing), never per request.

### D9. Runtime DB role & secrets **[SIGN-OFF]** (ops)
- **Recommendation:** separate roles `bos_owner` (DDL), `bos_app` (DML, RLS enforced), `bos_readonly`; secrets from environment/secret store, not committed toml; CORS allow-list.

## 4. Roadmap (small iterations, in order)

| # | Goal | Scope | Acceptance criteria | Owner |
|---|---|---|---|---|
| I0 | Repo hygiene & CI | `.gitignore` build outputs (`target*`, `data/page-artifacts`, `scratch`, `dist`); CI running `cargo fmt --check`, `clippy`, `cargo test --workspace`, UI tests | CI green on `main`; no generated files tracked | cloud-deployer + rust-developer |
| I1 | Finish auth & bootstrap (in flight) | Backend Engineer's `fix-iter1-security`: first-run bootstrap, `role.onInsert` audit ctx (F2), session middleware (F3) | QA F1, F2, F3 closed with repro tests; anonymous calls to data APIs return 401 | rust-developer (Backend Engineer) |
| I2 | Real tenant isolation | Non-superuser `bos_app` role, `FORCE ROW LEVEL SECURITY`, tenant/user taken from session not header, non-nil `user_id` | Automated test: tenant A token cannot read/write tenant B rows on every table/route; test fails if app connects as superuser | rust-developer + tester |
| I3 | API contract v1 | `/api/v1` prefix, unified error envelope, fix F4/F5/F8/F10/F11, OpenAPI output | Contract tests per endpoint; no raw DB/serde text in any 4xx/5xx | rust-developer |
| I4 | Decide & encode D1-D9 | ADRs in `docs/adr/`; UUIDv7, decimal policy, outbox semantics | Signed-off ADRs; decimal round-trip tests; IDs v7 | planner/CTO, sign-off CEO + Md Aziz |
| I5 | Package contract | SemVer, engine range, dependency resolution, upgrade hooks, migration ledger | Install/upgrade/downgrade-refusal tests on `bos.auth` + one sample package | rust-developer + package-developer |
| I6 | Studio usable end-to-end | F6, F13, F15, null-safe shell, Playwright in setup | Playwright smoke: login -> studio -> page designer -> save, green in CI | svelte-developer + tester |
| I7 | Observability & perf baseline | Metrics, request ids, slow-query log; `benches/` for compiler and CRUD | Published p95 budgets (e.g. CRUD < 50 ms warm); cold-start issue F9 explained/fixed | rust-developer + cloud-deployer |
| I8 | Docs re-sync | Reconcile README/docs/stale-docs; add docs/18, 19 or remove links | Every README link resolves; drift list below closed | planner + detector |
| I9 | Pilot module | One small business package (e.g. Customer/Item master) on the hardened core | Built only via public contracts; isolation, upgrade and contract tests pass | package-developer |

Module/package development should start at I9, after I1-I5 are done.

## 5. Docs vs code drift
- `README.md` links `docs/01_bos_script_specification.md`, `02_rust_compiler_architecture.md`, `11_adaptive_multi_tenant_storage_strategies.md`, `18_...`, `19_...` etc.; these names live in `stale-docs/` (or nowhere for 18/19), not `docs/`. README links also use absolute `file:///d:/...` paths.
- Storage strategy names: `docs/README.md` / docs/04 say 4 models; IR implements `shared_rls|partitioned|schema_per_company|table_per_company`; `bos-node.toml` comment says `shared_rls|schema_per_tenant|db_per_tenant`. None of schema_per_tenant/db_per_tenant exists in code.
- docs/08 says session middleware protects routes, but the running build allowed anonymous access (QA F3); fix is uncommitted WIP.
- docs/06 claims "exact 128-bit monetary math"; QuickJS uses a JS `ctx.decimal` helper (`quickjs_engine.rs:1440`) - verify precision claims with tests before relying on them.
- `stale-docs/06` and `08` claim "100% verified" reports; current QA baseline contradicts parts (F2, F4, F5). Stale docs should be archived with a banner, not linked.
- Test scripts (`test_security_api.ps1`, `test_advanced_features.ps1`) use `sec_*`/`customer` tables from the old spec (QA F7).
