# Token-use baseline (proxy metrics)

Measured with `scripts/token_metrics.ps1` (reproducible; approx tokens = bytes/4, a heuristic, not a tokenizer).
These are PROXIES. Real per-session token use must be measured over real sessions with Claude Code `/usage` / `/context` or `ccusage daily` / `ccusage session`, comparing similar tasks before vs after.

## Before (commit c648f6e8, 24 Oct 2026 ~15:50 IST)
- Always-loaded at session start: AGENTS.md 10,654 B + CLAUDE.md 150 B = 10,804 B (~2,701 tokens est.). No Cursor/Antigravity rules.
- Directory-scoped AGENTS.md: none.
- Git tracked files: 36,039 (target/ 31,587, target-ledger/ 3,795, dist/ 167).
- Indexable files after .gitignore + .cursorignore: 406 files, 4,760,923 B (~1.19M tokens est. if read in full).
- Git object store: 6,726 loose objects counted, size-pack 3.97 GiB.
- Metrics script runtime: ~193 s (dominated by 36k tracked paths).
