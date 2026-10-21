# Database Architecture & Runtime Execution Engine

> **Status:** Production Architecture & Specification  
> **Document:** `docs/03_database_and_runtime_engine.md`  
> **Target Version:** BOS Core v2.0+

---

## 1. Multi-Tenancy & Database Topology

BOS is architected for enterprise-scale multi-tenancy with 4 adaptive storage strategies configured per cluster or company:

1. **Shared Database, Row-Level Tenant Isolation (`shared_rls`):**
   - High-density SaaS deployment where all tenant data resides in unified tables with an indexed `_tenant_id` column.
   - Enforced automatically at the database level via PostgreSQL **Row-Level Security (RLS)**:
     ```sql
     ALTER TABLE "customer" ENABLE ROW LEVEL SECURITY;
     
     CREATE POLICY tenant_isolation_policy ON "customer"
       USING ("_tenant_id" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid)
       WITH CHECK ("_tenant_id" = NULLIF(current_setting('bos.current_tenant_id', true), '')::uuid);
     ```
2. **Schema Per Company (`schema_per_company`):**
   - Each company gets an isolated PostgreSQL schema (e.g., `company_us`, `company_eu`).
3. **Table Per Company (`table_per_company`):**
   - Tables are suffixed per company (e.g., `customer$company_us`).
4. **Partitioned Ledger (`partitioned`):**
   - Large financial ledgers are partitioned horizontally by company ID or posting date.

---

## 2. PostgreSQL DDL Mapping Specification

BOS Script types map directly to optimized PostgreSQL data structures:

| BOS-Script Type | PostgreSQL Representation | Constraints / Indexes |
| :--- | :--- | :--- |
| `UUID` | `UUID` | Default `gen_random_uuid()` |
| `String(N)` | `VARCHAR(N)` | Check length constraint |
| `String` (unbounded) | `TEXT` | Standard text |
| `Decimal(P, S)` | `NUMERIC(P, S)` | Exact financial precision (128-bit) |
| `Integer` | `BIGINT` or `INTEGER` | Signed integer |
| `DateTime` | `TIMESTAMPTZ` | Stored in UTC |
| `Date` | `DATE` | Standard ISO date |
| `Boolean` | `BOOLEAN` | True/False |
| `Enum(...)` | `VARCHAR(50)` | Check constraint on allowed values |
| `Json` | `JSONB` | GIN indexable |
| `Formula` | `GENERATED ALWAYS AS (...) STORED` | In-database PostgreSQL pushdown |
| `FlowField` | Generated Pushdown View (`{table}_view`) | Real-time SIFT aggregation |

### System Columns (Auto-injected by Compiler)
Every compiled table automatically receives standard system audit metadata:
- `_id`: Primary key (UUID)
- `_tenant_id`: Tenant reference (UUID)
- `_company_id`: Multi-company partition (UUID, when `data_per_company = true`)
- `_created_at`: Creation timestamp (`TIMESTAMPTZ DEFAULT NOW()`)
- `_updated_at`: Modification timestamp (`TIMESTAMPTZ DEFAULT NOW()`)
- `_system_version`: Monotonic row version for Optimistic Concurrency Control (OCC)

---

## 3. Formula Columns & FlowField Pushdown Views

### 3.1 PostgreSQL Stored Generated Columns
Field-level formulas declared in BOS-Script are lowered directly into PostgreSQL physical generated columns:
```sql
ALTER TABLE "sales_line"
  ADD COLUMN "line_amount" NUMERIC(18,2)
  GENERATED ALWAYS AS ("quantity" * "unit_price" * (1.0 - "discount_pct" / 100.0)) STORED;
```
This guarantees zero application CPU overhead and indexability across all 4 multi-tenant storage models.

### 3.2 FlowField Pushdown SQL Views (`{table}_view`)
To avoid N+1 database roundtrips, the compiler automatically generates companion views computing SIFT aggregations:
```sql
CREATE OR REPLACE VIEW "customer_view" AS
SELECT
    t.*,
    (SELECT COALESCE(SUM(amount), 0) FROM "cust_ledger_entry" le WHERE le."customer_no" = t."code") AS "balance",
    (SELECT COALESCE(COUNT(*), 0) FROM "cust_ledger_entry" le WHERE le."customer_no" = t."code" AND le."open" = TRUE) AS "open_ledger_count"
FROM "customer" t;
```

---

## 4. Multi-Tier Hybrid Runtime Execution Engine

```
┌────────────────────────────────────────────────────────────────────────┐
│                        BOS Host Runtime Engine                         │
│                                                                        │
│  ┌───────────────────────┐ ┌────────────────────┐ ┌──────────────────┐ │
│  │ QuickJS Sandbox (8MB) │ │ wasmi 2.0 Wasm Host│ │ Sidecar Host IPC │ │
│  │ - Rec vs. xRec Diffs  │ │ - Pure Rust Engine │ │ - Loopback IPC   │ │
│  │ - OnValidate Cascades │ │ - Fuel Metering    │ │ - Python/Node/Go │ │
│  │ - Exact BigInt Decimals││ - WAT & WASM Byte  │ │ - Health Probes  │ │
│  │ - Event Dispatch      │ │ - Typed Invocations│ │ - External SDKs  │ │
│  └───────────┬───────────┘ └─────────┬──────────┘ └─────────┬────────┘ │
│              │                       │                      │          │
│              └───────────────────────┼──────────────────────┘          │
│                                      ▼                                 │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ Host Bridge Services (ctx.events, ctx.wasm, ctx.sidecar, ctx.db) │  │
│  └───────────────────────────────────┬──────────────────────────────┘  │
│                                      ▼                                 │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ PostgreSQL Connection Pool & AMDP Pushdown (sqlx::PgPool)        │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### 4.1 QuickJS Sandbox Engine
- Memory ceiling: 8 MB. Stack limit: 512 KB.
- Exact financial arithmetic via 128-bit scaled BigInt (`ctx.decimal(val)`).
- Dynamic TableModel gateway: `Customer.get(pk)`, `Customer.findMany(q)`, `Customer.create(d)`, `Customer.lock(pk)`.
- Pre-compiled ahead-of-time with pure JavaScript emitted by `bosc-analyzer::ts_stripper`.

### 4.2 Sandboxed WebAssembly Host (`wasmi 2.0`)
- Pure Rust interpreter running in-process with zero external C/LLVM dependencies.
- **Instruction Fuel Quotas:** Configurable fuel allocation (e.g. 50,000 units) guarantees termination of infinite loops.
- Supports both WAT text format and pre-compiled `.wasm` binaries.
- Invocation in BOS-Script: `ctx.wasm.invoke("module_name", "func_name", paramA, paramB)`.

### 4.3 Out-of-Process Sidecar Host
- Companion microservices running as isolated sidecar containers or processes.
- High-speed local IPC / loopback HTTP connection using `reqwest` with pure Rust TLS (`rustls`).
- Automatic on-demand `/health` probes.
- Invocation in BOS-Script: `ctx.sidecar.call("service_name", "/path", payload)`.

### 4.4 Decoupled Event-Subscriber Engine
- Base tables and codeunits publish lifecycle events (`OnBeforeInsert`, `OnAfterInsert`, `OnBeforeModify`, `OnAfterModify`, `OnBeforeDelete`, `OnAfterDelete`, `OnValidate<Field>`).
- Subscribers are persisted in PostgreSQL catalog `_bos_event_subscribers` and cached in thread-safe memory `DashMap`.
- Subscribers execute in ascending priority order (e.g., priority `10` executes before priority `100`).
- Custom logic can publish arbitrary events: `ctx.events.publish("publisher", "eventName", data)`.

---

## 5. High-Concurrency Control & Optimistic Locking

- **Optimistic Concurrency Control (OCC):** Every record contains a monotonic `_system_version` column. Every API update must send the version it read: a missing version returns HTTP `428 Precondition Required` (`RuntimeError::VersionRequired`), a stale one returns `409 Conflict` (`RuntimeError::ConcurrencyConflict`).
- **Unit of work:** Each codeunit call, record write, transition or event publish runs in one database transaction together with its triggers, subscribers and outbox rows; any error rolls everything back. See [`27_transactional_integrity_and_schema_migration.md`](./27_transactional_integrity_and_schema_migration.md).
- **Pessimistic Row Locking (`SELECT ... FOR UPDATE`):** Exposed via `POST /api/entities/{table}/{id}/lock`, providing strict serialized exclusivity for critical ledger postings.
- **Pessimistic Table Locking (`LOCKTABLE`):** Mimicking Dynamics NAV's `Table.LOCKTABLE()`, exposed via `POST /api/entities/{table}/lock` executing `LOCK TABLE "{table}" IN ROW EXCLUSIVE MODE`.
- **GraphQL-Like Query Protocol:** `POST /api/query` supporting projection (`select`), dynamic filtering (`filter`), sorting (`order_by`), pagination (`limit`, `offset`), and multi-level lateral joins (`include`).
