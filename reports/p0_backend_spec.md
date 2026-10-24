# P0 backend implementation spec (security profiles)
For: Backend Engineer. Author: CTO. Date: 2026-10-24. Base: branch `fix-iter1-security` (`0e0b974f`).
Governing docs: `reports/cto_phased_security_plan.md` (APPROVED), `docs/adr/0001`-`0009`. This is a spec only; no code is included in this branch.
Line numbers refer to `fix-iter1-security` at `0e0b974f` and will drift, so match on the function names.

## 0. Scope
**In P0:** profile enum and resolution, startup-check framework, dev localhost bind, dev stand-in auto-login, profile header and UI banner hook, DB profile marker, profile-gating of the iteration-1 behaviours.
**Out of P0:** TOTP, OpenBao, DB roles, CORS allow-list enforcement, hashed sessions, signing (P1-P3). For these, P0 only adds **check stubs** that report `NotImplemented`. That is fatal in production and a warning in staging (except the TOTP-required flag, sec 3).

## 1. PR order (small, independently mergeable)
| PR | Title | Depends |
|---|---|---|
| PR-1 | `SecurityProfile` enum, parsing, resolution, immutability, logging | - |
| PR-2 | Startup-check framework + exit codes + check stubs | PR-1 |
| PR-3 | Dev localhost-only bind (+ default host changes) | PR-1, PR-2 |
| PR-4 | Profile header `X-BOS-Profile` + `/api/node/info` profile + UI banner hook | PR-1 |
| PR-5 | DB profile marker (write at create, check at open) | PR-2 |
| PR-6 | `AuthKernel` boundary + `DevStandInAuth` auto-login (localhost) | PR-1, PR-3, PR-4 |
| PR-7 | Profile-gate iteration-1 behaviours (emergency/fallback, tenant header, default tenant) | PR-6 |
Each PR: `cargo test -p bosc-runtime` green, plus the tests named below.

## 2. PR-1: SecurityProfile
**Files:** new `crates/bosc-runtime/src/security_profile.rs`; `crates/bosc-runtime/src/lib.rs` (add `pub mod security_profile;` next to `pub mod auth_guard;`); `crates/bosc-runtime/src/node_config.rs` (`SecuritySection`, lines 235-252); `crates/bosc-cli/src/main.rs` (`NodeCommands::Run` ~130 and the handler at 959-1023; `Commands::Serve` ~70 and 611-644).
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityProfile { Development, Staging, Production }

impl SecurityProfile {
    pub fn parse(s: &str) -> Result<Self, ProfileError>;          // trim, case-insensitive; "dev"/"prod" NOT accepted (explicit names only)
    pub fn as_str(&self) -> &'static str;
    pub fn is_dev(&self) -> bool;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ProfileSource { Cli, Env, ConfigFile, Default }

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedProfile { pub profile: SecurityProfile, pub source: ProfileSource }

pub fn resolve_profile(cli: Option<&str>, env: Option<&str>, file: Option<&str>) -> Result<ResolvedProfile, ProfileError>;
// order: cli > env (BOS_SECURITY_PROFILE) > file ([security] profile) > Default => Production
// empty string counts as "not set"; invalid value at any level => Err (do NOT fall through)
```
- Config: `[security] profile = "development"` → add `#[serde(default)] pub profile: Option<String>` to `SecuritySection`. The template (`default_template()`, line 378+) gets `# profile = "production"   # development | staging | production` and `host = "127.0.0.1"`.
- CLI: on `NodeCommands::Run` and `Commands::Serve` add `--security-profile <P>` (`#[arg(long, value_name="PROFILE")]`) and `--dev` (conflicts with `--security-profile`; same as development).
- **Immutability:** store as `Arc<ResolvedProfile>` in an `Extension`, next to `Extension(node_config_arc)` (`server.rs` ~352). `NodeConfig` keeps no mutable copy, there's no setter and no route. Expose a read-only `ctx.kernel.node.profile` to the QuickJS context (string). Remove `profile` from any config-save path: `POST /api/auth/config` must not accept it, and if present it gets 400 `PROFILE_IMMUTABLE`.
- **Logging** at startup (before bind), on one line: `security_profile=development source=cli`. In development a WARN line `DEVELOPMENT PROFILE - NOT SECURE - localhost only` is printed at start and every 10 min (tokio interval). Replace the misleading "running with default production node profile" text (`main.rs:983`) with the resolved profile.
- **Tests** (`crates/bosc-runtime/src/security_profile.rs` `#[cfg(test)]`):
  - `resolve_defaults_to_production`: all None → Production, Default.
  - `resolve_cli_beats_env_and_file`; `resolve_env_beats_file`; `resolve_file_used_last`.
  - `resolve_invalid_value_is_error_not_fallback`: cli Some("prod") → Err even if env valid.
  - `resolve_empty_string_is_unset`.
  - `parse_case_insensitive`: "Staging" → Staging.
  - Integration `tests/p0_profile.rs::auth_config_rejects_profile_change`: POST `/api/auth/config` with `{"profile":"development"}` → 400 `PROFILE_IMMUTABLE`.
**Out of scope:** hot reload (there won't be any).

## 3. PR-2: Startup checks
**Files:** new `crates/bosc-runtime/src/startup_checks.rs`; call it from `main.rs` after `BosEngine::connect` (line 1006) and **before** `TcpListener::bind` (line 1023); also call it in `Commands::Serve` (611-619).
```rust
pub enum CheckId { Bind, DbRole, Rls, SecretProvider, NoFallbackLogin, Cors, TrustStore, PlatformMfa, ClockSync, DbMarker, TlsOrProxy }
pub enum CheckStatus { Pass, Fail(String), Warn(String), NotImplemented, Skipped }
pub struct CheckResult { pub id: CheckId, pub status: CheckStatus }
pub struct StartupReport { pub profile: SecurityProfile, pub results: Vec<CheckResult> }
impl StartupReport { pub fn is_fatal(&self) -> bool; }
pub async fn run_startup_checks(p: SecurityProfile, cfg: &NodeConfig, engine: &BosEngine, bind: &SocketAddr) -> StartupReport;
pub fn severity(p: SecurityProfile, id: CheckId, s: &CheckStatus) -> Severity; // Fatal | Warning | Info
```
The 11 production checks (plan sec 2.4) and their P0 implementation:
| # | CheckId | P0 implementation | production | staging | development |
|---|---|---|---|---|---|
| 1 | (profile resolved) | logged by PR-1 | - | - | - |
| 2 | Bind | loopback test (PR-3) | Pass required (TLS: #11) | any | **loopback only, fatal** |
| 3 | DbRole | `SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname = current_user` | fatal if either true | fatal | skipped |
| 4 | Rls | catalog: every tenant table `relrowsecurity AND relforcerowsecurity` + a RESTRICTIVE policy (`pg_policy.polpermissive = false`) | fatal | warning (P0), fatal from P1 | skipped |
| 5 | SecretProvider | `NotImplemented` until P2; file provider detected → Fail | fatal | warning | skipped |
| 6 | NoFallbackLogin | `auto_fallback_on_auth_failure == false && !maintenance_mode` and no `dev.admin` user | fatal | fatal | skipped |
| 7 | Cors | `network.cors_origins` non-empty and no `*` | fatal | fatal | skipped (localhost forced) |
| 8 | TrustStore | `NotImplemented` until P2 | fatal | warning | skipped |
| 9 | PlatformMfa | **TOTP-required flag stub**: `security.mfa_required` resolves to true in staging and production (owner: staging requires TOTP for QA too); the check returns `NotImplemented` until TOTP lands (P2) | fatal | **fatal** (owner decision) | skipped (flag false) |
| 10 | ClockSync | `NotImplemented` (P2) | fatal | warning | skipped |
| 11 | DbMarker | PR-5 | must be `production` | must be `staging` (or `production` refused, see PR-5) | must be `development` |
| + | TlsOrProxy | `network.tls` or `network.trusted_proxies` set | fatal | warning | skipped |
**Consequence, stated plainly:** with P0 alone a **production** node won't start (checks 5, 8, 9 and 10 are NotImplemented), and neither will a **staging** node (check 9). That's intentional: it holds until P1/P2 deliver. Staging gets a temporary escape `BOS_STAGING_ALLOW_UNIMPLEMENTED=1`, which demotes **only** NotImplemented to a warning, is logged loudly, and is removed in P2. There's no such escape for production.
- Config: `[security] mfa_required` is computed from the profile, not settable. Exposed as `NodeSecurity::mfa_required()`.
- Output format (stderr), one line per check, then a summary:
```
[startup-check] FAIL  db_role        current_user 'postgres' is superuser (production requires bos_app; see D9)
[startup-check] N/I   platform_mfa   TOTP not implemented yet (P2) - required in production
[startup-check] PASS  cors           2 origins
startup aborted: 3 fatal checks failed (profile=production). Exit code 77.
```
- Exit codes: `0` ok; `78` config error (invalid profile, invalid config, EX_CONFIG); `77` startup check failed (EX_NOPERM); `75` dependency unreachable (DB down, EX_TEMPFAIL).
- Also emit audit event `node.startup` (ADR 0007) with `details.checks` once the DB is reachable.
- **Tests** (`tests/p0_startup_checks.rs`):
  - `prod_with_superuser_db_is_fatal`: report from a fake `DbRoleProbe { super: true }` → `is_fatal()`.
  - `prod_not_implemented_is_fatal`; `staging_not_implemented_is_warning_except_mfa`; `staging_escape_env_demotes_only_not_implemented`.
  - `dev_skips_db_role_and_cors`.
  - `staging_mfa_required_true`, `prod_mfa_required_true`, `dev_mfa_required_false`.
  - `cors_wildcard_fails_in_staging`.
  - `exit_code_mapping` (77/78/75).
  Make probes injectable (`trait DbProbe`) so the tests need no Postgres; one `#[ignore]` integration test uses a real DB.

## 4. PR-3: Dev localhost-only bind
**Evidence:** `node_config.rs:73-75` `default_host()` returns `"0.0.0.0"`; template `node_config.rs:394` `host = "0.0.0.0"`; `bosc-cli/src/main.rs:618` `format!("0.0.0.0:{}", port)` (Serve); `main.rs:988` `format!("{}:{}", host, port)` (node run).
**Changes**
- `default_host()` → `"127.0.0.1"`; template `host = "127.0.0.1"` with the comment `# staging/production: set explicitly, e.g. "0.0.0.0" behind TLS proxy`. Update the test at `node_config.rs:461+` if it asserts the host.
- `main.rs:618` → use `--host` (new arg, default `127.0.0.1`) instead of the literal `0.0.0.0`.
- Parse the address as `SocketAddr` (use `tokio::net::lookup_host` for names) **before** binding:
```rust
pub fn dev_bind_allowed(addrs: &[SocketAddr]) -> Result<(), BindError>;
// Ok only if every resolved addr ip().is_loopback() (127.0.0.0/8 accepted? NO: exactly 127.0.0.1 or ::1, per plan sec 2.3)
```
  The rule is exact: IPv4 must equal `127.0.0.1` and IPv6 must equal `::1`. `localhost` is accepted only if **all** resolved addresses are those. `0.0.0.0`, `::`, `127.0.0.2`, LAN and public addresses are refused in development.
- Error (exit 77): `development profile only listens on 127.0.0.1 or ::1 (got 0.0.0.0:3000). For LAN/QA access run with --security-profile staging.`
- Docker: `Dockerfile`/compose for dev must publish `127.0.0.1:3000:3000`. Inside a container, `BOS_DEV_IN_CONTAINER=1` permits binding `0.0.0.0` **only** when combined with the development profile, and prints a warning naming the required `127.0.0.1:` port publish. Document this in `docs/09_bosc_cli_reference.md` (a later docs PR).
- Staging/production: any address is allowed (TLS check #11 applies).
- **Tests:** `dev_bind_accepts_127_0_0_1`, `dev_bind_accepts_ipv6_loopback`, `dev_bind_rejects_0_0_0_0`, `dev_bind_rejects_ipv6_unspecified`, `dev_bind_rejects_127_0_0_2`, `dev_bind_rejects_lan_192_168`, `dev_bind_localhost_all_loopback_ok`, `staging_bind_any_ok`, `default_host_is_loopback` (`NetworkSection::default().host == "127.0.0.1"`).
**Out of scope:** TLS termination itself.

## 5. PR-4: Profile header and UI banner hook
- Middleware `profile_header` in `server.rs` `create_node_router` (layer next to `require_session`, ~348). It adds `X-BOS-Profile: development|staging` on **every** response (API and static). Production adds nothing unless `[network] expose_profile_header = true`.
- `/api/node/info` (public, `auth_guard.rs` `is_public_route`) adds `"securityProfile": "<p>"` and `"banner": {"level":"danger","text":"DEVELOPMENT - NOT SECURE"}` (staging: `warning`, `"STAGING - test data only"`; production: `null`).
- UI hook: `ui/src/lib/components/shell/shellStore.svelte.js` reads `node/info.banner`; `AppShell.svelte` renders a fixed top bar when non-null. UI work is a separate small PR, owned by the frontend; this spec defines only the contract.
- **Tests:** `dev_responses_carry_profile_header`, `prod_responses_have_no_profile_header`, `node_info_reports_profile_and_banner`.

## 6. PR-5: DB profile marker
**Evidence:** DB creation paths are `crates/bosc-runtime/src/db_manager.rs:334` `create_database`, `lib.rs:966` `BosEngine::create_database`, `lib.rs:1704` `first_run_provision` and `meta_manager.rs:263` `provision_system_kernel`. Open/switch paths are `BosEngine::connect` (used at `main.rs:1006`) and `lib.rs:798` `switch_database`.
```sql
CREATE TABLE IF NOT EXISTS _bos_node_marker (
  marker_version int  NOT NULL DEFAULT 1,
  db_uuid        uuid NOT NULL,            -- v7, generated once
  profile        text NOT NULL CHECK (profile IN ('development','staging','production')),
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by_node uuid NULL,
  is_masked_copy boolean NOT NULL DEFAULT false,
  singleton      boolean NOT NULL DEFAULT true UNIQUE CHECK (singleton)
);
```
- **Write:** in `MetaManager::provision_system_kernel` (so every creation path gets it), using the node's current profile. An existing marker is never overwritten.
- **Check:** `pub async fn check_db_marker(pool: &PgPool, p: SecurityProfile) -> Result<MarkerState, MarkerError>`, called in the startup check (#11) and in `switch_database` before the switch is committed.
| DB marker → / node profile ↓ | development | staging | production | missing |
|---|---|---|---|---|
| development | open | refuse | **refuse** ("holds production data") | open, then write marker `development` (legacy DBs) |
| staging | refuse | open | refuse | refuse; `bosc db mark --profile staging` |
| production | refuse | refuse | open | refuse; `bosc db mark --profile production` |
- `bosc db mark --profile <p>`: sets a missing marker. `bosc db remark --from development --to staging --confirm`: audited, and production → development is **never** allowed. Implementing these CLI commands is in PR-5; masked copies (`bosc db clone --mask`, which sets `is_masked_copy = true` and `profile = development`) are **out of scope** (P1) and only noted here.
- Refusal error: `MarkerError::ProfileMismatch { db: "production", node: "development" }` → exit 77 at startup; HTTP 409 `DB_PROFILE_MISMATCH` from `switch_database`.
- Dev extra: refuse a non-local DB host (`database.url` host not `localhost`/`127.0.0.1`/`::1`/a Docker service name) unless `[database] dev_allow_remote_db = true` (WARN).
- **Tests:** `marker_written_on_provision`, `marker_not_overwritten`, `dev_refuses_production_db`, `prod_refuses_development_db`, `staging_refuses_missing_marker`, `dev_adopts_legacy_db_without_marker`, `switch_database_refuses_mismatch_409`, `remark_prod_to_dev_forbidden`.

## 7. PR-6: AuthKernel boundary and dev auto-login
**Evidence:** `auth_guard.rs` `resolve_session` (112-176) builds `SessionContext` with `user_id: Uuid::nil()` (lines 128, 172); `require_session` (185+).
- Add `crates/bosc-runtime/src/auth_kernel.rs` with the trait and `VerifiedSession` from ADR 0004. `BosAuthKernel` wraps the existing `AuthManager.VerifySession` call (moved from `resolve_session`). `DevStandInAuth` lives behind `#[cfg(feature = "dev-profile")]` (default feature on; release builds use `--no-default-features`).
- Dev auto-login rules (all must hold, otherwise behave as staging):
  1. profile == Development;
  2. the request's peer address (`ConnectInfo<SocketAddr>`, **not** `x-forwarded-for`) is loopback;
  3. `[security] dev_auto_login = true` (default true in development);
  4. there's no valid token on the request (a real token always wins).
  The result is `VerifiedSession { user_id: DEV_ADMIN_ID, tenant_id: <first active tenant or dev tenant>, roles: ["SUPER"], issued_by: DevStandIn, profile: Development, .. }`, with `DEV_ADMIN_ID` a fixed v7-format constant documented in ADR 0004. The response gets `X-BOS-Stand-In: dev-admin`.
- `router.into_make_service_with_connect_info::<SocketAddr>()` is needed at `main.rs:1023+` (serve call) so the peer address is available.
- Stand-in rejection: `fn reject_stand_in(s: &VerifiedSession, node: SecurityProfile) -> Result<(), AuthError>`, called inside `require_session` and in any handler that accepts a receipt. If not Development and the session is a stand-in → 401 `STAND_IN_REJECTED` + audit `auth.stand_in.rejected`.
- `dev.admin` is **not** written to the user table (it's synthetic), so check #6 can spot a real `dev.admin` row as an error in production.
- **Tests:** `dev_loopback_no_token_gets_dev_admin`, `dev_non_loopback_peer_gets_401` (test with a peer of 10.0.0.5 even though bind is loopback, e.g. via a proxy), `dev_auto_login_disabled_gets_401`, `dev_real_token_wins_over_stand_in`, `staging_never_issues_stand_in`, `stand_in_session_rejected_outside_dev`, `stand_in_receipt_rejected_outside_dev`, `x_forwarded_for_does_not_enable_auto_login`.
**Out of scope:** hashed session tokens, non-nil user id from bos.auth (P1, D2 addendum d), MFA.

## 8. PR-7: Profile-gating iteration-1 behaviours
| # | Behaviour (evidence) | development | staging | production |
|---|---|---|---|---|
| 1 | Anonymous `/api` gate `require_session` (`server.rs` ~348; `auth_guard.rs` 185+) | layer stays; no-token loopback requests get the stand-in (PR-6); non-loopback gets 401 | enforced as-is | enforced as-is |
| 2 | Public route list `is_public_route` (`auth_guard.rs` 27-57), incl. `GET /api/databases` + `test-connection` | same list | same list, **plus** `/api/databases*` public only while `needs_bootstrap()` is true (D2 addendum a) | same as staging |
| 3 | Emergency login `emergency_admin` / `emergency_username` (`server.rs` ~690-707; config `node_config.rs:239-256`) | disabled (stand-in replaces it); returns 403 `EMERGENCY_DISABLED_IN_PROFILE` | HTTP path disabled: 403; CLI break-glass only (P1, addendum e) | same as staging |
| 4 | Auto-fallback `admin` login (`server.rs` ~856-863; default true `node_config.rs:277`, template `:431`) | disabled (403) | disabled; startup check #6 is **fatal** if `auto_fallback_on_auth_failure = true` | same |
| 5 | Default of `auto_fallback_on_auth_failure` | change `default_bool_true` → `false` (line 250-251, 277) and template line 431 → `false`; update the assertion at `node_config.rs:463` | | |
| 6 | `maintenance_mode` (`node_config.rs:239`) | ignored for login (log WARN) | startup check #6 fatal if true | fatal if true |
| 7 | `x-bos-tenant-id` header for lookup (`auth_guard.rs` `resolve_session` ~119-123; `server.rs` `extract_context` ~451) | honoured (lets a dev switch tenants) | **ignored** for identity; tenant comes from the session record. Until P1 hashed sessions land, the header is accepted only if `VerifySession` confirms the token belongs to that tenant (current RLS behaviour); log a deprecation WARN | same as staging |
| 8 | `default_tenant()` fallback (`auth_guard.rs` 86-100) | allowed | allowed only as the lookup hint during P0/P1, never as the resulting identity; removed in P1 | same |
| 9 | `x-forwarded-for` / `x-real-ip` as client IP (`auth_guard.rs` ~132) | used for logs only | used only if the peer is in `[network] trusted_proxies`, otherwise the peer address | same |
| 10 | Lookup context with `roles: ["SUPER"]` (`auth_guard.rs` ~125-130) | unchanged in P0 | unchanged in P0; replaced in P1 by a kernel system context with only `auth.session.verify` | same |
| 11 | Identity-header stripping, `extract_context` no implicit roles (`server.rs` ~466-471), setup detection `identity_state`, RoleLogic arity fix (`quickjs_engine.rs` ~1807) | **unchanged, all profiles** | unchanged | unchanged |
- Implement as `fn login_bypass_allowed(p: SecurityProfile) -> bool { false }` (i.e. HTTP bypass is never allowed in any profile after P0), keeping `identity_state` for setup only.
- **Tests** (`tests/p0_profile_gating.rs`): `emergency_login_403_in_all_profiles`, `fallback_admin_login_403_in_all_profiles`, `staging_start_fails_with_auto_fallback_true`, `auto_fallback_default_is_false`, `prod_tenant_header_does_not_change_identity`, `dev_tenant_header_honoured`, `xff_ignored_without_trusted_proxy`, `iter1_tests_still_pass` (keep `tests/iter1_security.rs` green unchanged).

## 9. Docs to update alongside (small docs PRs)
`docs/08` (auth: profile table, stand-in), `docs/09_bosc_cli_reference.md` (`--security-profile`, `--dev`, `--host`, `bosc db mark/remark`, exit codes), `docs/13` (sec 4.3 emergency login is now CLI-only), `docs/00_architecture_overview.md` (link to `docs/adr`), and the `bos-node.toml` sample.

## 10. Definition of done for P0
- [ ] PR-1 to PR-7 merged locally with the tests above passing.
- [ ] `bosc node run --dev` on a clean machine: listens on 127.0.0.1, auto-logs in, shows header and banner, creates a DB marked `development`.
- [ ] `bosc node run --dev` with `host = "0.0.0.0"`: exits 77 with the staging hint.
- [ ] `bosc node run` (no profile) against a superuser DB: exits 77 listing the fatal production checks.
- [ ] A dev node pointed at a DB marked `production`: exits 77.