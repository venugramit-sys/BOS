# bosc-ir architecture

## Purpose
Serializable intermediate representation shared by all crates: tables, fields, FlowFields, triggers, state machines, package manifests/bundles/artifacts, and page IR (authored v2 + compiled).

## Map
`src/lib.rs` ModelIR, TableIR, ColumnIR, FlowFieldIR, StateMachineIR, PackageManifestIR, PackageBundleIR...; `src/page.rs` PageIR/PageExtensionIR (authored) and CompiledPageIR/ResolvedControlIR.

## Contracts / constraints
Persisted in `_bos_meta_objects.definition`: new fields must be `#[serde(default)]` so old JSON loads. Changing IR is effectively a data contract change.

## Specs
docs/02, docs/03_compiler_and_ir_architecture.md, docs/10_modular_packaging_system.md

_Keep current: update when modules, public types or dependencies change._

