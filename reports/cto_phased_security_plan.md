# BOS phased security plan (CTO, plan only)
Date: 2026-10-24. Author: CTO / Chief Architect. Status: **PROPOSAL, not signed off.**
Owner direction: BOS core gets open, fast development access now, and full security is enforced only before market launch. Formats are decided now so nothing has to be rewritten later.
Depends on D1/D2/D3/D9 (`reports/cto_proposals_d1_d2_d3_d9.md`) and `reports/cto_totp_plan.md`, which are **not signed off**. D6 (money) is approved and not touched.

## 0. What this supersedes
| Earlier text | Now |
|---|---|
| D2 (auth), D2 addendum (a)-(e) | Same end state, but enforced **per profile** (sec 2). Addendum (b) no default password, (d) nil user = 401 and (e) CLI-only break-glass apply in staging/production. In development a dev auto-login exists instead. |
| D3.5 signing & trust, D3.10 publishing flow | Formats (sec 3.5-3.6) are frozen now. The publisher role, TOTP and OpenBao signing move to the "build later" phases (sec 4). Development accepts unsigned packages. |
| D9 (DB roles, secrets, CORS) | DB roles, OpenBao and the CORS allow-list are built later (sec 4). Development allows a superuser DB, file/env secrets and localhost CORS. |
| TOTP plan sec 1-16 | The design is unchanged and becomes phase P2/P3 work. In development `ctx.kernel.mfa.*` is a stub. |
| "OpenBao is required" wording anywhere | Replaced by the `SecretProvider`/`KeyProvider` traits (sec 5.7). OpenBao is the staging/production implementation. |

## 1. Examples first
- **Asha (core dev)** runs `bosc node run`. The node uses `security_profile = "development"`, listens on 127.0.0.1:8080, logs her in as `dev.admin` automatically, shows a red "DEVELOPMENT - NOT SECURE" bar and accepts her unsigned `bos.sales` package. Setup takes seconds.
- **Ravi (QA)** wants to test from another laptop on the LAN. Development refuses to bind to `192.168.1.20`, so he runs the node with `security_profile = "staging"`. That needs real login, a non-superuser DB role, a CORS list and synthetic data only. TOTP is optional for testers.
- **First pilot customer**: the node runs `production`. It won't start until TOTP, OpenBao, signed packages, DB roles and TLS are all in place.

## 2. `security_profile`

### 2.1 How it is set (startup only)
- Precedence: CLI flag `--security-profile` > env `BOS_SECURITY_PROFILE` > `bos-node.toml [security] profile`. If nothing is set the node uses **production**, so a missing setting is never insecure. `bosc node run --dev` is shorthand for development, for convenience.
- The profile is read **once at startup** into an immutable `SecurityProfile` value in the kernel. There is no API, Studio setting, package hook or tenant setting that can change it. Packages can read it (`ctx.kernel.node.profile`) but can't write it. Changing it means editing config and restarting, and the restart is audited (`node.profile.started`).
- A build feature can remove development mode completely: release images are built with `--no-default-features --features prod-only`, and development can't be selected in them.

### 2.2 Control-by-profile matrix
| Control | development | staging | production |
|---|---|---|---|
| Bind address | **127.0.0.1 / ::1 only**; anything else refuses to start | any; TLS strongly recommended | any; **TLS required** (or a TLS-terminating proxy with `trusted_proxies` set) |
| Login | optional; auto-login as `dev.admin` (SUPER) if `dev_auto_login = true` | required (bos.auth) | required |
| Default/seed password | allowed (`dev.admin`/random, printed to console) | none (addendum b) | none |
| TOTP / MFA | off (stub returns "not required") | optional per tenant; required for platform operators | **required** for platform realm; tenant policy enforced |
| Step-up | stub passes | enforced | enforced (publish 5 min single use) |
| Emergency / fallback login | off (auto-login makes it unnecessary) | CLI break-glass only | CLI break-glass only; `auto_fallback_on_auth_failure` must be false |
| Anonymous `/api` gate (`auth_guard`) | dev session auto-injected | enforced | enforced |
| Packages | unsigned local allowed (warning) | signed required; test keys allowed | signed by trusted, unrevoked, in-scope keys only |
| Secrets | `.bos/secrets.toml` or env (file provider) | OpenBao (or cloud KMS) | OpenBao or cloud KMS; file provider forbidden |
| MFA secret encryption | local dev key file | Transit | Transit; fail closed |
| DB role | superuser allowed | `bos_app` (non-superuser, no BYPASSRLS) | D9 roles in full; startup checks it |
| RLS | created (formats identical), FORCE on | FORCE + RESTRICTIVE | FORCE + RESTRICTIVE, verified at startup |
| CORS | `http://localhost:*`, `http://127.0.0.1:*` | explicit allow-list | explicit allow-list, no `*` |
| Rate limits / lockout | off | on (relaxed) | on (TOTP plan sec 7) |
| Audit log | written (same shape), no hash chain required | written, hash chain | hash chain + WORM mirror |
| Data | synthetic only; dev DB marker required | synthetic or masked only | real data |
| Banner | UI red bar, log line on every start + every 10 min, header `X-BOS-Security-Profile: development` | amber "STAGING" bar + header | no banner (header optional, off by default) |

### 2.3 Development binding rule (exact)
- Allowed addresses: `127.0.0.1` and `::1`, plus `localhost` only if it resolves to one of those. `0.0.0.0`, `::`, LAN or public addresses mean the node **refuses to start** with: "development profile only listens on 127.0.0.1/::1; use staging for LAN access". Note that the current default `host = "0.0.0.0"` (`crates/bosc-runtime/src/node_config.rs:74`, `:394`) and the CLI's `0.0.0.0:{port}` (`crates/bosc-cli/src/main.rs:618`) must become profile-aware: the default becomes 127.0.0.1 in development.
- Docker in development: the container binds 0.0.0.0 *inside* the container, so the check uses the published host port. Compose files must publish `127.0.0.1:8080:8080`. An env flag `BOS_DEV_IN_CONTAINER=1` allows the inside bind only together with that check in docs; the CLI warns.
- LAN/QA access always requires **staging**. There is no "dev but on LAN" option.

### 2.4 Startup checks (production refuses to start if any fail)
1. Profile resolved and logged; it's production when unset.
2. Bind/TLS rule (sec 2.2).
3. DB connection role: `rolsuper = false`, `rolbypassrls = false`, not the table owner (`SELECT rolsuper, rolbypassrls FROM pg_roles WHERE rolname = current_user`).
4. Every tenant table has RLS `ENABLE` + `FORCE` and the RESTRICTIVE tenant policy (catalog query on `pg_class.relforcerowsecurity`, `pg_policy`).
5. SecretProvider/KeyProvider is a non-file implementation and is reachable and unsealed (a test sign/verify round-trip on a canary key).
6. `auto_fallback_on_auth_failure = false`, `maintenance_mode = false`, no seed/default users, no `dev.admin` user present.
7. CORS list is non-empty and has no `*`.
8. Trust store loaded, signed by the root key and fresher than 7 days. All installed packages verify.
9. Platform-realm MFA enabled, and at least 2 platform Super Admins with TOTP enrolled.
10. Clock-sync check (TOTP plan sec 11).
11. The DB is **not** marked as a dev DB (sec 2.5).
Staging runs the same checks but treats 9 and 10 as warnings. Development runs only 2 (localhost) and 11-reverse (sec 2.5).

### 2.5 Keeping real data off dev nodes
- **Dev DB marker**: table `_bos_node_marker(profile text, created_at timestamptz, db_uuid uuid)` is written when the database is first provisioned. A development node only opens DBs whose marker says `development`. A staging or production node **refuses** a DB marked `development`. A dev node refuses a DB marked `production` ("this database holds production data").
- Changing a marker needs `bosc db remark --from development --to staging` with a confirmation, and it's audited. production → development is **not allowed**; use a masked copy (`bosc db clone --mask`).
- Development refuses DB hosts that are not localhost or a Docker network name unless `dev_allow_remote_db = true`, which also prints a warning.
- Dev seed data is generated (`bosc dev seed`) and marked `is_synthetic`. Imports in development show "do not import real customer data".
- Production backups restored onto a dev node fail because of the marker check.

## 3. DECIDE NOW: formats (no enforcement)
These are frozen now so dev code and data already match production. Each has a version field so it can evolve without rewrites.

### 3.1 IDs: UUIDv7 plus readable names
```
id         uuid  PRIMARY KEY DEFAULT bos_uuid_v7()   -- RFC 9562 v7, time-ordered
code       text  NOT NULL                            -- readable name, unique per tenant (+company)
```
Generated by the kernel (`Uuid::now_v7()`), never by clients. `Uuid::nil()` is never a valid id, in any profile.

### 3.2 Tenant ID on every row
```
tenant_id  uuid NOT NULL REFERENCES _bos_tenants(tenant_id)
company_id uuid NULL      -- only for company-scoped tables; NULL means tenant-wide, never "visible to all"
-- every tenant table: UNIQUE (tenant_id, code), indexes lead with tenant_id
-- session setting: SET LOCAL bos.current_tenant_id = '<uuid>'  (from the verified session only)
```
The column exists and RLS policies are generated in **all** profiles. Development may connect as a superuser (which bypasses RLS), but the policy text is identical. This fixes the permissive NULL-company policy (proposals sec 0, `bosc-codegen-db/src/lib.rs:895-905`) at the format level.

### 3.3 Logical table names
Packages refer to logical names `<package>.<Object>` (e.g. `bos.sales.SalesOrder`). Physical names are resolved by the kernel per layout (D1: `shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`), and package code never builds SQL table names. The mapping is stored in `_bos_object_map(logical_name, layout, physical_schema, physical_table, version)`. The hard-coded `comp_default` / `default$` (proposals sec 0) is removed from formats.

### 3.4 Kernel / bos.auth boundary and hooks
The same API in every profile. Development uses a permissive implementation, not a different API.
```rust
trait AuthKernel {                    // kernel side (Rust)
    fn verify_session(&self, token: &str) -> Result<VerifiedSession>;   // receipt: {user_id, tenant_id, roles, mfa_at, profile}
    fn hash_session_token(&self, token: &str) -> [u8; 32];              // stored hashed (SHA-256), never raw
    fn require_capability(&self, s: &VerifiedSession, cap: &str) -> Result<()>;
    fn mfa_verify(&self, s: &VerifiedSession, code: &str) -> Result<MfaReceipt>;   // TOTP plan sec 4
    fn step_up(&self, s: &VerifiedSession, purpose: StepUpPurpose, max_age: Duration) -> Result<StepUpReceipt>;
    fn audit(&self, e: AuditEvent);
}
```
- `bos.auth` owns users, roles, login UI, password policy, enrolment screens and tenant MFA policy. The kernel owns token hashing, receipts, crypto, capability gates and audit writing.
- Dev implementation `DevAuthKernel`: `verify_session` returns `dev.admin` (SUPER) when auto-login is on; `mfa_verify` and `step_up` return a receipt marked `"stub": true`; `require_capability` allows everything except `node.profile.change`, which doesn't exist. Receipts carry `profile`, so a stub receipt is rejected by staging/production code paths.

### 3.5 Package manifest, SemVer, checksum
```json
{
  "manifest_version": 2,
  "id": "bos.tax.in",                 // lowercase, dot namespace; bos.* / vits.* reserved for VITS
  "version": "1.4.0",                 // SemVer 2.0.0; published versions immutable
  "layer": "localization",            // base | localization | industry | tenant
  "engine": ">=0.9.0 <2.0.0",         // node-semver range syntax
  "dependencies": { "bos.core": "^1.2.0" },
  "capabilities": ["db.read", "db.write", "net.outbound:gst.gov.in"],
  "source": { "repo": "https://github.com/vits/bos-packages", "commit": "a1b2c3d" },
  "checksum": { "algo": "sha256", "value": "<64 hex>" }   // filled by build; over canonical bundle
}
```
Canonical bundle: sorted-key UTF-8 JSON manifest (with `checksum.value` blank while hashing) + `0x00` + tar (fixed file order, mtime 0, uid/gid 0, mode 0644/0755). `sha256 = SHA-256(manifest || 0x00 || archive)`, lowercase hex. This replaces the 64-bit DefaultHasher (`package_manager.rs:36-42`).

### 3.6 Signature format (Ed25519, detached)
File `<id>-<version>.sig.json`, next to the bundle:
```json
{
  "sig_version": 1,
  "alg": "ed25519",
  "package_id": "bos.tax.in",
  "version": "1.4.0",
  "layer": "localization",
  "sha256": "<64 hex>",
  "key_id": "bos-pkg-vits",
  "key_version": 3,
  "signer": "vits" ,                    // or partner id, e.g. "partner:acme"
  "signed_at": "2026-10-24T10:30:00Z",
  "source_commit": "a1b2c3d",
  "signature": "<base64 64-byte Ed25519 over the 32 raw digest bytes>"
}
```
Development: the file may be missing (unsigned), or signed with a dev key `dev-local` that only development trusts.

### 3.7 Audit event shape
```json
{
  "event_version": 1,
  "id": "<uuidv7>",
  "at": "2026-10-24T10:30:00.123Z",       // UTC
  "type": "pkg.publish.signed",           // dot-separated, from a registry of types
  "outcome": "success",                   // success | failure | denied
  "reason": null,                         // machine code on failure
  "actor": { "user_id": "<uuid>", "realm": "platform", "name": "priya", "via": "cli" },
  "tenant_id": "<uuid|null>", "company_id": "<uuid|null>",
  "target": { "kind": "package", "id": "bos.tax.in", "version": "1.4.0" },
  "details": { },                         // type-specific, no secrets/OTPs/tokens ever
  "request": { "ip": "203.0.113.5", "request_id": "<uuid>", "node_id": "<uuid>" },
  "profile": "production",
  "prev_hash": "<sha256 hex|null>", "hash": "<sha256 hex>"   // chain fields; may be null in dev
}
```

## 4. BUILD LATER: phases and triggers
| Phase | Trigger | Items |
|---|---|---|
| **P0 now** | started | `security_profile` + matrix switches, startup checks framework, dev DB marker, localhost bind rule, banners/header; formats sec 3; trait abstractions sec 5.7 with File providers; iteration-1 fixes (sec 6) |
| **P1** | before any shared/LAN QA or staging server | staging profile working: real login only, no defaults (addendum b), CORS allow-list, `bos_app` non-superuser role + FORCE/RESTRICTIVE RLS, hashed session tokens, cookie/stream ticket (addendum c), nil user = 401 (addendum d), CLI break-glass (addendum e), `/api/databases*` SUPER-only (addendum a), audit hash chain |
| **P2** | before the first external pilot (any real customer data) | OpenBao 3-node raft + backups + Shamir (sec 5), TOTP for platform realm and tenant policy (TOTP plan M0-M4), rate limits/lockout, full D9 roles (`bos_owner/migrator/app/worker/readonly/control`), signed packages required, trust store + revocation list, production startup checks enforced, TLS |
| **P3** | before public launch / marketplace opening | VITS Package Publisher role + two-person grant, publish step-up (D3.10), partner key registration + scope, WORM audit mirror, quarterly recovery drill passed once, external penetration test, `prod-only` release build without development mode |
| **P4** | before regulated tenants (finance/health/government) | dedicated DB per regulated tenant by default, per-tenant Transit keys, optional cloud KMS auto-unseal, dedicated cluster option, data residency evidence |

Rule: a phase's items must pass their acceptance tests before the trigger event. The CEO signs off on each gate.

## 5. OpenBao failure and recovery

### 5.1 Topology
3-node integrated raft cluster (tolerates 1 node down) on separate hosts/zones in the India region. TLS between nodes. Audit device enabled to file + remote syslog.

### 5.2 Backups (raft snapshots)
- `bao operator raft snapshot save` **every hour**, kept 48 h. Daily copies are kept 35 days and monthly copies 12 months.
- Snapshots are encrypted (age/GPG to a backup public key whose private key is held offline) **before** they leave the host. They're stored in 2 places: the primary object store (versioned, object lock) and **offsite** in a different provider/region.
- Snapshots contain sealed data. Restoring them still needs the unseal keys (sec 5.3), so a stolen snapshot alone is useless.
- **Restore tested** monthly and automatically on a scratch cluster (restore, unseal, sign/verify canary, decrypt canary MFA secret), plus the quarterly drill (sec 5.8).

### 5.3 Unseal: Shamir shares
- **3-of-5** shares, PGP-encrypted to each holder at `operator init`. Holders are 5 different people: CEO, CTO, Head of Ops, Security lead and one independent director/trustee. No two report to each other where possible, and never all in one city.
- Each share is stored on a hardware token or in a sealed envelope in that holder's bank locker or safe, plus an encrypted copy in a separate password vault. A holder register (name, date, location type) is kept with no share content.
- Re-key (`bao operator rekey`) when a holder leaves, or yearly.
- The root token is revoked after setup, and a new one is generated only with a quorum (`generate-root`) for emergencies.

### 5.4 Auto-unseal option
Production may use **cloud KMS auto-unseal** (AWS KMS / GCP KMS / Azure Key Vault) so restarts don't need people. In that case the Shamir shares become **recovery keys** (3-of-5) for root generation and rekey. The trade-off is that loss of the KMS key equals loss of the cluster, so the KMS key must have deletion protection plus a 30-day pending-delete window. The owner decision says cloud KMS is optional: the default is Shamir and auto-unseal is opt-in.

### 5.5 Offline root signing key (package signing survives loss)
- Two levels. An **offline root key** (Ed25519) is generated on an air-gapped machine, split 3-of-5 (Shamir, same holder model) and kept in escrow (bank lockers + a legal escrow agent). It signs only the **trust store**.
- The **online signing key** (`bos-pkg-vits` in Transit) signs packages and is listed in the trust store.
- If OpenBao is lost: rebuild OpenBao, create a new `bos-pkg-vits` key (new version line), hold a root ceremony (3 holders) that signs a trust store listing the new key, then publish. Existing packages stay valid because their old public key stays in the trust store (not revoked) unless it was compromised.
- If the online key is compromised: a root ceremony revokes it, and the affected packages are re-signed.
- If the root key is lost: every node needs a new root distributed out-of-band (release + manual import). That's costly, which is why it's escrowed.

### 5.6 Fail closed vs fallback, per function and profile
| Function | development | staging | production |
|---|---|---|---|
| Login (password) | not using OpenBao (file provider) | works if DB up (passwords are Argon2id in DB, not in OpenBao) | same: login doesn't depend on OpenBao |
| MFA verify (needs Transit decrypt) | stub | fail closed for MFA users; break-glass via CLI | **fail closed**: MFA users can't log in; CLI break-glass on node host only, audited |
| Package signing/publish | dev key file | fail closed | **fail closed**; no local fallback key |
| Package install verify | n/a | works offline (public keys in trust store) | works offline: needs no OpenBao |
| DB credentials | file/env | dynamic or static from OpenBao; cached lease until expiry | running nodes keep current lease (TTL 24 h, renew at 50%); new nodes can't start; after expiry **fail closed** |
| Tenant secrets (API keys etc.) | file/env | fail closed per call | fail closed per call; cached values in memory only (max 15 min) |
Corruption (raft data unreadable): treat as loss. Stop the cluster, restore the latest good snapshot to a new cluster, unseal with 3 shares, verify canaries, then repoint nodes. Writes made after the snapshot (e.g. new MFA enrolments) are lost, and those users re-enrol.

### 5.7 Abstraction traits
```rust
#[async_trait]
pub trait SecretProvider: Send + Sync {
    fn name(&self) -> &'static str;                                   // "file" | "openbao" | "aws-kms" ...
    async fn health(&self) -> ProviderHealth;                         // ok | sealed | unreachable
    async fn get(&self, r: &SecretRef) -> Result<SecretValue>;        // r = "bao:kv/bos/db/app#password", "env:BOS_DB_PASSWORD", "file:..."
    async fn put(&self, r: &SecretRef, v: SecretValue) -> Result<Version>;
    async fn delete(&self, r: &SecretRef) -> Result<()>;
    async fn lease_db_credentials(&self, role: &str) -> Result<Lease<DbCredentials>>;
    async fn renew(&self, lease_id: &str) -> Result<Duration>;
}

#[async_trait]
pub trait KeyProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn health(&self) -> ProviderHealth;
    async fn create_key(&self, key_id: &str, kind: KeyKind) -> Result<KeyVersion>;   // Aes256Gcm | Ed25519
    async fn encrypt(&self, key_id: &str, plaintext: &[u8], aad: &[u8]) -> Result<Ciphertext>;   // envelope, carries key version
    async fn decrypt(&self, key_id: &str, ct: &Ciphertext, aad: &[u8]) -> Result<Zeroizing<Vec<u8>>>;
    async fn rewrap(&self, key_id: &str, ct: &Ciphertext) -> Result<Ciphertext>;
    async fn sign(&self, key_id: &str, digest32: &[u8; 32]) -> Result<Signature>;   // returns key_version
    async fn public_key(&self, key_id: &str, version: u32) -> Result<PublicKey>;
    async fn rotate(&self, key_id: &str) -> Result<KeyVersion>;
}
```
Implementations: `FileSecretProvider`/`FileKeyProvider` (development only; keys in `.bos/dev-keys/`, refused by staging/production), `OpenBaoProvider` (KV v2 + Transit + database engine), `CloudKmsProvider` (AWS/GCP/Azure; optional). They're selected by `[secrets] provider = "..."`. Packages see only `ctx.kernel.secrets.get(ref)` with capability checks, never the provider.

### 5.8 Recovery drill (quarterly)
Steps: (1) announce the drill window; (2) take the latest snapshot from **offsite** storage; (3) build a fresh 3-node cluster from scratch with infra code; (4) restore and unseal with 3 holders (at least 1 remote); (5) run the canaries: decrypt a test MFA secret, sign + verify a test package digest, lease DB creds; (6) point a staging node at it and run login + MFA + install tests; (7) simulate online-key loss: root ceremony signs a new trust store; (8) record timings and gaps, and fix within 30 days.
Targets: **RTO 4 h** for the cluster (MFA/signing back), **RPO 1 h** (hourly snapshots). Login without MFA is not affected (sec 5.6).

## 6. Backend Engineer iteration-1 changes (branch `fix-iter1-security`, read-only)
Evidence: `git diff $(git merge-base main fix-iter1-security) fix-iter1-security`. The commits are `f68a276a`, `9f62b527` and `0e0b974f`, which change `crates/bosc-runtime/src/auth_guard.rs` (new), `server.rs`, `quickjs_engine.rs`, `lib.rs`, `tests/iter1_security.rs` and `.gitignore`. Note that the main tree is currently on `chore-single-node-run` with other uncommitted work; I didn't touch it.

**Stay as-is in all profiles (correctness, not policy)**
| Change | Where | Why all profiles |
|---|---|---|
| Lifecycle trigger arity fix: `onInsert(rec, ctx)` gets ctx (RoleLogic F2) | `quickjs_engine.rs` ~1807-1814; test `tests/iter1_security.rs` | a bug fix; dev must behave like prod |
| Identity-based setup detection (`IdentityState::needs_bootstrap`, `needsBootstrap`, `usersCount`, `rolesCount`) | `auth_guard.rs` `identity_state`; `server.rs` `auth_status` | correct first-run logic everywhere |
| Strip client identity headers (`x-bos-session`, `x-bos-user-id`, `x-bos-roles`) | `auth_guard.rs` `strip_identity_headers` | even dev auto-login must inject a server-built session, never trust clients |
| No implicit SUPER/Admin when roles header missing (`unwrap_or_default()`) | `server.rs` `extract_context` ~466-471 | dev gets SUPER from the dev session, not from a missing header |
| Kernel keeps no session store; verification via `AuthManager.VerifySession` | `9f62b527`; `auth_guard.rs` `resolve_session` | matches the boundary (sec 3.4) |
| Emergency/fallback restricted to maintenance mode, or auto-fallback **only** when identity is empty | `server.rs` `auth_login_handler` ~690-707, ~856-863 | strictly safer; staging/production go further (below) |
| Public-route list + `GET /api/databases` and `test-connection` only for setup | `auth_guard.rs` `is_public_route` | same list in all profiles; addendum (a) tightens it in P1 |

**Become profile-gated**
| Change / behaviour | development | staging/production |
|---|---|---|
| Anonymous `/api` gate (`require_session`, `server.rs` ~348) | the layer stays; a `DevAuthKernel` session for `dev.admin` is injected when there's no token (localhost only) | enforced as written |
| Emergency `emergency_admin` / `emergency_username` login (`node_config.rs:239-254`) | unnecessary (auto-login) | removed from HTTP; CLI break-glass only (addendum e); `auto_fallback_on_auth_failure` (default true, `node_config.rs:277`) must be false, startup check |
| `bos_sess_emergency_*` / `bos_sess_fallback_*` tokens | not issued | never issued |
| `default_tenant()` fallback and `x-bos-tenant-id` header for lookup | allowed | tenant comes from the hashed session record (D2); header ignored |

**Gaps to fix in P1 (found in the diff)**
- `resolve_session` returns `user_id: Uuid::nil()` (`auth_guard.rs`), so it can't satisfy addendum (d) "nil user = 401". It needs `VerifySession` to return the user id.
- The lookup context runs `VerifySession` with `roles: ["SUPER"]`. It should use a dedicated kernel system context with only the `auth.session.verify` capability.
- The client IP comes from `x-forwarded-for` without a trusted-proxy check, so the address is spoofable. Use `trusted_proxies`.
- Default `host = "0.0.0.0"` (`node_config.rs:74`) conflicts with the development bind rule (sec 2.3).

## 7. Decisions (approved by Md Aziz, 2026-10-24)
The phased security plan is **APPROVED**. The former open questions are decided:
1. **Default profile when unset: production.** This applies everywhere, including the `bosc` CLI. Development must be chosen explicitly (`--dev`, `--security-profile development`, env or toml).
2. **Shamir holders (3-of-5):** Md Aziz; two senior VITS staff members; two sealed copies kept in separate safes. Holder names and safe locations go in the holder register only (sec 5.3), never in the repo.
3. **No cloud KMS auto-unseal.** OpenBao uses Shamir unseal only (sec 5.4 auto-unseal is not adopted; it could only be revisited by a new owner decision).
4. **Staging requires TOTP for QA testers** as well as platform operators (matrix sec 2.2: the staging TOTP row becomes "required"; startup check 9 is enforced in staging, not only a warning).
5. **RTO 4 h / RPO 1 h** accepted for OpenBao (MFA and signing).
6. **D1/D2/D3/D9 end states accepted as phase-gated targets** (enforced per phase table sec 4, profile matrix sec 2.2).