# bosc-ir - agent notes

Scope: Serializable intermediate representation shared by all crates: tables, fields, FlowFields, triggers, state machines, package manifests/bundles/artifacts, and page IR (authored v2 + compiled).
Owner: rust-developer. Specs: docs/02, docs/03_compiler_and_ir_architecture.md, docs/10_modular_packaging_system.md.

## Gotchas
- Persisted in `_bos_meta_objects.definition`: new fields must be `#[serde(default)]` so old JSON loads. Changing IR is effectively a data contract change.

## Verify
`cargo test -p bosc-ir` then `cargo clippy -p bosc-ir`.
## Always
- Root rules in ../../AGENTS.md apply (zero mocks, no todo!/unimplemented!, detector loop, docs sync).
- Read ARCHITECTURE.md here before exploring; search with rg/ast-grep and read line ranges, not whole large files.
- If you change module structure or public API, update ARCHITECTURE.md and docs/CODEMAP.md in the same commit.

