# bosc-cli - agent notes

Scope: `bosc` binary: build, run, deploy, test-live, detect-db, serve, provision, create-db, `package {init|check|build|install|list}`, `node ...` subcommands.
Owner: rust-developer. Specs: docs/09_bosc_cli_reference.md, docs/10, docs/14_distribution_and_packaging_guide.md.

## Gotchas
- Thin wiring layer; put logic in library crates, not here.

## Verify
`cargo build -p bosc-cli; cargo run -p bosc-cli -- --help` then `cargo clippy -p bosc-cli`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

