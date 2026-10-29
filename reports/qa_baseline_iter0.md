# BOS Baseline QA – Iteration 0

Date: 2026-10-24 (IST) · Tester: QA agent (Grok Bot) · Scope: exploratory baseline, no product code modified.

## 0. Test-basis note
`docs/18_qa_test_task_matrix.md` and `docs/19_qa_tester_agent_architecture_and_verification_specification.md` are linked from README.md but **do not exist anywhere in the repo** (I searched recursively, including stale-docs/ and research_notes/). So this run was an exploratory pass, not a matrix-driven one.

## 1. Environment & startup
- PostgreSQL was already listening on :5432 (PG service, DB `my_db`, shared_rls, 31 meta-objects, provisioned 2026-10-22).
- Backend `target\debug\bosc.exe` was **already running** on :3000 (PID 19488, started 09:19 IST). I reused it as-is and did not restart it.
- I started the UI with `npm run dev` (Vite 8.3.3, ready in 1.1 s, :5173). I stopped it at the end of the run.
- Playwright's bundled Chromium is not installed (`npx playwright install` was never run). I used `channel: 'msedge'` (system Edge, headless) instead.
- Artifacts: crawl script `reports/qa_iter0_crawl.cjs`, raw results `reports/qa_iter0_ui.json`, Vite log `reports/qa_iter0_vite.log`, screenshots in `reports/qa_iter0_screens/`.

## 2. What works
- Backend health: `/api/health` returns db up. Status endpoints `/api/auth/status`, `/api/setup/status`, `/api/meta/system-status`, `/api/node/info`, and `/api/detect-db` all respond in about 20 ms.
- Meta catalog: `/api/meta/objects/User` returns 200, and an unknown object gives a clean 404. `/api/v1/pages/manifest`, `/api/v1/pages/UserList`, and `/api/v1/pages/property-schema` return 200. `/api/packages/bos.auth/export` returns 200.
- Input validation: malformed JSON gives 400 and missing fields give 422 with clear messages. `limit` is clamped to 1..1000.
- Login rejects bad credentials with a generic message (401 INVALID_CREDENTIALS, no user enumeration).
- Script compiler returns structured diagnostics (line/column) for invalid source.
- UI: all 4 Vite entry points load with no failed network calls:
  - `index.html` (Sign-In)
  - `dev-studio.html` (31 meta-objects, module tree, menus)
  - `api-studio.html` (55 endpoints, LIVE 3000 badge, every endpoint panel opens)
  - `showcase.html` (all 11 component panels render)
- Most of these entry points have zero console errors (see F6 for the exception).

## 3. Findings (ranked)

### BLOCKER
**F1 – No user can sign in to the main workspace.** `user` and `role` tables are empty (`GET /api/entities/user` gives total 0), but `/api/auth/status` reports `needsBootstrap:false, needsSetup:false`. As a result the setup wizard never appears and the only screen is Sign-In. `emergency.admin` (advertised by `/api/node/info`) is also rejected with 401.
Repro: open http://localhost:5173/ → Sign-In → no valid credentials exist. Run `GET /api/entities/user`, then `GET /api/auth/status`.
Screens: `qa_iter0_screens/_.png`, `_index_html.png`.
Impact: AppShell, workspace, and every post-login screen could not be tested.

**F2 – Creating a Role fails in business logic.** `POST /api/entities/role {"code":"QA_ITER0","name":"QA iter0 role"}` returns 400 `Business logic exception in 'role.onInsert' (TypeError): cannot read property 'audit' of null [RoleLogic:21]`. The trigger context is missing an `audit` service, so Roles (and probably every table whose trigger audits) cannot be inserted. This also blocks fixing F1 through the API.

### MAJOR
**F3 – Unauthenticated access to all data APIs.** Entity CRUD, `/api/query`, `/api/packages/*/export`, `/api/databases`, and `/api/permissions/check` all work with no token or session. `POST /api/permissions/check {"entity":"user","action":"read"}` returns `allowed:true` anonymously.
Repro: run any curl call without auth headers.

**F4 – Unknown or legacy entity names return a misleading DB error.** `GET /api/entities/customer`, `/api/entities/sec_user`, and `/api/entities/nonexistent_tbl` return 400 `current transaction is aborted, commands ignored until end of transaction block` instead of 404 "entity not found". The real failure is masked, and the transaction handling is wrong: the first statement fails and the next query reports the aborted state. Raw DB errors also leak to the client.

**F5 – `/api/query` is broken.** `POST /api/query {"entity":"user"}` returns 400 `operator does not exist: jsonb || json`, an SQL type bug in the query builder. Unknown entities leak `relation "no_such" does not exist`.

**F6 – Showcase crashes in UserWidget.** On `showcase.html` load: `pageerror: Cannot read properties of null (reading 'name')` in dropdown-menu-trigger → UserWidget → TopNavBar → AppShell. UserWidget assumes a logged-in user, so the same crash likely hits the real AppShell whenever the session is null.
Screen: `_showcase_html.png`, `_showcase_html_nav0.png`.

**F7 – Test scripts are stale against the current API and schema.** `test_security_api.ps1` and `test_advanced_features.ps1` target `customer`, `customer_ledger_entry`, `sec_role`, `sec_user`, `sec_user_role`, and `sec_session_token`. None of these exist in the current `my_db` catalog (the tables are now `user`, `role`, `user_role`, and so on), so the scripts cannot pass.

### MINOR
**F8 – Wrong HTTP method on GET routes.** `GET /api/codeunits` and `GET /api/auth/profile` return 405 with an empty body. The UI and docs imply these are readable; list codeunits and profile should be GET (or documented).

**F9 – Slow endpoints.** First `/api/health` took 2.1 s; `/api/databases` took 4.0 s; first call to `/api/entities/user` after idle took 2.1 s (pool warmup or a cluster-wide size scan). The first UI load of `/` took 7.8 s (cold Vite) and about 2 s warm.

**F10 – Raw serde/axum errors exposed.** 422 and 400 bodies are plain text (`Failed to deserialize the JSON body…`) rather than the standard JSON error envelope (`code`, `message`, `field_errors`) that other errors use, so the UI can't render them consistently.

**F11 – `POST /api/tests/run {}` reports `success:true` with 0 tests.** A run with zero tests should not report green.

**F12 – Missing QA docs.** README links docs/18 and docs/19, but both files are missing.

### UX
**F13 – Unknown routes silently show Sign-In.** `/does-not-exist` renders the Sign-In page (identical screenshot `_does_not_exist.png`), with no 404 page.

**F14 – Doc title encoding.** The runtime document title contains a non-ASCII separator ("BOS · Workspace"), which rendered as mojibake in the tooling output. Verify charset meta/encoding.

**F15 – Dev Studio "Collapse navigation" leaves a nearly empty canvas** (`_dev_studio_html_nav6.png`, 22 KB vs 66 KB baseline). Worth a visual check that a re-expand affordance is visible.

**F16 – Playwright browsers are not installed** even though `playwright` is a devDependency, so UI automation fails out of the box. Add `npx playwright install chromium` to the setup docs.

## 4. Not covered (blocked or out of scope)
- All post-login screens (blocked by F1).
- Package install, database create/drop, and switch: skipped as destructive to the shared `my_db`.
- `cargo test --workspace`: not run in this pass.
- Mobile layout: only the Sign-In page was captured (`root_mobile.png`).
