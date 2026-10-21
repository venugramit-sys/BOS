# 11. Adaptive Multi-Tenant & Multi-Company Storage Strategies

> **Specification & Live Implementation Guide for Dynamic Database Provisioning Topology in BOS**

---

## 1. Overview & Architectural Motivation

In modern enterprise business operating systems, data isolation cannot be locked into a single rigid topology. Depending on organizational size, regulatory mandates (GDPR, HIPAA, ITAR, GoBD), performance SLAs, and hosting cost models, different deployment profiles demand different physical storage mechanics.

BOS supports **four distinct storage & multi-tenancy strategies** selectable at database creation / provisioning time. The entire BOS compiler and runtime ecosystem automatically adapts its DDL generation, indexing, schema namespaces, and query execution to match the active strategy.

---

## 2. The Four Supported Storage Strategies

| Strategy ID | Canonical Name | Isolation Level | DDL & Physical Mechanics | Ideal Enterprise Use Case |
| :--- | :--- | :--- | :--- | :--- |
| **`shared_rls`** | **Shared Table + Row-Level Security** *(Default)* | Logical (Engine RLS) | All tenants/companies share single physical tables in `public`. `_tenant_id` and `_company_id` are injected. Protected via PostgreSQL native RLS policies. | SaaS startups, high tenant density, cost-optimized cloud hosting. |
| **`partitioned`** | **Declarative Table Partitioning** | Physical Disk Partitioning | PostgreSQL native `PARTITION BY LIST ("_tenant_id")`. The engine creates child tables on disk (`{table}_default`, `{table}_tenant_{code}`). Uniqueness constraints automatically composite with partition keys. | High query throughput ERPs, instant zero-vacuum tenant offboarding (`DROP TABLE partition`). |
| **`schema_per_company`** | **Dedicated Schema-per-Company** | Namespace Isolation | Company-specific tables live in isolated PostgreSQL schemas (`comp_{code}`). Global master tables remain in `public`. Runtime dynamically sets `search_path = "comp_{code}", "public"`. | Multi-subsidiary conglomerates with independent statutory reporting and per-company backup/restore. |
| **`table_per_company`** | **Physical Table-per-Company** | Physical Table Isolation | Dynamics NAV / Business Central pattern: physical tables prefixed per company (`{company_code}${table}`). Unified views generated for reporting. Eliminates table-level locks between subsidiaries. | Corporate groups requiring zero lock contention and complete physical separation of subsidiary transactional tables. |

---

## 3. End-to-End Implementation Across the BOS Stack

### A. BOS-IR (`bosc-ir`)
The intermediate representation defines `StorageStrategyIR`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageStrategyIR {
    #[serde(rename = "shared_rls")]
    SharedRLS,
    #[serde(rename = "partitioned")]
    Partitioned,
    #[serde(rename = "schema_per_company")]
    SchemaPerCompany,
    #[serde(rename = "table_per_company")]
    TablePerCompany,
}
```

### B. Database Code Generator (`bosc-codegen-db`)
`PostgresGenerator` accepts the chosen strategy via `PostgresGenerator::with_strategy(strategy)`:
- **Kernel Provisioning:** Stores the selected strategy into `_bos_sys_info.storage_strategy`.
- **System Table Upgrade:** Executes `ALTER TABLE IF EXISTS "_bos_sys_info" ADD COLUMN IF NOT EXISTS "storage_strategy" VARCHAR(50) NOT NULL DEFAULT 'shared_rls';`.
- **Adaptive DDL Generation:**
  - `SharedRLS`: Emits single table with `ENABLE ROW LEVEL SECURITY` and `CREATE POLICY`.
  - `Partitioned`: Emits `PARTITION BY LIST ("_tenant_id")`, `PRIMARY KEY ("_tenant_id", "_id")`, and automatically generates the default partition `{table}_default` plus tenant partitions `{table}_tenant_{code}`.
  - `SchemaPerCompany`: Emits tables into dedicated schemas (`comp_default`) and public fallback.
  - `TablePerCompany`: Emits physical tables prefixed with `{company_code}${table}`.

### C. Runtime Metadata Engine (`bosc-runtime`)
- `MetaManager::register_meta_object`:
  1. Inspects active database's `storage_strategy` from `_bos_sys_info`.
  2. Generates live DDL tailored to the strategy.
  3. Executes topology post-processing: provisions partition tables for active tenants, company schemas for active companies, or company-prefixed physical tables.
- `BosEngine::apply_isolation_context`:
  1. Sets PostgreSQL session variables: `bos.current_tenant_id` and `bos.current_company_id`.
  2. Evaluates the active database strategy: for `schema_per_company`, dynamically routes `search_path` to `"comp_{company_code}", "public"`.

### D. REST API Endpoints (`bosc-runtime::server`)
- `GET /api/system/storage-strategies`: Returns metadata, descriptions, isolation levels, and recommendations for all supported strategies.
- `POST /api/databases`: Accepts `storage_strategy` in payload (`"shared_rls"`, `"partitioned"`, `"schema_per_company"`, `"table_per_company"`).
- `GET /api/databases/{name}`: Returns the provisioned `storage_strategy` in database inspection details.

### E. Svelte 5 Testing Studio (`ui/src/App.svelte`)
- **Provisioning UI Card:** Includes interactive strategy selection dropdown with human-readable descriptions.
- **Schema Inspector:** Renders visual status badges for active strategies (`STRATEGY: PARTITIONED`, `STRATEGY: SCHEMA_PER_COMPANY`, `STRATEGY: TABLE_PER_COMPANY`, `STRATEGY: SHARED_RLS`).
- **Automated Test Runner:** Expanded to 22 steps validating the entire storage strategy lifecycle.

---

## 4. Verification & Live Execution Proof

All four storage strategies were created, switched, populated with live meta-objects, and inspected against PostgreSQL 18.4:

```text
=== 1. Testing GET /api/system/storage-strategies ===
Strategy: shared_rls - Shared Table + Row Level Security (RLS) (Logical (Row-level Security))
Strategy: partitioned - Declarative Table Partitioning (PostgreSQL Native) (Physical (Separate Disk Partitions per Tenant))
Strategy: schema_per_company - Dedicated Schema-per-Company (Namespace Isolation) (Logical / Namespace (PostgreSQL Schema Separation))
Strategy: table_per_company - Physical Table-per-Company (Dynamics NAV/BC Pattern) (Physical (Separate Tables per Company))

=== Testing Strategy: shared_rls (DB: bos_test_shared) ===
Created DB bos_test_shared (shared_rls): True
Registered Meta-Object: Vendor -> vendor
Physical Tables in bos_test_shared:
  -> vendor (BASE TABLE)

=== Testing Strategy: partitioned (DB: bos_test_part) ===
Created DB bos_test_part (partitioned): True
Registered Meta-Object: Vendor -> vendor
Physical Tables in bos_test_part:
  -> vendor (BASE TABLE, Partition Root)
  -> vendor_default (BASE TABLE, Default Partition)
  -> vendor_tenant_default (BASE TABLE, Tenant Disk Partition)

=== Testing Strategy: schema_per_company (DB: bos_test_schema) ===
Created DB bos_test_schema (schema_per_company): True
Registered Meta-Object: Vendor -> vendor
Physical Tables in bos_test_schema:
  -> vendor (BASE TABLE in schema comp_default)
  -> vendor (BASE TABLE in schema public)

=== Testing Strategy: table_per_company (DB: bos_test_tbl) ===
Created DB bos_test_tbl (table_per_company): True
Registered Meta-Object: Vendor -> vendor
Physical Tables in bos_test_tbl:
  -> default$vendor (BASE TABLE)
  -> vendor (BASE TABLE)

All 4 Storage Strategies Verified Successfully!
All 22 E2E Steps Executed and PASSED 100% Cleanly!
```
