# Rust Compiler (`bosc`) Architecture

> **Status:** Matches the workspace crates  
> **Document:** `docs/02_rust_compiler_architecture.md`

The compiler turns `.bos` files into a `ModelIR` and PostgreSQL DDL. Scripts run in QuickJS inside `bosc-runtime`. The Studio UI is handwritten Svelte 5. It is not generated from `page` objects.

## Workspace crates

```
crates/
├── bosc-syntax/       # Lexer, recursive-descent parser, AST
├── bosc-analyzer/     # Lowering, Oxc syntax checks, type erasure, script optimizer
├── bosc-ir/           # ModelIR (tables, triggers, codeunits, pages as data)
├── bosc-codegen-db/   # PostgreSQL DDL, migrations, SIFT indexes, RLS, formulas
├── bosc-runtime/      # QuickJS host, data engine, wasmi Wasm host, HTTP API
└── bosc-cli/          # `bosc` command line
```

There is no `bosc-codegen-wasm`, `bosc-codegen-ui`, or `bosc-lsp` crate. Pages are not compiled to Svelte. Triggers are not compiled to Wasm. Editor help is the Monaco shadow TypeScript model plus `/api/meta/ambient.d.ts` and `monacoBosScript.js`.

## Pipeline

1. **Parse.** `bosc-syntax` reads meta-object keywords. A script body is kept as source text with its original line breaks. `Parser::with_source` is what the compile path uses, so `when` guards can wrap that source.
2. **Check embedded TypeScript.** `oxc_parser` parses each body (wrapped so a top-level `return` is legal) and reports syntax errors as `BOS1001` with line and column. Oxc does not typecheck against `ambient.d.ts`.
3. **Erase types.** The executable JavaScript comes from the line-preserving eraser in `ts_stripper.rs`. Oxc's printer does not keep the original line breaks, and QuickJS stacks use those lines, so the eraser stays the emitter.
4. **Optimize the JavaScript.** After erasure, `script_opt` does two rewrites:
   - `for (const row of rows) { Model.get(row.field) }` becomes one `Model.getMany` plus a `Map` lookup. The helper `__bosCollect` is inserted only when a hoist happened.
   - Arithmetic on `Decimal` / `NUMERIC` fields becomes `ctx.decimal(...).add/sub/mul/div(...).toString()` so PostgreSQL receives an exact numeric string.
5. **Lower.** Field expressions become SQL. `formula = quantity * unitPrice` and `check = quantity >= 0` become quoted snake_case fragments (`GENERATED ALWAYS AS` and a check constraint). A string formula is stored as written. `copyFrom` becomes an `onValidate{field}` trigger. `series = "SO"` becomes a `beforeInsert` that calls `ctx.call("NumberSeriesManagement", "getNext", "SO")` when the field is empty. `onModify(...) when status` runs the body only when `rec.status` differs from `xRec.status`.
6. **Diagnostics and tier.** `BOS3001` notes a hoist. `BOS3002` warns when a `get` stays inside a `for...of` that was not rewritten. `BOS3200` is an info fuel estimate. A codeunit method runs as PL/pgSQL only when every statement translates (assignments of literals, a single comparison, `testField`, `throw`, `ctx.audit.log`). Anything else, including queries and loops, stays on QuickJS. `@pushdown` on a body that does not fully translate is error `BOS3301`. The generator does not emit a function that skips the leftover lines.
7. **Database.** `bosc-codegen-db` emits DDL. Shared and partitioned tables get `ENABLE ROW LEVEL SECURITY` and a `tenant_isolation_<table>` policy on `_tenant_id`. A FlowField equality filter such as `customerNo == rec.no` becomes a SIFT index on `(_tenant_id, customer_no) INCLUDE (amount)`. A measured FlowField with no equality filter warns `BOS3101` and the index is only `_tenant_id`. Formula columns emit `GENERATED ALWAYS AS (...) STORED`.
8. **Run.** Each script runs on a QuickJS runtime checked out of a thread-local pool of four. The pool is the warm cache. Bytecode is not stored. A procedure's `return` is captured and becomes the value of `ctx.call`.

## What the runtime executes

- Partial reads: `select` is a list of names or `{ field: true }`.
- `get(pk, { select })` uses that projection.
- `getMany(keys)` is one `IN` query, at most 1000 keys. Each row includes `__bosKey`.
- `createMany(records)` is one `INSERT ... VALUES (...), (...)`.
- `modify(record)` writes the dirty patch, not every field.
- `require(cond, message)` throws when `cond` is falsy.
- Wasm modules load in `wasmi` with fuel. From script: `ctx.wasm.invoke(module, function, a, b)` passes two numbers and returns a number. There is no Wasm memory cap in the host.
