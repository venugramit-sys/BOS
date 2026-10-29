# ADR 0007: Audit event shape with hash chaining
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.7; TOTP plan sec 10; D3.10.

## Decision
```json
{
  "event_version": 1,
  "id": "<uuidv7>",
  "seq": 18234,
  "at": "2026-10-24T05:00:00.123Z",
  "type": "auth.login.success",
  "outcome": "success",
  "reason": null,
  "actor": { "user_id": "<uuid|null>", "realm": "tenant", "name": "priya", "via": "ui", "stand_in": false },
  "tenant_id": "<uuid|null>",
  "company_id": "<uuid|null>",
  "target": { "kind": "user", "id": "<uuid>", "version": null },
  "details": {},
  "request": { "ip": "203.0.113.5", "request_id": "<uuid>", "node_id": "<uuid>" },
  "profile": "production",
  "prev_hash": "<64 hex|null>",
  "hash": "<64 hex>"
}
```
- `type` is a dot-separated name from a registry (`docs/adr` annex, kept in code as `audit_types.rs`). `outcome` is `success|failure|denied`.
- **Never** stored in an event: passwords, OTP codes, session tokens, secrets or TOTP seeds.
- Chain: `hash = SHA-256( prev_hash_hex || "\n" || JCS(event without "hash") )`, one chain per node and stream (`platform`, `tenant:<uuid>`). The first event has `prev_hash = null`. `seq` is gap-free per chain.
- Storage: table `_bos_audit_log` is append-only (a trigger blocks UPDATE/DELETE, and the app role has INSERT only).

## Consequences
Tampering or deletion breaks the chain and shows up in `bosc audit verify`.

## Alternatives considered
Plain log lines (not queryable); an external SIEM only (a dependency in dev).

## Reversibility
REVERSIBLE through `event_version`; the chain rule is COSTLY.

## Phase / enforcement
P0: same shape in all profiles (`hash` may be null in development). P1: chain required in staging/production. P3: daily WORM mirror.