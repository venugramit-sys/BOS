# How to run and test BOS (step-by-step, verified 24 Oct 2026, Aziz-XPS, branch main)

Every command below was run for real; outputs are trimmed. Open **PowerShell** in `D:\Work Studio\Bos` first (`cd 'D:\Work Studio\Bos'`).

## 1. Prerequisites

| Tool | Check command | Real output | Install if missing | Result |
|---|---|---|---|---|
| PostgreSQL | `Get-Service *postgres*` | `Running postgresql-x64-18` | https://www.postgresql.org/download/windows/ (user `postgres`, password `postgres`, port 5432) | PASS |
| psql | `& 'C:\Program Files\PostgreSQL\18\bin\psql.exe' --version` | exists (note: `psql` is **not on PATH**, so plain `psql --version` fails) | add `C:\Program Files\PostgreSQL\18\bin` to PATH | PASS (with note) |
| Rust | `rustc --version`; `cargo --version` | `rustc 1.97.1`, `cargo 1.97.1` | https://rustup.rs | PASS |
| Node | `node --version`; `npm --version` | `v24.18.1`, `11.16.0` | https://nodejs.org (LTS) | PASS |

The database used is `postgres://postgres:postgres@localhost:5432/my_db` (set in `nodes/dev.toml`).

## 2. Start a node: `npm run dev` — PASS
```
npm run dev
```
This runs `node scripts/run-node.mjs --config nodes/dev.toml --ui` (API on :3000, UI on :5173). First run compiles Rust and can take a few minutes. Real output (trimmed):
```
INFO bosc_runtime::nas_worker: Navision Application Server (NAS) background worker started
INFO bos::pages: page catalog resolved pages=10
Server Node ready. Clients can connect with just User ID & Password.
VITE v8.3.3  ready in 2373 ms
  Local:   http://localhost:5173/
```
Open http://localhost:5173/ in a browser. Use `http://localhost`, not `127.0.0.1`: Vite only listened on `localhost` (a request to `127.0.0.1:5173` was refused). To stop: press Ctrl+C, or `npm run node:kill` (frees port 3000).
Full log: `reports\how_to_run_dev_output.log`.

## 3. Custom node config + `node check` — PASS
Make a copy (outside product folders) and edit it:
```
Copy-Item nodes\dev.toml reports\custom-node.toml
# edit: id = "node-custom-01", port = 3100, and change "../" paths to "../../" (file is one folder deeper)
cargo run -q -p bosc-cli -- node check --config reports/custom-node.toml
```
Output:
```
  ✓ TOML schema valid
  ✓ Node ID: node-custom-01
  ✓ Network: 127.0.0.1:3100
  ✓ UI Static Hosting: DISABLED (path: ../../ui/dist)
Testing database connection to 'postgres://***:***@localhost:5432/my_db'...
  ✓ Database connection established successfully!
✅ NODE CONFIGURATION VERIFIED - READY FOR PRODUCTION
```
Run it with `cargo run -p bosc-cli -- node run --config reports/custom-node.toml`. (Note: `npm run node:check` checks `bos-node.toml` in the root, not your copy.)

## 4. Tests for each team change (node running from step 2)

| # | Test | Command | Expected | Actual | Result |
|---|---|---|---|---|---|
| 1 | Anonymous query blocked | `Invoke-WebRequest http://127.0.0.1:3000/api/query -Method POST -Body '{"sql":"select 1"}' -ContentType application/json` | 401 | `401` | PASS |
| 2 | Health | `Invoke-WebRequest http://127.0.0.1:3000/api/health` | 200 | `200 {"database":"up","status":"healthy","version":"0.1.0"}` | PASS |
| 3 | Auth status | `Invoke-WebRequest http://127.0.0.1:3000/api/auth/status` | 200 + JSON | `200 {"isProvisioned":true,"needsBootstrap":false,"needsSetup":false,"rolesCount":2,"usersCount":1,...}` | PASS (users/roles now populated, unlike iteration 0) |
| 4 | Role save crash fixed | `POST /api/roles {"code":"QA_TEST","name":"QA Test"}` | role created | `401` (auth required); login as `emergency.admin` also `401`, no password known | **NOT VERIFIED** (blocked: no credentials) |
| 5 | `bosc serve` removed | `cargo run -q -p bosc-cli -- serve --help` | unknown command | `error: unrecognized subcommand 'serve'`; `--help` lists build, run, deploy, test-live, detect-db, provision, create-db, package, node | PASS |
| 6 | run.ps1 gone | `Test-Path run.ps1` | False | `False` | PASS |
| 7 | Tracked file count | `(git ls-files \| measure).Count` | ~544 | `686` | **FAIL** |
| 8 | ccusage | `ccusage --version` | version | `ccusage 20.0.42` (global npm, `%APPDATA%\npm`) | PASS |
| 9 | ast-grep | `ast-grep --version` | version | `ast-grep 0.51.0` (global npm; `sg` alias deprecated) | PASS |
| 10 | Serena | `.mcp.json` → `C:/Users/mades/.local/bin/serena.exe start-mcp-server ...`; check `& C:/Users/mades/.local/bin/serena.exe --help` | help text | `Usage: serena [OPTIONS] COMMAND [ARGS]...` | PASS |
| 11 | ADR records | `ls docs\adr` | ADR files | 0001-uuidv7-ids-and-readable-names, 0002-tenant-id-on-every-row-uniform-rls, 0003-logical-table-names, 0004-kernel-auth-boundary-and-dev-stand-in, 0005-package-manifest-semver-checksum, 0006-detached-ed25519-signature, 0007-audit-event-shape-hash-chain, 0008-security-profile, 0009-secret-and-key-provider, README.md | PASS |

## Notes
- `git status` on main shows uncommitted changes to many `crates/` files and `Cargo.lock`, plus `packages/bos.auth.bospkg` and `packages/bos.auth/bos.auth.bospkg`. The .bospkg files were last written 5:17 PM IST, **before** this guide was made, so they were left as-is (not restored). `cargo test` was not run.
- `rg` (ripgrep), which AGENTS.md recommends, is not installed/on PATH.
- All processes started for this guide were stopped.
