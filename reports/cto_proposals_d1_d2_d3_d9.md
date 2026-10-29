# CTO Proposals: D1 Tenancy, D2 Auth, D3 Packages, D9 Roles/Secrets/CORS

Date: 2026-10-24 (IST). Author: CTO agent. Status: **PROPOSAL, not approved.** D6 (money) is approved and not revisited here.
Read-only review of `D:\Work Studio\Bos` (working tree on `fix-iter1-security` with the Backend Engineer's uncommitted WIP; code evidence below may change as that WIP lands). No code changed, nothing built or run.

Legend: **REVERSIBLE** = can change later with a config switch or small refactor. **COSTLY** = reversible but needs data migration or a contract break. **IRREVERSIBLE** = once customers/packages depend on it, effectively permanent.

Example tenants used throughout:
- **Shop**: 3 users, one company, a few thousand invoices a year, price-sensitive.
- **Manufacturer**: 500 users, 6 companies (subsidiaries in 3 states), heavy MRP/ledger volume, wants per-company backup.
- **Bank-like**: regulated (RBI/SEBI-style), needs data residency in India, its own encryption keys, provable isolation, audit for every privileged action.

---

## 0. What exists today (evidence)

| Topic | What the code does today | Where |
|---|---|---|
| Strategy enum | `shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`; unknown strings silently fall back to `shared_rls` | `crates/bosc-ir/src/lib.rs:58-86` |
| Strategy scope | Chosen **per database** at create time (`create_database`, default `"shared_rls"`), stored in `_bos_sys_info.storage_strategy` | `crates/bosc-runtime/src/db_manager.rs:334-371`, docs/04 sec 2.1 |
| RLS | `ENABLE ROW LEVEL SECURITY` + one **permissive** policy for `shared_rls`/`partitioned` only. No `FORCE`. Company clause allows rows when company GUC is empty or `_company_id IS NULL`. docs/04 shows `AS RESTRICTIVE`, code does not | `crates/bosc-codegen-db/src/lib.rs:895-905` |
| schema_per_company | Hard-coded schema `comp_default` plus a `public` copy of every table; no RLS emitted | `bosc-codegen-db/src/lib.rs:686-733` |
| table_per_company | Hard-coded prefix `default$<table>`; no RLS emitted | `bosc-codegen-db/src/lib.rs:735+` |
| Per-company fan-out | `meta_manager.rs:438-567` creates partitions/schemas/prefixed tables for active tenants/companies; `relations.rs:193-203` resolves table names per strategy | |
| Tenant context | Tx-local GUCs `bos.current_tenant_id/company_id/user_id` via `set_config(...,true)` (good) | `crates/bosc-runtime/src/db_session.rs:68` |
| Tenant source | Client header `x-bos-tenant-id`, else "first active tenant"; `user_id: Uuid::nil()`; middleware re-inserts header | `crates/bosc-runtime/src/auth_guard.rs:113-159, 241`, `server.rs:455-468` |
| DB login | `postgres://postgres:postgres@...` (superuser, bypasses all RLS) in committed `bos-node.toml`; toml comment lists `schema_per_tenant`/`db_per_tenant` which do not exist in code | `bos-node.toml [database]` |
| CORS | `CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any)`; `cors_origins` config is read but not applied | `server.rs:164-167`, `node_config.rs:70,90` |
| Emergency | `maintenance_mode`, emergency SUPER session, `auto_fallback_on_auth_failure = true` default | `node_config.rs:239-277`, `bos-node.toml [security]` |
| Login flow | Already mostly in package: `packages/bos.auth/auth.bos` `codeunit AuthManager` verifies password via `ctx.crypto.argon2Verify`, generates token with `ctx.crypto.randomBytes(32)`, **stores the raw token** in `UserSession.token`, and **defaults roles to `["SUPER","Admin"]` when a user has none** (`auth.bos:664-722`) | |
| Kernel crypto exposed to JS | `__host_argon2_hash`, `__host_argon2_verify`, `__host_random_bytes` (UUIDv4 bytes = OS CSPRNG) | `crates/bosc-runtime/src/quickjs_engine.rs:805-834, 1541-1544` |
| First admin | `seed_auth_super_admin` with hard-coded password `Boss@123` into first active tenant | `crates/bosc-runtime/src/package_manager.rs:426-438` |
| Package manifest | `version: String`, `dependencies: Vec<String>` (IDs only, no ranges), no engine range | `crates/bosc-ir/src/lib.rs` `PackageManifestIR` |
| Package checksum | `DefaultHasher` 64-bit (`format!("{:016x}", hasher.finish())`), **not SHA-256** as docs/10 sec 3.1 claims; not a security integrity check | `package_manager.rs:36-42` |
| Install | dependency presence check, then upsert of `version` (silently overwrites, no up/downgrade logic) | `package_manager.rs:52-61, 233-251` |

**Docs note:** `docs/11` is the Svelte 5 UI Studio spec, not tenancy. Tenancy detail lives in `docs/04` sec 3 and the archived `stale-docs/11_adaptive_multi_tenant_storage_strategies.md` (which describes `comp_{code}` schemas and `{company_code}$table`; the code still hard-codes `comp_default` / `default$`). The README link to docs/11 is stale.

---

## D1. Tenancy: a provisioning wizard instead of one shared default

### D1.1 Principle
There is no silent default. Every BOS node and every tenant is created through a **Provisioning Wizard** (Studio UI + identical `bosc node init` / `bosc tenant create` CLI, both writing the same declarative `tenancy.plan.json`). The wizard recommends; the admin confirms. The unknown-string fallback to `shared_rls` (`bosc-ir/src/lib.rs:85`) must become an error.

### D1.2 Two levels, kept separate (the key design idea)
1. **Placement** (where a tenant lives): `pooled` (shares a database with other tenants) or `dedicated` (its own database, optionally its own Postgres cluster / region / keys).
2. **Layout** (how tables look inside that database): the four existing strategies `shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`.

The existing code already stores the strategy per database, so "dedicated DB per tenant + any layout" is a natural fit. The toml names `schema_per_tenant`/`db_per_tenant` are retired; `db_per_tenant` becomes *placement = dedicated*.

### D1.3 Wizard steps and questions
**Node wizard (once per node; `bosc node init`)**
1. Node role (all_in_one / interactive / nas_worker / api_gateway; already in `bos-node.toml`).
2. Hosting & residency: region (e.g. India Mumbai/Hyderabad per docs/30), allow cross-region? (y/n).
3. Postgres targets: one or more **database pools** (connection definitions referencing secrets, never inline passwords; see D9).
4. Tenancy modes this node will allow: pooled, dedicated, or both. Default layouts allowed (e.g. pooled DBs only `shared_rls`/`partitioned`).
5. Security floor: require `FORCE RLS` in pooled DBs (always yes, not editable), require TLS to DB, require per-tenant encryption keys (optional).
6. Creates roles (D9), kernel tables, writes `tenancy.plan.json`, runs isolation self-test (D1.5) and refuses to go live if it fails.

**Tenant wizard (per tenant; `bosc tenant create`)**
1. Size: users now / in 3 years; companies; expected yearly document volume.
2. Compliance: regulated? residency? own keys? separate backups/restore per company?
3. Migration source: new, NAV/BC import (suggests `table_per_company`), other ERP.
4. Budget tier (affects whether dedicated is offered).
5. Output: recommended placement + layout with plain reasons, editable; then provisions and runs the isolation test for that tenant.

Recommendation rules (shown to admin, not hidden):
| Answer pattern | Recommended | Example |
|---|---|---|
| <= 50 users, 1-3 companies, not regulated | pooled + `shared_rls` | Shop |
| High volume or many companies, wants per-company backups | dedicated DB + `schema_per_company` (or pooled + `partitioned` if cost-sensitive) | Manufacturer |
| Regulated / own keys / residency | dedicated DB (optionally dedicated cluster) + `schema_per_company` or `shared_rls` inside, FORCE RLS still on | Bank-like |
| NAV/BC migration | dedicated + `table_per_company` | Legacy NAV customer |

### D1.4 Routing: request -> tenant -> connection -> schema/table
One kernel component, the **Tenant Router**, with a catalog in a control database (`_bos_tenants` gains `placement`, `db_ref`, `layout`, `state`, `layout_version`):
1. Identity comes from the **verified session** (D2), never from `x-bos-tenant-id`. The header may only *select among* tenants the session is already authorised for; otherwise 403. `default_tenant` "first active tenant" logic is removed.
2. Router looks up tenant -> `db_ref` -> connection pool (pool per database, LRU-limited for many dedicated DBs).
3. Opens tx, sets GUCs (`db_session.rs:68` pattern, tx-local) including user id (non-nil).
4. Table resolution by layout (extend `relations.rs:193-203`):
   - `shared_rls`: `public."customer"`; RLS filters.
   - `partitioned`: `public."customer"`; RLS filters + partition pruning on `_tenant_id`.
   - `schema_per_company`: `SET LOCAL search_path = "c_<company_code>", "t_shared"` (tx-local, schema names from catalog, quoted/validated, never from request text); master tables in shared schema.
   - `table_per_company`: `"<company_code>$customer"` names generated from catalog (replacing hard-coded `default$`).
5. Every query plan is cached per (layout, company) key so routing adds no per-request DDL lookups.

### D1.5 Isolation guarantees per layout (what actually enforces it)
All layouts: app connects as `bos_app` (non-superuser, non-owner, `NOBYPASSRLS`), `_tenant_id` stays on every row (even in dedicated DBs, so moves are copies), tenant id comes from session.
| Layout | Enforcement | Residual risk |
|---|---|---|
| pooled `shared_rls` | `FORCE ROW LEVEL SECURITY` + **RESTRICTIVE** tenant policy (`USING` and `WITH CHECK`); company clause must not pass when GUC empty (fix current `IS NULL` escape at `lib.rs:900`) | Policy bug = cross-tenant leak; noisy neighbour |
| pooled `partitioned` | Same as above + partitions; optional per-partition tablespace | Same as above; partition count limits (~low thousands) |
| `schema_per_company` | Inside a tenant's DB: schema `USAGE` grants per company role or RLS on `_company_id`; across tenants: separate DB (pooled schema_per_company across tenants is **not** offered) | search_path mistakes; catalog bloat with many companies |
| `table_per_company` | Same as schema_per_company; generated names only | Name generation bugs |
| dedicated DB (any layout) | Separate database (or cluster) + separate credentials per DB + RLS inside | Ops cost; connection count |

Proof, not trust: an **isolation test suite** runs at node init, tenant create, after every migration, and in CI: as tenant A, try read/write/update/delete/`COPY`/function calls against B for every table; fail if `current_user` is superuser or has `BYPASSRLS`, or any tenant table lacks `FORCE`.

### D1.6 Migration between strategies
Supported moves, all driven by `tenancy.plan.json` diffs and the same tool:
- **pooled -> dedicated** (most common, e.g. Shop grows): *online*. Steps: (1) provision target DB with same package versions; (2) logical copy filtered by `_tenant_id` (snapshot + change capture via outbox/logical replication); (3) verify row counts + per-table checksums + isolation test; (4) brief write freeze (seconds-minutes) for final delta; (5) flip router `db_ref`; (6) keep source rows read-only for N days; (7) purge. **Rollback**: flip `db_ref` back before purge (deltas replicated back or freeze re-run).
- **shared_rls <-> partitioned** in same DB: *offline per table* or online via create-new-table + copy + swap; verify counts/checksums.
- **shared/partitioned -> schema_per_company / table_per_company** (layout change): *offline window* recommended (rewrites names and search paths); done on a dedicated DB copy, so rollback = keep old DB.
- Verification checklist always: counts, checksums per company, FK integrity, sequence/number-series positions, attachments, outbox drained, isolation test green.
- Package code must never reference physical names; it uses logical table names resolved by the router. This is what makes layout migration possible at all.

### D1.7 Mixing on one node; fixed vs per-tenant
- Mixing: yes. One node can serve pooled `shared_rls` tenants and dedicated tenants with any layout. One *database* has exactly one layout.
- **Fixed at node creation**: control DB location, region/residency policy, role names model, allowed modes. COSTLY to change.
- **Per tenant**: placement, layout, keys, backup policy. REVERSIBLE via D1.6 migrations (COSTLY for layout changes).

### D1.8 Options
- A) Shared RLS default (my earlier D1 rec) - rejected by owner.
- B) **Wizard with placement x layout (recommended)**.
- C) Dedicated DB for everyone - strongest, but cost/ops too high for 3-user shops at scale.

### D1.9 Reversibility
| Element | Mark | Why |
|---|---|---|
| `_tenant_id` on every row, everywhere | IRREVERSIBLE (decide now) | Removing/adding later means rewriting every table and package |
| Packages use logical names only | IRREVERSIBLE | Once packages use physical names, layout migration breaks them |
| Tenant from session, never header | REVERSIBLE technically, but a security contract | |
| Placement/layout per tenant | REVERSIBLE (COSTLY) | Needs data migration |
| Offering `partitioned` / `table_per_company` | REVERSIBLE | Can hide in wizard until hardened (they have no RLS today outside shared/partitioned) |
| Control-DB / router design | COSTLY | Everything resolves through it |

---

## D2. Auth: login delivered by `bos.auth`, kernel provides only secure primitives

### D2.1 Principle
Rust has **no login flow**. `bos.auth` (already ~1.6k lines: `User`, `Role`, `UserSession`, `UserSecuritySetting`, `SecurityAuditLog`, `AuthConfiguration`, `AuthManager`, `LoginPage`) owns the user experience and policy. The kernel owns the few things that, if a package got them wrong, would break *every* tenant: secrets, token issuance/verification, binding identity to the request, and tenant selection. Today's code is close in shape (`ctx.crypto.*` exist) but the package mints and stores sessions itself, stores raw tokens, and defaults to SUPER roles (`auth.bos:708-722`) - this is exactly what the boundary below removes.

### D2.2 Kernel hooks (minimal, in Rust, exposed as `ctx.kernel.*` capabilities)
| Hook | Purpose |
|---|---|
| `crypto.randomBytes(n)` | OS CSPRNG (exists). |
| `crypto.passwordHash(pw)` / `passwordVerify(pw, handle)` | Argon2id with kernel-owned params + pepper from secret store; packages get an opaque **credential handle**, never the hash string. Rehash-on-login when params change. |
| `crypto.hmac/sign/verify(keyRef, data)` | Keys referenced by name, never returned. For TOTP seeds, SAML/OIDC assertion checks, webhook signatures. |
| `identity.verifyOidc(providerRef, idToken)` / `verifySaml(...)` | JWKS fetch, signature, issuer/audience/nonce/expiry checks in Rust. |
| `session.issue(principalId, tenantId, {amr, ttl, ipLock})` | Only path to a session. Kernel stores **SHA-256(token)** only, returns raw token once, sets `HttpOnly; Secure; SameSite=Lax` cookie. Requires a *verification receipt* (from passwordVerify/verifyOidc/an approved MFA verify) from the same request. |
| `session.revoke(id)`, `session.list(principal)` | Revocation; packages see metadata, never tokens. |
| Request identity binding | Middleware resolves cookie/bearer -> session -> `{tenant, company, user, roles}` -> GUCs. Packages read `ctx.identity`, cannot write it. |
| `rateLimit.check(key, policy)` | Kernel token buckets per IP/user/tenant; floor limits on login endpoints that packages can tighten but not loosen. |
| `audit.security(event)` | Append-only, kernel-stamped actor/time/ip, hash-chained; packages can add events, never delete. |
| `secrets.get(ref)` | Scoped to the package's declared secrets (e.g. `google.client_secret`). |
| Capability gates | Manifest declares `capabilities: ["auth.provider", "session.issue", "net.fetch:accounts.google.com"]`; installer shows them, admin approves; sandbox denies anything undeclared. |

### D2.3 What packages may NEVER do
- Read raw password hashes, raw session tokens, TOTP seeds or private keys.
- Issue a session without a kernel verification receipt from that request.
- Set or change `tenant_id`, `user_id` or GUCs; choose a tenant outside the session's grants.
- Grant `SUPER`/system roles (role grants to system roles go through a kernel-audited API requiring an existing SUPER + reason).
- Disable kernel floors: rate-limit floor, audit, minimum hash cost, cookie flags, lockout floor.
- Make network calls or read secrets not declared and approved.

### D2.4 Bootstrap, first admin, emergency
- **First admin**: `bosc tenant create` (or wizard) prints a one-time setup URL with a random token (15 min TTL, single use, stored hashed); admin sets password + enrols MFA. Remove hard-coded `Boss@123` (`package_manager.rs:437`).
- **Emergency (break-glass)**: only via host CLI `bosc emergency open --tenant X --reason "..." --ttl 30m` with OS-level access; issues time-boxed session, banner in UI, every action audited, auto-notifies tenant admins. Remove `auto_fallback_on_auth_failure` and its default `true` (`node_config.rs:277`); `maintenance_mode` may stay as read-only mode, never as auth bypass.
- If `bos.auth` is broken/uninstalled, the kernel serves a minimal **recovery login** (password only, SUPER only, CLI-enabled) so the platform is never locked out and never open.

### D2.5 Extension points with examples
Extensions are packages in the Tenant/Industry layer extending `bos.auth` via events and `codeunit` interfaces:
- **Google SSO**: `acme.auth.google` declares `auth.provider`, secret `google.client_secret`, net `accounts.google.com`. Its `onLoginStart` redirects; `onCallback` calls `ctx.kernel.identity.verifyOidc("google", idToken)`, maps email -> `User`, then `ctx.kernel.session.issue(...)` with the receipt.
- **TOTP MFA**: `bos.auth.mfa_totp` subscribes `AuthManager.onPrimaryVerified`; stores seed via `secrets`/sealed field; verifies with `ctx.kernel.crypto.totpVerify(sealedRef, code)` -> receipt with `amr: ["pwd","otp"]`.
- **Password policy**: override `PasswordPolicy.validate(pw, user)` (min length 12, breach list, history 5). Kernel still enforces its floor (min 8, Argon2id).
- **Session policy**: `onSessionIssue` may shorten TTL, add IP lock; cannot lengthen beyond tenant max.

### D2.6 Containing a bad extension
Capability gates + kernel floors + receipts mean a buggy/malicious extension can at worst deny login for its tenant, not forge identity or cross tenants. Plus: per-extension kill switch (`bosc package disable`), automatic disable after N unhandled errors on login path with fallback to base `bos.auth`, signed-package requirement for anything requesting `auth.provider`/`session.issue` (D3), and sandbox CPU/memory/time limits per call.

### D2.7 Options
- A) Rust login flow - rejected by owner.
- B) **Package-delivered flow + minimal kernel (recommended)**.
- C) Package does everything incl. tokens (today's WIP shape) - not safe; one bug mints sessions.

### D2.8 Reversibility
| Element | Mark | Why |
|---|---|---|
| Opaque server sessions, hashed at rest | COSTLY | Changing token format logs everyone out; fine early |
| Kernel/package boundary & capability names | IRREVERSIBLE once 3rd-party auth packages exist | Public contract |
| Argon2id + pepper | COSTLY | Pepper loss = all passwords invalid; rehash-on-login makes params REVERSIBLE |
| Login UI, providers, MFA, policies | REVERSIBLE | Package code |
| Removing emergency auto-fallback | REVERSIBLE | Config |

---

## D3. Package management plan

### D3.1 Versioning & manifest (`bos.json` v2)
```json
{
  "package_id": "bos.financials",
  "version": "2.3.1",
  "layer": "base",
  "engine": ">=0.4.0 <0.6.0",
  "dependencies": { "bos.core": "^1.2.0", "bos.auth": "^1.0.0" },
  "optional_dependencies": { "bos.tax.in": "^1.0.0" },
  "capabilities": ["events.publish", "net.fetch:gst.gov.in"],
  "secrets": [],
  "publisher": "vits", "signature": "sig/...",
  "object_id_range": "50000-59999"
}
```
- SemVer 2.0: MAJOR = breaking public contract (removed field, changed type, removed event), MINOR = additive, PATCH = fixes. Pre-release tags for staging.
- `engine` range checked against kernel version at install; refuse otherwise.
- Today `dependencies` is a list of IDs (`PackageManifestIR`); accept old form as `"*"` during transition.

### D3.2 Resolution & lockfile
- Resolver (PubGrub-style, deterministic) picks highest compatible versions; one version per package per tenant (no side-by-side duplicates - data tables cannot be duplicated).
- `bos.lock.json` per tenant (and per node for node-wide packages): exact versions, SHA-256 of bundle, signer key id, resolved layer order. Stored in DB (`_bos_meta_packages` + new `_bos_package_lock`) and exportable to git.
- Install/upgrade always produces a **plan** (packages, schema diff from `schema_sync.rs`, data hooks, capabilities requested) for dry-run and approval.

### D3.3 Registry
- Levels: **local** (folder/file `.bospkg`, dev), **private** (per customer/partner, auth required), **public** (VITS-curated). Same HTTP protocol (index JSON + immutable blobs), so one client.
- India hosting: primary public registry in India region (per docs/30), CDN mirror; air-gapped nodes use a mirrored folder. Published versions are immutable; yank (hide from resolution) instead of delete.

### D3.4 Install / upgrade / rollback
1. Resolve -> plan -> verify signatures/checksums -> capability approval.
2. Pre-checks: engine, dependencies, backup point (snapshot or `pg_dump` of affected tables for small tenants; PITR marker for large).
3. **Expand** schema (additive DDL via schema_sync), run `onUpgrade(from,to,ctx)` data hooks in batches (resumable, idempotent, progress recorded in migration ledger - see readiness D8), switch code (bytecode cache, `_bos_sys_package_vault`), later **contract** (drop old columns) only in a following version after N days.
4. Per tenant, per DB: routed through D1 router; a pooled DB upgrades all its tenants together (they share tables), so per-tenant version skew is allowed only across dedicated DBs.
5. **Rollback**: code rollback always possible while schema is in expand phase (old code ignores new columns). Data hooks must declare `reversible: true` with `onDowngrade` or the plan states "rollback = restore backup point". Downgrade of MAJOR is refused without restore.

### D3.5 Signing & trust
- Replace 64-bit `DefaultHasher` checksum (`package_manager.rs:36-42`) with SHA-256 over canonical bundle bytes (fixes docs/10 claim) and Ed25519 signatures (minisign/Sigstore-style).
- Trust levels: VITS-signed (any capability), partner-signed (key registered per node/tenant), unsigned (dev nodes only; never on production; never with auth/net/secret capabilities).
- Key rotation and revocation list fetched from registry.

### D3.6 Layering Base > Localization > Industry > Tenant
- `layer` in manifest; a package may depend only on same or lower layers.
- Extensions: `tableextension` adds fields (namespaced by package, e.g. `in_gstin`), `pageextension` adds/moves controls, event subscribers add logic. Overrides of logic allowed only at declared **extension points** (events/interfaces) - no monkey-patching of base code.
- Conflicts: two packages adding the same field name -> install refused (namespacing avoids it); two subscribers on same event -> ordered by layer then declared `priority` then package id (deterministic); two page extensions moving the same control -> higher layer wins, warning in plan; a tenant-layer package can never alter Base security objects (`bos.auth` tables) except via auth extension points (D2).

### D3.7 Stable public contract
- Every table/field/event/codeunit method gets a **permanent ID** (number within publisher's `object_id_range`, NAV-style, or UUID) separate from its name. Renames use `renamedFrom` (already in `schema_sync.rs`) and keep the ID.
- Public = tables, fields, events (`name@v1`), API endpoints, codeunit methods marked `public`. Everything else internal.

### D3.8 Deprecation
Mark `@deprecated(since, removeIn, replacement)`; warnings in compiler and Studio; minimum one MINOR release + 6 months (or 2 MINORs) before removal in next MAJOR; registry shows dependents count before removal.

### D3.9 Reversibility
| Element | Mark | Why |
|---|---|---|
| Permanent object IDs & ID ranges | IRREVERSIBLE | Customer data and extensions bind to them |
| SemVer meaning / public contract scope | IRREVERSIBLE once 3rd parties publish | |
| One version per package per DB | COSTLY | |
| Signature algorithm/key format | COSTLY | Rotation needs re-signing |
| Registry hosting/location | REVERSIBLE | Mirrorable |
| Lockfile format | REVERSIBLE (versioned) | |

### D3.10 Package publishing flow
> Depends on D2 (kernel step-up receipts), D3 (manifest v2, signing) and D9 (OpenBao), none of which are signed off yet. MFA step-up detail is in `reports/cto_totp_plan.md` sec 15a. Owner decision used: only VITS signs Base/Localization; partners sign Industry/Tenant.

**Roles (platform realm, separation of duties)**
- New platform role **VITS Package Publisher**, separate from Super Admin. A Super Admin does **not** get publish rights automatically and can't grant the role to themselves.
- Granting or removing the role needs **two different Super Admins** (one requests, the other approves with their own fresh TOTP within 15 min). Every grant is audited and appears on the publisher list in the console.
- Optional (recommended for Base): a **second-person approval**, where another Publisher or a release manager approves the release before signing.
- Partner side: each partner organisation has **Partner Publisher** users in its own partner account. They can sign only inside the partner's registered scope.

**What gets signed**
1. The canonical bundle: manifest v2 (sorted-key JSON, UTF-8, no whitespace variance) + archive (tar with fixed order, fixed timestamps, no owner info), so two builds of the same commit give the same bytes.
2. The CLI/kernel computes **SHA-256** of `manifest || 0x00 || archive` (the same rule everywhere) and keeps the 32-byte digest.
3. The digest is sent to **OpenBao Transit** `sign` with the Ed25519 key `bos-pkg-vits` (key type `ed25519`, `exportable = false`, `allow_plaintext_backup = false`). OpenBao returns `vault:vN:<signature>`. The private key never leaves OpenBao. The signed message is the digest (pure Ed25519 over the 32 bytes), so verifiers only need the digest and the public key.
4. The signature file `bundle.sig.json` records `{package_id, version, layer, sha256, key_id: "bos-pkg-vits", key_version: N, signed_at, source_commit, publisher_id}`.

**Keys, rotation, revocation**
- Transit key versions: rotate yearly or after an incident (`rotate`). `min_encryption_version` doesn't apply to signing; instead each version's public key is published in the **trust store** with valid-from/valid-to dates.
- The **trust store** (signed by a separate offline VITS root key) lists: VITS key versions (scope `layer in {base, localization}` plus any layer), partner keys (scope below), and the **revocation list** (key id + version + revoked_at + reason, plus individually revoked package versions). Nodes fetch it from the India registry daily and on every install. Air-gapped nodes import it by file.
- Revoking a key: packages signed by it **after** `revoked_at` are rejected. Packages signed before stay valid unless the reason is `compromised`, in which case all of them are rejected and must be re-signed.

**Verification on install** (kernel, before any DDL)
1. Recompute SHA-256 from the bundle bytes and compare with the signature file (constant-time). A mismatch rejects the install.
2. Look up `key_id`/`key_version` in the trust store. It must be unrevoked, in date and **scope-allowed for this package's `layer` and `package_id` namespace**.
3. Verify the Ed25519 signature on the digest with the public key.
4. Check that the registry index entry (immutable) has the same SHA-256.
5. Record the verification result in the install plan and in the lockfile (`sha256`, `key_id`, `key_version`).
Production nodes reject unsigned or unverifiable packages, and there is no bypass flag. Dev nodes may allow unsigned **local** packages without auth/net/secret capabilities (D3.5).

**Partner keys and scope**
- Registration: the partner generates an Ed25519 key in **their own** OpenBao/KMS (or on a hardware token) and submits the public key plus a proof-of-possession signature over a VITS challenge. VITS checks the partner contract and assigns a **namespace** (e.g. `acme.*`). Two Super Admins approve, and the key is added to the trust store with its scope.
- Scope rules enforced at verify time: partner keys may sign only `layer in {industry, tenant}` **and** only `package_id` under their namespace. A partner key can never sign `base` or `localization`, or `bos.*`, `vits.*` or another partner's namespace. A tenant-specific partner package can also be limited to listed tenant ids.
- Partner revocation: partner-requested or VITS-initiated, added to the revocation list, and effective on the next trust-store refresh (forced refresh on installs).

**Audit record (append-only, tamper-evident)**
Table `platform_publish_log` in the control DB, written only by the kernel and hash-chained (each row stores the SHA-256 of the previous row plus its own content). It's also mirrored daily to WORM storage. Fields: publish id (UUIDv7), publisher user id and name, approver id (if any), step-up receipt id, time (UTC), package id, version, layer, SHA-256, key id, key version, signature, source repo + commit, registry (public/private), outcome (success/failed + reason), client IP, CLI/console. Failed attempts are logged too.

**CLI flow** (example: publishing `bos.tax.in` 1.4.0, Localization)
```
bosc login --realm platform            # password + TOTP, platform host
bosc package build packages/bos.tax.in # reproducible bundle, prints SHA-256
bosc package publish dist/bos.tax.in-1.4.0.bospkg --registry public
  -> checks: you have VITS Package Publisher; version 1.4.0 not yet published; manifest layer = localization
  -> "Enter TOTP code to sign bos.tax.in 1.4.0 (sha256 9f3a...c21)": 482913
  -> step-up receipt (5 min, single use, bound to this package id + version + sha256)
  -> [if approval required] waits for approver; approver runs: bosc package approve <publish-id> (with their own TOTP)
  -> digest sent to OpenBao Transit sign (bos-pkg-vits v3)
  -> uploads bundle + bundle.sig.json; registry re-verifies; audit row written
  -> "Published bos.tax.in 1.4.0, key bos-pkg-vits v3, publish id 01J..."
```
**Console (Studio) flow**
1. Platform console -> Packages -> "Publish new version". Upload or pick a CI-built bundle.
2. The screen shows package id, version, layer, SHA-256, source commit, dependency ranges, capabilities and a diff from the previous version.
3. Publisher clicks "Sign and publish". A TOTP prompt appears showing the package and short checksum.
4. If approval is required, the release shows as "Awaiting approval". The approver opens it, reviews it and approves with their own TOTP.
5. Progress shows signing, upload and verification, then a success page with the publish id and a link to the audit entry.

**Failure cases** (all fail closed, all audited)
| Case | Result |
|---|---|
| Step-up receipt older than 5 min, already used, or for another package/version/checksum | `step_up_expired` / `step_up_mismatch`; new TOTP code needed; nothing signed |
| OpenBao unreachable / sealed / Transit error | **Fail closed**: "Signing service unavailable, nothing was published". No local fallback key exists |
| Version already published | Rejected: published versions are immutable. Publish a new version (e.g. 1.4.1); a broken one can be **yanked**, never replaced |
| Checksum mismatch (bundle changed after review, upload corruption, registry re-verify fails) | Rejected; upload discarded; alert to Publishers |
| User lacks Publisher role, or partner key out of scope (e.g. partner tries `layer: base` or `bos.*`) | Rejected before signing |
| Key revoked or past valid-to | Rejected; asks for the current key version |
| Approver is the same person as publisher | Rejected (two-person rule) |

**Reversibility**: role model and audit format are REVERSIBLE (versioned). The canonical bundle and digest rule plus the signature format are **COSTLY**, because every published package and node verifier depends on them. Trust-store scope rules are REVERSIBLE (data). The root key choice is **COSTLY** (re-bootstrapping trust on every node).
---

## D9. DB roles, secrets, CORS (aligned with D1/D2)

### D9.1 Roles (per database; dedicated DBs get their own set)
| Role | Rights | Used by |
|---|---|---|
| `bos_owner` | Owns all objects; NOLOGIN | Nobody logs in as it |
| `bos_migrator` | LOGIN, member of `bos_owner`, DDL | Package installer/migrations only, short-lived connection |
| `bos_app` | LOGIN, DML on tenant tables, `NOSUPERUSER NOBYPASSRLS`, not owner; EXECUTE on kernel functions | Runtime request path |
| `bos_worker` | Like `bos_app`, plus outbox/job queue tables | NAS worker node |
| `bos_readonly` | SELECT, RLS enforced | Reporting/BI |
| `bos_control` | Access to control DB (tenant catalog, router) only | Router |
| Per-tenant roles | Only for dedicated/regulated tenants: `t_<code>_app`, own password/cert | Bank-like |

Superuser is never in `bos-node.toml`. Boot check: runtime refuses to start if its role is superuser or `BYPASSRLS`, or owns tenant tables.

### D9.2 Secrets
- `bos-node.toml` holds **references** only: `url_secret = "env:BOS_DB_APP_URL"` or `"keystore:bos/db_app"` or `"vault:kv/bos/prod/db_app"`.
- Providers in order: env vars (containers), OS keystore (Windows Credential Manager / DPAPI, Linux secret service) for on-prem NAS nodes, HashiCorp Vault / cloud KMS for SaaS (India region). Pepper, JWT/OIDC client secrets, package secrets, per-tenant encryption keys all go through `secrets.get(ref)` (D2).
- Rotate committed `postgres:postgres`; add secret scanning to CI.

### D9.3 CORS
- Apply `cors_origins` (today ignored: `server.rs:164-167` uses `Any`). Default: same-origin only (UI served by node). Explicit allow-list per node, optionally per tenant custom domains from control DB. `*` refused when cookies/credentials are used; never reflect arbitrary `Origin`. Allowed headers list explicit; `x-bos-tenant-id` removed from allowed headers once D1 routing lands.

### D9.4 Reversibility
| Element | Mark |
|---|---|
| Role model & names | COSTLY (grants in every DB and migration) |
| Secret references in toml | REVERSIBLE |
| CORS allow-list | REVERSIBLE |
| Pepper storage | COSTLY (see D2) |

---

## Decisions to sign off (Md Aziz / CEO)
- [ ] D1-a: Placement (pooled/dedicated) x Layout (4 strategies), chosen by wizard; no silent default; unknown strategy = error.
- [ ] D1-b: `_tenant_id` on every row in every layout (IRREVERSIBLE).
- [ ] D1-c: Packages use logical names only; router resolves physical names (IRREVERSIBLE).
- [ ] D1-d: Launch with `shared_rls` (pooled) + dedicated DB; keep `partitioned`, `schema_per_company`, `table_per_company` behind "preview" until each passes the isolation suite.
- [ ] D2-a: Login flow in `bos.auth`; kernel owns hashing, sessions (hashed), identity binding, tenant, rate-limit floor, audit, secrets, capability gates.
- [ ] D2-b: Remove `auto_fallback_on_auth_failure`, hard-coded `Boss@123`, SUPER-by-default roles; CLI-only break-glass.
- [ ] D3-a: SemVer + engine range + dependency ranges + lockfile + SHA-256/Ed25519 signing.
- [ ] D3-b: Permanent object IDs with publisher ID ranges (IRREVERSIBLE).
- [ ] D3-c: Expand/contract upgrades; rollback = code rollback in expand phase, else restore.
- [ ] D9-a: Role set above; runtime refuses superuser/BYPASSRLS.
- [ ] D9-b: Secrets by reference (env / OS keystore / Vault); CORS allow-list default same-origin.

## Open questions for Md Aziz
1. Is per-tenant version skew needed (tenant A on Financials 2.3, B on 2.4) in pooled DBs? If yes, pooled tenants must be grouped by version.
2. Do regulated tenants need their own Postgres cluster or is a dedicated database enough?
3. Object IDs: NAV-style numeric ranges (familiar to partners) or UUIDs?
4. Should VITS be the only signer for Base/Localization layers?
5. Which secrets backend for SaaS in India (Vault self-hosted vs cloud KMS)?

## Suggested iteration order
1. **I2a Roles & secrets (D9)**: `bos_app`/`bos_migrator`, boot refusal, FORCE + RESTRICTIVE RLS, secret refs, CORS allow-list. Small, unblocks everything.
2. **I2b Kernel auth hooks (D2)**: hashed sessions, `session.issue` with receipts, identity binding, tenant from session, remove fallbacks; adapt `bos.auth` to call them. Coordinate with Backend Engineer's `fix-iter1-security`.
3. **I2c Isolation test suite** (D1.5) in CI.
4. **I3 Router + control catalog + wizard v1** (pooled shared_rls + dedicated DB; replace `comp_default`/`default$` hard-codes).
5. **I5a Package manifest v2, SHA-256, resolver, lockfile, plan/dry-run (D3)**.
6. **I5b Upgrade hooks, ledger, rollback; signing**.
7. **I5c Pooled -> dedicated migration tool** (D1.6).
8. Later: harden and un-preview `partitioned`, `schema_per_company`, `table_per_company`; registry public launch.

---

## D2 addendum: iteration-1 open items decided

Source: read-only `git diff` of the uncommitted work on `fix-iter1-security` (2026-10-24, IST) and `docs/08_backend_api_server_reference.md`. These are CTO decisions for iteration 1 and fit D2/D9 above.

| # | Item | What the WIP does now | Decision | Rationale |
|---|---|---|---|---|
| a | `/api/databases*` public? | docs/08 sec 2.1 lists `/api/databases*` as public. WIP `auth_guard.rs::is_public_route` makes only `GET /api/databases` and `/api/databases/test-connection` public; switch, create, install and tenants need a session. | **Keep the BE's gating and go further.** Only during first-run setup (no users yet, checked server-side) may `GET /api/databases` and `test-connection` be anonymous, and only from loopback or a one-time setup token. After bootstrap, every `/api/databases*` route needs a SUPER session; create, switch and install also need a recent re-auth and are audited. | Listing databases tells an anonymous caller the cluster layout. `test-connection` lets anyone probe the network (SSRF) or check guessed passwords. Mutating routes can take over the node. |
| b | SeedSuperAdmin `Boss@123` | Hard-coded in `packages/bos.auth/auth.bos:1053-1057` and `package_manager.rs:437`. | **Remove it. No default password, ever.** First admin is created by (1) the setup wizard, where the admin chooses a password that passes policy, or (2) a single-use setup link printed by the host CLI (15 min, stored hashed). Any existing account with a known default must change its password at next login. | A public default password means every new or forgotten node can be taken over. |
| c | SSE streams without `Authorization` header | `EventSource` is used in `ui/src/lib/page-kit/loader.svelte.js` (`/api/v1/pages/manifest/stream`) and `ui/src/lib/services/notificationStore.svelte.js`. Browsers cannot add headers to it. | **Primary: the session cookie** (`HttpOnly; Secure; SameSite=Lax`, same-origin) is accepted on stream routes, with an Origin check. **For other origins: a short-lived stream ticket.** `POST /api/v1/streams/ticket` (authenticated) returns a random ticket stored hashed: single use, 60 s TTL, tied to session, user, tenant and stream path. The ticket is spent when the stream opens. **Never** put a session or bearer token in the query string. | Query strings end up in logs, proxies, browser history and Referer headers. A single-use ticket that expires in 60 s is useless if it leaks. |
| d | VerifySession returns no user id | WIP `server.rs` now skips profile updates when `ctx.user_id.is_nil()` and returns `"id": "admin"` as a fallback, so the nil-user problem is only worked around. | **Fix it in the kernel. A session without a real user id is invalid (401).** The kernel session record stores user id, tenant and company (D2.2). The guard fills `SessionContext.user_id` from it and sets `bos.current_user_id`. Remove the `"admin"` fallback id. | Audit, RLS `_created_by`/`_updated_by` and permissions all need a real user. With a nil user every action is anonymous and the audit trail can't be used. |
| e | Emergency access | WIP removed the in-memory emergency session store and now checks `security.emergency_username` together with `maintenance_mode`. `auto_fallback_on_auth_failure` (default `true`) is still in `node_config.rs`. | **Break-glass from the host CLI only.** `bosc emergency open --tenant X --reason "..." --ttl 30m` needs OS-level access to the node. It issues one time-boxed SUPER session through the kernel session store (hashed). The UI shows a banner, every action is audited, and tenant admins are notified. **Remove `auto_fallback_on_auth_failure`.** `maintenance_mode` becomes read-only mode only and never bypasses login. No emergency login with just a username over HTTP. | Any HTTP path that gets around authentication can be attacked remotely. Needing host access plus a short time box and an audit trail keeps recovery possible without leaving a backdoor. |

### Docs to update
- `docs/08_backend_api_server_reference.md` sec 2.1: replace the public-route list (remove `/api/databases*`, describe the setup-only exception). Sec 2: add cookie auth and stream tickets for SSE, and say tokens are never accepted in query strings. Mark `/api/databases*` rows as SUPER-only.
- `docs/13_server_node_and_nas_architecture.md` and `bos-node.toml` comments: remove `auto_fallback_on_auth_failure`, define `maintenance_mode` as read-only, describe CLI break-glass.
- `docs/09_bosc_cli_reference.md`: add `bosc emergency open/close` and the setup-link command.
- `packages/bos.auth/README.md`: remove the default password and document first-admin setup.
- `docs/06_quickjs_sandbox_and_execution_runtime.md`: kernel session/identity hooks (D2.2) once implemented.
