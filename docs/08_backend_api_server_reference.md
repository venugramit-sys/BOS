# Backend HTTP & Query API Reference

> **Document ID:** `BOS-DOC-08`  
> **Source Verification:** `crates/bosc-runtime/src/server.rs`  
> **Target Audience:** API Consumers, Frontend Engineers, Integration Developers

---

## 1. Overview & Architecture

The BOS Backend API server is built on **Axum 0.8**, Tokio, and Tower. It provides a secure, fully featured RESTful and JSON-RPC API supporting:
- Multi-database lifecycle management.
- Dynamic entity CRUD with automatic schema resolution.
- GraphQL-like nested entity queries (`/api/query`).
- Optimistic Concurrency Control (OCC) and pessimistic row locking.
- Atomic state machine workflow transitions.
- Privileged statutory audit overrides.
- In-process WebAssembly and loopback sidecar execution.

---

## 2. Authentication & Session Middleware (`require_session`)

Every request to a protected API endpoint must include a valid session token via the `Authorization` header (`Bearer <token>`) or `x-session-token` header:

```http
Authorization: Bearer sess_sec_tok_12345
```

### 2.1. Public vs Protected Routes
- **Public Endpoints** (No token required): `/api/health`, `/api/auth/status`, `/api/auth/bootstrap`, `/api/auth/login`, `/api/setup/*`, `/api/detect-db`, `/api/databases*`.
- **Protected Endpoints**: All entity CRUD (`/api/entities/*`), script compilation/apply, codeunit execution, and admin operations.
- **Superuser Roles**: Users with roles `SUPER`, `Admin`, or `Global Admin` have unrestricted administrative bypass privileges.

---

## 3. Core API Endpoint Catalog

### 3.1. Health & Database Diagnostics

#### `GET /api/health`
Returns system status, active database name, and kernel health.
- **Response:**
  ```json
  {
      "status": "healthy",
      "database": "bos_dev",
      "kernel_version": "0.1.0",
      "uptime_seconds": 3600
  }
  ```

#### `GET /api/detect-db`
Probes local ports and returns detected database cluster engines (PostgreSQL, MySQL, SQL Server, Redis, ClickHouse, SQLite).

---

### 3.2. Database & Multi-Tenancy Management

| Method | Endpoint | Description |
| :--- | :--- | :--- |
| `GET` | `/api/databases` | Lists all databases in the connected PostgreSQL cluster. |
| `POST` | `/api/databases` | Creates a new database with specified options and storage strategy. |
| `GET` | `/api/databases/current` | Returns details and statistics for the currently connected database. |
| `POST` | `/api/databases/switch` | Switches the runtime's active database connection pool. |
| `POST` | `/api/databases/test-connection` | Validates a candidate connection URL without switching. |
| `POST` | `/api/databases/{name}/packages/install` | Installs a `.bospkg` package bundle into the specified database. |
| `GET` | `/api/tenants` | Lists all provisioned tenants. |
| `POST` | `/api/tenants` | Provisions a new enterprise tenant. |
| `GET` | `/api/companies` | Lists all companies within the tenant. |
| `POST` | `/api/companies` | Provisions a new operating company. |

---

### 3.3. Script Compilation & Schema Sync

#### `POST /api/script/compile`
Compiles BOS-Script source into AST, runs semantic analysis and type stripping, and returns the generated `ModelIR` and diagnostics without modifying the database.
- **Payload:**
  ```json
  {
      "script": "table Note { id: UUID; title: String(100); primaryKey(id); }",
      "package_name": "bos.notes",
      "apply": false
  }
  ```

#### `POST /api/script/apply`
Compiles the script, generates database DDL, and applies all non-destructive migration steps atomically.
- **Payload:**
  ```json
  {
      "script": "...",
      "package_name": "bos.notes",
      "allow_destructive": false
  }
  ```

---

### 3.4. Generic Entity CRUD & Concurrency Control

All business tables are automatically exposed as RESTful endpoints under `/api/entities/{table}`:

#### `GET /api/entities/{table}/schema`
Returns column definitions, data types, constraints, and FlowFields for the entity.

#### `GET /api/entities/{table}`
Returns paginated entity records. Supports sorting and filtering.

#### `POST /api/entities/{table}`
Inserts a new record, executing companion `tablelogic` `onInsert` triggers.
- **Payload:**
  ```json
  {
      "code": "CUST-001",
      "name": "Acme Industrial"
  }
  ```

#### `PUT / PATCH /api/entities/{table}/{id}` (Optimistic Concurrency Control)
Updates an existing entity. To prevent race conditions, the request body should include the known `_system_version`.
- **Payload:**
  ```json
  {
      "name": "Acme Industrial Global",
      "_system_version": 2
  }
  ```
- **Concurrency Guard**: If another user has updated the record and bumped `_system_version` to `3`, the request is rejected with **`409 Conflict`**.

#### `POST /api/entities/{table}/{id}/lock` (Pessimistic Row Locking)
Executes `SELECT ... FOR UPDATE` on the specified row within an active transaction, serializing high-contention sequential workflows.

#### `DELETE /api/entities/{table}/{id}`
Deletes the entity, executing `onDelete` validation triggers. If active child records exist (e.g. open ledger entries), the deletion is blocked by referential integrity triggers.

---

### 3.5. Base Query Protocol (`POST /api/query`)

Provides single-trip, nested, GraphQL-like data retrieval with FlowField pushdown:

- **Request:**
  ```json
  {
      "entity": "customer",
      "select": ["code", "name", "balance"],
      "filter": {
          "name": { "$like": "Acme%" }
      },
      "include": [
          {
              "relation": "ledger_entries",
              "entity": "customer_ledger_entry",
              "foreign_key": "customer_code",
              "limit": 5
          }
      ],
      "orderBy": [
          { "field": "name", "desc": false }
      ],
      "limit": 10,
      "offset": 0
  }
  ```
- **Response:**
  ```json
  {
      "count": 1,
      "data": [
          {
              "_id": "b3e64f89-c451-419b-a320-cb1067d5ce89",
              "code": "CUST-001",
              "name": "Acme Industrial",
              "balance": 1540.00,
              "_system_version": 2,
              "ledger_entries": [
                  { "entry_no": 1001, "amount": 500.00, "open": true },
                  { "entry_no": 1002, "amount": 1040.00, "open": true }
              ]
          }
      ]
  }
  ```

---

### 3.6. Workflows & State Machine Transitions

#### `POST /api/entities/{table}/{id}/transition`
Executes an atomic workflow transition defined in a `statemachine`.
- **Payload:**
  ```json
  {
      "transition": "approve",
      "params": {
          "comment": "Credit limit confirmed"
      }
  }
  ```
- **Behavior**: Verifies that the transition is valid from the record's current state, evaluates guard scripts, checks user roles, updates the status field, logs state history, and executes `on_transition` hooks.

#### `GET /api/entities/{table}/{id}/state-history`
Returns the complete audit log of all historical state transitions for the record.

---

### 3.7. Codeunits & Procedure Execution

#### `POST /api/codeunits/{name}/run`
Executes an exposed codeunit procedure.
- **Payload:**
  ```json
  {
      "method": "ValidateFleetDispatch",
      "params": {
          "vin": "1HGCR2F83HA000000",
          "capacityTons": 25.5
      }
  }
  ```
- **Response:**
  ```json
  {
      "success": true,
      "result": { "valid": true, "status": "Approved" },
      "fuel_used": 10000,
      "logs": ["Vehicle dispatch parameters successfully verified by engine"]
  }
  ```
