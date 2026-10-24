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

## After (chore-token-savings @ 4ec2e965, measured 2026-10-24 IST)

| Metric | Before (c648f6e8) | After | Change |
|---|---|---|---|
| Always-loaded root context (AGENTS.md + CLAUDE.md) | 10,804 B (~2,701 tok) | 12,540 B (~3,135 tok) | +1,736 B (+~434 tok): new "Token discipline" section + compact instructions |
| Directory-scoped AGENTS.md (on demand only) | 0 | 8 files, 7,817 B (~1,954 tok) | loaded only inside that crate/ui |
| Always-on Cursor/Antigravity rules | 0 | 0 (4+4 glob-scoped rules, each < 800 B) | none forced into every session |
| Git tracked files | 36,039 | 544 | -35,495 (target/, target-ledger/, dist/ untracked; still on disk) |
| Indexable files (.gitignore + .cursorignore) | 406 / 4,760,923 B | 459 / 4,811,460 B | +53 files (new docs/configs); build output was already .cursorignore'd |
| Example `cargo test -p bosc-syntax` output via hook | 29 lines raw | 7 lines | ~76% fewer lines; full-workspace run 222 -> 48 lines |
| size-pack | 3.97 GiB | 3.97 GiB | unchanged (history not rewritten, by design) |

Honest reading: the fixed per-session cost went UP slightly (~434 tokens). The savings are behavioural and on-demand:
crate detail moved into nested files, test logs filtered, cheap-model subagents for reading/testing, Serena symbol
lookups instead of whole-file reads, and /clear + /compact habits. These proxies cannot show real spend; measure real
usage with `/usage` / `/context` in Claude Code and `ccusage daily` (local, reads ~/.claude logs) over 1-2 weeks
before vs after.
