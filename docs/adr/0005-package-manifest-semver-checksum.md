# ADR 0005: Package manifest, SemVer, ranges, SHA-256 over a reproducible bundle
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.5; D3.1-D3.4.

## Context
The checksum today is a 64-bit `DefaultHasher` (`crates/bosc-runtime/src/package_manager.rs:36-42`), which is neither stable across builds nor secure.

## Decision
`bos.json`, manifest v2:
```json
{
  "manifest_version": 2,
  "id": "bos.tax.in",
  "version": "1.4.0",
  "layer": "localization",
  "engine": ">=0.9.0 <2.0.0",
  "dependencies": { "bos.core": "^1.2.0" },
  "capabilities": ["db.read", "db.write", "net.outbound:gst.gov.in"],
  "source": { "repo": "https://github.com/vits/bos-packages", "commit": "a1b2c3d4" },
  "checksum": { "algo": "sha256", "value": "" }
}
```
- `id`: `^[a-z][a-z0-9]*(\.[a-z][a-z0-9-]*)+$`. The `bos.*` and `vits.*` prefixes are reserved for VITS. `layer` is one of `base|localization|industry|tenant`.
- `version`: SemVer 2.0.0. A published (id, version) is **immutable**.
- Ranges (`engine`, `dependencies`) use node-semver syntax: `^`, `~`, `>=`, `<`, `x`, hyphen ranges. Pre-releases match only when the range names a pre-release.
- **Canonicalization**
  1. Manifest: JSON with keys sorted by UTF-8 bytes (recursively), no insignificant whitespace, UTF-8 without BOM, numbers as written, `checksum.value` set to `""` while hashing (RFC 8785 JCS-compatible).
  2. Archive: POSIX ustar, entries sorted by byte-wise path, `/` separators, no `./` prefix, mtime 0, uid/gid 0, uname/gname empty, mode 0644 for files and 0755 for directories/executables. No symlinks, devices or absolute paths. LF line endings are preserved as committed. Not compressed for hashing (gzip may be used for transport only).
  3. `sha256 = SHA-256( canonical_manifest || 0x00 || archive )`, written as 64 lowercase hex characters into `checksum.value` in the shipped manifest.
- The lockfile records `{id, version, sha256, key_id, key_version}` per package.

## Consequences
Two builds of the same commit produce the same digest, and CI can re-verify it. The old checksum values are recomputed at the next build.

## Alternatives considered
Hashing the gzip output (not reproducible across zlib versions); a per-file hash list only (larger and a more complex verifier).

## Reversibility
COSTLY: published digests depend on the rules. Future changes are made via `manifest_version`.

## Phase / enforcement
P0: build emits it, install computes it and warns on mismatch. P2: a mismatch is a hard reject in staging/production.