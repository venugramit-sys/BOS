---
name: bos-package-build
description: Build, check and install a BOS package (.bospkg) with bosc. Use when creating or changing anything under packages/ or .bos meta-objects.
---

# bos-package-build

1. Read docs/10_modular_packaging_system.md section for the change (not the whole file) and packages/<id>/bos.json.
2. `cargo build -p bosc-cli` (reuse an existing build if sources unchanged).
3. Check: `cargo run -p bosc-cli -- package check` (see docs/09 for flags) - fix every diagnostic.
4. Build: `cargo run -p bosc-cli -- package build -i <source.bos> -o packages/<id>.bospkg --package-id <id> --name "<name>" --version <semver> --publisher "<publisher>"`.
5. Install into a LOCAL dev DB only: `... package install -i packages/<id>.bospkg -d <local dev url>`; then `package list` to confirm.
6. Report: version, diagnostics fixed, tables created/changed. Never install into a shared or production DB without owner approval.
