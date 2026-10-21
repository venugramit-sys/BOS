# BOS Database Lifecycle Management & Testing Studio

**Document Version:** 1.0.0  
**Status:** Live & Production Ready  
**Date:** October 2026  
**Related Components:** [`crates/bosc-runtime/src/db_manager.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-runtime/src/db_manager.rs), [`crates/bosc-runtime/src/server.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-runtime/src/server.rs), [`ui/src/App.svelte`](file:///d:/Work%20Studio/Bos/ui/src/App.svelte)

---

## 1. Executive Summary

In enterprise Business Operating Systems (BOS), database lifecycle management must be dynamic, tenant-isolated, and automated. BOS provides full native database lifecycle administration—enabling developers and enterprise administrators to:
1. Detect running database instances (PostgreSQL 18.4, SQLite, etc.) with zero manual configuration.
2. Probe connection URLs, test network latencies, and retrieve database server engine versions.
3. List all PostgreSQL catalogs with human-readable storage sizes (`pg_database_size`).
4. Provision new PostgreSQL databases on-demand with automatic installation of enterprise cryptographic and identity extensions (`uuid-ossp`, `pgcrypto`).
5. Dynamically switch the runtime database pool without terminating or restarting the backend process.
6. Inspect deep table catalogs, row count estimates, and table storage footprints.
7. Execute DDL/DML maintenance statements and safely truncate tables.
8. Forcefully drop obsolete or testing databases (`DROP DATABASE ... WITH (FORCE);`) with automatic fallback to maintenance catalogs.
9. Verify all operations visually and through automated end-to-end testing via the Svelte 5 API Studio.

---

## 2. Architecture & Concurrency Model

### 2.1 Dynamic Engine Database Switching

In standard Rust Axum / SQLx backends, `PgPool` is typically initialized once at startup and frozen for the life of the application. In `bosc-runtime`, `BosEngine` encapsulates the connection pool and active URL inside thread-safe read-write locks:

```rust
pub struct BosEngine {
    pool: Arc<RwLock<PgPool>>,
    active_url: Arc<RwLock<String>>,
}
```

When an enterprise administrator invokes `/api/databases/switch`:
1. `BosEngine::switch_database(new_name)` constructs the new connection URL.
2. Tests the connection to the target database before touching the existing pool.
3. Acquires write locks on `pool` and `active_url`.
4. Replaces the inner pool with a freshly configured `PgPoolOptions::new().max_connections(20)`.
5. Closes the old connection pool cleanly.
6. All subsequent entity queries and state transitions automatically execute against the new database without downtime.

### 2.2 Maintenance Isolation for `CREATE` and `DROP`

PostgreSQL imposes strict transaction restrictions on database creation and destruction:
- `CREATE DATABASE` cannot execute inside a transaction block.
- `DROP DATABASE` cannot execute while any connection is active on the target catalog.

To guarantee fault-tolerant operations, `crates/bosc-runtime/src/db_manager.rs` connects to the maintenance catalog (`postgres`):
- To drop a database: `DROP DATABASE "<name>" WITH (FORCE);` forcibly terminates existing backend sessions before dropping.
- If the database being dropped is the current active runtime catalog, `BosEngine` automatically migrates its active connection to `postgres` before issuing the drop command.

---

## 3. Database Lifecycle REST API Reference

All routes are served by Axum on port `3000` (e.g., `http://127.0.0.1:3000`).

### 3.1 List Databases
- **Endpoint:** `GET /api/databases`
- **Description:** Queries `pg_database` catalog, returning names, formatted sizes, raw bytes, and current active status.
- **Sample Response:**
```json
[
  {
    "name": "bos_dev",
    "size_pretty": "8390 kB",
    "size_bytes": 8591039,
    "is_current": true
  },
  {
    "name": "postgres",
    "size_pretty": "9022 kB",
    "size_bytes": 9238207,
    "is_current": false
  }
]
```

### 3.2 Test Connection String
- **Endpoint:** `POST /api/databases/test-connection`
- **Payload:**
```json
{
  "url": "postgres://postgres:postgres@localhost:5432/bos_dev"
}
```
- **Sample Response:**
```json
{
  "success": true,
  "latency_ms": 27,
  "server_version": "PostgreSQL 18.4 on x86_64-windows, compiled by msvc-19.44.35227, 64-bit",
  "database_name": "bos_dev",
  "message": "Connected successfully to PostgreSQL database"
}
```

### 3.3 Create Database
- **Endpoint:** `POST /api/databases`
- **Payload:**
```json
{
  "name": "bos_finance_2026",
  "init_extensions": true
}
```
- **Sample Response:**
```json
{
  "success": true,
  "name": "bos_finance_2026",
  "message": "Database 'bos_finance_2026' created successfully"
}
```

### 3.4 Inspect Database & Table Catalog
- **Endpoint:** `GET /api/databases/{name}`
- **Sample Response:**
```json
{
  "name": "bos_dev",
  "size_pretty": "8390 kB",
  "size_bytes": 8591039,
  "connection_count": 3,
  "is_current": true,
  "tables": [
    {
      "table_name": "customer",
      "table_type": "BASE TABLE",
      "row_count": 4,
      "total_size": "48 kB"
    },
    {
      "table_name": "purchase_order",
      "table_type": "BASE TABLE",
      "row_count": 3,
      "total_size": "64 kB"
    },
    {
      "table_name": "customer_ledger_entry",
      "table_type": "BASE TABLE",
      "row_count": 2,
      "total_size": "40 kB"
    }
  ]
}
```

### 3.5 Switch Active Runtime Database
- **Endpoint:** `POST /api/databases/switch`
- **Payload:**
```json
{
  "name": "bos_finance_2026"
}
```
- **Sample Response:**
```json
{
  "success": true,
  "active_database": "bos_finance_2026",
  "message": "Switched active BOS database to 'bos_finance_2026'"
}
```

### 3.6 Manage Database (Execute SQL / Truncate / Rename)
- **Endpoint:** `POST /api/databases/{name}/manage`
- **Execute SQL Payload:**
```json
{
  "action": "execute_sql",
  "sql": "CREATE TABLE audit_trail_demo (id serial primary key, action text);"
}
```
- **Sample Response:**
```json
{
  "success": true,
  "database": "bos_finance_2026",
  "action": "execute_sql",
  "rows_affected": 0,
  "message": "SQL executed successfully"
}
```

### 3.7 Drop Database
- **Endpoint:** `DELETE /api/databases/{name}`
- **Sample Response:**
```json
{
  "success": true,
  "name": "bos_finance_2026",
  "message": "Database 'bos_finance_2026' dropped successfully"
}
```

---

## 4. BOS Svelte 5 Testing Studio (`ui/`)

The BOS API Testing Studio is built using **Svelte 5 Runes** (`$state`, `$derived`, `$effect`) and **Vite**, configured with an active reverse proxy forwarding `/api` requests to the Rust Axum backend.

### 4.1 Feature Categories & 52-Endpoint Preset Catalog

The studio provides **14 specialized category tabs** covering all 52 backend routes with ready-to-run JSON payloads:

| Category Tab | Icon | Domain | Key Endpoints Exposed |
|---|---|---|---|
| **⚡ All Endpoints** | `Sparkles` | Master Registry | Complete 52-route unified directory with real-time text filter. |
| **📜 DSL & Scripts** | `FileCode` | DSL & Hot Migration | `POST /api/script/compile`, `POST /api/script/apply`, `POST /api/meta/tables/{table}/script`, `POST /api/meta/table-extensions`, `POST /api/tests/run`. |
| **📦 Packages** | `Layers` | Modular Deployment | `GET /api/meta/packages`, `POST /api/packages/install`, `GET /api/packages/{id}/export`, `POST /api/databases/{name}/packages/install`. |
| **📊 Entities & CRUD** | `Database` | Generic CRUD & OCC | `GET/POST /api/entities/{table}`, `GET/PUT/PATCH/DELETE /api/entities/{table}/{id}`, `/schema`, `/batch`, `POST /api/entities/{table}/{id}/lock`. |
| **🔀 Workflows & States** | `Sliders` | Document Lifecycles | `POST /api/entities/{table}/{id}/transition`, `GET /api/entities/{table}/{id}/state-history`, `POST /api/entities/{table}/{id}/privileged-override`. |
| **🔍 Query & SIFT Views**| `Search` | Relational GraphQL | `POST /api/query`, `GET /api/views/{name}`, `POST /api/views/{name}/refresh`, `POST /api/queries/{name}/run`. |
| **📻 Event Engine** | `Radio` | Decoupled Events | `GET/POST /api/events/subscribers`, `POST /api/events/publish`, `GET/POST /api/events/outbox`, `POST /api/events/outbox/process`. |
| **⏰ Jobs & Workers** | `Clock` | Asynchronous Batch Queue | `POST /api/jobs/enqueue`, `POST /api/jobs/process-next`. |
| **🧩 Codeunits** | `Code2` | Sandboxed Business Logic | `POST /api/codeunits`, `POST /api/codeunits/{name}/run`. |
| **⚙️ Wasm Sandbox** | `Cpu` | WebAssembly Plugins | `GET/POST /api/wasm/modules`, `POST /api/wasm/{module}/{func}/invoke`. |
| **🌐 Sidecar IPC** | `Network` | Loopback Microservices | `GET/POST /api/sidecars`, `GET /api/sidecars/{name}/health`, `POST /api/sidecars/{name}/call`. |
| **🛡️ Security & RBAC** | `Shield` | Identity & Isolation | `POST /api/permissions/check`, `GET/POST /api/tenants`, `GET/POST /api/companies`. |
| **📑 Meta Catalog** | `Table` | System Kernel Metadata | `GET /api/health`, `GET /api/meta/system-status`, `POST /api/meta/provision`, `GET/POST /api/meta/objects`, `GET /api/meta/ambient.d.ts`. |
| **🗄️ DB & Cluster** | `Settings` | Database Lifecycle | `GET /api/detect-db`, `GET/POST /api/databases`, `POST /api/databases/test-connection`, `POST /api/databases/switch`, `GET /api/databases/current`, `POST /api/database/current/manage`, `POST /api/entities/{table}/lock`, `DELETE /api/databases/{name}`. |

### 4.2 Live entity Create and Update

The studio reads `GET /api/meta/objects?type=table` and adds a **Create** and an **Update** call for every table. Choosing one calls `GET /api/entities/{table}/schema` and fills the body from `sample_create_payload` or `sample_update_payload`. Unique text fields on a create body get a short suffix so a second send does not collide. Update reads one existing row, puts its id in the path, copies `_system_version`, and keeps the current field values. The entity picker on an `/api/entities/...` request switches the table and rebuilds that body. **Refresh entities** reloads the list after a table is applied.

### 4.3 Interactive Response Inspector
The studio features a sticky API Response Inspector with:
- Live HTTP Status Badges (`200 OK`, `400 BAD REQUEST`, `409 CONFLICT`, `422 UNPROCESSABLE`)
- Precision Request Latency Display (e.g. `14ms`)
- Formatted JSON response viewer with one-click copy feedback
- Active session header injector (`x-tenant-id`, `x-company-id`, `x-user-id`, `x-roles`)
- Execution logs and response headers tabs

---

## 5. Automated Verification Checklist

All 17 integration steps run synchronously in the studio and pass cleanly:

- [x] Step 1: System Health Check (`GET /api/health`)
- [x] Step 2: Database Provider Auto-Detection (`GET /api/detect-db`)
- [x] Step 3: List PostgreSQL Databases (`GET /api/databases`)
- [x] Step 4: Test Connection URL & Latency (`POST /api/databases/test-connection`)
- [x] Step 5: Provision Sandbox Database (`POST /api/databases`)
- [x] Step 6: Inspect Sandbox Database Schema (`GET /api/databases/bos_e2e_sandbox`)
- [x] Step 7: Execute DDL in Sandbox DB (`POST /api/databases/bos_e2e_sandbox/manage`)
- [x] Step 8: Drop Sandbox Database (`DELETE /api/databases/bos_e2e_sandbox`)
- [x] Step 9: Create Customer with RLS Context (`POST /api/entities/customer`)
- [x] Step 10: Query Customer Entities (`GET /api/entities/customer`)
- [x] Step 11: Create Purchase Order Draft (`POST /api/entities/purchase_order`)
- [x] Step 12: State Transition Draft → PendingApproval (`POST /api/entities/purchase_order/{id}/transition`)
- [x] Step 13: State Transition PendingApproval → Approved (`POST /api/entities/purchase_order/{id}/transition`)
- [x] Step 14: Verify State Machine Audit Trail (`GET /api/entities/purchase_order/{id}/state-history`)
- [x] Step 15: Create Write-Once Ledger Entry (`POST /api/entities/customer_ledger_entry`)
- [x] Step 16: Execute Privileged Audit Override (`POST /api/entities/customer_ledger_entry/{id}/privileged-override`)
- [x] Step 17: Verify Statutory Audit Trail Log (`GET /api/entities/_system_privileged_audit_log`)
