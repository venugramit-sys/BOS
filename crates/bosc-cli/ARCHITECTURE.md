# bosc-cli architecture

## Purpose
`bosc` binary: build, run, deploy, test-live, detect-db, serve, provision, create-db, `package {init|check|build|install|list}`, `node ...` subcommands.

## Map
`src/main.rs` (~60 KB) clap commands wiring syntax -> analyzer -> codegen-db -> runtime.

## Contracts / constraints
Thin wiring layer; put logic in library crates, not here.

## Specs
docs/09_bosc_cli_reference.md, docs/10, docs/14_distribution_and_packaging_guide.md

_Keep current: update when modules, public types or dependencies change._

