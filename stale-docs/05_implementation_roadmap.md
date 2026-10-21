# BOS Core Engine Implementation Roadmap & Milestones

> **Status:** Draft / Action Plan  
> **Document:** `docs/05_implementation_roadmap.md`  

---

## 1. Roadmap Overview & Timeline

The core engine development is structured into 6 focused phases designed to validate core primitives before building higher abstractions.

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│ Phase 1:         │     │ Phase 2:         │     │ Phase 3:         │
│ Grammar, AST &   │────►│ DB Codegen &     │────►│ QuickJS Host     │
│ Semantic IR      │     │ Postgres Schema  │     │ (shipped)        │
└──────────────────┘     └──────────────────┘     └──────────────────┘
                                                            │
                                                            ▼
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│ Phase 6:         │     │ Phase 5:         │     │ Phase 4:         │
│ Stdlib & Sample  │◄────│ Dev Tooling &    │◄────│ Svelte 5 UI      │
│ Base ERP Apps    │     │ Monaco Editor    │     │ (handwritten)    │
└──────────────────┘     └──────────────────┘     └──────────────────┘
```

---

## 2. Phase-by-Phase Execution Plan

### Phase 1: AST, Parser & Semantic Lowering (Rust)
- **Goal:** Parse BOS Script syntax, validate semantics, resolve cross-entity references, and emit `BOS-IR`.
- **Key Deliverables:**
  - `bosc-syntax`: Lexer and parser for BOS keywords (`table`, `page`, `tableextension`, etc.).
  - `bosc-analyzer`: Symbol table, scope resolver, typechecker, and extension merger.
  - `bosc-ir`: Normalized representation of schema, logic, and views.
- **Verification:** Unit tests parsing complex multi-table schemas with table extensions and verifying merged AST.

### Phase 2: PostgreSQL Infrastructure & Migration Codegen
- **Goal:** Compile `SchemaIR` into production-ready PostgreSQL infrastructure and automated migrations.
- **Key Deliverables:**
  - `bosc-codegen-db`: Generates PostgreSQL DDL (`CREATE TABLE`, `ALTER TABLE`, `CREATE INDEX`).
  - Native RLS generator translating `permissionset` rules into PostgreSQL policies.
  - Diff-based migration DAG generator with transactional safety guards.
- **Verification:** Automated migration tests running against live PostgreSQL Docker container, creating schemas and running RLS access tests.

### Phase 3: Host runtime (shipped as QuickJS, not Wasm codegen)
Procedural logic runs in `bosc-runtime` on QuickJS. Wasm modules are loaded by `wasmi` and called with `ctx.wasm.invoke`. There is no `bosc-codegen-wasm` crate and codeunits are not compiled to Wasm.

### Phase 4: Studio UI (handwritten)
The Studio under `ui/` is Svelte 5 written by hand. `page` objects are stored as metadata. They are not compiled into Svelte components. There is no `bosc-codegen-ui` crate.

### Phase 5: Editor help (Monaco, not a language server)
`bosc-cli` builds and serves packages. The editor uses a shadow TypeScript model, `/api/meta/ambient.d.ts`, and `monacoBosScript.js`. There is no `bosc-lsp` crate and no VS Code extension.

### Phase 6: BOS Standard Library & Reference ERP Modules
- **Goal:** Production validation using realistic enterprise workloads.
- **Key Deliverables:**
  - Standard Library: `NumberSeries`, `AuditLog`, `Security`, `DocAttachments`, `WorkflowEngine`.
  - Reference ERP App: General Ledger, Customer/Vendor Management, Sales Order processing.
- **Verification:** End-to-end transaction test: creating a customer, raising a sales order, calculating tax, posting to ledger, and viewing in Svelte UI.

---

## 3. Success Criteria & Verification Matrix

| Metric | Target | Verification Method |
| :--- | :--- | :--- |
| **Script host** | QuickJS, 8 MB, 512 KB stack, fuel, warm pool of 4 | `bosc-runtime` unit tests |
| **Wasm host** | `wasmi` with fuel, numeric `ctx.wasm.invoke` | `wasm_engine` unit tests |
| **Database** | DDL, tenant RLS, SIFT `INCLUDE`, stored formulas | `bosc-codegen-db` unit tests |
| **UI** | Handwritten Svelte Studio. Pages are not generated | Studio sources under `ui/` |
