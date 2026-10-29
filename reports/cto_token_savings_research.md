# CTO research: free, proven ways to cut AI token use in BOS

Date: 24 Oct 2026 (IST). Author: CTO agent. Read-only research; nothing installed or changed.
Already done (branch `chore-token-savings`, not merged): `.cursorignore`, `.cursorindexingignore`, `.claude/settings.json` deny-read, trimmed `AGENTS.md` (+ `docs/agents_full_reference.md`), `CLAUDE.md -> @AGENTS.md`.
Pending owner decision: root `.gitignore` + untracking ~35k build files; D1-D9 proposals.

Repo shape checked: 7 crates (`bosc-analyzer`, `bosc-cli`, `bosc-codegen-db`, `bosc-codegen-svelte`, `bosc-ir`, `bosc-runtime`, `bosc-syntax`), `ui/src` ~242 files (Svelte 5), `packages/`, 19 specs in `docs/`. Only one `AGENTS.md` (root). No `.cursor/rules`.

Star counts read from the GitHub API on 24 Oct 2026. "Savings" are ESTIMATES unless a source is cited.

## Ranked table

| # | Method / tool | IDEs | Effort | Expected savings | Privacy | Verdict |
|---|---|---|---|---|---|---|
| 1 | Nested `AGENTS.md` per crate + `ui/` (short, ~30-60 lines) | Cursor, Antigravity (directory-scoped AGENTS.md, documented), Claude Code (via nested CLAUDE.md that imports AGENTS.md) | Low | Estimate: avoids re-exploring a crate each session; root file stays small | Local | DO (quick win) |
| 2 | Session hygiene: `/clear` between tasks, `/compact <focus>`, "Compact instructions" in CLAUDE.md, specific prompts, plan mode | Claude Code (Cursor/Antigravity: new chat per task) | None | Anthropic docs: stale context is re-sent every message; `/clear` costs nothing, `/compact` itself is a large request | Local | DO (habit, AGENTS.md rule) |
| 3 | Model routing: cheap model for reading/tests/subagents, top model only for synthesis; lower `/effort` for simple tasks | All | None | Anthropic docs: Sonnet for most work, `model: haiku` for simple subagents; thinking billed as output | n/a | DO (team policy) |
| 4 | Cursor rules `.cursor/rules/*.mdc` with scoped `globs` (rust.mdc for `crates/**`, svelte.mdc for `ui/**`); Antigravity `.agents/rules/*.md` with `trigger: glob` | Cursor; Antigravity (documented glob/model-decision/manual triggers, 12,000 char limit per file) | Low | Estimate: rules load only when matching files are touched instead of always | Local | DO |
| 5 | Skills for occasional workflows (migrations, packaging, release, QA run) moved out of AGENTS.md | Claude Code (`.claude/skills`), Antigravity (`.agents/skills/<name>/SKILL.md`; legacy workflows stopped 19 Oct 2026), Cursor (rules "agent requested"/commands) | Low-Med | Anthropic docs: skills load on demand; keep CLAUDE.md under ~200 lines | Local | DO |
| 6 | Code-map files: per-crate `ARCHITECTURE.md` (purpose, key types, entry points, depends-on) + one `docs/CODEMAP.md` | All | Med (one-off, must be kept current) | Anthropic docs cite a "codebase-overview" skill as replacing multi-file exploration; size of saving is an estimate | Local | DO (phase 2, with "update on change" rule) |
| 7 | Subagents for verbose work (tests, logs, doc fetch) returning summaries; cheap model in frontmatter | Claude Code (`.claude/agents/*.md`); Cursor/Antigravity have equivalent background agents | Low | Anthropic docs: verbose output stays in subagent context. Caution: agent teams use ~7x tokens (docs) | Local | DO, small and scoped |
| 8 | Hooks that filter output (e.g. `cargo test` / `pnpm test` -> failures only) | Claude Code (PreToolUse); Cursor has hooks too (not verified here) | Low-Med | Anthropic docs: log grep via hook cuts "tens of thousands of tokens to hundreds" (their example) | Local | DO (phase 2) |
| 9 | ripgrep-first habit + ast-grep for structural search (16,339 stars, MIT, pushed 23 Oct 2026, Rust + TS support) | Any agent with a shell | Low (single binary) | Estimate: targeted matches instead of whole-file reads | Local | DO (rg already standard; ast-grep optional, needs install approval) |
| 10 | MCP/tool pruning: disable unused MCP servers, prefer CLIs (`gh`, `cargo`) | All | None | Anthropic docs: CLIs add no per-tool listing; `/context` shows what fills the window | Local | DO |
| 11 | Serena MCP (oraios/serena, 30,673 stars, pushed 20 Oct 2026, license "Other") - LSP-based symbol find/edit | Claude Code, Cursor, any MCP client; Antigravity via MCP (not verified) | Med (Python/uv + rust-analyzer; Svelte LS support not verified) | Anthropic docs: code-intelligence plugins replace grep + multi-file reads; Serena-specific % not verified | Local (runs on machine) | PILOT on crates/ only |
| 12 | Usage measurement: Claude Code `/usage`, `/context`, `/insights`; ccusage (19,158 stars, pushed 24 Oct 2026) reads local logs | Claude Code (ccusage also other CLIs per its docs, not verified) | None-Low | Saves nothing directly; gives a baseline to prove the others work | Local (reads `~/.claude` logs) | DO (built-ins now; ccusage optional) |
| 13 | Prompt caching | Automatic in Claude Code / Cursor / Antigravity | None | Anthropic docs: automatic; misses after idle > TTL and when tool lists or CLAUDE.md change | n/a | KEEP STABLE: don't churn rules/MCP mid-session |
| 14 | Context7 (upstash/context7, 63,569 stars, MIT, pushed 23 Oct 2026) - fetch current library docs | All MCP clients | Low | Estimate: avoids wrong-API retries for Svelte 5 / crates | CLOUD: queries go to Upstash servers (no code needed, but prompts/library names leave the box) | OPTIONAL, owner approval; never paste code into queries |
| 15 | Repomix (29,006 stars, MIT, pushed 24 Oct 2026) - packs repo into one file, `--compress` (tree-sitter) | Any | Low | Packing a whole repo INCREASES tokens per call; useful only for one-off reviews of a single crate | CLI is local; repomix.com web upload and remote mode are NOT | AVOID as daily habit; allow local `--compress` for single-crate reviews |

## Per-tool notes (short)
- **Nested AGENTS.md:** one per crate (`crates/bosc-*/AGENTS.md`) and `ui/AGENTS.md`: purpose, invariants, test command, "don't touch". Antigravity loads directory AGENTS.md walking up from the edited file (antigravity.google/docs/rules). For Claude Code add `CLAUDE.md` with `@AGENTS.md` beside each.
- **Cursor rules:** `.mdc` with `description`, `globs`, `alwaysApply: false`. Keep root rule tiny. Source: cursor.com/docs (rules); exact loading behaviour not re-verified today.
- **Antigravity:** rules in `.agents/rules/*.md` need YAML `trigger`; only immediate children scanned unless listed in `.agents/rules.json`. Workflows are deprecated -> use skills.
- **Claude Code docs source:** docs.claude.com/en/docs/claude-code/costs (fetched 24 Oct 2026): `/clear`, `/compact`, model choice, MCP overhead, hooks, skills, subagents, agent-team ~7x cost, CLAUDE.md < 200 lines.
- **Serena:** strongest semantic-navigation option for Rust; risk = extra process, memory, tool list cost in context; pilot and measure with `/usage` before adopting.
- **Context7:** only cloud-touching tool on the list; data sent is the query, not files, but flag for owner.
- **Repomix:** fine for "review this crate once"; harmful for routine sessions. Never use the website with BOS code.
- **Multi-agent team:** briefs must name files/paths, ask for summaries not dumps, cap parallel agents, shut down when done, cheap models for reading (matches CEO's existing direction).

## Avoid
- Repomix whole-repo packs in normal sessions; repomix.com uploads.
- Large always-on rules or long CLAUDE.md; changing rules/MCP mid-session (breaks prompt cache).
- Agent teams / many parallel agents for small tasks (~7x tokens per Anthropic docs).
- Any hosted "code indexing" service that uploads source (privacy).

## Phased rollout
**Phase 0 - measure (no approval needed beyond normal use):** record baseline with `/usage` and `/context` for a typical task.
**Phase 1 - quick wins (owner approval to commit files on a local branch, e.g. extend `chore-token-savings`):**
1. Nested `AGENTS.md` (+ `CLAUDE.md` import) for 7 crates and `ui/`.
2. `.cursor/rules/rust.mdc`, `svelte.mdc`; `.agents/rules/` equivalents with glob triggers.
3. Add session-hygiene + model-routing + "rg first, read ranges" rules and a "Compact instructions" section to AGENTS.md.
4. Disable unused MCP servers per IDE (user setting, owner approval).
**Phase 2 - medium (owner approval):**
5. Per-crate `ARCHITECTURE.md` + `docs/CODEMAP.md`, with a rule to update on structural change.
6. Skills for migrations/packaging/QA; Claude subagents (test-runner, log-reader) on a cheap model.
7. Output-filter hooks for `cargo test` / `pnpm test` (Windows: PowerShell script).
**Phase 3 - pilots (owner approval; installs required):**
8. Serena on `crates/` for one week; compare `/usage` with baseline.
9. ast-grep binary; ccusage for weekly reports.
10. Context7 only if owner accepts queries leaving the machine.
Also unblocks savings: the pending root `.gitignore` / untracking decision.
