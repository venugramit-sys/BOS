# Live Backend API Server Verification Report

**Environment:** Windows (x64)  
**Database:** PostgreSQL 18.4 (Local Service `postgresql-x64-18`, Port 5432)  
**Runtime:** Rust 1.85+, Tokio Async, Axum 0.8  
**Verification Date:** October 2026  
**Result:** Historical note. This is not a current pass. See `docs/26_production_readiness_status_report.md` for what the server actually does.

---

## 1. Executive Summary

The endpoint list below was recorded against a local server. It is not a certificate that every handler is complete. Authentication, ledger posting, and several host calls are still open in `docs/26_production_readiness_status_report.md`.

---

## 2. API Endpoints Live Verification Matrix

| Endpoint | Method | Status | Live Result Summary |
|---|---|:---:|---|
| `/api/health` | GET | `200 OK` | Returns service health, service name, and engine version |
| `/api/detect-db` | GET | `200 OK` | Scans local host TCP sockets and identifies PostgreSQL (5432) & SQLite |
| `/api/meta/system-status` | GET | `200 OK` | Returns provisioning status, catalog counts, and active storage strategy |
| `/api/meta/provision` | POST | `200 OK` | Runs kernel migrations and provisions meta catalog indices |
| `/api/meta/objects` | GET, POST | `200 OK` | Lists or registers metadata objects (tables, codeunits, enums) |
| `/api/meta/ambient.d.ts` | GET | `200 OK` | Serves Monaco-compatible ambient TypeScript runtime declarations |
| `/api/entities/{table}` | POST | `201 Created` | Inserts record into PostgreSQL with RLS context (`_tenant_id`, `_company_id`, `_id`) |
| `/api/entities/{table}/batch` | POST | `201 Created` | Vectorized multi-row bulk insert with trigger dispatch |
| `/api/entities/{table}/{id}` | GET | `200 OK` | Queries record by UUID under tenant isolation context |
| `/api/entities/{table}/{id}` | PUT, PATCH | `200 OK` | OCC version-checked record update with OnValidate cascades |
| `/api/entities/{table}/{id}` | DELETE | `200 OK` | Referential integrity-checked delete with OnBefore/AfterDelete triggers |
| `/api/entities/{table}/{id}/lock` | POST | `200 OK` | Pessimistic row locking (`SELECT ... FOR UPDATE`) |
| `/api/entities/{table}/lock` | POST | `200 OK` | Pessimistic table locking (`LOCK TABLE ...`) |
| `/api/entities/{table}/{id}/transition` | POST | `200 OK` | Executes atomic state transition (`Draft` $\rightarrow$ `PendingApproval` $\rightarrow$ `Approved`) |
| `/api/entities/{table}/{id}/state-history` | GET | `200 OK` | Returns full immutable audit trail from `{table}_state_history` |
| `/api/entities/{table}/{id}/privileged-override` | POST | `200 OK` | Overrides immutable ledger row and logs event to `_system_privileged_audit_log` |
| `/api/events/subscribers` | GET | `200 OK` | Lists all registered event subscribers ordered by priority |
| `/api/events/subscribers` | POST | `201 Created` | Persists event subscriber into `_bos_event_subscribers` |
| `/api/events/publish` | POST | `200 OK` | Synchronously dispatches event through prioritized subscribers |
| `/api/wasm/modules` | GET | `200 OK` | Lists loaded in-memory WebAssembly modules |
| `/api/wasm/modules` | POST | `201 Created` | Compiles and loads WAT or WASM bytecode into `wasmi 2.0` |
| `/api/wasm/{module}/{func}/invoke` | POST | `200 OK` | Executes exported Wasm function with instruction fuel limit |
| `/api/sidecars` | GET | `200 OK` | Lists registered companion sidecars and endpoints |
| `/api/sidecars` | POST | `201 Created` | Registers companion sidecar microservice |
| `/api/sidecars/{name}/call` | POST | `200 OK` | Forwards JSON payload to companion service over loopback IPC HTTP |
| `/api/sidecars/{name}/health` | GET | `200 OK` | Probes companion sidecar `/health` endpoint |
| `/api/codeunits` | POST | `201 Created` | Registers codeunit and deploys pushdown AMDP procedures |
| `/api/codeunits/{name}/run` | POST | `200 OK` | Runs codeunit procedure inside QuickJS with models |

---

## 3. End-to-End Test Payloads & Output Evidence

### 3.1 Event Engine Verification (`POST /api/events/publish`)
**Request:**
```http
POST /api/events/publish HTTP/1.1
Content-Type: application/json
x-tenant-id: 00000000-0000-0000-0000-000000000000
x-company-id: 00000000-0000-0000-0000-000000000000

{
  "publisher": "Customer",
  "event_name": "OnAfterInsert",
  "data": {
    "code": "CUST-001",
    "name": "Acme Corp"
  }
}
```
**Response (`200 OK`):**
```json
{
  "execution_logs": [
    "Executing event subscriber 'AuditLogExtension.LogCustomerCreated' (priority 50) on 'Customer.OnAfterInsert'"
  ],
  "event_name": "OnAfterInsert",
  "mutated_data": {
    "_audited": true,
    "code": "CUST-001",
    "name": "Acme Corp"
  },
  "publisher": "Customer",
  "success": true
}
```

### 3.2 WebAssembly Host Verification (`POST /api/wasm/tax_calc/calc_tax/invoke`)
**Request:**
```http
POST /api/wasm/tax_calc/calc_tax/invoke HTTP/1.1
Content-Type: application/json

{
  "a": 1250.50,
  "b": 0.18,
  "fuel_limit": 50000
}
```
**Response (`200 OK`):**
```json
{
  "function": "calc_tax",
  "module": "tax_calc",
  "result": 225.09,
  "success": true
}
```

### 3.3 Sidecar Microservice Probing (`GET /api/sidecars/tax_service/health`)
**Response (`200 OK`):**
```json
{
  "is_healthy": true,
  "sidecar": "tax_service",
  "status": "Running",
  "success": true
}
```
