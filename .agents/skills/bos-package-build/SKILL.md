---
name: bos-package-build
description: Build, check, install and proactively test a BOS package (.bospkg) with bosc and runtime verification. Use when creating or changing anything under packages/ or .bos meta-objects.
---

# bos-package-build

1. Read `packages/<id>/bos.json` and verify dependencies and meta-objects.
2. Build the package bundle:
   `cargo run -p bosc-cli -- package build packages/<id> -o packages/<id>.bospkg`
   (Verify 0 errors, 0 warnings).
3. Install into local dev DB:
   `cargo run -p bosc-cli -- package install -i packages/<id>.bospkg -d postgres://postgres:postgres@localhost:5432/bos_dev`
4. Proactive Verification Tests against Delivery:
   - Run procedure checks: test newly authored codeunit procedures via API (`/api/codeunits/<name>/run` or direct RPC).
   - Run trigger checks: test page action triggers via `POST /api/v1/pages/<Page>/triggers/action.<Action>` with valid and boundary payloads to verify `return_value` and ensure no unhandled QuickJS runtime errors (`TypeError`, `ReferenceError`).
   - Run UI checks: verify that dialogs have `inFooterBar = true;` on footer actions and that primary actions trigger on Enter key.
5. If any test fails, proactively diagnose, fix the `.bos` source, recompile, reinstall, and retest until 100% green before handing off.
6. Check `docs/planning/bos_core_and_script_improvements.md` and log any core engine bottlenecks, missing compiler warnings, or language improvement ideas discovered during authoring.
7. Report: package version, objects compiled/tested, and test verification summary. Never install into a shared or production DB without owner approval.
