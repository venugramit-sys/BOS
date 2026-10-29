# QA Developer Journey - iteration 1 (24 Oct 2026, Aziz-XPS, main)

Node started with `npm run dev` (Rust build 1m 11s, Vite ready in 20.6 s, API :3000, UI http://localhost:5173). Browser: system Edge via Playwright (`reports/qa_dev_journey*.cjs`). Raw log: `reports/qa_dev_journey_log.json`, node log: `reports/qa_dev_journey_node.log`. Screenshots: `reports/qa_dev_journey_screens/`. No product code changed.

## Login blocker
- `/api/auth/status`: `isProvisioned:true, needsSetup:false, needsBootstrap:false, usersCount:1, rolesCount:2` -> no setup screen; only Sign In.
- No password is documented anywhere (README, AGENTS.md, docs/, nodes/dev.toml, ADRs). `nodes/dev.toml` names `emergency.admin` but no password; ADR 0004 describes a dev stand-in session for `dev.admin` (roles SUPER) but no `DevStandIn` code exists in `crates/`, and the UI has no dev-login option.
- Needed to continue: the password of the one existing user (or of `emergency.admin`), or a documented dev reset/bootstrap command. No passwords were guessed.

## PASS / FAIL
| # | Step | Result | Evidence |
|---|---|---|---|
| 1 | First load / login screen | **FAIL** (renders, but Vite error overlay covers it; Sign In unclickable) | 01_first_load.png, 03_login_empty_submit.png |
| 2 | Setup flow | N/A (server says no setup needed) | - |
| 3 | Login | **BLOCKED** (no credentials) | 06_login_fallback_form_emergency_nopass.png |
| 4 | Studio navigation (dev-studio menus, Ctrl+K, + Object wizard) | PASS (UI only, pre-login) | 02__dev_studio_html.png, 08-11 |
| 5 | Create table meta-object | **FAIL** (designer opens locally, all API calls 401, no auth message) | 07, 12, 13 |
| 6 | Create/edit page meta-object | BLOCKED | - |
| 7 | BOS-Script trigger in Code tab | PARTIAL (Monaco editor + snippets load; typed trigger) | 14_code_tab.png, 15_syntax_error_typed.png |
| 8 | Compile feedback / deliberate syntax error | **FAIL** (`POST /api/script/compile` 401; "1 Error" badge but no readable diagnostic surfaced) | 15, 16_save_apply_result.png |
| 9 | Package view | BLOCKED (`/api/meta/packages` 401; tree shows bos.auth with 0 objects) | 02__dev_studio_html.png |
| 10 | Role create + save (audit-null fix) | **NOT VERIFIED** (`POST /api/roles` 401) | qa_dev_journey_log.json |
| 11 | API studio / showcase pages load | PASS | 02__api_studio_html.png, 02__showcase_html.png |
| 12 | Mobile login view | PASS (renders) | 05_mobile.png |

## Weak areas (ranked)
### Blocker
1. **No documented way to log in on a dev node.** Repro: fresh `npm run dev`, open :5173 -> Sign In only; no creds in docs. Blocks steps 3,5,6,8,9,10. Fix: document/seed a dev user, or implement ADR 0004 dev stand-in (`dev.admin`) for profile=development.
2. **Login page module fails to load: Vite overlay `page LoginPage: 401 ... MISSING_TOKEN`** from `vite.config.js` `sync` plugin when resolving `@bos-page/LoginPage.svelte` (500, then 404 retries). The overlay intercepts clicks so even the fallback form's Sign In can't be pressed. Repro: load `/`, see overlay. Screens 01, 03. The page plugin fetches pages with auth before anyone can log in (chicken-and-egg); LoginPage must be fetchable anonymously. Page shows "LoginPage host warming: direct auth active".
### Major
3. **Dev Studio works unauthenticated with silent 401s.** Create Meta-Object -> Table "QaItem" -> Create & Open opens a designer; status bar says "Catalog synced" and "0 objects" while every `/api/meta/*` call 401s. No "please sign in" message; work appears saved but isn't. Screens 12, 13.
4. **Request flood while typing in the Code editor**: each keystroke fires `GET /api/meta/objects/<partial word>` and `/api/entities/<partial>/schema` (b, br, bro, brok...), ~110 requests in a few seconds. Needs debounce/caching.
5. **Compile errors not surfaced**: Compile with a deliberate syntax error -> 401 from `/api/script/compile`, and the editor only shows a "1 Error" counter with no readable message/toast. (Auth cause can't be split from diagnostic UX until login works.)
6. **Role save (audit-of-null fix) still unverified** - second iteration blocked by credentials.
### Minor
7. Signing in with a username and empty password gives no validation message (screen 06).
8. Save & Apply button not found in Code view (only "Apply F5"/"Compile") - inconsistent naming between Design and Code tabs (Playwright timed out after 33 s looking for it).
9. Monaco's overlay intercepts normal clicks (automation needed force-click); fine for humans but hurts e2e testability.
10. Build menu only says "Apply is available in the active designer" - no build/compile-all action.
### UX / DX friction (timings)
- First `npm run dev`: Rust compile 71 s, Vite cold 20.6 s, first page load 12.3 s (networkidle). Later loads 0.8-1.7 s; dialog opens ~1.3 s.
- Login page text "LoginPage host warming: direct auth active" plus a Retry button is confusing to end users.
- "Catalog synced" status is misleading when unauthenticated.
- No dev quick-start in README describing first login.

## Suggestions
1. Make LoginPage (and other public pages) resolvable without a token in the Vite page plugin; set `server.hmr.overlay` behaviour so plugin errors don't block the app.
2. Ship a documented dev credential path: `bosc provision --dev-admin` or ADR 0004 stand-in, and put it in README "First login".
3. Studio: redirect to login (or show a banner) on any 401; disable Create when unauthenticated.
4. Debounce editor symbol lookups (>=300 ms) and cache 401/404.
5. Show compile diagnostics inline (Monaco markers + problems panel) and an auth error toast when compile is refused.
6. Re-run this journey once credentials exist to verify role save and package view.

## Out of scope / notes
- Steps after login could not be executed. `git status` was not changed by this run; only files under `reports/` were written.
