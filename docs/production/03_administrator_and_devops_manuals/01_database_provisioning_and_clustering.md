# Volume 3: Administrator & DevOps Manuals
## 01. Database Provisioning & Storage Strategies

This manual provides enterprise database administrators with procedures for probing database infrastructure, provisioning new tenant databases, and configuring multi-tenant isolation topologies.

---

## 🔎 Step 1: Probing Local Database Providers

The `bosc` CLI includes a real-time detector utility that discovers and tests connectivity to locally installed database engines (PostgreSQL, MySQL, SQL Server, Redis, ClickHouse, and SQLite):

```powershell
bosc detect-db
```

**Diagnostic Output:**
```
[Probing Database Providers...]
✓ PostgreSQL 16.2 running on localhost:5432 (Status: Online, Auth: SCRAM-SHA-256)
✗ MySQL (port 3306): Not reachable
✗ SQL Server (port 1433): Not reachable
✓ Redis 7.2 running on localhost:6379 (Status: Online)
✓ SQLite 3.45: Engine ready
```

---

## 🏗️ Step 2: Creating a New Database (`create-db`)

To create a new PostgreSQL database and initialize the BOS System Kernel:

```powershell
bosc create-db `
  --name bos_production `
  --cluster-url postgres://postgres:SecretPassword@localhost:5432/postgres `
  --strategy shared_rls `
  --tenant "TENANT_ACME" `
  --company "ACME_CORP_IN" `
  --packages "bos.core"
```

### CLI Parameters Explained:
- `--name`: Name of the new PostgreSQL database to create.
- `--cluster-url`: Administrative connection string to the PostgreSQL cluster (connecting to the maintenance database `postgres`).
- `--strategy`: Multi-tenancy isolation model:
  - `shared_rls`: Single shared schema with PostgreSQL Row-Level Security on `tenant_id` (recommended for multi-tenant SaaS).
  - `schema_per_company`: Dedicated PostgreSQL schema per company with shared kernel tables (recommended for regulated entities).
  - `table_per_company`: Prefixed tables per company (recommended for NAV/BC migrations).
- `--tenant`: Initial primary tenant identifier (UUID or alphanumeric code).
- `--company`: Initial primary company name.
- `--packages`: Comma-separated list of package bundles to auto-import upon creation.
- `--no-base-packages`: Optional flag to skip importing default authentication/security packages.

---

## ⚙️ Step 3: Provisioning an Existing Database (`provision`)

If an empty database was already created manually by a cloud DBA (e.g., AWS RDS, Azure Database for PostgreSQL, DigitalOcean Managed DB), initialize the BOS kernel with:

```powershell
bosc provision `
  --db postgres://bos_user:SecretPassword@db-cluster.internal:5432/bos_production `
  --strategy shared_rls
```

### Installed Kernel Tables:
1. `_bos_sys_info`: Version information, schema version, cluster node locks.
2. `_bos_meta_packages`: Registry of installed packages, checksums, and semver.
3. `_bos_meta_objects`: Unified GIN-indexed catalog of all compiled meta-objects.
4. `_bos_tenants`: Tenant directory with subscription state and quotas.
5. `_bos_companies`: Registered legal companies linked to tenants.
6. `_bos_audit_log`: Cryptographic SHA-256 hash-chained audit ledger.
7. `_bos_event_outbox`: Transactional outbox table for decoupled events.
