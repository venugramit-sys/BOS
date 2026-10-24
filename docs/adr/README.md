# Architecture Decision Records (ADRs)

These record the **formats locked in Phase 0** of the phased security plan (`reports/cto_phased_security_plan.md`, approved by Md Aziz on 2026-10-24). A format is frozen now in every profile. Its *enforcement* is phase-gated (P0-P4, see the plan sec 4).

| # | Title | Status | Enforcement starts |
|---|---|---|---|
| [0001](0001-uuidv7-ids-and-readable-names.md) | UUIDv7 IDs plus readable names | Accepted | P0 (format), P1 (nil id rejected) |
| [0002](0002-tenant-id-on-every-row-uniform-rls.md) | Tenant ID on every row; uniform RLS generation | Accepted | P0 (generated), P1 (non-superuser role) |
| [0003](0003-logical-table-names.md) | Logical table names mapped by the kernel | Accepted | P0 |
| [0004](0004-kernel-auth-boundary-and-dev-stand-in.md) | Kernel / bos.auth API boundary with dev stand-in | Accepted | P0 (API + stand-in), P1/P2 (real impl) |
| [0005](0005-package-manifest-semver-checksum.md) | Package manifest, SemVer, ranges, SHA-256 reproducible bundle | Accepted | P0 (format), P2 (verified) |
| [0006](0006-detached-ed25519-signature.md) | Detached Ed25519 signature format | Accepted | P2 (required), P3 (publisher flow) |
| [0007](0007-audit-event-shape-hash-chain.md) | Audit event shape with hash chaining | Accepted | P0 (shape), P1 (chain), P3 (WORM) |
| [0008](0008-security-profile.md) | `security_profile` matrix and resolution order | Accepted | P0 |
| [0009](0009-secret-and-key-provider.md) | SecretProvider / KeyProvider abstraction | Accepted | P0 (file impl), P2 (OpenBao) |

## Rules
- Each ADR has a status, a date and these sections: Context, Decision, Consequences, Alternatives, Reversibility, Phase/enforcement.
- An accepted ADR is changed only by a new ADR that supersedes it. The old one gets `Status: Superseded by NNNN`.
- Every format carries a version field (`*_version`), so it can change without rewriting old data.
- Source documents: `reports/cto_phased_security_plan.md`, `reports/cto_proposals_d1_d2_d3_d9.md` (D1/D2/D3/D9 end states accepted as phase-gated targets), `reports/cto_totp_plan.md`.