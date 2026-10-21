# Volume 3: Data Access ORM (`models.*` / `ctx.db.*`), Filtering & Runtime SDK (`ctx.*`)

This volume documents the complete **QuickJS Data Access ORM** (`models.<Table>` and `ctx.db.<Table>`), filter operators, query options, and the `ctx` Host Runtime SDK implemented in `crates/bosc-runtime/src/quickjs_engine.rs`, along with how Monaco Code Studio provides 100% zero-manual-typing autocompletion for every method.

---

## 1. Three Equivalent Ways to Access a Table Repository

Inside any `tablelogic`, `codeunit`, or `statemachine` script, every table in the active database is accessible through three equivalent bindings:

1. **Explicit `models.<Table>` Registry (Recommended)**:
   ```ts
   const note = models.Note.first({ where: { code: "N-100" } });
   ```
2. **Session Context `ctx.db.<Table>`**:
   ```ts
   const notes = ctx.db.Note.findMany({ where: { content: { contains: "urgent" } } });
   ```
3. **Direct Capitalized Global Table Proxy (`<Table>.*`)**:
   - `QuickJsEngine` installs a global `Proxy` on `globalThis` so any capitalized table identifier (`Note`, `Customer`, `SalesHeader`) resolves automatically to `__getModel("<Table>")`:
   ```ts
   const count = Note.count({ status: "Active" });
   ```

---

## 2. Complete `TableModel<TRecord>` Method Reference

The methods below are the ones `QuickJsEngine` installs on every table repository. Monaco offers the same names.

| Method | Signature | SQL Compiled by `QuickJsEngine` | Return Value |
| :--- | :--- | :--- | :--- |
| **`get`** | `get(pk, opts?)` | `SELECT <select_or_jsonb> FROM "<tbl>" t WHERE t."<pk_col>"::text = $1 LIMIT 1` | `TRecord \| null` |
| **`findById`** | `findById(pk, opts?)` | Alias for `get(pk, opts?)` | `TRecord \| null` |
| **`getOrThrow`** | `getOrThrow(pk, opts?)` | Calls `get(pk, opts?)`; throws `Error("Record not found in <tbl> with key '<pk>'")` if `null` | `TRecord` |
| **`first`** | `first(query?)` | `SELECT <select_or_jsonb> FROM "<tbl>" t WHERE ... ORDER BY ... LIMIT 1` | `TRecord \| null` |
| **`findFirst`** | `findFirst(query?)` | Alias for `first(query?)` | `TRecord \| null` |
| **`last`** | `last(query?)` | `SELECT <select_or_jsonb> FROM "<tbl>" t WHERE ... ORDER BY t."<pk_col>" DESC LIMIT 1` | `TRecord \| null` |
| **`findMany`** | `findMany(query?)` | `SELECT <select_or_jsonb> FROM "<tbl>" t WHERE ... ORDER BY ... LIMIT <take\|100> OFFSET <skip>` | `TRecord[]` |
| **`getMany`** | `getMany(keys)` | One `WHERE "<pk>"::text IN (...)` for at most 1000 keys. Each row adds `__bosKey`. | `TRecord[]` |
| **`exists`** | `exists(where?)` | `SELECT EXISTS(SELECT 1 FROM "<tbl>" t WHERE ...)` | `boolean` |
| **`count`** | `count(where?)` | `SELECT COUNT(*)::bigint FROM "<tbl>" t WHERE ...` | `number` |
| **`create`** | `create(data)` | `INSERT INTO "<tbl>" (...) VALUES ($1, ...) RETURNING to_jsonb("<tbl>".*) as data` | `TRecord \| null` |
| **`insert`** | `insert(data)` | Alias for `create(data)` | `TRecord \| null` |
| **`createMany`** | `createMany(records[])` | One `INSERT ... VALUES (...), (...) RETURNING to_jsonb(t.*)`. Empty input returns `[]`. | `TRecord[]` |
| **`update`** | `update(pk, patch)` | `UPDATE "<tbl>" SET "col1" = $1, ... WHERE "<pk_col>"::text = $N RETURNING to_jsonb("<tbl>".*)` | `TRecord \| null` |
| **`modify`** | `modify(record)` or `modify(pk, patch)` | A record writes `__bosDirtyPatch()` (changed fields only). A key plus patch writes that patch. | `TRecord` |
| **`upsert`** | `upsert({ where, create, update? })` | `INSERT INTO "<tbl>" (...) VALUES (...) ON CONFLICT ("<where_key>") DO UPDATE SET ... RETURNING to_jsonb("<tbl>".*)` | `TRecord \| null` |
| **`delete`** | `delete(pk)` | `DELETE FROM "<tbl>" WHERE "<pk_col>"::text = $1` | `boolean` |
| **`lock`** | `lock(pk)` | `SELECT to_jsonb(t.*) as data FROM "<tbl>" t WHERE t."<pk_col>"::text = $1 FOR UPDATE LIMIT 1` | `TRecord \| null` |

---

## 3. Filtering (`where`), Operators, Sorting (`orderBy`), Pagination & Projection (`select`)

### 3.1. All Supported `where` Filter Operators

`QuickJsEngine::append_where_map` compiles filter objects into 100% parameterized SQL (`$1, $2, ...`) to prevent SQL injection:

| Operator | Syntax Example | Compiled PostgreSQL Clause |
| :--- | :--- | :--- |
| **Direct Equality** | `{ status: "Active" }` | `t."status" = $1` |
| **Direct `null`** | `{ deleted_at: null }` | `t."deleted_at" IS NULL` |
| **`eq`** | `{ status: { eq: "Active" } }` | `t."status" = $1` *(or `IS NULL` if `null`)* |
| **`ne` / `not`** | `{ status: { ne: "Archived" } }` | `t."status" != $1` *(or `IS NOT NULL` if `null`)* |
| **`gt`** | `{ amount: { gt: 1000 } }` | `t."amount" > $1` |
| **`gte`** | `{ amount: { gte: 1000 } }` | `t."amount" >= $1` |
| **`lt`** | `{ amount: { lt: 5000 } }` | `t."amount" < $1` |
| **`lte`** | `{ amount: { lte: 5000 } }` | `t."amount" <= $1` |
| **`contains`** | `{ title: { contains: "urgent" } }` | `t."title"::text ILIKE '%' \|\| $1 \|\| '%'` |
| **`startsWith`** | `{ code: { startsWith: "INV-" } }` | `t."code"::text ILIKE $1 \|\| '%'` |
| **`endsWith`** | `{ email: { endsWith: ".org" } }` | `t."email"::text ILIKE '%' \|\| $1` |
| **`ilike`** | `{ title: { ilike: "%Q4_2026%" } }` | `t."title"::text ILIKE $1` |
| **`like`** | `{ code: { like: "SO-2026-%" } }` | `t."code"::text LIKE $1` |
| **`in`** | `{ status: { in: ["Draft", "Open"] } }` | `t."status" IN ($1, $2)` |
| **`notIn`** | `{ status: { notIn: ["Cancelled"] } }` | `t."status" NOT IN ($1)` |
| **`isNull`** | `{ approved_by: { isNull: false } }` | `t."approved_by" IS NOT NULL` |
| **`OR`** | `{ OR: [{ status: "Draft" }, { priority: { gte: 5 } }] }` | `((t."status" = $1) OR (t."priority" >= $2))` |
| **`AND`** | `{ AND: [{ amount: { gte: 100 } }, { amount: { lte: 500 } }] }` | `((t."amount" >= $1) AND (t."amount" <= $2))` |

### 3.2. Real-World Use Case: Complex Search, Projection & Atomic Upsert

```bos
codeunit NoteSyncService {
    procedure syncAndSearchNotes(rec: Note): void {
        // 1. Atomic Upsert with explicit conflict key, create payload, and update patch
        const synced = models.Note.upsert({
            where: { code: rec.code },
            create: {
                code: rec.code,
                content: rec.content,
                content_2: rec.content_2
            },
            update: {
                content: rec.content,
                content_2: "Updated: " + (rec.content_2 || "")
            }
        });

        // 2. Multi-condition search with OR group, column projection (`select`), sorting, and pagination
        const matchingNotes = models.Note.findMany({
            where: {
                content: { contains: "invoice" },
                OR: [
                    { code: { startsWith: "URG-" } },
                    { content_2: { isNull: false } }
                ]
            },
            select: ["code", "content"], // or { code: true, content: true }
            orderBy: { code: "asc" },
            take: 25,
            skip: 0
        });

        ctx.ui.success("Synced note " + synced.code + " (" + matchingNotes.length + " related matches found)");
    }
}
```

---

## 4. The `ctx` Host Runtime SDK

Inside `QuickJsEngine`, the global `ctx` object provides sandboxed access to host primitives:

### 4.0. What the host does before your script runs

- Decimal fields written as `rec.amount * rec.qty` are rewritten to `ctx.decimal(rec.amount).mul(rec.qty).toString()`.
- `for (const line of lines) { Item.get(line.itemNo) }` is rewritten to one `Item.getMany`.
- `require(cond, message)` throws when `cond` is falsy.
- A procedure `return` is the value `ctx.call` gives back.
- QuickJS runtimes are reused from a thread-local pool of four. Bytecode is not saved.

### 4.1. Session Identity
- **`ctx.tenantId`** (`string`): Active tenant UUID.
- **`ctx.companyId`** (`string`): Active company UUID.
- **`ctx.userId`** (`string`): Authenticated user UUID.

### 4.2. Exact Fixed-Point Monetary Arithmetic (`ctx.decimal`)
JavaScript IEEE-754 numbers (`0.1 + 0.2 === 0.30000000000000004`) are unsafe for accounting. `ctx.decimal(val)` uses `BigInt`-backed fixed-point arithmetic with 6-decimal internal precision (`SCALE = 1_000_000n`) and half-up commercial rounding:
```ts
const subtotal = ctx.decimal("1250.55");
const discount = subtotal.mul("0.15").round(2);       // "187.58"
const netAmount = subtotal.sub(discount);             // "1062.97"
const taxAmount = netAmount.mul("0.0825").round(2);   // "87.70"
rec.total_amount = netAmount.add(taxAmount).toNumber(); // 1150.67
```

### 4.3. Non-Blocking UI Toast & Messaging API (`ctx.ui.*`)
 triggers and codeunits can queue structured notifications in `rec._system_messages` and dispatch them to the frontend UI toast stack:
- `ctx.ui.toast(messageOrOptions, titleOrOptions?)` (also global `toast(...)`)
- `ctx.ui.message(message, type?)` (also `ctx.message(...)` and global `message(...)`)
- `ctx.ui.info(message, titleOrOptions?)`
- `ctx.ui.success(message, titleOrOptions?)`
- `ctx.ui.warning(message, titleOrOptions?)`
- `ctx.ui.error(message, titleOrOptions?)`
- `ctx.ui.confirm(message, defaultAnswer?)` records an info toast and returns `defaultAnswer`, or `true`. It does not wait for a click.
- `ctx.ui.openModal(page, params)`, `navigate(url)`, `setFieldVisible(field, visible)`, and `setFieldEditable(field, editable)` append a directive. `getDirectives()` returns them.

```ts
ctx.ui.warning("Order exceeds standard $5,000 approval threshold", {
    title: "Approval Required",
    duration: 8000,
    dismissible: true,
    action: {
        label: "Open Approval Request",
        url: "/pages/ApprovalEntries?doc=" + rec.code
    }
});
```

### 4.4. Cryptography, Statutory Audit, Events, WASM & Sidecars
- **`ctx.crypto.sha256(input)`**: Returns hex-encoded SHA-256 digest via Rust `sha2` crate.
- **`ctx.crypto.randomBytes(len).toHex()`**: Generates random hex token.
- **`ctx.audit.log(message)`**: Writes an immutable entry to `_system_privileged_audit_log`.
- **`ctx.events.publish(publisher, eventName, payload)`**: Synchronous in-memory pub/sub dispatch via `EventRegistry`.
- **`ctx.wasm.invoke(moduleName, funcName, a, b)`**: Calls a compiled WebAssembly numeric routine via `WasmEngine`.
- **`ctx.sidecar.call(serviceName, path, payload)`**: Invokes a registered external microservice via `SidecarHost`.
