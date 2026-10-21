# BOS Script: World-Class Business Logic Framework & Developer Productivity Engine

> **Status:** Brainstorming & Architecture Blueprint  
> **Document:** `docs/01_bos_script_framework_deep_dive.md`  
> **Target Audience:** Core Engine Architects, ERP Developers, Enterprise Architects  

---

## 1. Vision: Why a Dedicated Enterprise Framework?

Traditional business application development is plagued by two extremes:
1. **Legacy ERP Languages (AL/CAL, ABAP, Apex):** Built-in enterprise concepts (Ledgers, FlowFields, Validation triggers), but archaic syntax, cumbersome tooling, slow compile cycles, and closed ecosystems.
2. **Modern General-Purpose Frameworks (Node/TypeScript + Prisma/TypeORM/Next.js):** Modern syntax and vast libraries, but **zero enterprise semantics**. Developers waste 80% of their time writing repetitive boilerplate: multi-tenant isolation, audit trails, optimistic locking, ledger posting routines, permission checks, dimension accounting, and safe schema migrations.

**BOS Script unifies the best of both worlds:**
A modern, type-safe TypeScript extension engineered specifically for high-velocity enterprise software, backed by a sub-second Rust compiler, compiling to native PostgreSQL, sandboxed backend WASM, and reactive Svelte 5 UI.

---

## 2. Core Pillars of the BOS Script Framework

```
┌────────────────────────────────────────────────────────────────────────┐
│                        BOS Script Framework                            │
├───────────────────────┬────────────────────────┬───────────────────────┤
│   1. Data & Records   │  2. Business Logic     │  3. Enterprise Core   │
│  - Active Record &    │  - Two-Phase Triggers  │  - Immutable Ledgers  │
│    Snapshot (Rec/xRec)│  - Event Interceptors  │  - Posting Engine     │
│  - Field Validation   │  - Finite State Mach.  │  - FlowFields (Aggs)  │
│  - Inline Query Expr. │  - Temporal History    │  - Number Series (BOS) │
├───────────────────────┴────────────────────────┴───────────────────────┤
│                4. Developer Experience & Productivity                  │
│  - Zero-Boilerplate Metadata       - Integrated Unit Testing & Mocks   │
│  - Compile-Time Safety Guarantee   - Full LSP Autocompletion & Hints   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Deep Dive: Accessing & Manipulating Meta-Objects

### 3.1 The Typed Record Model (`Rec` and `xRec`)

In business applications, knowing what changed before an update is critical for validation and audit logs. BOS Script introduces first-class record snapshots:

```typescript
table SalesHeader {
    id: UUID = auto();
    orderNo: String(30) { notNull = true; unique = true; };
    customerId: UUID { notNull = true; relation = Customer.id; };
    status: Enum("Draft", "Released", "Invoiced", "Cancelled") = "Draft";
    totalAmount: Decimal(18, 2) = 0.00;

    // Field-level trigger with old vs new comparison
    status.onChange(oldStatus, newStatus) {
        if (oldStatus == "Released" && newStatus == "Draft") {
            // Ensure no warehouse activity has commenced
            WarehouseActivity.verifyNoOpenPicks(this.id);
        }
    }

    onBeforeModify() {
        // `this` represents the incoming state (Rec)
        // `this.xRec` represents the unmodified database state (xRec)
        if (this.xRec.status == "Invoiced") {
            error("Cannot modify an already invoiced sales order.");
        }
    }
}
```

### 3.2 Validation Triggers: `set` vs `setAndValidate`

In standard ORMs, setting `order.total = 500` bypasses business rules. In BOS Script, validation logic is first-class:

```typescript
codeunit SalesOrderService {
    public static changeCustomer(orderId: UUID, newCustomerId: UUID) {
        const order = SalesHeader.get(orderId);

        // Standard assignment (no trigger run)
        order.notes = "Customer updated via portal";

        // Assignment WITH full validation cascade
        // This triggers Customer credit checks, updates currency, payment terms, and addresses
        order.customerId.validate(newCustomerId);

        order.save(); // Atomically checks optimistic lock and saves
    }
}
```

### 3.3 Declarative FlowFields (Virtual Reactive Calculations)

FlowFields eliminate manual aggregation queries (`SELECT SUM(...)`). The compiler lowers them into optimized SQL subqueries or joins automatically:

```typescript
table Customer {
    id: UUID = auto();
    name: String(100);

    // FlowField: Virtual computed balance from CustomerLedgerEntry
    balance: Decimal(18, 2) {
        computed = sum(CustomerLedgerEntry.amount where {
            customerId == this.id && open == true
        });
        editable = false;
    };

    // FlowField with Date Filtering (FlowFilters)
    netSalesPeriod: Decimal(18, 2) {
        computed = sum(SalesInvoiceLine.lineAmount where {
            customerId == this.id &&
            postingDate >= context.filterDateStart &&
            postingDate <= context.filterDateEnd
        });
        editable = false;
    };
}

// In Business Logic:
const cust = Customer.get(custId);
// Zero SQL query executed for balance yet! Lazy or on-demand:
cust.calcFields("balance", "netSalesPeriod");
console.log(`Open Balance: ${cust.balance}`);
```

### 3.4 First-Class Inline Query Syntax

Rather than constructing fragile string-based SQL or complex query builder chains, BOS Script features compile-time type-checked query expressions:

```typescript
codeunit InventoryAlertService {
    public static getCriticalLowStockItems(warehouseId: UUID): Array<StockSummary> {
        // Compile-time verified against metadata schema
        const results = query ItemLedgerEntry
            where {
                locationId == warehouseId &&
                remainingQuantity > 0
            }
            groupBy { itemId, locationId }
            aggregate {
                totalStock = sum(remainingQuantity),
                earliestExpiry = min(expirationDate)
            }
            having {
                totalStock < 10
            }
            orderBy { totalStock asc }
            limit 100;

        return results;
    }
}
```
**Compiler Magic:** The Rust compiler checks all field names, types, and aggregation validity at compile time. It generates a single high-performance parameterized PostgreSQL query with no runtime ORM translation overhead.

---

## 4. Rich Framework Business Primitives

### 4.1 Sequential Number Series (Built in BOS)

Enterprise applications require gapless or sequential document numbers (e.g., `SO-2026-0001`, `INV-2026-0492`). Built directly in BOS as an application module (`bos.core.number_series`), number series management uses standard BOS tables and codeunits rather than hardcoded Rust kernel or `ctx` primitives:

```typescript
table SalesHeader {
    no: String(30) {
        caption = "Order No.";
        onInsert() {
            if (!this.no) {
                // Invokes the standard BOS application codeunit (bos.core.number_series)
                this.no = NumberSeriesManagement.getNext({
                    seriesCode: "SALES_ORDER",
                    date: this.orderDate,
                    allowManual: false
                });
            }
        }
    };
}
```

### 4.2 Posting Engine & Immutable Ledger Architecture

A fatal flaw of modern CRUD apps is directly modifying historical balances. BOS Script enforces the **ERP Posting Engine Pattern**:
- **Documents (Mutable):** `SalesOrder`, `PurchaseOrder`, `JournalLine`.
- **Ledgers (Strictly Immutable):** `GeneralLedgerEntry`, `CustomerLedgerEntry`, `ItemLedgerEntry`.

```typescript
interface LedgerEntry {
    entryNo: Integer = autoIncrement();
    postingDate: Date;
    documentNo: String(30);
    amount: Decimal(18, 2);
    // Ledger entries NEVER have onDelete or onModify triggers; they are write-once!
}

table CustomerLedgerEntry implements LedgerEntry {
    immutable = true; // Compiler strictly forbids UPDATE or DELETE statements!
    
    customerId: UUID { relation = Customer.id; };
    open: Boolean = true; // Only specific reconciliation flags can be mutated via system code
    remainingAmount: Decimal(18, 2);
    
    // Closed by subsequent compensating entry or payment reconciliation
}
```

### 4.3 Declarative Finite State Machine (FSM) Workflow

State transitions are often scattered across dozens of controllers. In BOS Script, workflows are declared natively in metadata:

```typescript
table PurchaseOrder {
    id: UUID = auto();
    status: Enum("Draft", "Submitted", "Approved", "Rejected", "Ordered", "Received") = "Draft";

    stateMachine status {
        transition Submit {
            from: "Draft";
            to: "Submitted";
            guard: (doc) => doc.lines.length > 0 && doc.totalAmount > 0;
            onTransition: (doc) => ApprovalWorkflow.initiate(doc);
        };

        transition Approve {
            from: "Submitted";
            to: "Approved";
            roles: ["PurchasingManager", "CFO"];
            guard: (doc) => doc.totalAmount <= context.user.approvalLimit;
        };

        transition Reject {
            from: "Submitted";
            to: "Rejected";
            roles: ["PurchasingManager", "CFO"];
            params: { reason: String(250) };
            onTransition: (doc, params) => Notification.send(doc.createdBy, params.reason);
        };
    }
}
```
**Productivity Impact:**
- Automatically generates UI action buttons (`Submit`, `Approve`, `Reject`) on pages.
- Automatically disables actions if guards or user roles fail.
- Emits transition history to audit logs without writing extra code.

### 4.4 Non-Invasive Event Subscribers & Interceptors

Extensions customize logic without modifying original source code:

```typescript
// Third-party Tax App Extension:
codeunit IndianGstTaxExtension {
    [EventSubscriber(SalesHeader, "onBeforePost", priority = 100)]
    public static validateGstRules(sender: SalesHeader, args: CancelEventArgs) {
        if (!sender.gstIdentificationNo && sender.totalAmount > 50_000) {
            args.cancel("GSTIN is required for orders exceeding 50,000 INR.");
        }
    }

    [EventSubscriber(SalesHeader, "onAfterPost")]
    public static postEWayBill(sender: SalesHeader, invoiceNo: String) {
        EWayBillQueue.schedule(invoiceNo);
    }
}
```

---

## 5. What the compiler and runtime actually shorten

| You write | What runs |
| :--- | :--- |
| `table` plus keys and FlowFields | PostgreSQL table, indexes, and SIFT `INCLUDE` indexes. FlowFields are SQL subqueries. |
| `formula = quantity * unitPrice` and `check = quantity >= 0` | A stored generated column and a check constraint. |
| `page CustomerCard` | Metadata in the catalog. The Studio does not generate a Svelte page from it. |
| `statemachine` | Guards and transition scripts in QuickJS. The page runtime does not build the buttons. |
| Tenant columns | Shared and partitioned tables get a tenant RLS policy. Scripts still have to use the model API. |

There is no `audit = true` switch, no `TestData` fixture helper, and no generated Svelte CRUD page.

## 6. Edit, compile, apply

1. The Studio editor checks script syntax through the compile API. Diagnostics come from the Rust compiler (Oxc for script syntax, plus the analyzer). There is no language server process.
2. Apply lowers the file to `ModelIR`, writes the catalog, and emits DDL.
3. Triggers and codeunits run in QuickJS. They are not compiled to Wasm or to Svelte.
