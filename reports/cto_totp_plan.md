# CTO Plan: TOTP Multi-Factor Authentication for BOS

Date: 2026-10-24 (IST). Author: CTO agent. Status: **PLAN, depends on proposals not yet signed off.**
This plan builds on the kernel hooks proposed in `reports/cto_proposals_d1_d2_d3_d9.md` (D2.2 kernel hooks, D2.3 package boundary, D9 secrets). D1, D2, D3 and D9 are **not signed off**. If the kernel/package boundary changes, sections 4 and 5 change with it. No code was changed.

Owner decisions taken as input (2026-10-24): tenants sharing a database share package versions; regulated tenants get their own database by default (a dedicated cluster is a premium option); UUIDs plus readable names; only VITS signs Base/Localization packages, partners sign Industry/Tenant; secrets go through self-hosted **OpenBao** (cloud KMS optional).

## 0. What exists today (evidence)
- `packages/bos.auth/auth.bos:48-55` enum `MfaMethod { None, TOTP, EmailOTP, SMSOTP, WebAuthn }` (`extensible = true`).
- `auth.bos:181-209` table `UserSecuritySetting` has `mfaMethod`, `mfaEnabled` and **`mfaSecret: String(128)`**, a plain column readable by package code. This plan **replaces** that column with an encrypted, kernel-only store.
- `bos.json` lists `mfa_totp` under `optional_features`. I found no TOTP verification code in the kernel. `quickjs_engine.rs:805-834` exposes only random bytes and argon2 hash/verify.

## 1. Standards and verified parameters
Sources read: RFC 6238 (rfc-editor.org/rfc/rfc6238), RFC 4226 (HOTP), Google Authenticator Key URI Format wiki (github.com/google/google-authenticator/wiki/Key-Uri-Format).
- **TOTP = HOTP(K, T)**, with `T = floor((unix_time - T0) / X)`, `T0 = 0`, `X = 30 s` (RFC 6238 sec 4). T must work past 2038 (use 64-bit).
- **HOTP**: HMAC-SHA-1 over the 8-byte big-endian counter, then dynamic truncation (offset = low 4 bits of the last byte, 31-bit value) mod 10^digits (RFC 4226 sec 5.3).
- **Secret length**: RFC 4226 requires at least 128 bits and recommends 160 bits. RFC 6238 says keys SHOULD be the length of the HMAC output (20 bytes for SHA-1). **BOS: 20 random bytes (160 bits), Base32 without padding.**
- **Window**: RFC 6238 sec 5.2 recommends allowing at most one time step of network delay. **BOS accepts T-1, T and T+1 (±1 step), checked in that order.**
- **No reuse**: RFC 6238 sec 5.2 says the verifier MUST NOT accept a second use of an OTP after it was validated once. **BOS stores the last accepted step and rejects any step <= it.**
- **Throttling**: RFC 4226 sec 7.3 recommends throttling and lockout after failed attempts (see section 7).
- **Drift**: RFC 6238 sec 6 allows recording a per-token drift. BOS records the drift but never widens the window automatically beyond ±1. A larger drift needs re-sync by the user (enter two consecutive codes) after another factor.
- **URI**: `otpauth://totp/ISSUER:ACCOUNT?secret=BASE32&issuer=ISSUER&algorithm=SHA1&digits=6&period=30`. The wiki recommends both the label prefix and the `issuer` parameter, and notes that Google Authenticator ignores `algorithm`, and on some platforms `digits` and `period`. **That is why the defaults are fixed at SHA1 / 6 digits / 30 s.** SHA-256 and 8 digits are a later per-tenant option only, with a warning about authenticator support.
- **Test vectors** (must pass in CI): RFC 6238 Appendix B, ASCII secret `12345678901234567890`, 8 digits, SHA-1: t=59 -> `94287082`, t=1111111109 -> `07081804`, t=1234567890 -> `89005924`, t=2000000000 -> `69279037`, t=20000000000 -> `65353130`. RFC 4226 Appendix D: counter 0 -> `755224`.

Example URIs:
- Platform admin: `otpauth://totp/VITS%20BOS%20Platform:ops.ravi?secret=...&issuer=VITS%20BOS%20Platform&algorithm=SHA1&digits=6&period=30`
- Tenant user (shop "Sharma Traders"): `otpauth://totp/BOS%20-%20Sharma%20Traders:priya@sharma.in?secret=...&issuer=BOS%20-%20Sharma%20Traders&...`
Different issuers keep a person's platform and tenant entries visibly separate in the authenticator app.

## 2. Realms: platform operators vs tenants
| | **Platform realm** (VITS operators) | **Tenant realm** (each tenant) |
|---|---|---|
| Who | VITS Super Admins and support operators | Tenant admins, tenant users |
| Where accounts live | Control database (D1 router catalog), not in any tenant DB | Tenant database, `bos.auth` tables |
| MFA | **Mandatory, no grace period** after first login (enrol at first sign-in, before anything else) | Policy per tenant and per role (section 3) |
| Encryption key | OpenBao Transit key `bos-mfa-platform` | One Transit key per tenant: `bos-mfa-t-<tenant_uuid>`, and for a dedicated-cluster tenant optionally its own OpenBao namespace |
| Login URL / cookie | Separate host (e.g. `ops.<node>`), separate cookie name and scope | Tenant host or path |

**Hard separation rules (kernel-enforced):**
- A tenant session can never name or reach a platform account. Platform accounts aren't in tenant DBs, so no tenant role, package or SQL can see them.
- A tenant admin can't create, promote, reset or disable a platform admin. There is no API for that from the tenant realm. Platform role grants happen only in the platform realm, with step-up and two operators (section 8).
- A platform operator acting inside a tenant uses an **impersonation session**: time-boxed, reason required, a banner shown to the tenant, audited in both realms. It is never a silent login as a tenant user.
- Same person, two realms = two accounts and two TOTP secrets. They're never linked automatically.

## 3. Tenant policy
Stored per tenant (setting `mfa_policy`) with per-role overrides:
- `mode`: `off` | `optional` (users may enrol) | `required`.
- `required_roles`: e.g. `["TENANT_ADMIN", "FINANCE_APPROVER"]`.
- `grace_days`: e.g. 7. During grace a user can log in but sees an enrolment banner. After grace, login moves straight into enrolment and nothing else works until it's done.
- `remember_device_days`: 0 by default. If above 0, a signed device cookie skips TOTP on that browser for N days (never for admin roles).
- Kernel floor: the policy can't make platform MFA optional and can't drop below kernel rate limits.

Examples:
- **3-user shop**: `optional`, owner encouraged. Recommended preset: required for the owner (tenant admin) only, grace 14 days.
- **500-user manufacturer**: `required` for `TENANT_ADMIN` and finance/approver roles, `optional` for shop-floor users, grace 7 days, no remember-device for admins.
- **Regulated bank-like tenant** (own DB): `required` for everyone, grace 0-3 days, remember-device off, own Transit key, step-up window 5 minutes.

## 4. Kernel hooks (Rust) vs bos.auth responsibilities
Builds on D2.2. **The package never sees the TOTP secret in plaintext, nor its ciphertext's key.**

| Kernel hook (Rust, `ctx.kernel.mfa.*`) | What it does |
|---|---|
| `totp.beginEnrollment(principal)` | Generates 20 bytes from the OS CSPRNG, encrypts with OpenBao Transit (section 5), stores it as a **pending** factor, returns `{factorId, otpauthUri, qrSvg}` **once** to the enrolling user's own response only. The URI/QR is never logged, cached or audited. |
| `totp.confirmEnrollment(factorId, code)` | Verifies the first code (±1, constant-time). On success the factor becomes active and recovery codes are generated (returned once). Pending factors expire after 10 min. |
| `totp.verify(principal, code, purpose)` | Decrypts in kernel memory, computes HOTP for T-1/T/T+1, compares in constant time, enforces `step > last_used_step`, updates `last_used_step` and drift atomically (row lock), zeroises the secret. Returns a **verification receipt** (`amr: otp`, time, purpose) usable by `session.issue` / step-up. |
| `recovery.generate(principal)` | 10 codes, each 10 characters Base32 with a hyphen (`K7QF-M2XP9A`), ~50 bits each. Stored as Argon2id hashes with pepper. Old set invalidated. Returned once. |
| `recovery.redeem(principal, code)` | Single use, constant-time. Returns a receipt with `amr: recovery`, which allows only a limited session that must re-enrol. |
| `factor.reset(principal, by, reason)` | Disables factors and revokes sessions. Requirements depend on the realm (section 8). |
| `stepUp.require(purpose, maxAgeMin)` | Checks whether the session has an OTP/recovery receipt younger than N minutes. Otherwise returns `step_up_required`. |
| Rate limiter, audit, clock check | Section 7, 10, 11. |

| `bos.auth` package (BOS-script) | Responsibility |
|---|---|
| Policy | Stores `mfa_policy`, decides when MFA is required (role, grace, remember-device), calls kernel hooks. |
| UI | Enrolment page (shows QR from `qrSvg` or renders the URI client-side; offers "can't scan" manual key), challenge page, recovery-code page (download/print, "I saved them" checkbox), admin reset screens, security settings. |
| Messages | Notifies users of enrolment, reset, recovery code use, new device (email via notification system, docs/15). |
| Reporting | MFA coverage per role, users in grace, users locked. |

**Packages may never:** read secrets or ciphertext, compute or verify codes themselves, mark a factor active, skip `step > last_used_step`, issue a session without a kernel receipt, touch the platform realm, or lower kernel rate limits.

## 5. Secret protection with OpenBao Transit (envelope encryption)
- Each factor gets its own random 256-bit **data key**, from Transit `datakey` (wrapped). The TOTP secret is encrypted locally with AES-256-GCM under that data key, and the factor id is bound as associated data. Only the **wrapped** data key and the ciphertext are stored in Postgres.
- To verify, the kernel asks Transit to decrypt the wrapped data key, decrypts the secret, computes, then zeroises. A small in-memory cache of unwrapped data keys (max 60 s, max N entries, never on disk) cuts Transit calls under load.
- Keys: `bos-mfa-platform` (platform realm) and `bos-mfa-t-<tenant_uuid>` per tenant. Transit key rotation is enabled yearly. Rewrapping is a background job using Transit `rewrap`, with no user impact.
- OpenBao policies: the node's AppRole can only call `encrypt/decrypt/datakey/rewrap` on its realm's keys, never `export`. Platform and tenant keys use separate policies.
- Cloud KMS (optional) plugs in behind the same kernel interface.
- **OpenBao unavailable:**
  - Platform realm: **fail closed.** No platform admin login or step-up. Break-glass is the two-operator CLI procedure (section 8), which needs OpenBao back up or the sealed offline recovery procedure.
  - Tenant realm: **fail closed for TOTP verification as well** (the secret can't be decrypted). Recovery codes don't need OpenBao (they're Argon2 hashes), so users can still sign in with one. A policy flag lets a tenant choose `degrade_to_recovery_only` (default) or `block`. Never fail open.
  - Health check and alert when Transit latency is above 200 ms or there are errors.

## 6. Enrolment flow (example: Priya, tenant admin at the manufacturer)
1. Priya logs in with her password and gets a password receipt. Policy says MFA is required and her grace period has ended, so she's sent to enrolment with a **restricted session** that can only enrol.
2. `beginEnrollment` returns the QR and URI (TLS only, `Cache-Control: no-store`). She scans it in Google or Microsoft Authenticator.
3. She types the 6-digit code. `confirmEnrollment` verifies it, activates the factor and returns 10 recovery codes once. The UI makes her download or print them and tick "saved".
4. The kernel upgrades the session to full (`amr: pwd, otp`), audits `mfa.enrolled` and emails her "MFA was turned on for your account".
Re-enrolling (new phone, still has old phone): step-up with the old code first, then the same flow. The old factor is removed only after the new one is confirmed.

## 7. Rate limiting and lockout (RFC 4226 sec 7.3)
- Per account: 5 failed codes in 15 min means a 15-minute lock, with backoff doubling after that. After 20 failures in 24 h, the factor is locked until a reset or recovery code. Each lockout is emailed and audited.
- Per IP and per tenant: kernel token buckets (D2.2) also limit password and OTP attempts to stop spraying across accounts.
- Recovery code attempts share the same counters.
- Error messages are generic ("code not accepted"). They never say whether the password was right before the OTP step in a way that helps guessing.

## 8. Device loss and reset flows
| Who | Flow |
|---|---|
| Tenant user, self-service | Password + one recovery code -> restricted session -> must re-enrol now. All other sessions are revoked. User notified. |
| Tenant user, admin-assisted | A tenant admin with **step-up** (fresh OTP within 5 min) and a typed reason resets the user's factor. The user must re-enrol at next login. Audited, and the user plus other tenant admins are notified. Optional policy: two tenant admins for admin-role users. |
| Tenant admin (last admin locked out) | Another tenant admin resets them. If none exists: a VITS platform operator via an **impersonation-free support reset**, with an identity check recorded (ticket id), step-up, audit in both realms, and the tenant notified. |
| VITS Super Admin | **Two VITS operators + host CLI only**: `bosc platform mfa-reset --user ops.ravi --ticket SR-123`. Operator A starts it, operator B approves with their own OTP within 15 min. Runs on the node host (OS access). All platform sessions of the target are revoked, it's audited, and every platform admin is alerted. No HTTP path exists. |

## 9. Step-up (fresh MFA within N minutes)
Default N = 10 minutes (tenant can lower it, regulated preset 5). Actions that need step-up:
- Granting or removing roles, especially admin and `SUPER` roles; creating API keys.
- All `/api/databases*` mutating routes (create, switch, install), tenant creation/move (D1 migrations).
- Package install, upgrade, rollback, disable (D3); package **publish** uses a stricter single-use 5-minute receipt (sec 15a).
- Emergency/break-glass open (D2 addendum e) and impersonation.
- Secret changes: OpenBao references, SSO client secrets, SMTP passwords.
- MFA changes: own factor change, resetting another user's MFA, regenerating recovery codes, changing the tenant MFA policy.
- Exporting bulk data, changing bank/payment details (later business packages declare these through `stepUp.require`).

## 10. Audit events
All go to the kernel security audit (hash-chained, D2.2) with realm, tenant, actor, target, IP, user agent, time (UTC stored, shown in the tenant's timezone) and outcome. They never include codes, secrets or URIs.
`mfa.enroll.started`, `mfa.enroll.confirmed`, `mfa.enroll.expired`, `mfa.verify.success`, `mfa.verify.failed`, `mfa.verify.replay_rejected`, `mfa.lockout`, `mfa.unlock`, `mfa.recovery.generated`, `mfa.recovery.used`, `mfa.recovery.exhausted_warning` (<= 2 left), `mfa.reset.self`, `mfa.reset.admin`, `mfa.reset.platform_two_person` (with both operator ids), `mfa.stepup.success`, `mfa.stepup.failed`, `mfa.policy.changed`, `mfa.drift.detected` (more than 1 step), `mfa.kms.unavailable`, `mfa.key.rotated`, `mfa.impersonation.started/ended`.

## 11. Time: drift and server clock
- Servers must run NTP/chrony (Windows: w32time against reliable sources). Node health check compares system time against at least 2 NTP sources at startup and every 10 minutes. If the offset is above 5 s it raises a warning, above 15 s it **refuses platform-realm OTP verification** and alerts (a bad server clock breaks everyone's codes or widens attack windows).
- User phone drift: the window is ±1 step (up to about 59 s each way including the current step). Drift is recorded. If a user keeps hitting T+1/T-1, the UI suggests turning on automatic time on the phone. Re-sync needs two consecutive codes after another factor.
- Store all times in UTC. Display in local time (e.g. IST) only.

## 12. Threat model notes
- **Phishing / real-time relay (AiTM)**: TOTP is **not phishing-resistant**. A fake login page can relay a code within 30 s. Mitigations: separate platform host, login alerts, short step-up window, device/IP anomaly alerts. **Recommend WebAuthn/passkeys** (already in `MfaMethod`) as the next factor, mandatory for VITS Super Admins once available.
- **Database theft**: secrets are useless without OpenBao. Recovery codes are Argon2id + pepper.
- **Malicious or buggy package**: can't read secrets or mint receipts (kernel boundary), so the worst case is denying MFA UI for its own tenant.
- **Insider (one VITS operator)**: platform resets need two people, are audited, and every admin is alerted.
- **Replay**: last-used-step check. **Brute force**: 10^6 codes x 3 steps; with 5 tries per 15 min an attacker has a negligible chance (about 1 in 66,000 per 15-minute window before lockout escalation).
- **Timing attack**: constant-time compare (e.g. Rust `subtle` crate) over all window candidates without early exit.
- **Shoulder-surfing the QR**: shown once, no-store, never logged, and the manual key is hidden behind "show key".
- **SIM swap**: not relevant (no SMS); `SMSOTP` stays disabled by default.

## 13. Data model sketch (plain words)
Platform realm (control DB) and tenant realm (tenant DB) have the same shape, in separate databases:
- **MFA factor** (kernel-owned table, not readable by packages): factor id (UUIDv7), user id, type (`totp`), status (`pending`/`active`/`disabled`), friendly name ("Priya's phone"), algorithm/digits/period (SHA1/6/30), encrypted secret, wrapped data key, key name and version, last used step, recorded drift (steps), failed count, locked until, created at, confirmed at, last used at.
- **Recovery code** (kernel-owned): id, user id, set id, Argon2id hash, used at, created at.
- **MFA policy** (package-owned, `bos.auth`): tenant id, mode, required roles, grace days, remember-device days, step-up minutes, OpenBao-down behaviour.
- **User MFA status** (package-readable view from kernel): user id, enrolled yes/no, methods, recovery codes left, grace ends at, locked yes/no. Replaces `UserSecuritySetting.mfaSecret` and `mfaEnabled` (to be removed via `renamedFrom`/expand-contract, D3).
- **Step-up receipts**: kept on the kernel session record (last OTP time, amr list).
- **Remember-device tokens** (kernel): hashed token, user, device label, expires at.

## 14. Extension points
- `onMfaRequired(user, context)`: a policy hook (e.g. require MFA only off the office network). It can **add** requirements, never remove kernel or tenant-mandated ones.
- `onMfaEnrolled / onMfaReset / onRecoveryUsed`: notifications, e.g. a Slack or email alert package.
- Custom enrolment/challenge pages: `pageextension` on the `bos.auth` MFA pages (branding, help text).
- New factor types (WebAuthn, hardware OATH tokens) via the extensible `MfaMethod` enum plus a kernel verifier. A new verifier is a kernel addition, not a package, because it touches secrets.
- Business packages mark their own sensitive actions with `stepUp.require("purpose", minutes)`.

## 15. Rollout iterations with acceptance tests
Prerequisites: D2 kernel session/receipt hooks and D9 OpenBao wiring (proposed, not signed off).
| # | Scope | Acceptance tests |
|---|---|---|
| M0 | Kernel TOTP/HOTP core + tests | RFC 6238 Appendix B and RFC 4226 Appendix D vectors pass; ±1 window; replay of the same step rejected; constant-time compare reviewed; 64-bit T beyond 2038 tested |
| M1 | OpenBao Transit envelope encryption, per-realm/tenant keys, unavailable behaviour | Ciphertext only in DB (scan test finds no Base32 secrets); with OpenBao stopped, platform login is refused and tenant falls back to recovery-only; key rotation + rewrap job keeps codes working |
| M2 | Platform realm mandatory MFA + two-person CLI reset | A platform admin without MFA can do nothing but enrol; a tenant session can't reach any platform account (negative tests); reset needs two distinct operators' OTPs |
| M3 | bos.auth enrolment UI, recovery codes, tenant policy (optional/required/per role/grace) | Works in Google and Microsoft Authenticator (manual test on Android + iOS); URI never in logs (log grep test); recovery codes single use; grace expiry forces enrolment; `mfaSecret` column removed |
| M4 | Rate limiting, lockout, admin-assisted reset, audit events | 6th bad code within 15 min locks; every event in section 10 is emitted with no secret fields; admin reset needs step-up |
| M5 | Step-up on sensitive actions + clock checks | Each action in section 9 returns `step_up_required` without a fresh receipt; NTP offset above 15 s blocks platform verification |
| M6 | WebAuthn/passkeys (next phase) | Passkey login for platform admins; then becomes mandatory for them |

## 15a. Package publishing step-up (see `reports/cto_proposals_d1_d2_d3_d9.md` D3.10)
Depends on the D2 kernel receipts and D3/D9, which aren't signed off.
- Applies to the platform role **VITS Package Publisher** (separate from Super Admin; granted only by two Super Admins, each with fresh TOTP) and to partner Partner Publishers in their own realm.
- **Before each publish**: a fresh TOTP is entered on the publish screen or in the CLI prompt. The kernel issues a **publish receipt** valid for **5 minutes**, **single use**, bound to `package_id + version + SHA-256 + publisher`. A normal 10-minute session step-up does not count. Publishing always asks for a new code.
- Reuse rule: the same TOTP step can't be used twice (last-used-step check, sec 4), so one code can't approve two publishes.
- Approver (if required): their own TOTP and their own receipt; the approver must be a different person.
- OpenBao down: TOTP verification and Transit signing both fail closed, so nothing is published (sec 5).
- Audit events added: `pkg.publish.stepup.success/failed`, `pkg.publish.signed`, `pkg.publish.rejected` (reason: expired, mismatch, immutable, checksum, scope, revoked, kms_unavailable), `pkg.publisher.role.granted/revoked`, `pkg.partner_key.registered/revoked`.
- Acceptance tests (add to M5): publish without fresh TOTP is refused; a receipt for version 1.4.0 can't sign 1.4.1; a receipt older than 5 min is refused; a second publish with the same receipt is refused; a Super Admin without the Publisher role is refused.
## 16. Open questions for Md Aziz
1. Should "remember this device" be allowed at all for tenant users, and the maximum days?
2. When OpenBao is down, is recovery-code-only sign-in acceptable for tenants (recommended) or should tenants be fully blocked?
3. Should a last-admin-locked-out tenant be reset by VITS support (with an identity check), or must tenants keep two admins (enforced)?
4. Should passkeys become mandatory for VITS Super Admins as soon as M6 ships?
5. Should step-up default to 10 minutes, and which business actions (e.g. payment runs, bank detail changes) should business packages be required to protect?
6. Are email notifications enough for MFA events, or is SMS/WhatsApp needed for Indian tenants (note SIM-swap risk)?
