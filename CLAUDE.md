@AGENTS.md

Claude Code: AGENTS.md is the single source of agent rules for this repo; the full original charter is in docs/agents_full_reference.md. Crate- and ui-level CLAUDE.md files import their local AGENTS.md/ARCHITECTURE.md and load only when you work in that folder.

# Compact instructions
When compacting, keep: the current task and acceptance criteria, files changed and why, failing test names with the key error lines, decisions made and open questions. Drop: full file contents, full test/build logs, exploration that led nowhere.
