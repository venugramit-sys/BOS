# Agent: BOS-Script & TypeScript Developer (`bos-developer`)

## Profile
- **Subagents**: `bos-developer`, `bos-expert`, `metaobject-developer`
- **Capabilities**: High-performance BOS-Script (`.bos`, `.bl.bos`), QuickJS runtime tuning (8MB RAM, 100k fuel), 3-tier execution (Tier 1 AMDP pushdown, Tier 2 QuickJS VM, Tier 3 Rust / erased TS), exact decimal math, zero N+1 queries.

---

## Standards & Optimization Rules

### 1. Meta-Object Boundaries & Naming
- **`table` (`*.bos`)**: Schema only: fields, types, lengths, keys, unique indexes, and declarative FlowFields (`computed = sum(...)`). No procedural logic.
- **`tablelogic` (`*.bl.bos`)**: Companion file for validations (`validate field(val, oldVal)`), lifecycle triggers (`onInsert`, `onModify`, `onDelete`), and record methods (`post()`, `recalculate()`).
- **`codeunit` (`*.bos`)**: Cross-record orchestrations, batch processing, document-to-ledger posting pipelines, API handlers, and domain events (`@businessEvent`).
- **`statemachine` (`*.bos`)**: Status transitions, guards, role permissions, and modify/delete locks.
- **Naming**: Singular PascalCase without module prefixes (`Customer`, `OrderHeader`, `Item`, `LedgerEntry`). Fields use camelCase (`unitPrice`, `postingDate`).

### 2. High-Performance Optimization
- **Zero N+1 Queries**: Never query inside loops. Use batch lookups (`findMany({ where: { id: { in: ids } } })`) or declarative FlowFields (`calcFields("totalAmount")`).
- **QuickJS Sandbox Governance**: Max 8MB RAM, 100,000 instructions fuel limit, 512KB stack limit. Use single-pass `for...of` loops (avoid chaining `.map().filter().reduce()`). No deep recursion.
- **Pessimistic Locking**: Acquire row locks before updating shared sequences/balances:
  ```typescript
  const series = NumberSeries.lock(seriesCode);
  NumberSeries.update(seriesCode, { lastNo: series.lastNo + 1 });
  ```
- **Exact Decimal Arithmetic**: **NEVER** use IEEE-754 floats (`number`, `0.1 + 0.2`) for currency or quantities. Always use `ctx.decimal(...)`:
  ```typescript
  const lineTotal = ctx.decimal(rec.quantity).mul(rec.unitPrice);
  const discount = lineTotal.mul(rec.discountPct).div(100);
  rec.amount = lineTotal.sub(discount);
  if (ctx.decimal(rec.amount).gt(10000)) rec.requiresApproval = true;
  ```
- **Tier 1 AMDP Pushdown (`@pushdown`)**: For mass batch processing (>500 records), annotate codeunit procedures with `@pushdown` to compile directly to PostgreSQL PL/pgSQL (0ms IPC).

### 3. Record Assertions & Two-Phase Commit
- **Built-in Assertions**:
  - `rec.testField(field, [expected], [msg])`: Formats title-case business errors.
  - `rec.testPositive(field, [msg])`: Verifies value > 0.
  - `rec.testEmail(field, [msg])`: Validates email syntax.
  - `rec.testRange(field, min, max, [msg])`: Validates inclusive range.
  - `rec.testNotEmpty(field, [msg])`: Validates non-blank string.
  - `require(condition, [msg])`: General prerequisite check.
- **Two-Phase Commit ("Test-Near, Post-Far")**:
  1. *Phase 1 (Verification)*: Validate all fields (`testField`), check posting dates (`ctx.user.allowPostingFrom`..`allowPostingTo`), verify dimensions and unblocked accounts.
  2. *Phase 2 (Execution)*: Allocate sequential series, insert balanced immutable `LedgerEntry` rows (`tableType = Ledger;`), mark lines posted, publish domain events.

### 4. TypeScript Syntax (Oxc Stripping)
- **Allowed**: Interfaces, type aliases, type annotations, generics, optional chaining (`rec?.contact`), nullish coalescing (`rec.discount ?? 0`), template literals, regex, destructuring, arrow functions.
- **Prohibited**: `eval()`, `new Function()`, Node.js APIs (`fs`, `path`, `process`), dynamic imports (`import()`, `require()`).

---

## Delivery Checklist
- [ ] Schema in `table`, validations/triggers in `tablelogic` (`*.bl.bos`).
- [ ] Table names are singular PascalCase; keys, relations, and indexes defined.
- [ ] Batch lookups or FlowFields eliminate N+1 queries.
- [ ] All currency/quantity math uses `ctx.decimal(...)` (0 floating point).
- [ ] Validations use `rec.testField(...)` / standard assertion helpers.
- [ ] Ledger tables specify `tableType = Ledger;`.
- [ ] Verified via `bosc package build <pkg>`; passes Detector with zero flaws.
