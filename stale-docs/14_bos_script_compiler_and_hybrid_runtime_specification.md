# BOS-Script, Compiler & Hybrid Execution Engine Specification

> **Document:** `docs/14_bos_script_compiler_and_hybrid_runtime_specification.md`  
> **Status:** Agreed Architecture & Production Specification  
> **Target Version:** BOS Core v2.0+  
> **Inspiration & Parity:** Microsoft Dynamics 365 Business Central (AL Engine) & SAP AMDP (ABAP Managed Database Procedures)

---

## 1. Executive Summary

BOS-Script is a high-productivity, enterprise-grade business language that unifies:
1. **Declarative Relational Metadata:** High-level definitions of tables, columns, indexes, SIFT FlowFields, and state machines.
2. **Embedded Pure TypeScript/JavaScript:** Familiar, expressive business logic inside triggers (`onInsert`, `onModify`, `onDelete`, `onValidate`) and codeunits.
3. **The 3-Tier Hybrid Execution Pipeline:**
   - **Tier 1 (BOS-AMDP):** In-database PostgreSQL stored procedures/functions for mass batch processing and FlowField aggregations (0ms network IPC).
   - **Tier 2 (QuickJS Bytecode VM):** Sandboxed, lightweight in-memory JavaScript virtual machine (8 MB ceiling, 512 KB stack) for single-document interactive validations and workflows.
   - **Tier 3 (Rust Core Native Kernel):** Hardware CSPRNG entropy, transactional connection pooling, Tokio async threading, and L1 shared memory caching.

---

## 2. BOS-Script Language Architecture

### 2.1 File Syntax & Grammar

BOS-Script files (`.bos`) combine declarative metadata with embedded procedural blocks:

```typescript
table SalesHeader {
    caption = "Sales Order Header";
    tableType = Normal;
    dataPerCompany = true;

    orderNo: String(20) {
        notNull = true;
        caption = "Order No.";
    };
    customerNo: String(20) {
        notNull = true;
        caption = "Customer No.";
    };
    postingDate: Date {
        caption = "Posting Date";
    };
    totalAmount: Decimal(18, 2) {
        caption = "Total Order Amount";
    };
    status: Enum("Draft", "PendingApproval", "Approved", "Released", "Posted") {
        default = "Draft";
    };

    primaryKey(orderNo);
    key(customerNo, postingDate);

    // ========================================================
    // TIER 2: QUICKJS BYTECODE (Interactive Document Trigger)
    // ========================================================
    onInsert(rec, ctx) {
        require(rec.postingDate, "Posting Date is required.");

        const customer = Customer.get(rec.customerNo);
        if (!customer) {
            throw new Error(`Customer '${rec.customerNo}' does not exist.`);
        }
        if (customer.isBlocked) {
            throw new Error(`Customer '${customer.name}' is currently blocked.`);
        }

        if (!rec.orderNo) {
            rec.orderNo = ctx.call("NumberSeriesManagement", "getNext", "SO");
        }

        ctx.audit.log(`Created Sales Order ${rec.orderNo} for customer ${customer.name}`);
    }

    onModify(rec, xRec, ctx) {
        if (xRec.status === "Posted") {
            throw new Error("Cannot modify a posted sales order.");
        }
    }
}
```

### 2.2 Entity Logic Companion: `tablelogic` and `tablelogicextension`

To keep data definitions strictly structural, field validations, lifecycle triggers, and record methods are written in the companion file (`SalesHeader.bl.bos`) using `tablelogic` and `tablelogicextension`:

```typescript
// SalesHeader.bl.bos
tablelogic SalesHeaderLogic for SalesHeader {
    validate customerNo(val) {
        const customer = Customer.get(val);
        if (!customer) {
            throw new Error(`Customer '${val}' does not exist.`);
        }
        if (customer.isBlocked) {
            throw new Error(`Customer '${customer.name}' is currently blocked.`);
        }
    }

    validate postingDate(val, oldVal) {
        if (val < ctx.user.allowPostingFrom || val > ctx.user.allowPostingTo) {
            throw new Error(`Posting Date ${val} is outside your permitted posting range.`);
        }
    }

    beforeInsert(rec, ctx) {
        rec.postingDate = rec.postingDate || ctx.user.workDate;
        if (!rec.orderNo) {
            rec.orderNo = NumberSeriesManagement.getNext("SO");
        }
    }
}

// SalesHeader.ext.bl.bos (Modular Extension)
tablelogicextension SalesHeaderAuditExt for SalesHeader {
    afterModify(rec, oldRec, ctx) {
        if (rec.status !== oldRec.status) {
            ctx.audit.log(`Order ${rec.orderNo} status transitioned from ${oldRec.status} to ${rec.status}`);
        }
    }
}
```

When compiled, the semantic analyzer (`bosc-analyzer`) seamlessly synthesizes these field validations and lifecycle hooks into the target entity's trigger pipeline, executing identically across the QuickJS virtual machine and AMDP tiers.

---

## 3. The 3-Tier Hybrid Execution Pipeline

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                                    BOS-SCRIPT SOURCE                                   │
└───────────────────────────────────────────┬────────────────────────────────────────────┘
                                            │
                                            ▼
                       ┌─────────────────────────────────────────┐
                       │      BOS Semantic Compiler (Rust)       │
                       │   Static Analysis & Target Inference    │
                       └────────────────────┬────────────────────┘
                                            │
            ┌───────────────────────────────┼───────────────────────────────┐
            │ Target A                      │ Target B                      │ Target C
            ▼                               ▼                               ▼
┌───────────────────────┐       ┌───────────────────────┐       ┌───────────────────────┐
│     Tier 1: AMDP      │       │    Tier 2: QuickJS    │       │     Tier 3: Rust      │
│  PostgreSQL Pushdown  │       │   Bytecode Engine     │       │   Native Kernel FFI   │
├───────────────────────┤       ├───────────────────────┤       ├───────────────────────┤
│ • Batch Ledger Posts  │       │ • Document Triggers   │       │ • CSPRNG & Crypto     │
│ • SIFT FlowFields     │       │ • Interactive Rules   │       │ • SQL Transaction Pool│
│ • Bulk Price Updates  │       │ • Dynamic Prefixes    │       │ • OS Sockets & Tokio  │
│ • Stock Revaluations  │       │ • Approval Workflows  │       │ • L1 Shared Memory    │
│ ⚡ In-DB (0ms IPC)    │       │ ⚡ Sub-0.1ms Bytecode │       │ ⚡ Machine Code (SIMD)│
└───────────────────────┘       └───────────────────────┘       └───────────────────────┘
```

### 3.1 Tier 1: In-Database AMDP Pushdown (PL/pgSQL)
For high-volume transaction loops (e.g. posting 5,000 order lines to General Ledger entries), transferring records across network sockets or between Rust and JavaScript creates unnecessary overhead.
* Marked with `@pushdown` or `@amdp`:
  ```typescript
  codeunit LedgerPostingEngine {
      @pushdown
      public postBatch(batchId: String, ctx: BosSessionContext): Integer {
          // Compiled directly into a PostgreSQL PL/pgSQL function:
          // bos_proc_ledgerpostingengine_postbatch($1, $2)
      }
  }
  ```
* Runs 100% inside PostgreSQL's execution engine with zero network round-trips.
* **Automatic DDL Deployment:** `PostgresGenerator::generate_plpgsql_procedure` compiles pushdown methods into native `CREATE OR REPLACE FUNCTION "bos_proc_<codeunit>_<method>"(p_params JSONB, p_ctx JSONB)` routines automatically deployed during package and script compilation (`POST /api/script/apply`).

### 3.2 Tier 2: Sandboxed QuickJS Virtual Machine
For single-document validations, master entity assertions, approval routing, and user calculations:
* **Memory Sandbox:** Hard limit of **8 MB RAM** per execution context.
* **Stack Guard:** Hard limit of **512 KB** call stack to prevent recursion overflows.
* **Fuel Governor:** Evaluates maximum instruction quotas (e.g. 100,000 steps).
* **Worker thread:** `execute_trigger_async` runs QuickJS with `tokio::task::spawn_blocking`. The engine is created on that worker and checks a runtime out of a thread-local pool of four. Drop returns the runtime to the pool. QuickJS bytecode is not written to disk.
* **Compiler, not a second language service:** Oxc reports syntax errors (`BOS1001`). The line-preserving eraser produces the JavaScript that runs. Decimal field math and simple `for...of` + `Model.get(row.field)` calls are rewritten before execution. A codeunit method runs in PostgreSQL when every line translates. A query, a loop, or any other unrecognized line stays on QuickJS. `@pushdown` on a body that cannot be translated completely is error `BOS3301`.

### 3.3 Tier 3: Rust Native Kernel
Low-level operations that require raw OS access:
* Cryptographic accelerators (`sha256`, `hmac`, `uuidV4`, `uuidV7`).
* Transaction lifetime management (`sqlx::Transaction`).
* Outbound network egress and rate-limited HTTP dispatch.

---

## 4. First-Class Data Access Model API

Within Tier 2 scripts, every table defined in the catalog is exposed as a first-class global symbol (`Customer`, `SalesHeader`, `Item`, `Vendor`, etc.) or via dynamic `ctx.db[table]`:

```typescript
// 1. Point Lookup
const cust = Customer.get("CUST001");
const custStrict = Customer.getOrThrow("CUST001");

// 2. Querying with Filter DSL
const activeCustomers = Customer.findMany({
    where: { status: "Active", creditLimit: { gt: 50000 } },
    orderBy: { balance: "desc" },
    take: 20
});

// 3. Existence & Counting
const hasUnposted = SalesHeader.exists({ status: "Draft" });
const pendingCount = SalesHeader.count({ status: "PendingApproval" });

// 4. Persistence & Mutations
const newCust = Customer.create({
    customerNo: "CUST002",
    name: "Globex Corporation",
    creditLimit: 75000.00
});

Customer.update("CUST002", { creditLimit: 90000.00 });
Customer.delete("CUST002");

// 5. Upsert (ON CONFLICT DO UPDATE)
const synced = Customer.upsert({
    where: { customerNo: "CUST-SYNC" },
    create: { customerNo: "CUST-SYNC", name: "Synced Company" },
    update: { name: "Synced Company (Updated)" }
});

// 6. Pessimistic Row Lock (SELECT ... FOR UPDATE)
const lockedRecord = Customer.lock("CUST001");
```

---

## 5. Enterprise Session Context (`BosSessionContext`)

Every script execution receives an immutable, verified context object (`ctx`):

```typescript
declare interface BosSessionContext {
    // 1. Security Envelope
    readonly tenantId: string;
    readonly companyId: string;
    readonly userId: string;
    readonly userRoles: string[];

    // 2. User Functional Parameters & WorkDate
    readonly user: {
        readonly workDate: string;          // ISO Date: e.g. "2026-10-09"
        readonly allowPostingFrom: string;  // Posting boundary lower limit
        readonly allowPostingTo: string;    // Posting boundary upper limit
        readonly defaultLocation: string;   // Default warehouse branch
        readonly timeZone: string;          // e.g. "Asia/Kolkata", "UTC"
        readonly locale: string;            // e.g. "en-IN", "en-US", "de-DE"
        readonly currencyCode: string;      // Base currency: "INR", "USD", "EUR"
    };

    // 3. Exact 128-Bit Monetary Math (Zero IEEE-754 Float Errors)
    readonly decimal: (value: string | number) => BosDecimal;

    // 4. Business Calendar & Fiscal Calculations
    readonly calendar: {
        readonly isWorkingDay: (date: string) => boolean;
        readonly addWorkingDays: (startDate: string, days: number) => string;
        readonly getFiscalPeriod: (date: string) => string;
        readonly getFiscalYear: (date: string) => string;
    };

    // 5. Outbound Sandboxed HTTP Client
    readonly http: {
        readonly get: (url: string, headers?: Record<string, string>) => HttpResponse;
        readonly post: (url: string, body: any, headers?: Record<string, string>) => HttpResponse;
    };

    // 6. Structured Audit Telemetry
    readonly audit: {
        readonly log: (message: string, payload?: Record<string, any>) => void;
    };

    // 7. Dynamic Database Gateway
    readonly db: {
        readonly [tableName: string]: TableModel<any>;
        readonly getModel: <T = any>(tableName: string) => TableModel<T>;
    };
}
```

---

## 6. The 5-Pillar Enterprise Extension Framework

To allow third-party developers, partners, and power users to extend BOS safely without breaking upgrades or introducing npm bloat:

```
                                  ┌───────────────────────────┐
                                  │   BOS Core Engine (Rust)  │
                                  └─────────────┬─────────────┘
                                                │
         ┌──────────────────┬───────────────────┼───────────────────┬──────────────────┐
         ▼                  ▼                   ▼                   ▼                  ▼
  [1. Event Sub-     [2. WebAssembly     [3. Sidecar Micro-  [4. Modular App-   [5. Low-Code
      scribers]          (Wasm) Engine]      services (gRPC)]    Source Packages]   Formulas]
  • Publish-Subscribe • Heavy Algorithms • Bank Gateways     • Namespaced       • Excel-like
  • Upgrade-Safe      • Barcode/PDF Gen  • AI / OCR Models   • Table Extensions   expressions
  • Pure BOS-Script   • High-Perf Math   • Isolated Process  • Full Lifecycle   • Power Users
```

### Pillar 1: Publish-Subscribe Event Pattern (The Business Central Standard)
Base application code is never modified. Base code exposes event hooks, and extensions subscribe to them:

```typescript
// In custom extension package: packages/partner.logistics/OrderHook.bos
codeunit LogisticsOrderSubscriber {
    [EventSubscriber(Table::SalesHeader, "onBeforeInsert")]
    public assignShippingCarrier(rec: SalesHeaderRecord, ctx: BosSessionContext) {
        if (!rec.carrierCode) {
            rec.carrierCode = "DHL_EXPRESS";
            rec.shippingCost = ctx.decimal(25.00).toNumber();
        }
    }
}
```

### Pillar 2: WebAssembly (Wasm) Native Plugin Sandbox
For heavy computational tasks (barcode/QR generation, custom compression, PDF layout, complex engineering formulas):
* Written in Rust, Go, C++, or Zig and compiled to `.wasm` or WAT.
* Loaded by `WasmEngine` on `wasmi`, with fuel (default 100,000 for a direct call, 50,000 for the numeric helpers). The host does not set a Wasm memory ceiling.
* From a script, only a two-number export is callable:
  ```typescript
  const tax = ctx.wasm.invoke("tax_module", "apply", rec.netAmount, rec.rate);
  ```

### Pillar 3: Out-of-Process Sidecar Microservices
For heavy integrations requiring full operating systems, machine learning models, or external SDKs (Stripe, OpenAI, Avalara Tax):
* Runs as an isolated container sidecar on localhost.
* Communicates over internal mutual-TLS gRPC or Unix Domain Sockets (`IPC`).
* Invoked via:
  ```typescript
  const taxResult = ctx.services.invoke("tax-calculator", "calculateTax", { lines: rec.lines });
  rec.taxAmount = taxResult.totalTax;
  ```

### Pillar 4: Modular Namespaced Packages (`.bospkg`)
All customizations travel in self-contained, signed packages:
* **Table Extensions:** Add custom fields and keys to base tables non-invasively (`tableextension SalesHeaderExt extends SalesHeader`).
* **Page Extensions:** Add controls, actions, and custom views to base UI pages (`pageextension SalesOrderCardExt extends SalesOrderCard`).
* **Schema Synchronization:** Verified cryptographically and synchronized non-destructively.

### Pillar 5: Safe Low-Code Customizations for Power Users
* **Formula Columns:** Excel-like calculated expressions compiled to PostgreSQL `GENERATED ALWAYS AS (...) STORED` columns.
* **Declarative Validation Rules:** Configured via the Admin UI and executed with a strict 1,000-instruction fuel cap.

---

## 7. Enterprise Runtime Invariants & Verified Capabilities (Core v2.0+)

The following enterprise capabilities are actively implemented and verified across `bosc-codegen-db`, `bosc-analyzer`, and `bosc-runtime`:

### 7.1 Defense-in-Depth Immutable Ledgers (Gap-02 / Option A + B)
- **Engine Guard (Option A):** `bosc-runtime` inspects `sub_type == 'Ledger'`. Direct `PUT`, `PATCH`, and `DELETE` requests are rejected with `RuntimeError::ImmutableLedger` (`HTTP 403 Forbidden`).
- **PostgreSQL Database Trigger (Option B):** `bosc-codegen-db` generates native `BEFORE UPDATE OR DELETE` PostgreSQL triggers (`guard_immutable_<table_name>`) checking `current_setting('bos.privileged_override', true) = 'true'`.
- **Statutory Privileged Override:** Certified superusers execute emergency corrections via `POST /api/entities/{table}/{id}/privileged-override`. Transactions execute with session elevation tokens and persist audit records in `_system_privileged_audit_log`.

### 7.2 In-Table Draft Lifecycle Engine
- Tables with `tableType = Document` or status columns utilize native in-table draft states (`status = 'Draft'`, `is_draft = true`).
- **`onDraftSave`**: Automatically dispatched on both `insert_record` and `update_record` when a document is in draft state.
- **`onDraftActivate`**: Automatically dispatched in `transition_state` when a document transitions from `"Draft"` to an active status, executing strict business invariant validations prior to commitment.
- **`onDraftDiscard`**: Automatically dispatched in `delete_record` when a draft document is discarded.

### 7.3 Enterprise Record Prototype in QuickJS Sandbox
Records in QuickJS trigger blocks (`rec` and `xRec`) are backed by an ERP prototype providing:
- `rec.testField(field, [expectedValue])`: Asserts field equality or non-blank value, throwing a structured error if the assertion fails.
- `rec.calcFields(...fields)`: Dynamically queries and resolves virtual SIFT FlowField subqueries via host bridge `__host_db_calc_flow_fields` and merges calculated values onto `rec`.

### 7.4 Transactional Test Runner DSL (`@test` Codeunits)
- Codeunits decorated with `@test` provide an in-engine test execution harness.
- Invoked via `POST /api/tests/run` (optional `{ "filter": "CodeunitName" }`).
- Discovered test methods execute within isolated database transactions that are guaranteed to auto-rollback (`tx.rollback()`), guaranteeing zero test pollution.

### 7.5 Validation Cascades (`record.field.validate(value)`)
- Field-level `validate(value) { ... }` blocks defined on table fields in BOS-Script are synthesized into `onValidate_<col>` triggers in `TableIR`.
- Both `insert_record` and `update_record` dispatch `TriggerEventIR::OnValidate(col)` triggers for changed fields prior to modify triggers, with strict error propagation.

### 7.6 Enterprise Meta-Objects: `permissionset`, `view`, and `query`

#### 1. `permissionset` (Declarative Security & Row-Level Access Guard)
- **Grammar & AST:**
  ```typescript
  permissionset SalesRep {
      caption = "Sales Representative Role";
      permissions = {
          Customer = RIM;
          SalesHeader = RIMD where { salesRepId == currentUserId };
      };
      execute = [SalesPostService, TaxCalculator];
  }
  ```
- **Dual-Layer Architecture:**
  - **L1 In-Memory Fast Guard (`bosc-runtime`):** `check_permission(&ctx, entity, action)` validates table-level CRUD permissions and codeunit execution privileges in sub-milliseconds without disk I/O.
  - **L2 Defense-in-Depth Database RLS (`bosc-codegen-db`):** Row-level `where` expressions compile to native PostgreSQL Row-Level Security policies (`CREATE POLICY ... USING (...)`) enforced at the database kernel level against `current_setting('bos.current_user_id', true)::uuid`.
- **API Exposure:** `POST /api/permissions/check` evaluates real-time access for any user and entity action.

#### 2. `view` (Relational Projection & Read-Model Engine)
- **Grammar & AST:**
  ```typescript
  view CustomerBalanceView {
      caption = "Customer Balances";
      materialized = false;
      select {
          id as customerId,
          name as customerName,
          totalBalance
      }
      from Customer
      left join SalesHeader for (SalesHeader.customerId == Customer.id)
      where { Customer.isActive == true };
  }
  ```
- **DDL Generation:**
  - Generates standard PostgreSQL views (`CREATE OR REPLACE VIEW ...`) or materialized projections (`CREATE MATERIALIZED VIEW ...` with unique index on `_id`).
- **REST Endpoints:**
  - `GET /api/views/{name}`: Query projected views with dynamic pagination (`limit`, `offset`), sorting (`order_by`), and structured filtering.
  - `POST /api/views/{name}/refresh`: Concurrently or synchronously trigger materialized view refresh (`REFRESH MATERIALIZED VIEW ...`).

#### 3. `query` (Compiled Analytical Data Pipelines)
- **Grammar & AST:**
  ```typescript
  query TopCustomersByVolume {
      caption = "Top Customers By Volume";
      elements {
          dataitem(cust; Customer) {
              column(custId; id) {}
              column(custName; name) {}
              dataitem(orders; SalesHeader) {
                  left join (orders.customerId == cust.id);
                  column(totalAmount; amount; sum) {}
              }
          }
      }
      groupBy(cust.id, cust.name);
      having { sum(orders.amount) > 10000 };
      orderBy(totalAmount desc);
  }
  ```
- **Analytical Engine & Codegen:**
  - Compiles declarative multi-table hierarchies, join predicates, aggregations (`sum`, `avg`, `count`, `min`, `max`), and groupings into optimized, parameterized SQL queries.
  - Registered in `_bos_meta_objects` catalog for cluster-wide discovery.
- **REST Execution Endpoint:**
  - `POST /api/queries/{name}/run`: Executes named queries with tenant isolation context and dynamic parameter binding, returning streaming JSON row datasets.

