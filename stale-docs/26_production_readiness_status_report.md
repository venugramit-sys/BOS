# 26. Production Readiness & Product Status Report

**Date:** 2026-10-15
**Scope:** Whole workspace: `crates/` (6 crates, ~21k lines Rust), `ui/src` (159 files, ~23k lines), `packages/` (4 packages), `docs/`, root scripts.
**Method:** Static deep-read of every crate, the UI and all packages, cross-checked against `docs/`; `cargo test --workspace` and `cargo clippy --workspace` executed; critical findings re-verified by hand.

---

## 1. Verdict

**BOS is NOT production ready.** It is a promising, real (not mocked) **ERP framework kernel** — compiler, Postgres runtime, and developer studio — but it is not yet a product a business can run on.

Three independent blockers, any one of which stops a production launch:

1. **No security boundary.** Every API route is unauthenticated, identity comes from client-supplied headers, and a missing `x-roles` header defaults to `Admin`.
2. **No business application.** There are zero ERP domain packages (no GL, customers, vendors, items, sales, purchasing, posting). Only auth/security and two toy test packages exist.
3. **No end-user UI.** `page` meta-objects are parsed and designable but never rendered. The business shell opens placeholder tabs.


| Area                                       | Readiness | One-line status                                                          |
| ------------------------------------------ | --------- | ------------------------------------------------------------------------ |
| Lexer / parser (`bosc-syntax`)             | **75%**   | All declared meta-objects parse; no error recovery                       |
| Analyzer / IR (`bosc-analyzer`, `bosc-ir`) | **50%**   | Broad lowering; type checking is superficial; TS is regex-stripped       |
| DB codegen (`bosc-codegen-db`)             | **65%**   | Table/view/index/RLS DDL, enforced FKs, migration planner; Postgres-only |
| Runtime engine (`bosc-runtime`)            | **70%**   | Transactional units of work, mandatory OCC, schema sync, fuel metering   |
| API server security                        | **5%**    | Unauthenticated, permissive CORS, SQL injection surfaces                 |
| Developer Studio UI                        | **70%**   | Live-backed designers + Monaco; solid Svelte 5                           |
| End-user ERP UI                            | **~3%**   | No metadata-driven page renderer                                         |
| Business packages                          | **5%**    | Auth/security only; auth codeunits don't authenticate                    |
| Ops / engineering hygiene                  | **10%**   | No git, CI, Docker, LICENSE, metrics, graceful shutdown                  |
| Docs accuracy                              | **40%**   | Many "FIXED / 100% verified / zero mocks" claims are overstated          |


Build health: `cargo test --workspace` → **44 passed, 0 failed** (syntax 5, analyzer 11, codegen 4, runtime 24, ir 0, cli 0). `cargo clippy` → **104 warnings**, all style-level (collapsible `if`, too many arguments), no errors. No `todo!()`/`unimplemented!()` anywhere. Note that 44 unit tests for a ~44k-line platform is thin, and there are no integration tests against a live database in the test suite.

*Update after the §3.2 remediation:* `cargo test --workspace` → **80 passed, 0 failed** (syntax 5, analyzer 13, codegen 18, runtime 37, plus 7 real-PostgreSQL integration tests in `bosc-runtime/tests/transactional_integrity.rs`).

---



## 2. What is genuinely ready (real, non-mocked, good quality)

- **Parser surface** for table, tableextension, tablelogic(+ext), statemachine(+ext), codeunit, interface, enum(+ext), permissionset, view, query, page(+ext), report, custommetadata (`crates/bosc-syntax/src/parser.rs:163-179`).
- **Table → PostgreSQL DDL**: system columns, primary/secondary/unique indexes, SIFT-style covering indexes, FlowField views, immutable-ledger guards, RLS policies, four storage strategies (SharedRLS / Partitioned / SchemaPerCompany / TablePerCompany), table-extension `ADD COLUMN` (`crates/bosc-codegen-db/src/lib.rs:247-597`).
- **Runtime record lifecycle**: insert/modify/delete triggers with Rec/xRec, optimistic concurrency via `_system_version`, ledger update/delete blocking (`crates/bosc-runtime/src/lib.rs:1017-1041, 1638-1694`).
- **State machines**: guarded, role-checked transitions with conditional UPDATE and history rows (`lib.rs:2227-2384`).
- **Transactional outbox** tables and `enqueue_outbox_tx` (`event_engine.rs:119-258`); meta-cache invalidation via LISTEN/NOTIFY (`lib.rs:266-279`).
- **Sandboxes**: QuickJS with 8 MB memory cap and stack limit (`quickjs_engine.rs:32-42`); Wasm via `wasmi` with real fuel metering (`wasm_engine.rs:69-96`); sidecar HTTP invoker (`sidecar_host.rs`).
- **Transactional integrity** (added after this report's first edition): one transaction per unit of work, enforced foreign keys, schema migration engine, mandatory OCC and fuel metering, verified by real-PostgreSQL integration tests (`27_`).
- **Developer Studio UI**: Table/Page/Query/State-machine/Permission designers, Monaco BOS-Script IntelliSense, live entity datasheet — all wired to real `/api/`* endpoints. Clean Svelte 5 runes: zero `$:`, `export let`, `on:click`, `createEventDispatcher`.
- **CLI**: `build`, `deploy`, `run`, `serve`, `provision`, `create-db`, `detect-db`, `package build/install/list` all do real work (`crates/bosc-cli/src/main.rs`).

---



## 3. Critical & high findings



### 3.1 Security (CRITICAL — blocks any deployment)


| #   | Finding                                                                                                                                                        | Evidence                                                                                 |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| S1  | ~55 routes mounted with no auth middleware                                                                                                                     | `server.rs:43-99`                                                                        |
| S2  | Identity from headers; missing `x-roles` ⇒ `["Admin"]`, missing `x-user-id` ⇒ random UUID, missing tenant ⇒ nil UUID                                           | `server.rs:105-136`                                                                      |
| S3  | `check_permission` exists but is only exposed as `POST /api/permissions/check`; never enforced on CRUD/codeunit/meta routes; Admin short-circuits              | `lib.rs:715-750`                                                                         |
| S4  | `privileged-override` lets anyone rewrite immutable ledger rows                                                                                                | `server.rs:572-606`, `lib.rs:2184-2224`                                                  |
| S5  | CORS `allow_origin(Any)/methods(Any)/headers(Any)`                                                                                                             | `server.rs:38-41`                                                                        |
| S6  | SQL built with `format!`: named-query params, UPDATE SET values, `ILIKE '%{}%'` filters, view joins/WHERE, RLS `USING ({filter})`, plpgsql `GetRecord` filters | `lib.rs:843-858, 1665-1688, 2449`; `codegen-db/lib.rs:852-856, 1001-1007, 1063-1065`     |
| S7  | Scripts get `__host_db_raw_query` / `__host_db_raw_exec` with only a deny-list; DELETE/UPDATE/INSERT allowed                                                   | `quickjs_engine.rs:392-478`                                                              |
| S8  | Passwords: salted single-round SHA-256 (GPU-crackable), not Argon2id/bcrypt                                                                                    | `lib.rs:183-211`                                                                         |
| S9  | `bos.security` labels SHA-256 output as `$argon2id$` — misleading fake                                                                                         | `packages/bos.security/tables/User.bos:45-48`                                            |
| S10 | Both `AuthManager` codeunits only check strings are non-empty; no credential verification, no session issuance                                                 | `bos.auth/codeunits/AuthManager.bos:11-17`, `bos.security/codeunits/AuthManager.bos:2-8` |
| S11 | Default `postgres:postgres` credentials in CLI defaults, detector, run/test scripts                                                                            | `bosc-cli/main.rs:40-67`, `detector.rs:88`, `run.ps1:2`, `run.bat:7`                     |
| S12 | Any client can create/drop databases, install packages, load Wasm modules, register sidecars (SSRF)                                                            | `server.rs:45-99`                                                                        |




### 3.2 Correctness / data integrity (CRITICAL–HIGH) — **RESOLVED**

All findings in this section are closed. The design and its verification are in [`27_transactional_integrity_and_schema_migration.md`](./27_transactional_integrity_and_schema_migration.md).

| #   | Original finding                                                                                                                                     | Resolution |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| C1  | **Codeunit execution is not transactional.** A failing posting routine leaves partial writes                                                         | ✅ One `DbSession` per unit of work (`begin_scope` → `*_in` → `finish`); any error rolls back every write (`27_` §2) |
| C2  | **Test runner "auto-rollback" is fake**: tests ran against `pool` and mutated real data                                                              | ✅ Tests run inside one scope that is always rolled back (`27_` §3) |
| C3  | Triggers/events run outside the record write transaction                                                                                             | ✅ Triggers, subscribers, transitions and outbox rows share the write's transaction (`27_` §2) |
| C4  | **Foreign keys never emitted**; `on_delete` hard-coded `RESTRICT`                                                                                    | ✅ `relation` + `onDelete` become real FKs on every company copy through `bos_ensure_relation` (`27_` §4) |
| C5  | **No schema migration engine**: additive `ADD COLUMN IF NOT EXISTS` only                                                                             | ✅ Model-vs-catalog diff, rename support, destructive-change guard, dry run and `_bos_schema_migrations` journal (`27_` §5) |
| C6  | OCC skipped when client omits `_system_version` (lost updates)                                                                                       | ✅ Version mandatory: missing → `428`, stale → `409` (`27_` §6) |
| C7  | Number series falls back to a random UUID suffix on DB error                                                                                         | ✅ Application package `packages/bos.ledger`. `NumberSeriesManagement.getNext` increments `NumberSeries` in PostgreSQL. |
| C8  | QuickJS limit is a 5 s wall-clock interrupt, not the documented 100k fuel                                                                            | ✅ Interrupt-driven fuel metering, 100,000 default, per-codeunit `max_fuel`; wall clock is only a 30 s backstop (`27_` §7) |




### 3.3 Half-baked / fake paths


| Path                                              | What it claims              | What it does                                                                                                        | Evidence                                                |
| ------------------------------------------------- | --------------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------- |
| `ctx.postLedger`                                  | ledger posting              | Not on the host. Posting belongs in an application codeunit.                                                        | `test_ledger_and_dimension_host_apis_removed`           |
| `ctx.dimension`                                   | Dimension accounting        | Not on the host. Dimensions belong in application tables.                                                           | `test_ledger_and_dimension_host_apis_removed`           |
| `ctx.ui.confirm`                                  | User confirmation           | Throws `ConfirmRequired` unless the run supplied `ui_answers` for that message.                                    | `quickjs_engine.rs` `confirm`                           |
| `ctx.security.hasRole`                            | Role check                  | Exact match against the signed-in session roles. A missing session is not Admin.                                   | `server.rs` bearer session                              |
| HTTP bridge                                       | Outbound HTTP               | Refuses non-HTTP schemes and reserved documentation hosts. A failed request is an error.                           | `refuse_unreal_http`                                    |
| TypeScript in BOS-Script                          | Typechecked TypeScript      | Oxc reports syntax (`BOS1001`). `ctx.*` members are checked (`BOS1002`). This is not a general TypeScript checker. | `host_member_diagnostics`                               |
| Structured action IR                              | Every statement as an action | `return` and declarations stay in the action list as `NotLowered`. QuickJS runs the script. Pushdown refuses them. | `lower_statements`                                      |
| AMDP / plpgsql                                    | Every codeunit as SQL       | A method moves to PostgreSQL only when every line translates. Otherwise it stays on QuickJS. `@pushdown` on a partial body is error `BOS3301`. | `bosc-codegen-db/src/pushdown.rs`                       |
| `query` meta-object                               | Executable query            | `POST /api/queries/{name}/run` runs the stored SELECT with bound parameters.                                        | `execute_named_query`                                   |
| page / report                                     | A screen of the source table | The workspace loads the page or report definition and the rows of its source table.                                | `PageSurface.svelte`                                    |
| Package bundle                                    | Ships all meta-objects      | The bundle includes every object kind the compiler lowered. Schema sync still applies table DDL only.              | `compile_package_bundle`                                |
| Sidecar                                           | Managed sidecar process     | A command is spawned. The URL must be loopback HTTP. A URL with no command stays registered until health succeeds. | `sidecar_host.rs`                                       |
| Multi-DB                                          | Postgres/SQLite/MSSQL/MySQL | Discovery returns PostgreSQL only. The engine opens PostgreSQL.                                                    | `DatabaseDetector::scan_all`                            |
| Health check                                      | Readiness                   | `SELECT 1`. The route answers 503 when PostgreSQL does not.                                                        | `health_check`                                          |
| API identity                                      | Caller role                 | `Authorization: Bearer` loads `_bos_sessions`. Header role lists are ignored. The first account is created by bootstrap. | `require_session`                                       |




### 3.4 Hard-coded values

- The shell shows the signed-in account. There is no built-in `USR-001`.
- Page and report objects from the catalog open in the workspace.
- API Studio calls the same origin and sends the signed-in bearer token.
- `bosc build` uses the source file name as the package name.
- Showcase customer/item arrays (`ComponentShowcase.svelte:123-145`, `DataTablePanel.svelte:3-21`) — acceptable for a design lab, must never ship as product data.
- `test_security_api.ps1:28` uses `hash_placeholder`; `audit_keywords.json` is empty.



### 3.5 Robustness & operations

- `block_in_place` + synchronous QuickJS on async request paths can starve Tokio workers under load (`quickjs_engine.rs` host bridges).
- Error shape `{"error": true, "message": ...}` contradicts the QA spec `{ "error": "..." }` (`server.rs:25-32`).
- No graceful shutdown (`bosc-cli/main.rs:544`), no metrics, no `DATABASE_URL` env config, no rate limiting, no request size limits, no audit of who did what (identity is spoofable).
- No `.git`, no CI, no Dockerfile/compose, no LICENSE file, no backup/restore or upgrade runbook.
- UI: no test pipeline (Playwright installed but unused), no i18n, JS-only, end-user shell never mounts toasts.



### 3.6 Docs vs code drift


| Doc claim                                                                                     | Reality                                                                        |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `08_live_backend_api_server_verification_report.md`: "Zero mocks… 100% PASS"                  | Stub auth codeunits, fake argon2, stubbed ctx APIs                             |
| `24_security_and_user_management...`: Permission / SecurityFilter tables, zero-mock mandate   | Not implemented; package violates its own mandate                              |
| `AGENTS.md`: "12 meta-objects"                                                                | `01_bos_script_must_have_features...` lists 14                                 |
| Numbering                                                                                     | Six `01_` files, two `19_` files, two `05_` files across two doc trees         |


---



## 4. Meta-object end-to-end status


| Meta-object              | Parse | IR        | DB codegen           | Runtime      | UI runtime        | Packaged        |
| ------------------------ | ----- | --------- | -------------------- | ------------ | ----------------- | --------------- |
| table / tableextension   | ✅     | ✅         | ✅ (FKs, migrations)  | ✅            | ❌                 | table ✅ / ext ❌ |
| tablelogic (+ext)        | ✅     | ✅         | via triggers         | ✅            | —                 | ✅               |
| codeunit                 | ✅     | ✅         | ❌ (plpgsql orphaned) | ✅ QuickJS    | —                 | ✅               |
| statemachine (+ext)      | ✅     | ✅         | history table        | ✅            | ❌                 | SM ✅ / ext ❌    |
| view                     | ✅     | ✅         | ✅                    | ✅            | ❌                 | ❌               |
| permissionset            | ✅     | ✅         | catalog + RLS        | not enforced | ❌                 | ❌               |
| enum (+ext)              | ✅     | ✅         | `VARCHAR(50)`        | partial      | ❌                 | enum ✅ / ext ❌  |
| query                    | ✅     | ✅         | catalog only         | partial      | ❌                 | ❌               |
| interface                | ✅     | ✅ (empty) | ❌                    | ❌            | —                 | ❌               |
| page (+ext)              | ✅     | ✅         | ❌                    | ❌            | ❌ (designer only) | ❌               |
| report                   | ✅     | ✅         | ❌                    | ❌            | ❌                 | ❌               |
| custommetadata           | ✅     | ✅         | ❌                    | ❌            | ❌                 | ❌               |
| xmlport / profile / menu | ❌     | ❌         | ❌                    | ❌            | ❌                 | ❌               |


---



## 5. What must change to make BOS a market leader

The benchmark is Business Central (AL + Web Client + AppSource), Salesforce (metadata platform + Lightning + AppExchange), and SAP S/4 (RAP/CDS). Each won on three things: **a trustworthy platform, a complete base application, and an ecosystem**. BOS's bet — modern open stack (Rust + Postgres + Svelte), TypeScript-flavoured scripting, metadata-first — is sound. The order of work matters.

### Phase 0 — Make it safe (blocker, ~2-4 weeks)

1. Real authentication: Argon2id in Rust (`argon2` crate), login endpoint, short-lived JWT or opaque session tokens stored hashed, refresh/revocation, MFA hook. Delete the duplicate `bos.auth` package; make `bos.security` the single identity package.
2. Auth middleware on every route; derive tenant/company/user/roles from the verified session only. Remove the `Admin` default.
3. Enforce `permissionset` on every entity/codeunit/meta call (row-level via existing RLS + object-level checks). Admin-only scope for database, package, Wasm, sidecar routes.
4. Parameterize all SQL (`sqlx` binds, `QueryBuilder`); replace raw-SQL script bridges with the model API or a strictly read-only, parameterized query surface.
5. CORS allow-list from config; request size limits; rate limiting; secrets from env/vault; remove `postgres:postgres` defaults.



### Phase 1 — Make it correct (~4-6 weeks)

1. ~~One database transaction per request/codeunit; real test isolation~~ (done, `27_` §2-3).
2. ~~Emit foreign keys; real `ON DELETE` semantics~~ (done, `27_` §4). Still open: enums as checked types.
3. ~~Schema diff & migration engine with data-preserving renames~~ (done, `27_` §5). Still open: upgrade/install codeunits (the BC equivalent) for data transformations during upgrades.
4. ~~Mandatory OCC on update~~ (done, `27_` §6). Number series are application objects in a BOS package (`27_` §1), not a runtime feature.
5. Oxc reports embedded-script syntax errors (`BOS1001`). The JavaScript that runs is still the line-preserving eraser, because QuickJS stacks use those line numbers. There is no typecheck against `ambient.d.ts`. Decimal math and simple `for...of` gets are rewritten. Unknown statements are not rejected.
6. ~~Instruction-count fuel for QuickJS~~ (done, `27_` §7). Still open: move script execution to a bounded blocking pool.



### Phase 2 — Metadata-driven page runtime (the product-defining piece, ~6-10 weeks)

The single biggest gap. Business Central and Salesforce are what users *see*: List, Card, Document (header + lines), Worksheet, Role Center, FactBoxes, actions, filters, search, keyboard-first entry. Build a renderer that takes compiled `page` IR and produces these screens from metadata, driven by real permissions, with `pageextension` merging, personalization, and mobile layouts. Generate navigation from metadata, not `SidebarNav.svelte`. Add `report` (print/PDF/Excel) and `query` execution.

### Phase 3 — Base application (~3-6 months, the moat)

No ERP platform wins without a credible base app. Minimum viable suite, modelled on BC's object design:

1. Foundation: Company, Number Series (with lines/dates), Dimensions (+ default dimensions, dimension set entries), Posting Groups, Payment Terms, Currency & exchange rates, Units of measure, Country/Post codes.
2. Finance: Chart of Accounts, General Journal, two-phase posting codeunit with balancing per document/date, G/L Entry + G/L Register (immutable), VAT/tax setup and entries, accounting periods and close.
3. Receivables / Payables: Customer, Vendor, ledger entries, detailed entries, application/unapplication, payment journals.
4. Sales & Purchasing: Quote → Order → Shipment/Receipt → Invoice → Credit Memo, posted document tables, document state machines, approvals.
5. Inventory: Item, Location, Item Journal, Item Ledger Entry, Value Entry, costing (FIFO/Average), reservations.

Each must ship with a test codeunit suite running in rolled-back transactions.

### Phase 4 — Developer & ecosystem (parallel with Phase 3)

1. A real language server and VS Code extension, plus a source-mapped debugger. Neither exists in this repo. Monaco completions and `ambient.d.ts` are the editor support that runs today.
2. Package versioning, dependency resolution, signed `.bospkg`, upgrade paths, a registry ("BOS AppSource").
3. Public, versioned REST/OData-style API generated from metadata, webhooks from the outbox, OpenAPI.
4. Extension safety: object ID ranges, breaking-change detection between package versions.



### Phase 5 — Operate at scale

Git + CI (build, clippy `-D warnings`, test, UI Playwright), Docker/Helm, Postgres migrations at deploy, Prometheus metrics + OpenTelemetry traces, readiness probes, graceful shutdown, backup/PITR runbook, per-tenant quotas, horizontal scale (shared meta-cache invalidation already exists via LISTEN/NOTIFY).

### Where BOS can genuinely leapfrog incumbents

- **AI-native**: metadata is machine-readable end-to-end — natural-language page/report generation, AI-authored BOS-Script with type-checked guardrails, copilots that respect permissionsets.
- **Open, modern stack**: Postgres + Rust + web-standard UI, self-hostable, no proprietary SQL Server or licence lock-in.
- **TypeScript developers** instead of a niche language (AL/ABAP/Apex) — far bigger talent pool, *if* typing is real (Phase 1.5).
- **Event-first architecture**: transactional outbox and state machines as first-class objects, which incumbents bolt on.

---



## 6. Recommended immediate actions (next 10 working days)

1. Initialise git, add CI running `cargo test`, `cargo clippy -- -D warnings`, and UI build.
2. Implement Phase 0 items 1-2 (auth + middleware) and remove the `Admin` default.
3. ~~Wrap codeunit execution and entity writes in a single transaction; fix the test-runner rollback.~~ Done (`27_`).
4. Parameterize the SQL sites listed in S6.
5. `08_` still overstates a clean pass. Duplicate `01_` / `19_` / `05_` filenames are still in `docs/`. Compiler, Wasm host, and gap-doc claims were aligned with the crates.

