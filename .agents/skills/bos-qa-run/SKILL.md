---
name: bos-qa-run
description: Run the BOS QA suites (Rust tests, page-kit tests, agent test scripts) and summarise only failures. Use for test matrix, regression or blast-radius checks.
---

# bos-qa-run

1. Blast radius: `node scripts/test-agent/track_changes.mjs` (or git diff --stat) to pick affected crates/views.
2. Rust: `cargo test -p <crate>` per affected crate (not the whole workspace unless asked). Integration tests in bosc-runtime need a local Postgres.
3. UI: `cd ui; npm run test:page-kit`; build check `npm run build`.
4. Agent suites when relevant: `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-agent/run_agent_tests.ps1`; API fuzz `node scripts/test-agent/fuzz_api_endpoints.mjs`; visual `node scripts/test-agent/visual_browser_test.mjs` (server must be running locally).
5. Report: pass/fail counts, each failing test with the 3-5 key error lines, suspected cause. Do not paste full logs.
