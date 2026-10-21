# BOS Script: Must-Have Features & Meta-Object Deep Dive

> **Status:** Architecture Blueprint & Meta-Object Catalog  
> **Document:** `docs/01_bos_script_must_have_features_and_meta_objects.md`  
> **Target:** Research catalog. The matrix below is the feature list we compared with other products. It is not a claim that every row runs.

**Running in the compiler and runtime today**

- Oxc syntax diagnostics (`BOS1001`) and a line-preserving TypeScript eraser.
- Optimizer: decimal field arithmetic, and `Model.get(row.field)` hoisted out of `for...of` into `getMany`.
- `formula` / `check` expressions, `copyFrom`, `series`, and `when` on triggers.
- `get` / `getMany` / `createMany`, object or list `select`, dirty `modify`, `require`, a warm QuickJS pool, and procedure return values through `ctx.call`.
- PostgreSQL tier for a codeunit method whose whole body translates. `BOS3301` when `@pushdown` cannot.
- SIFT indexes on the FlowField equality columns. `BOS3101` when a measured FlowField has none.
- Monaco shadow TypeScript, `ambient.d.ts`, and `monacoBosScript.js`.

**Not in this repository**

- `bosc-lsp`, a VS Code extension, a source-mapped debugger, or a profiler.
- Compiling pages to Svelte, or compiling codeunits to Wasm.
- A persisted QuickJS bytecode cache. The warm runtime pool is the cache.
- Translating arbitrary TypeScript into PL/pgSQL. Only a fully recognized body moves.
- Path reads such as `rec.customer.name` as a join.
- `ctx.postLedger` and `ctx.dimension`. Those belong in application packages.

---

## 1. Deep Research: World-Class Enterprise Feature Matrix

Following an exhaustive comparative analysis of the world's most successful enterprise platforms (Microsoft Dynamics 365 Business Central AL, SAP Cloud Application Programming Model / CDS, Salesforce Apex / Custom Metadata, and Frappe/Odoo), the following capabilities represent the **non-negotiable, must-have features** of BOS Script:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                   BOS Script Must-Have Enterprise Capabilities                         │
├──────────────────────────────┬─────────────────────────────┬───────────────────────────┤
│ 1. Data Integrity & Ledgers  │ 2. Extensibility & Metadata │ 3. Lifecycle & UX         │
│  - Immutable Ledgers         │  - Non-invasive Extensions  │  - First-Class Drafts     │
│  - Two-Phase Posting Pattern │  - Extensible Enums         │  - FlowFields & Filters   │
│  - Number Series (App Module)│  - Event Interceptors       │  - Finite State Machines  │
│  - Dimension Accounting      │  - Custom Metadata Types    │  - Rec vs xRec Snapshots  │
├──────────────────────────────┼─────────────────────────────┼───────────────────────────┤
│ 4. Transaction & Concurrency │ 5. Security & Isolation     │ 6. Developer Velocity     │
│  - Unit of Work (Topology)   │  - Row-Level Security (RLS) │  - Compile-Time SQL Valid.│
│  - Optimistic Concurrency    │  - Field-Level Masking      │  - Built-In Testing DSL   │
│  - Transactional Outbox      │  - Contextual Elevation     │  - Zero-Boilerplate CRUD  │
└──────────────────────────────┴─────────────────────────────┴───────────────────────────┘
```

### Detailed Breakdown of Must-Have Capabilities:

1. **Lean Draft Handling via Schema Attributes (BOS Standard):**
   - High-volume documents (Sales Orders, Invoices, Work Orders) handle drafts cleanly using userland status and draft columns (e.g. `is_draft: Boolean = true`, `status: OrderStatus = Draft`) within the `.bos` table schema, avoiding redundant shadow-table duplication.
2. **Declarative FlowFields & FlowFilters (Inspired by Dynamics Business Central):**
   - Calculated fields (Sum, Count, Average, Min, Max, Exist) defined declaratively. Zero database column storage.
   - Evaluated lazily or in bulk (`calcFields("balance", "netSales")`), translated by the Rust compiler into high-performance SQL subqueries.
   - **FlowFilters:** Filter FlowFields dynamically by date range, warehouse, or currency without modifying table records.
3. **Record Snapshots: `Rec` and `xRec`:**
   - Every modification lifecycle provides access to `this` (incoming state) and `this.xRec` (unmodified database state) for granular delta calculations and audit trails.
4. **Validation Cascading (`validate(field, value)`):**
   - Differentiates between raw attribute assignment (`record.field = val`) and business-rule validated assignment (`record.field.validate(val)`).
5. **Two-Phase Posting & Immutable Ledgers:**
   - Built-in framework support for double-entry ledger postings. Tables marked `immutable = true` can never be edited or deleted—only reversed or appended to via compensating entries (see `docs/17_gap_02_immutable_ledger_architecture_decision.md` for architectural decision options).
6. **Sequential Number Series (Built in BOS Application Module):**
   - Document numbering (e.g., `SO-2026-00001`, `INV-2026-00001`) is modeled and implemented natively within BOS application packages (`bos.core.number_series`) using standard BOS tables, optimistic locks, and Codeunit posting routines, completely decoupled from the Rust kernel and `ctx` runtime.
7. **Analytical Dimensions (Financial & Operations Tagging):**
   - Implemented natively in BOS application packages via normalized dimension set tables (`DimensionValue`, `DimensionSetEntry`) and dimension set ID foreign keys on document headers/lines, keeping kernel complexity zero.
8. **Native Query Pagination, Sorting & Wildcard Filtering:**
   - Production-grade query engine in `bosc-runtime` supporting `limit` (clamped 1-1000), `offset`, `page`, multi-column `sort`, and case-insensitive wildcard ILIKE/JSON filters with RFC HTTP headers (`x-total-count`, `x-page`, `x-limit`, `x-has-more`).
9. **Transactional Event Outbox Pattern (`_bos_event_outbox`):**
   - Guaranteed at-least-once asynchronous domain event delivery. Events are atomically enqueued in the same database transaction as business mutations and drained by background workers using PostgreSQL `SELECT ... FOR UPDATE SKIP LOCKED`.
10. **Extensible Enums & Polymorphic Codeunits:**
    - Extensions can add enum values (`enumextension`) and bind them to specific codeunit implementations (Strategy Pattern).

---

## 2. Exhaustive Catalog of BOS Meta-Objects

BOS Script organizes enterprise applications into **14 First-Class Meta-Objects**:

```
                         ┌───────────────────────────────┐
                         │      BOS Meta-Objects         │
                         └───────────────┬───────────────┘
                                         │
        ┌───────────────────┬────────────┴───────┬───────────────────┐
        ▼                   ▼                    ▼                   ▼
 1. Schema & Data    2. User Interface    3. Business Logic   4. Security & Ops
 ─────────────────   ─────────────────    ─────────────────   ─────────────────
 • table             • page               • tablelogic        • permissionset
 • tableextension    • pageextension      • tablelogicextension menu
 • view              • report             • codeunit          • custommetadata
 • query                                  • statemachine
                                          • interface
                                          • enum / enumext
```

---

## 3. Deep Dive into Each Meta-Object

### Meta-Object 1: `table` (Relational Entity & Data Kernel)
The core persistence definition. Represents an enterprise entity, its constraints, relations, and flowfields. Lifecycle scripts belong in `tablelogic`.

**Key Powers & Capabilities:**
- **Storage Profiles:** Standard, `draft = true` (draft-enabled), `immutable = true` (write-once ledger), `temporal = true` (system-versioned time travel), `audit = true`.
- **Field Types:** Strict enterprise primitives (`UUID`, `String(N)`, `Decimal(P, S)`, `Integer`, `Date`, `DateTime`, `Enum`, `Json`, `Blob`, `Dimensions`).
- **Keys:** Single and composite primary and secondary keys with sorting order and unique constraints.
- **Relations:** Strongly typed foreign keys with validation rules, cascading options (`Cascade`, `Restrict`, `SetNull`), and lookup filters.
- **FlowFields:** Declarative virtual aggregations (`sum`, `count`, `avg`, `min`, `max`, `exist`).
- **Triggers:** Written on `tablelogic` (`*.bl.bos`): `onInsert`, `onModify`, `onDelete`, `onRename`, `before*` / `after*`, and draft hooks.

```typescript
table SalesLine {
    id: UUID = auto();
    documentId: UUID {
        notNull = true;
        relation = SalesHeader.id { onDelete = Cascade; };
    };
    lineNo: Integer { notNull = true; };
    itemNo: String(20) {
        notNull = true;
        relation = Item.no;
    };
    description: String(100);
    quantity: Decimal(18, 4) {
        default = 1.0;
    };
    unitPrice: Decimal(18, 2);
    discountPercent: Decimal(5, 2) = 0.00;
    lineAmount: Decimal(18, 2) { editable = false; };
    dimensions: Dimensions; // Global dimension set (Cost Center, Dept)

    primaryKey(documentId, lineNo);
    key(itemNo);
}

tablelogic SalesLineLogic for SalesLine {
    validate itemNo(newItemNo) {
        const item = Item.get(newItemNo);
        item.testField("blocked", false);
        this.description = item.description;
        this.unitPrice = item.unitPrice;
        this.uom = item.baseUOM;
        this.unitPrice.validate(this.unitPrice);
    }

    validate quantity(qty) {
        if (qty < 0) error("Quantity cannot be negative");
        this.lineAmount = this.unitPrice * qty * (1 - this.discountPercent / 100);
    }

    validate unitPrice(price) {
        this.lineAmount = price * this.quantity * (1 - this.discountPercent / 100);
    }
}
```

---

### Meta-Object 2: `tableextension` (Non-Invasive Data Augmentation)
Allows developers and ISVs to extend upstream tables without touching the base codebase.

**Key Powers & Capabilities:**
- Add new fields and indexes. Executable checks for those fields go in `tablelogicextension`.
- Modify field captions and default lookup filters.

```typescript
tableextension SalesLineMedicalExt extends SalesLine {
    batchNumber: String(50) {
        caption = "Lot / Batch No.";
        notNull = true;
    };
    expiryDate: Date {
        caption = "Expiry Date";
    };

    key(itemNo, batchNumber);
}

tablelogicextension SalesLineMedicalLogic for SalesLine {
    validate expiryDate(expDate) {
        if (expDate <= context.today) error("Expired batches cannot be sold.");
    }

    onAfterInsert(rec, ctx) {
        LotTrackingEngine.registerLine(rec.documentId, rec.batchNumber);
    }
}
```

---

### Meta-Object 2b: `tablelogic` (`User.bl.bos`) (Entity Business Logic Separation)
Provides clean separation of declarative data schemas (`User.bos`) from imperative business logic, field validations, lifecycle triggers, and domain methods.

**Key Powers & Capabilities:**
- **Target Binding:** Binds directly to an existing table: `tablelogic UserLogic for User { ... }`.
- **Field Validation Triggers:** Declarative `validate <fieldName>(val [, oldVal]) { ... }` blocks executed prior to database writes.
- **Entity Lifecycle Triggers:** `beforeInsert`, `afterInsert`, `beforeModify`, `afterModify`, `beforeDelete`, `afterDelete`.
- **Custom Entity Methods:** Helper functions and procedural checks invoked directly on the table context.
- **Zero Schema Drift:** Compiles purely to runtime trigger routines without modifying relational physical tables.

```typescript
tablelogic UserLogic for User {
    validate email(val) {
        if (!val.includes("@") || !val.includes(".")) {
            throw new Error("Invalid email format");
        }
    }

    beforeInsert(rec, ctx) {
        rec.createdAt = new Date().toISOString();
        if (rec.password) {
            const salt = ctx.crypto.randomBytes(8).toHex();
            rec.passwordHash = `$sha256$${salt}$${ctx.crypto.sha256(salt + rec.password)}`;
            delete rec.password;
        }
    }

    isSuperAdmin(rec) {
        return rec.role === "Admin" && rec.isActive === true;
    }
}
```

---

### Meta-Object 2c: `tablelogicextension` (`User.ext.bl.bos`) (Non-Invasive Logic Augmentation)
Enables third-party packages and modular add-ons to non-invasively attach validations and lifecycle hooks to base or upstream tables without touching base code.

**Key Powers & Capabilities:**
- **Target Binding:** Augments existing tables: `tablelogicextension UserAuditLogic for User { ... }`.
- **Pre- and Post-Hook Augmentation:** Intercepts modifications or inserts (e.g. `afterModify(rec, oldRec, ctx)` to log security audits).
- **Additive Validations:** Adds custom domain checks to existing fields.

```typescript
tablelogicextension UserAuditLogic for User {
    afterModify(rec, oldRec, ctx) {
        if (rec.role !== oldRec.role) {
            ctx.logAudit("ROLE_CHANGE", rec.id, { from: oldRec.role, to: rec.role });
        }
    }
}
```

---

### Meta-Object 3: `page` (Reactive User Interface Contract)
Defines one screen: a single `source`, one `layout`, and only the field and command overrides the table does not already store. The full freeze is [`docs/28_page_meta_object_design.md`](28_page_meta_object_design.md).

**Key Powers & Capabilities:**
- **One source.** `source: Customer`, `source: view.CustomerBalance`, or `source: query.SalesByCustomer`. The kind is resolved into that one value. There is no `sourceTable`, `sourceView`, or `sourceQuery`.
- **Eight layouts.** `list`, `record`, `home`, `steps`, `board`, `split`, `api`, and `free`. A document is a record with an inline line part. A journal is a list with `editing: cell`. A wizard is `steps`.
- **Delta storage.** The page stores the field name plus exceptions (caption, locked, badge). Type and default caption stay on the table.
- **One command list.** `primary: true` pins the important button. The command calls a codeunit or a state-machine transition.

```bos
page SalesOrder {
    source: SalesHeader;
    layout: record;
    caption: "Sales Order";

    main {
        section(General) {
            field(orderNo) { editable: false; }
            field(customerId) { caption: "Customer"; }
            field(orderDate);
            field(status) { style: badge; }
        }
        part(Lines) {
            source: SalesLine;
            link: documentId = id;
            field(itemId);
            field(quantity);
            field(amount) { editable: false; }
        }
    }

    side {
        part(Customer; CustomerFact);
    }

    commands {
        command(Post) {
            caption: "Post";
            primary: true;
            visible: status == "Released";
            run: SalesPost.post;
        }
    }
}
```

---

### Meta-Object 4: `pageextension` (Non-Invasive UI Augmentation)
Adds, moves, or hides fields and commands on a page that already exists. Anchors are `addFirst`, `addLast`, `addBefore`, `addAfter`, `moveBefore`, `moveAfter`, `modify`, and `hide`. The base page file stays unchanged. See [`docs/28_page_meta_object_design.md`](28_page_meta_object_design.md).

---

### Meta-Object 5: `view` (Relational Projections & Read Models)
Virtual or materialized relational projections optimized for high-performance reading and analytical dashboards.

**Key Powers & Capabilities:**
- Compiles directly to PostgreSQL `CREATE VIEW` or `CREATE MATERIALIZED VIEW`.
- Supports automated concurrent refresh strategies.
- Enforces Row-Level Security through underlying table policies.

---

### Meta-Object 6: `query` (Compiled Analytical Data Sets)
Strongly typed multi-table analytical query pipelines with compile-time type-safety.

**Key Powers & Capabilities:**
- Inner, Left Outer, Full Outer, and Cross joins across tables.
- Server-side grouping, aggregations, and window functions.
- Direct output mapping to JSON, CSV, or typed TypeScript arrays.

---

### Meta-Object 7: `statemachine` (Declarative Business Workflows)
First-class state transition engine governing document lifecycles and approvals.

**Key Powers & Capabilities:**
- Declarative transition matrix (`from`, `to`, `guard`, `roles`, `onTransition`).
- Parameterized transitions (e.g. requiring a reason when rejecting).
- Auto-generates UI transition buttons, status badges, and audit history.

---

### Meta-Object 8: `codeunit` (Encapsulated Business Logic & Services)
Encapsulated procedural logic, posting engines, background workers, and API endpoints.

**Key Powers & Capabilities:**
- Compiled directly to sandboxed WASM bytecode for sub-millisecond execution.
- Transactional context: automatic commit on success, automatic rollback on unhandled error.
- Background worker execution: can run synchronously or scheduled via task queues.
- Web service exposure: `@exposed` attribute automatically generates OpenAPI and typed client RPC.

---

### Meta-Object 9: `permissionset` (Declarative Role & Attribute-Based Security)
Defines fine-grained data and execution access control.

**Key Powers & Capabilities:**
- Table-level permissions: Read, Insert, Modify, Delete (`RIMD`).
- Field-level masking and read-only protections.
- Row-Level Security (RLS) filter expressions compiled into native PostgreSQL policies.
- Execute permissions for Codeunits and Reports.

---

### Meta-Object 10: `menu` (Navigation Taxonomy & Command Palettes)
Defines application navigation trees, breadcrumbs, search indexing, and role-tailored workspaces.

---

### Meta-Object 11: `interface` & `enum` / `enumextension` (Polymorphic Extensibility)
Enables clean Strategy Patterns across modular packages.

**Key Powers & Capabilities:**
- Enums can be marked `extensible = true`. Downstream extensions can register new enum values.
- Interfaces define procedure contracts. Codeunits implement interfaces.
- Dynamic polymorphic dispatch based on selected enum value:
  ```typescript
  const handler: IPaymentProcessor = PaymentEngine.getProcessor(order.paymentMethod);
  handler.charge(order);
  ```

---

### Meta-Object 12: `custommetadata` (Configuration-As-Metadata)
Configurable rules, tax matrices, and integration parameters stored as deployable metadata.

**Key Powers & Capabilities:**
- Shipped with packages; version-controlled in Git.
- Resides in memory cache for zero-latency queries.
- Easily mockable in automated tests without database writes.

---

## 4. Meta-Object Freezing Protocol

To build a rock-solid compiler without costly rework, we will review, enhance, and **freeze each meta-object individually**:

| Step | Meta-Object | Focus for Freezing | Status |
| :---: | :--- | :--- | :---: |
| **01** | **`table` & `tableextension`** | Fields, Keys, FlowFields, Triggers, Drafts, Ledgers, Immutability | **Ready for Review** |
| **02** | **`statemachine` & Workflow** | States, Transitions, Guards, Role checks, Audit logging | Pending |
| **03** | **`codeunit`, `interface`, `enum`** | Posting logic, Transactions, WASM FFI, Dependency Injection | Pending |
| **04** | **`query` & `view`** | Aggregations, Joins, FlowFilters, Read models, Materialized Views | **Implemented & Verified** |
| **05** | **`page` & `pageextension`** | One `source`, eight layouts, delta storage, bos-ui renderer | Design revised — see docs/28_page_meta_object_design.md |
| **06** | **`menu` & Navigation** | Workspaces, Role centers, Search indexing, Breadcrumbs | Pending |
| **07** | **`permissionset` & RLS** | CRUD rights, Row security, Field-level masking, Elevation, Fast Guard | **Implemented & Verified** |
| **08** | **`custommetadata` & `report`** | Cached configuration, PDF/Excel document engines | Pending |
