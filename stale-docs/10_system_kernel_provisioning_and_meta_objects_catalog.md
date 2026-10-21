# BOS System Kernel Provisioning & Meta-Objects Catalog

**Document Version:** 1.0.0  
**Status:** Live & Production Ready  
**Date:** October 2026  
**Related Components:** [`crates/bosc-codegen-db/src/lib.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-codegen-db/src/lib.rs), [`crates/bosc-runtime/src/meta_manager.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-runtime/src/meta_manager.rs), [`crates/bosc-runtime/src/db_manager.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-runtime/src/db_manager.rs), [`crates/bosc-runtime/src/server.rs`](file:///d:/Work%20Studio/Bos/crates/bosc-runtime/src/server.rs), [`ui/src/App.svelte`](file:///d:/Work%20Studio/Bos/ui/src/App.svelte)

---

## 1. Executive Summary

In enterprise Business Operating Systems (BOS), databases cannot remain empty raw storage buckets. When a new database is provisioned, the BOS System Kernel must be initialized immediately at the time of database creation. This system foundation establishes:
1. Core PostgreSQL cryptographic and identity extensions (`uuid-ossp`, `pgcrypto`).
2. The core system catalog (`_bos_sys_info`, `_bos_meta_packages`).
3. The centralized meta-object repository (`_bos_meta_objects`), field data dictionary, relationship catalog, and workflow definitions (consolidated via JSONB `definition_ir`).
4. Multi-tenancy and company isolation registries (`_bos_tenants`, `_bos_companies`).
5. Statutory immutable audit logging (`_system_privileged_audit_log`).
6. Default system seeds (default tenant, default company, core package).

With this system kernel automatically provisioned during database creation, any subsequent creation or compilation of meta-objects (`table`, `statemachine`, `codeunit`, `enum`, etc.) dynamically persists its metadata into the system catalog and immediately compiles live PostgreSQL tables with system audit columns, Row-Level Security, indexes, and triggers.

---

## 2. Architecture & Lean System Kernel Schema (ADR 12 Locked)

```
┌────────────────────────────────────────────────────────────────────────┐
│               BOS Lean Minimal System Kernel Footprint                 │
├──────────────────────────────┬─────────────────────────────────────────┤
│ System & Metadata Catalog    │ Multi-Tenancy & Company Isolation       │
│  • _bos_sys_info             │  • _bos_tenants                         │
│  • _bos_meta_packages        │  • _bos_companies                       │
│  • _bos_meta_objects         │                                         │
│                              │ Compliance & Auditing                   │
│                              │  • _system_privileged_audit_log        │
│                              │  • bos_touch_updated_at()               │
└──────────────────────────────┴─────────────────────────────────────────┘
```

> **Architecture Decision Note (ADR 12):**  
> - Redundant relational tables (`_bos_meta_fields`, `_bos_meta_relations`, `_bos_meta_state_machines`) have been **permanently eliminated**. `_bos_meta_objects.definition_ir` (PostgreSQL JSONB with GIN indexing) serves as the Single Source of Truth for all columns, relations, and state machine definitions.  
> - `_bos_number_series` and `bos_get_next_number` have been **permanently eliminated from the database kernel and Rust runtime**. Number Series management is built 100% in BOS application packages (`bos.core.number_series`) using standard BOS tables and Codeunits, completely decoupled from Rust and `ctx`.

### 2.1 The Core Kernel Tables

| Table Name | Primary Key | Description |
|---|---|---|
| `_bos_sys_info` | `id` | Stores kernel version (`0.1.0`), schema version (`2026.10`), status, and storage strategy (`shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`). |
| `_bos_meta_packages` | `package_id` | Installed module/package registry (`bos.core`, publisher, version, status). |
| `_bos_meta_objects` | `object_id` | **Unified meta-object repository**: object type (`table`, `statemachine`, `codeunit`, `enum`, `page`), name, package name, table type, **complete AST/IR in `definition_ir` (JSONB)**, and checksum. |
| `_bos_tenants` | `tenant_id` | Multi-tenant isolation registry. Seeded with default tenant `00000000-0000-0000-0000-000000000000`. |
| `_bos_companies` | `company_id` | Legal operating company boundary (`dataPerCompany`). Seeded with default company. |
| `_system_privileged_audit_log`| `id` | Immutable statutory audit trail for legal overrides and ledger maintenance. |

### 2.2 Stored Functions & Triggers

1. **`bos_touch_updated_at() RETURNS trigger`**:
   Automatically updates `_updated_at = NOW()` and increments `_system_version = _system_version + 1` for optimistic concurrency.

---

## 3. Database Creation Provisioning Pipeline

When an administrator or API client invokes `POST /api/databases`:
```json
{
  "name": "bos_finance_db",
  "init_extensions": true,
  "provision_system": true
}
```

The pipeline executes the following sequence:
1. Validates the database name (`DatabaseManager::validate_db_name`).
2. Connects to the maintenance catalog (`postgres`) and executes:
   ```sql
   CREATE DATABASE "bos_finance_db";
   ```
3. Connects directly to the new database `bos_finance_db`.
4. Provisions extensions:
   ```sql
   CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
   CREATE EXTENSION IF NOT EXISTS "pgcrypto";
   ```
5. Executes the complete system kernel DDL (`PostgresGenerator::generate_system_kernel_ddl()`):
   - Provisions the core system tables (`_bos_sys_info`, `_bos_meta_packages`, `_bos_meta_objects`, `_bos_tenants`, `_bos_companies`, `_system_privileged_audit_log`).
   - Installs `bos_touch_updated_at` helper trigger.
   - Inserts initial system seeds (Default Tenant, Default Company, `bos.core` package).
6. Closes the initialization connection and reports success.

---

## 4. Meta-Object Dynamic Registration Engine

When meta-objects are created via `POST /api/meta/objects`:
1. Validates the meta-object name and type.
2. If `object_type == "table"`:
   - Converts the incoming column definitions into `ColumnIR`.
   - Constructs `TableIR` with system columns (`_id`, `_tenant_id`, `_company_id`, `_created_at`, `_created_by`, `_updated_at`, `_updated_by`, `_system_version`).
   - Generates live DDL including Row-Level Security policies (`ALTER TABLE ... ENABLE ROW LEVEL SECURITY`), indexes, and triggers (`guard_immutable_*` if Ledger).
   - If a state machine is defined, generates the `{table}_state_history` table and index.
   - Executes the DDL directly on PostgreSQL.
3. Inserts or updates the meta-object into `_bos_meta_objects` with full typed JSONB `definition_ir` (containing columns, constraints, foreign keys, and state machine) and SHA-256 checksum.
4. GIN index on `definition_ir` provides instant sub-millisecond AST queries without redundant auxiliary dictionary tables (ADR 12).
5. The entity is immediately available for CRUD operations via `/api/entities/{table}` and state transitions via `/api/entities/{table}/{id}/transition`.

---

## 5. REST API Reference

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/api/databases` | Create a new database with optional extensions and automatic BOS System Kernel provisioning (`provision_system: true`). |
| `POST` | `/api/meta/provision` | Provision or upgrade the BOS System Kernel on the active database on-demand. |
| `GET` | `/api/meta/system-status` | Get kernel status: `is_provisioned`, versions, meta-objects count, and tenants count. |
| `POST` | `/api/meta/objects` | Register a new meta-object (`table`, `statemachine`, etc.) and dynamically create PostgreSQL tables. |
| `GET` | `/api/meta/objects` | List all registered meta-objects with optional `?type=table` filter. |
| `GET` | `/api/meta/objects/{name}` | Inspect meta-object details, field dictionary, and relations graph. |

---

## 6. Svelte 5 Testing Studio Enhancements

[`ui/src/App.svelte`](file:///d:/Work%20Studio/Bos/ui/src/App.svelte) has been upgraded with:
1. **Creation Form:** "Provision BOS System Kernel" toggle (checked by default).
2. **Database Cards:** Shows `KERNEL v0.1.0` badge for provisioned databases or `RAW DATABASE` badge.
3. **Inspector Status:** Displays kernel version and registered meta-objects count with one-click "⚡ Provision BOS Kernel" button.
4. **Meta-Objects Registry Panel:**
   - Dynamic meta-object compilation and registration form (`Vendor`, `Item`, etc.).
   - Live dictionary table displaying fields, SQL types, and constraints.
5. **Automated Integration Suite:** Runs the full end-to-end test including provisioning, meta-object registration, and entity verification.
