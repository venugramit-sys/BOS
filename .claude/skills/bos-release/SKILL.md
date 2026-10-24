---
name: bos-release
description: Build the BOS distribution bundle locally (UI + bosc binary + configs + zip with SHA-256). Use for release candidates; never publishes or pushes.
---

# bos-release

1. Confirm detector verdict FLAWLESS and QA run green for the release scope.
2. `powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/build-distribution.ps1` (add `-SkipFrontend` only if ui/ unchanged).
3. Verify `dist/bos-server-v<version>-windows-x64.zip` and its `.sha256` match; smoke-test `dist/bos-server/bin/bosc.exe --help`.
4. dist/ is git-ignored: do not commit artifacts. Do not push, tag or upload anywhere without owner approval.
5. Report version, artifact paths, checksum, and docs/14 or docs/30 updates needed.
