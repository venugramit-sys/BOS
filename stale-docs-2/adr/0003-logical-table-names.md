# ADR 0003: Logical table names mapped by the kernel
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.3; D1.

## Context
Hard-coded physical names such as `comp_default` and `default$` (proposals sec 0) tie package code to one layout.

## Decision
- Packages and `.bos` code refer only to **logical names** `<package_id>.<ObjectName>`, e.g. `bos.sales.SalesOrder`. Inside its own package a short name (`SalesOrder`) is allowed and resolved at compile time.
- The kernel resolves physical names from:
```sql
CREATE TABLE _bos_object_map (
  map_version     int  NOT NULL DEFAULT 1,
  logical_name    text NOT NULL,          -- 'bos.sales.SalesOrder'
  tenant_id       uuid NULL,              -- NULL for pooled layouts
  company_id      uuid NULL,              -- set for table_per_company / schema_per_company
  layout          text NOT NULL CHECK (layout IN ('shared_rls','partitioned','schema_per_company','table_per_company')),
  physical_schema text NOT NULL,
  physical_table  text NOT NULL,
  object_version  text NOT NULL,          -- SemVer of the defining package
  PRIMARY KEY (logical_name, tenant_id, company_id)
);
```
- Physical naming rule: `pkg_<package_id with . -> _>__<object_snake>`, e.g. `pkg_bos_sales__sales_order`. Company tables append `__c_<company short uuid>`. Names longer than 63 bytes are cut and suffixed with 8 hex characters of SHA-256 of the full name.
- Package code never builds SQL identifiers. The kernel quotes all identifiers.

## Consequences
The storage layout can change without changing packages. There's one map lookup, cached per connection.

## Alternatives considered
Physical names in manifests (locks the layout); views per tenant (too many objects).

## Reversibility
COSTLY: physical names become migration state. The logical name format is frozen.

## Phase / enforcement
P0: map and resolver used by codegen; no new hard-coded physical names.