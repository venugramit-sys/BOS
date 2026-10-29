# ADR 0006: Detached Ed25519 signature format
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.6; D3.5, D3.10.

## Context
Owner decision: only VITS signs Base/Localization, and partners sign Industry/Tenant. Keys are in OpenBao Transit (P2) and never exported.

## Decision
File `<id>-<version>.sig.json`, stored next to the bundle:
```json
{
  "sig_version": 1,
  "alg": "ed25519",
  "package_id": "bos.tax.in",
  "version": "1.4.0",
  "layer": "localization",
  "sha256": "<64 lowercase hex from ADR 0005>",
  "key_id": "bos-pkg-vits",
  "key_version": 3,
  "signer": "vits",
  "signed_at": "2026-10-24T05:00:00Z",
  "source_commit": "a1b2c3d4",
  "publish_id": "<uuidv7>",
  "signature": "<base64 standard, 64 bytes>"
}
```
- Signed message = the **32 raw digest bytes** (pure Ed25519, RFC 8032), not the JSON. Verifiers check that `sha256` matches the bundle, then the signature against the trust-store public key for (`key_id`, `key_version`).
- `signer`: `vits`, `partner:<partner_id>` or `dev`. `key_id` is `^[a-z0-9-]{3,64}$`, and the version is an integer of 1 or more.
- Trust-store entry: `{key_id, key_version, alg, public_key_b64, signer, scope: {layers:[], namespaces:[]}, valid_from, valid_to, revoked_at?, revoke_reason?}`. The trust store itself is signed by the offline root key.
- Scope: VITS keys may sign any layer. Partner keys may sign only `industry|tenant` **and** only ids under their namespace.

## Consequences
Verification works offline. Rotation happens by adding a key version.

## Alternatives considered
Signing the JSON with JWS (canonicalization pitfalls); minisign (a second format to maintain); Ed25519ph (not supported everywhere).

## Reversibility
COSTLY: every node's verifier depends on it. Changes go through `sig_version`.

## Phase / enforcement
P0: the format and a `dev` key (development only). P2: signatures required in staging/production. P3: publisher role and step-up (D3.10).