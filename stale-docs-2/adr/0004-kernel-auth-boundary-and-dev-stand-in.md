# ADR 0004: Kernel / bos.auth API boundary with a development stand-in
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.4; D2; TOTP plan sec 4.

## Context
In `fix-iter1-security` the kernel already keeps no session store and calls `AuthManager.VerifySession` (`crates/bosc-runtime/src/auth_guard.rs` `resolve_session`). Development needs fast access without weakening the production paths.

## Decision
One trait, the same in every profile:
```rust
pub trait AuthKernel: Send + Sync {
    fn verify_session(&self, token: &str, req: &RequestMeta) -> Result<VerifiedSession, AuthError>;
    fn hash_session_token(&self, token: &str) -> [u8; 32];                 // SHA-256; raw tokens never stored
    fn require_capability(&self, s: &VerifiedSession, cap: &str) -> Result<(), AuthError>;
    fn mfa_verify(&self, s: &VerifiedSession, code: &str) -> Result<MfaReceipt, AuthError>;
    fn step_up(&self, s: &VerifiedSession, purpose: StepUpPurpose, max_age: Duration) -> Result<StepUpReceipt, AuthError>;
    fn audit(&self, e: AuditEvent);                                        // ADR 0007
}
pub struct VerifiedSession {
    pub session_version: u16,        // = 1
    pub session_id: Uuid,            // v7
    pub user_id: Uuid,               // never nil
    pub tenant_id: Uuid,             // from the session record, never from a header (staging/prod)
    pub company_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub realm: Realm,                // Platform | Tenant
    pub mfa_at: Option<DateTime<Utc>>,
    pub issued_by: Issuer,           // BosAuth | DevStandIn
    pub profile: SecurityProfile,    // profile of the node that issued it
}
```
- **bos.auth owns** users, roles, passwords, login UI, enrolment UI and tenant MFA policy. **The kernel owns** token hashing, receipts, crypto, capability gates and audit writing.
- **Dev stand-in** `DevStandInAuth` (only constructible when the profile is `development`): it issues sessions for `dev.admin` with `issued_by = DevStandIn` and `roles = ["SUPER"]`. `mfa_verify`/`step_up` return receipts with `stand_in: true`.
- **Rejection rule:** every consumer of a session or receipt checks `issued_by == DevStandIn || stand_in == true`. If so, and the node profile isn't `development`, it gets `AuthError::StandInRejected` (401) and an audit event `auth.stand_in.rejected`. In production builds without the dev feature, `DevStandInAuth` doesn't compile.
- Receipts are serialized with these fields: `{receipt_version, kind, session_id, purpose, issued_at, expires_at, stand_in, profile, bound_to}`.

## Consequences
Dev and production code call the same functions, so there's no `if dev` scattered through handlers. A leaked dev token is useless on staging or production.

## Alternatives considered
Disabling the auth middleware in dev (dev and prod paths would diverge); a fake bos.auth package (it could be installed by mistake on production).

## Reversibility
COSTLY for the trait shape (packages depend on `ctx.kernel.*`); REVERSIBLE for the internals.

## Phase / enforcement
P0: trait, stand-in, rejection rule. P1: real bos.auth path with hashed tokens and a non-nil user id. P2: real MFA/step-up (TOTP plan M0-M4).