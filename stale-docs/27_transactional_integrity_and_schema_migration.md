# 27. Transactional Integrity, Relations & Schema Migration

**Status:** Implemented (closes findings C1–C6 and C8 of [`26_production_readiness_status_report.md`](./26_production_readiness_status_report.md) §3.2).
**Crates:** `bosc-syntax`, `bosc-analyzer`, `bosc-ir`, `bosc-codegen-db`, `bosc-runtime`, `bosc-cli`. **UI:** `TableDesigner.svelte`, `studioApi.js`.
**Verification:** `crates/bosc-runtime/tests/transactional_integrity.rs` (real PostgreSQL, see §8).

This document is the single source of truth for how BOS keeps business data consistent: one database transaction per unit of work, enforced relations, a model-vs-database migration engine, mandatory optimistic concurrency, and an instruction budget for scripts.

---

## 1. Decision: number series live in BOS packages, not in Rust (C7)

Finding C7 (a number-series fallback to a random UUID suffix) is closed by **removal**, not by repair.

- The Rust runtime no longer offers any number-series API: `ctx.numSeries`, the `__host_num_series_next` host bridge and its ambient TypeScript declaration are gone. The regression test `test_number_series_host_api_removed` (`quickjs_engine.rs`) keeps them gone.
- Number series are modelled as ordinary BOS objects in an application package (a `NumberSeries` / `NumberSeriesLine` `table` and a `NumberSeriesManagement` `codeunit`), exactly as `10_`, `12_` and `01_bos_script_framework_deep_dive.md` §4.1 describe.
- Because a codeunit call is now one transaction (§2), "read last number, increment, write it back" inside that codeunit is atomic with the document that consumes the number. The row is locked for the rest of the unit of work (`ctx.db.update` locks with `SELECT … FOR UPDATE`), so two concurrent postings cannot draw the same number and a failed posting gives its number back.

> Plain English: the next invoice number is taken in the same "all or nothing" step as saving the invoice. If saving fails, the number is not used up, and two cashiers can never get the same number.

The `NumberSeries` package itself has not been authored yet; it is application work for the Phase 3 foundation (`26_` §5).

---

## 2. Unit of work (C1, C3)

Every top-level operation opens exactly one database session and commits or rolls back once:

| Entry point | Unit of work |
| --- | --- |
| `POST /api/codeunits/{name}/run`, `bosc run` | The procedure, every record it touches, every trigger, event subscriber and outbox row it causes |
| `POST/PATCH/DELETE /api/entities/...` | The write, `onValidate`/`onInsert`/`onModify`/`onDelete` triggers and the events they raise |
| `POST /api/entities/{table}/{id}/transition` | Guard, conditional state update, history row and transition events |
| `POST /api/events/publish` | All subscribers of the event |

Mechanics (`bosc-runtime/src/lib.rs`):

- `BosEngine::begin_scope(ctx)` opens a `DbSession` (one pooled connection, `BEGIN`, tenant/company/user GUCs set with `SET LOCAL`) and wraps it in an `ExecScope { db, ctx, services, depth }`.
- All work runs through the `*_in(&scope, …)` variants (`insert_in`, `update_in`, `delete_in`, `transition_in`, `EventRegistry::dispatch_in`, `ExecutionEngine::execute_procedure_in`). Nested codeunit calls and triggers receive the same scope, so the QuickJS host bridges (`ctx.db.*`) write through the same connection.
- `scope.db.finish(outcome)` commits on `Ok` and rolls back on `Err`. Any error raised by a script, a trigger, a guard, a constraint or a concurrency check therefore undoes **all** writes of that unit of work.
- After a statement fails inside the session, PostgreSQL rejects further statements; the session remembers the failure and the script returns `RuntimeError::TransactionAborted` even if the script caught the JavaScript exception, so it cannot swallow the failure and commit partial work.
- Outbox rows are enqueued inside the same transaction (`EventRegistry::enqueue_outbox_in`) and are only visible to the dispatcher after commit.

> Plain English: posting an order either saves the order, its lines, its ledger entries and its notifications together, or saves none of them.

---

## 3. Test isolation (C2)

`BosEngine::run_tests` (behind `POST /api/tests/run`) opens a scope, runs every `@test` procedure inside it, and **always rolls back**. Tests see their own writes while running and leave no data behind. The previous behaviour, which ran tests against the pool and rolled back an unused transaction, has been removed.

---

## 4. Relations become real foreign keys (C4)

### 4.1 Authoring

```bos
table SalesLine {
    documentNo: String(20) { notNull = true; relation = SalesHeader.no; onDelete = cascade; };
    customerNo: String(20) { relation = Customer.no; onDelete = setNull; };
    itemNo:     String(20) { relation = Item.no; };          // onDelete defaults to restrict
    primaryKey(documentNo, lineNo);
}
```

| Property | Values | Default | Rule |
| --- | --- | --- | --- |
| `relation` | `Table.field` | — | Must be a qualified `Table.field` reference; anything else is a compile error |
| `onDelete` | `restrict`, `cascade`, `setNull`, `noAction` (any casing or separators) | `restrict` | Only valid next to `relation`; `setNull` on a `notNull = true` field is a compile error |

The analyzer lowers these into `ForeignKeyIR { target_table, target_column, on_delete }` (`ForeignKeyIR::normalize_on_delete` in `bosc-ir`).

### 4.2 Installation

- `bosc-codegen-db/src/relations.rs` emits one `SELECT bos_ensure_relation(...)` per relation **after all tables** of a deployment exist, so declaration order does not matter. For every storage strategy it targets every physical copy: the current schema (SharedRLS / Partitioned), each company schema plus `public` (SchemaPerCompany), or each `company$table` (TablePerCompany).
- The kernel function `bos_ensure_relation` (installed with the kernel DDL) resolves the target table (same schema with prefix, same schema, `public` with prefix, `public`; `id` maps to `_id`), requires a unique index on the target column (or on `(_tenant_id, column)`, in which case a composite tenant-safe key is created), and creates `fk_{table}_{column}` (truncated to 63 characters).
- Existing data is protected: the constraint is added `NOT VALID` and then validated; if legacy rows violate it, the constraint stays in place for new writes and the report says `created_not_validated`.
- The schema synchronizer (§5) is the only installer. It reports each relation as `created`, `created_not_validated` or `unchanged`, recreates a constraint whose `onDelete` changed, and drops stale `fk_{table}_*` constraints for relations that were removed.

> Plain English: you cannot delete a customer who still has orders (unless the relation says `cascade`), and an order line can never point at an item that does not exist.

---

## 5. Schema migration engine (C5)

### 5.1 What it does

Every deployment path — `bosc deploy`, `bosc package install`, `POST /api/script/apply`, `POST /api/meta/tables/{table}/script`, `POST /api/meta/objects` (tables), `POST /api/meta/table-extensions` and package installs — runs `SchemaSync::sync` (`bosc-runtime/src/schema_sync.rs`) in **one transaction**:

1. Takes the advisory lock `hashtext('bos.schema_sync')`, so two deployments never interleave.
2. Applies table renames (`renamedFrom` on the table).
3. Finds every physical copy of each table (all company schemas and `company$table` copies; partitions excluded).
4. Diffs the declared `TableIR` against the live catalog (`bosc-codegen-db/src/migration.rs`) and plans ordered steps: `rename_column`, `add_column`, `change_type`, `change_default`, `drop_not_null` / `set_not_null`, `add_unique` / `drop_unique`, formula changes, `drop_column`.
5. Classifies each step as **safe** or **destructive**. Dropping a column, narrowing or converting a type (anything other than a lossless widening such as `VARCHAR(20)` → `VARCHAR(50)` or `INTEGER` → `BIGINT`) and turning a stored column into a formula are destructive.
6. Refuses destructive steps unless the caller passes `allow_destructive`. A blocked deployment changes nothing and returns `RuntimeError::SchemaSyncBlocked` (HTTP `409`).
7. Drops and recreates the `{table}_view` FlowField view around the change, runs the package DDL, then installs relations (§4.2).
8. Journals the run in `_bos_schema_migrations` (`package_name`, `source`, SHA-256 `checksum`, `steps`, `relations`, `destructive`, `applied_by`, `applied_at`).

Columns added by table extensions (from the request or already in the catalog) are merged into the declared shape, so deploying a base table never drops extension columns.

### 5.2 Renames without data loss

```bos
table SalesOrder {
    renamedFrom = "Order";                                  // table rename
    shipTo: String(100) { renamedFrom = "address"; };       // column rename
}
```

The data stays in place; only the name changes. Without `renamedFrom`, a renamed field is treated as "drop old, add new" and is therefore blocked as destructive.

### 5.3 Dry run and destructive approval

| Surface | Dry run | Allow destructive |
| --- | --- | --- |
| CLI `bosc deploy` | `--dry-run` (executes the plan and rolls back, printing the exact report) | `--allow-destructive` |
| CLI `bosc package install` | — | `--allow-destructive` |
| `POST /api/script/apply`, `POST /api/meta/tables/{table}/script`, `POST /api/meta/objects`, `POST /api/meta/table-extensions` | — | body field `allow_destructive: true` |
| `POST /api/packages/install`, `POST /api/databases/{name}/packages/install` | — | query `?allow_destructive=true` |
| Studio Table Designer | — | On `409`, a confirmation dialog explains the blocked change and retries with `allowDestructive` |

Responses of the script/table endpoints include `schema_sync` (`SchemaSyncReport { package_name, applied, dry_run, steps, blocked, relations, migration_id }`).

> Plain English: changing a field length or adding a field just happens. Deleting a field, or shrinking one so values could be cut off, needs an explicit "yes, I know this removes data".

---

## 6. Mandatory optimistic concurrency (C6)

- Every update through the API must carry the `_system_version` the client last read.
- The runtime locks the row (`SELECT … FOR UPDATE`), compares versions, and increments `_system_version` in the same statement as the change.
- Missing version → `RuntimeError::VersionRequired` → HTTP **`428 Precondition Required`** `{ "error": true, "version_required": true, "message": … }`.
- Stale version → `RuntimeError::ConcurrencyConflict` → HTTP **`409 Conflict`** `{ "error": true, "conflict": true, "message": … }`.
- Script code inside a unit of work (`ctx.db.update`) already holds the row lock for the whole transaction; it may pass `_system_version` to enforce a version it read earlier, and a mismatch raises the same conflict.
- The Studio datasheet sends `_system_version` with each cell edit, adopts the new version from the response, and reloads the page of rows after a `409`.

> Plain English: if two people edit the same customer at once, the second save is refused with "changed by another user" instead of silently overwriting the first.

---

## 7. Script instruction budget (C8)

- QuickJS execution is metered by **fuel**, not wall-clock time. The interrupt handler fires every `FUEL_STEPS_PER_TICK` (10,000) interpreter steps (function calls and loop back-edges); each tick consumes 10,000 fuel.
- The default budget is `DEFAULT_MAX_FUEL` = **100,000** per execution; a codeunit's `max_fuel` overrides it (`ExecutionLimits::with_fuel`).
- When the budget is exhausted the script is interrupted (scripts cannot catch it) and the unit of work fails with "Execution fuel exhausted", rolling back every write.
- A 30-second wall-clock limit remains only as a backstop for host calls that block (for example a slow database query); it is not the primary limit.
- Reported `fuel_used` is in blocks of 10,000.
- Memory (8 MB) and stack (512 KB) caps are unchanged.

---

## 8. Verification

Integration tests: `crates/bosc-runtime/tests/transactional_integrity.rs`. They create a throw-away database `bos_it_<uuid>` per test and drop it afterwards. Set `BOS_TEST_DATABASE_URL` to a PostgreSQL URL with `CREATEDB` rights; without it the tests are skipped.

| Test | Proves |
| --- | --- |
| `codeunit_failure_rolls_back_every_write` | C1 |
| `trigger_failure_rolls_back_insert_and_trigger_writes` | C3 |
| `test_runner_rolls_back_test_writes` | C2 |
| `relations_become_enforced_foreign_keys` | C4 (cascade, unchanged on redeploy, rule change to restrict) |
| `schema_changes_migrate_data_and_guard_destructive_steps` | C5 (rename keeps data, widening is safe, drop blocked then allowed, journal row) |
| `updates_require_the_current_version` | C6 (428 without version, 409 on stale version) |
| `fuel_budget_stops_runaway_scripts_and_rolls_back` | C8 |

Unit tests cover the migration planner (`bosc-codegen-db/src/migration.rs`), relation emission (`bosc-codegen-db/src/lib.rs`), relation and rename lowering (`bosc-analyzer`), and the number-series removal (`bosc-runtime`).
