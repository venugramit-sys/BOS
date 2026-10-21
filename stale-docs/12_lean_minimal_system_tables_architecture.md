# Architecture Decision Record (ADR): Lean Minimal System Tables & Single Source of Truth

**Document Version:** 1.0.0  
**Status:** APPROVED & LOCKED  
**Date:** October 2026  
**Authors:** DeepMind Antigravity Team  
**Scope:** Core Database Engine, Kernel Schema, Meta-Object Catalog, Dynamic Codegen  

---

## 1. Context & Problem Statement

In classical ERP and business application platforms (such as Dynamics NAV C/SIDE prior to the modern cloud era), systems relied on heavily normalized relational metadata tables:
- A table catalog (`Object`)
- A field catalog (`Field`)
- A relation catalog (`Table Relation`)
- A workflow state catalog

When initial scaffolding for BOS was drafted, legacy habits led to the creation of:
- `_bos_meta_fields`
- `_bos_meta_relations`
- `_bos_meta_state_machines`

### The Critical Flaws of Table Sprawl:
1. **Dual Source of Truth / Desynchronization Risk:**
   Every meta-object in BOS is compiled into a high-fidelity intermediate representation (`TableIR`, `StateMachineIR`, etc.) and saved as `_bos_meta_objects.definition_ir` (PostgreSQL `JSONB`). Creating separate relational tables for fields and relations created two competing sources of truth. Any schema migration or package upgrade required updating both JSONB and relational tables within multi-step transactions.
2. **Catalog Bloat:**
   A standard ERP system consists of hundreds of tables and thousands of fields. Inserting each column and constraint into relational rows severely slows down metadata compilation, package importing, and hot-reload.
3. **Database Pollution:**
   Developers and DBAs inspecting the database were presented with 10+ system tables before any business domain tables were even provisioned.

---

## 2. Locked Architectural Decision

> **DECISION LOCK:**  
> **Eliminate redundant metadata tables (`_bos_meta_fields`, `_bos_meta_relations`, `_bos_meta_state_machines`) entirely.**  
> The BOS System Kernel will maintain the **strict minimum** number of system tables.  
> **`_bos_meta_objects.definition_ir` (PostgreSQL JSONB) is the single authoritative source of truth** for all object structures (fields, types, constraints, relations, flowfields, and state machine transitions).

---

## 3. The Minimal System Kernel Specification (Core Tables)

The entire BOS system infrastructure is powered by only **5 core system tables** (plus the statutory audit log):

```
┌────────────────────────────────────────────────────────────────────────┐
│               BOS Lean Minimal System Kernel Footprint                 │
├──────────────────────────────┬─────────────────────────────────────────┤
│ System & Metadata Catalog    │ Multi-Tenancy & Company Isolation       │
│  • _bos_sys_info             │  • _bos_tenants                         │
│  • _bos_meta_packages        │  • _bos_companies                       │
│  • _bos_meta_objects         │                                         │
│                              │ Statutory Compliance & Ledger Security  │
│                              │  • _system_privileged_audit_log        │
│                              │  • bos_touch_updated_at()               │
└──────────────────────────────┴─────────────────────────────────────────┘
```

> **Architecture Decision Note (ADR 12 & ADR 24 Update):**  
> - Redundant relational tables (`_bos_meta_fields`, `_bos_meta_relations`, `_bos_meta_state_machines`) are **permanently eliminated**.  
> - `_bos_number_series` and `bos_get_next_number` are **permanently eliminated from the database kernel and Rust runtime**. Number series management is built 100% in BOS application packages (`bos.core.number_series`), not in Rust and not under `ctx`.
> - **Base Package Importation at Database Creation**: To ensure zero hardcoded application tables in the kernel, newly provisioned databases automatically import base packages (starting with `bos.security`, with extensible support for `bos.ledger` or user-specified packages) via `PackageManager::install_package`. This guarantees all security identities, roles, profiles, and permissions exist immediately upon database creation as first-class `.bos` meta-objects without violating minimal kernel invariants.

### 3.1 Table Catalog Reference

| Table Name | Primary Key | Purpose & Scope | Single Source of Truth For |
|---|---|---|---|
| **`_bos_sys_info`** | `id` | Kernel configuration, installed schema version, active storage strategy, and global platform flags. | System engine state & cluster topology. |
| **`_bos_meta_packages`** | `package_id` | Installed module and extension registry (`bos.core`, publisher, version, checksum). | Package identities and dependency manifests. |
| **`_bos_meta_objects`** | `object_id` | Authoritative meta-object registry for all packages and objects (`table`, `statemachine`, `codeunit`, `page`, `enum`, `interface`). | **All field schemas, foreign keys, constraints, and state transitions (via `definition_ir JSONB`).** |
| **`_bos_tenants`** | `tenant_id` | Cloud tenant boundary and organizational identity. | Tenant configurations and multi-tenant quotas. |
| **`_bos_companies`** | `company_id` | Legal operating entity / company boundary (`dataPerCompany`). | Company identities and legal ledger separation. |
| **`_system_privileged_audit_log`** | `id` | Tamper-evident immutable ledger audit log for statutory override compliance. | Statutory legal compliance and supervisory audit trails. |

---

## 4. Querying Metadata from `_bos_meta_objects.definition_ir`

PostgreSQL offers binary JSON (`JSONB`) with sub-millisecond retrieval and indexing capabilities that exceed normalized relational queries for hierarchical AST/IR structures.

### 4.1 Indexing Strategy
To ensure instantaneous lookups across fields, indices, and relations, `_bos_meta_objects` leverages a `GIN` path index:

```sql
CREATE INDEX IF NOT EXISTS "idx_bos_meta_objects_ir" 
ON "_bos_meta_objects" USING gin ("definition_ir" jsonb_path_ops);
```

### 4.2 Querying Columns Directly from JSONB
Instead of querying an auxiliary table:

```sql
-- Extract all columns and data types for a given table
SELECT 
    c->>'name' AS field_name,
    c->'data_type'->>'Primitive' AS field_type,
    (c->>'is_primary_key')::boolean AS is_primary_key,
    (c->>'is_nullable')::boolean AS is_nullable
FROM "_bos_meta_objects",
     jsonb_array_elements("definition_ir"->'columns') AS c
WHERE "object_name" = 'Vendor' AND "object_type" = 'table';
```

### 4.3 Querying Relations Directly from JSONB
```sql
-- Extract foreign key relations
SELECT 
    fk->>'name' AS fk_name,
    fk->'columns' AS local_columns,
    fk->>'foreign_table' AS foreign_table,
    fk->'foreign_columns' AS foreign_columns
FROM "_bos_meta_objects",
     jsonb_array_elements("definition_ir"->'foreign_keys') AS fk
WHERE "object_name" = 'PurchaseOrder';
```

### 4.4 Querying State Machines Directly from JSONB
```sql
-- Extract workflow state machine transitions
SELECT 
    "definition_ir"->'state_machine'->>'initial_state' AS initial_state,
    "definition_ir"->'state_machine'->'transitions' AS transitions
FROM "_bos_meta_objects"
WHERE "object_name" = 'PurchaseOrder';
```

---

## 5. Invariants for Developers

1. **NO Table Sprawl:** Never introduce sub-tables for child components of a meta-object. Any property of a meta-object belongs inside its IR definition in `_bos_meta_objects`.
2. **Number Series Built in BOS:** Never burn business sequencer tables into the database kernel or runtime context (`ctx`). Number series management is built cleanly in BOS (`bos.core.number_series`) using standard BOS tables and Codeunits.
3. **No Dual Writes:** Compiling a meta-object writes exactly one row to `_bos_meta_objects` and generates the physical application table DDL. No secondary inserts into auxiliary dictionary tables.
4. **Projection in Application Layer:** When REST API endpoints (e.g. `GET /api/meta/objects/:name`) return detailed field lists, the BOS Runtime extracts and projects the fields directly from `definition_ir` in memory.
