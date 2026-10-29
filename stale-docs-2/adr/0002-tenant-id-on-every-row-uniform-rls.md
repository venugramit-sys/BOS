# ADR 0002: Tenant ID on every row; uniform RLS generation across storage strategies
Status: Accepted. Date: 2026-10-24. Plan ref: sec 3.2; D1.

## Context
`crates/bosc-codegen-db/src/lib.rs:895-905` generates a permissive policy without FORCE that lets a NULL company through. `crates/bosc-ir/src/lib.rs:85` silently falls back for an unknown storage strategy. Layouts: `shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`.

## Decision
Every tenant-owned table, in **every** layout and profile:
```sql
tenant_id  uuid NOT NULL REFERENCES _bos_tenants(tenant_id),
company_id uuid NULL,             -- company-scoped tables only; NULL = tenant-wide, never "all"
-- indexes and unique keys lead with tenant_id
ALTER TABLE <t> ENABLE ROW LEVEL SECURITY;
ALTER TABLE <t> FORCE ROW LEVEL SECURITY;
CREATE POLICY bos_tenant_isolation ON <t> AS RESTRICTIVE FOR ALL
  USING      (tenant_id = current_setting('bos.current_tenant_id', true)::uuid)
  WITH CHECK (tenant_id = current_setting('bos.current_tenant_id', true)::uuid);
-- company-scoped tables add:
CREATE POLICY bos_company_isolation ON <t> AS RESTRICTIVE FOR ALL
  USING (company_id IS NULL AND <table is tenant-wide-readable>
         OR company_id = current_setting('bos.current_company_id', true)::uuid);
-- plus one PERMISSIVE policy (required so RESTRICTIVE policies have something to restrict):
CREATE POLICY bos_base ON <t> AS PERMISSIVE FOR ALL USING (true);
```
- `bos.current_tenant_id` / `bos.current_company_id` are set with `SET LOCAL` in each transaction, **only** from the verified session (ADR 0004), never from request headers in staging/production.
- `schema_per_company` and `table_per_company` *also* keep the columns and policies (defence in depth plus uniform migrations).
- Unknown strategy = **compile error**, not a fallback.
- An unset setting gives NULL, so no rows match (fail closed).

## Consequences
One codegen path and one set of tests. There's a small storage cost in the dedicated layouts.

## Alternatives considered
RLS only in `shared_rls` (diverging code paths); app-level filtering only (one bug leaks data).

## Reversibility
REVERSIBLE for the policy text (regenerate). COSTLY for removing columns later.

## Phase / enforcement
P0: generated identically in all profiles (development may connect as a superuser, which bypasses it). P1: staging uses `bos_app` (no BYPASSRLS). P2: production startup check verifies FORCE + RESTRICTIVE on every table.