# Volume 2: Procedural Logic — `tablelogic`, `codeunit`, Triggers, Events & Polymorphic Interfaces

This volume covers how executable business rules are authored, validated, and executed in **BOS-Script** using `tablelogic`, `tablelogicextension`, `codeunit`, `interface`, and `enum`.

---

## 1. Architectural Separation: `table` vs. `tablelogic`

BOS enforces a clean boundary between **data structure** and **domain behavior**:
- **`table <Name>`**: Holds physical columns, types, constraints, keys, and declarative FlowFields.
- **`tablelogic <LogicName> for <TableName>`**: Holds field validations (`validate <field>(val)`), row lifecycle triggers (`onInsert`, `onModify`, `onDelete`, etc.), and record methods.
- **`tablelogicextension <ExtName> extends <TableName>`**: Allows third-party or vertical packages to attach additional `validate` handlers and lifecycle triggers onto an existing table without touching its base `tablelogic`.

---

## 2. Field Validations (`validate <field>`) & Lifecycle Triggers

### 2.1. Complete Lifecycle Trigger Execution Order

When a record is inserted, modified, or deleted via `DataEngine`, `ExecutionEngine::dispatch_triggers` executes triggers in the following deterministic order:

```mermaid
sequenceDiagram
    participant Client as API / Codeunit
    participant DE as DataEngine
    participant EE as ExecutionEngine (QuickJS)
    participant PG as PostgreSQL (ACID Tx)

    Client->>DE: insert_record / update_record(table, payload)
    Note over DE: 1. Fetch pristine xRec (on update)
    loop For each changed field k in payload
        DE->>EE: TriggerEventIR::OnValidate(k)
        Note over EE: Runs `validate <k>(val)` in tablelogic<br/>+ tablelogicextension + EventRegistry
    end
    opt If rec.status == "Draft" or rec.is_draft == true
        DE->>EE: TriggerEventIR::OnDraftSave
    end
    DE->>EE: TriggerEventIR::OnBeforeInsert / OnBeforeModify (onInsert / onModify)
    Note over EE: Mutates `rec` in-place (e.g. normalizes fields,<br/>computes totals, emits ctx.ui.toast)
    DE->>PG: BEGIN; SET LOCAL bos.current_tenant_id...;<br/>INSERT / UPDATE (with OCC _system_version check); COMMIT;
    DE->>EE: TriggerEventIR::OnAfterInsert / OnAfterModify (afterInsert / afterModify)
    DE-->>Client: Mutated Record + _system_messages (Toasts)
```

### 2.2. Supported Trigger Names in `tablelogic`

| Trigger Signature | Firing Phase | Available Context Variables |
| :--- | :--- | :--- |
| `validate <fieldName>(val, oldVal)` | Whenever `<fieldName>` is present on insert, changed on update, or invoked via `rec.validate("<fieldName>", newVal)`. | `val`, `oldVal`, `rec`, `xRec`, `ctx`, `models` |
| `onInsert(rec)` / `beforeInsert(rec)` | Immediately before `INSERT INTO` (compute-first validation & field defaulting). | `rec`, `ctx`, `models` |
| `afterInsert(rec)` / `onAfterInsert(rec)` | After the `INSERT` transaction commits. | `rec`, `ctx`, `models` |
| `onModify(rec, xRec)` / `beforeModify(rec, xRec)` | Before `UPDATE`, with `rec` holding new values and `xRec` holding the prior database snapshot. | `rec`, `xRec`, `ctx`, `models` |
| `afterModify(rec, xRec)` / `onAfterModify(rec, xRec)` | After the `UPDATE` transaction commits. | `rec`, `xRec`, `ctx`, `models` |
| `onDelete(rec)` / `beforeDelete(rec)` | Before `DELETE FROM`. Throwing an `Error` or calling `error(...)` aborts deletion. | `rec`, `xRec`, `ctx`, `models` |
| `afterDelete(rec)` / `onAfterDelete(rec)` | After the `DELETE` transaction commits. | `rec`, `xRec`, `ctx`, `models` |
| `onDraftSave(rec)` | When a record with `status == "Draft"` or `is_draft == true` is inserted or modified. | `rec`, `xRec`, `ctx`, `models` |
| `onDraftActivate(rec)` | When transitioning a record out of `"Draft"` status via `transition_state`. | `rec`, `xRec`, `ctx`, `models` |
| `onDraftDiscard(rec)` | When deleting a record that is currently in `"Draft"` status. | `rec`, `xRec`, `ctx`, `models` |

### 2.3. Compiler sugar

These field and trigger forms are lowered by `bosc-analyzer`. They are not extra columns on the IR.

| Form | What is stored |
| :--- | :--- |
| `formula = quantity * unitPrice` | SQL `("quantity" * "unit_price")`, emitted as `GENERATED ALWAYS AS (...) STORED`. |
| `check = quantity >= 0` | SQL check `("quantity" >= 0)`. |
| `copyFrom = [Customer.name, Customer.paymentTermsCode]` | Trigger `onValidate{field}` that loads the source row and copies those fields. |
| `series = "SO"` | `beforeInsert` that calls `ctx.call("NumberSeriesManagement", "getNext", "SO")` when the field is empty. The call returns the procedure's `return` value. |
| `onModify(rec, xRec) when status { ... }` | The body runs only when `rec.status` differs from `xRec.status`. |

Inside a script, `rec.unitPrice * rec.quantity` on decimal fields is rewritten to `ctx.decimal(rec.unitPrice).mul(rec.quantity).toString()` before QuickJS runs.

---

## 3. Enterprise Record Prototype Helpers (`rec.*` & `xRec.*`)

Every `rec` and `xRec` object injected into `QuickJsEngine` inherits three Dynamics BC-style helper methods from `__recProto`:

1. **`rec.testField(fieldName, expectedValue?)`**:
   - If `expectedValue` is omitted: asserts that `rec[fieldName]` is not `null`, `undefined`, or `""`, throwing `Error("Field '<fieldName>' must not be blank.")` otherwise.
   - If `expectedValue` is provided: asserts `rec[fieldName] === expectedValue`, throwing `Error("Field '<fieldName>' assertion failed: expected '...', got '...'")` otherwise.
2. **`rec.calcFields(...fieldNames)`**:
   - Evaluates one or more declarative `flowField` subqueries against PostgreSQL via `__host_db_calc_flow_fields` and populates `rec[fieldName]` in-place.
3. **`rec.validate(fieldName, newValue?)`**:
   - Assigns `rec[fieldName] = newValue` (if provided) and invokes `validate<FieldName>(val, oldVal)` if defined in scope.

### Real-World Use Case: `SalesHeader` TableLogic with Credit Limit & Status Guards

```bos
tablelogic SalesHeaderLogic for SalesHeader {
    validate customer_id(val) {
        const cust = models.Customer.getOrThrow(val);
        if (cust.is_blocked) {
            throw new Error("Cannot create order: Customer " + cust.code + " is blocked.");
        }
        rec.currency_code = cust.currency_code || "USD";
        rec.payment_terms_code = cust.payment_terms_code || "NET30";
    }

    validate discount_pct(val) {
        if (val < 0 || val > 50) {
            throw new Error("Discount percentage must be between 0% and 50%.");
        }
    }

    onInsert(rec) {
        rec.testField("customer_id");
        if (!rec.status) {
            rec.status = "Draft";
        }
        ctx.ui.info("Sales Order initialized in Draft state", "Order Created");
    }

    onModify(rec, xRec) {
        // Prevent modifying financial fields once Released
        if (xRec.status === "Released" && rec.status === "Released") {
            if (rec.customer_id !== xRec.customer_id || rec.total_amount !== xRec.total_amount) {
                throw new Error("Reopen the Sales Order before modifying customer or totals.");
            }
        }

        // Credit limit check when order total increases
        if (rec.total_amount > xRec.total_amount) {
            const cust = models.Customer.get(rec.customer_id);
            if (cust) {
                cust.calcFields("balance_lcy");
                const projected = ctx.decimal(cust.balance_lcy || 0).add(rec.total_amount).toNumber();
                if (projected > cust.credit_limit) {
                    ctx.ui.warning(
                        "Customer projected balance ($" + projected + ") exceeds credit limit ($" + cust.credit_limit + ").",
                        { title: "Credit Limit Warning", duration: 8000 }
                    );
                }
            }
        }
    }

    onDelete(rec) {
        rec.testField("status", "Draft");
    }
}
```

---

## 4. The `codeunit` Meta-Object & Decorators

A `codeunit` encapsulates reusable domain procedures, batch jobs, event publishers/subscribers, and ledger posting routines.

### 4.1. Codeunit Method Decorators

| Decorator | Target | Behavior |
| :--- | :--- | :--- |
| `@businessEvent` | `procedure` | Declares a public domain event contract emitted to subscribers via `EventRegistry` / `ctx.events.publish`. |
| `@integrationEvent(includeSender, globalVarAccess)` | `procedure` | Declares an external integration / webhook event contract. |
| `@subscribeEvent(PublisherObject, "eventName")` | `procedure` | Registers the procedure in `EventRegistry` to run automatically whenever `PublisherObject` dispatches `"eventName"`. |
| `@pushdown` | `codeunit` or `procedure` | **BOS-AMDP**: Compiles the procedure's AST actions directly into a native PostgreSQL `PL/pgSQL` function (`bos_proc_<codeunit>_<method>(p_params JSONB, p_ctx JSONB)`) and executes it inside PostgreSQL with zero QuickJS overhead. |
| `@transactional` | `procedure` | Marks the procedure as requiring atomic transaction boundaries. |
| `@test` | `procedure` | Marks the procedure as an automated integration test executed by `DataEngine::run_tests` inside an **auto-rollback transaction** (`tx.rollback()`). |

### 4.2. Real-World Use Case: Two-Phase Sales Order Posting Codeunit + Event Subscriber

```bos
codeunit SalesPostService {
    @transactional
    procedure postSalesOrder(rec: SalesHeader): boolean {
        // Phase 1: Pre-validation
        rec.testField("status", "Released");
        rec.testField("customer_id");

        const lines = models.SalesLine.findMany({
            where: { document_id: rec.id, quantity: { gt: 0 } },
            orderBy: { line_no: "asc" }
        });

        if (lines.length === 0) {
            throw new Error("Cannot post Sales Order " + rec.order_no + ": nothing to post.");
        }

        // Compute exact fixed-point total using 128-bit BosDecimal
        let total = ctx.decimal("0.00");
        for (const line of lines) {
            const lineAmt = ctx.decimal(line.quantity).mul(line.unit_price).round(2);
            total = total.add(lineAmt);
        }

        // Phase 2: Create Immutable Customer Ledger Entry
        const ledgerEntry = models.CustomerLedgerEntry.create({
            customer_id: rec.customer_id,
            posting_date: rec.posting_date,
            document_no: rec.order_no,
            amount: total.toNumber(),
            remaining_amount: total.toNumber(),
            is_open: true
        });

        // Update document status
        models.SalesHeader.update(rec.id, {
            status: "Posted",
            total_amount: total.toNumber()
        });

        // Publish domain event to subscribers
        ctx.events.publish("SalesPostService", "onAfterPostSalesOrder", {
            order_id: rec.id,
            ledger_entry_id: ledgerEntry.id,
            posted_amount: total.toNumber()
        });

        ctx.audit.log("Posted Sales Order " + rec.order_no + " for amount " + total.toString());
        ctx.ui.success("Sales Order " + rec.order_no + " posted to ledger.", "Posting Complete");
        return true;
    }
}

codeunit LoyaltyPointsSubscriber {
    @subscribeEvent(SalesPostService, "onAfterPostSalesOrder")
    procedure awardPointsOnPost(evt: any): void {
        const points = Math.floor((evt.posted_amount || 0) / 10);
        if (points > 0) {
            ctx.audit.log("Awarded " + points + " loyalty points for order " + evt.order_id);
        }
    }
}
```

---

## 5. Polymorphic `interface` & Strategy `enum`

BOS-Script supports the **Dynamics BC Extensible Enum Strategy Pattern**: an `interface` defines method signatures, Codeunits implement the interface, and an `enum` binds each enum member to a concrete `codeunit`:

```bos
interface IShippingRateProvider {
    procedure calculateFreight(weightKg: Decimal, countryCode: String): Decimal;
}

enum ShippingCarrier implements IShippingRateProvider {
    extensible: true;
    FedEx = FedExRateCodeunit;
    DHL = DhlRateCodeunit;
}
```
