# Server Node & NAS (Navision Application Server) Architecture

> **Document ID:** `BOS-DOC-13`  
> **Source Verification:** `crates/bosc-runtime/src/node_config.rs`, `crates/bosc-runtime/src/nas_worker.rs`, `crates/bosc-cli/src/main.rs`  
> **Target Audience:** Systems Architects, DevOps Engineers, Enterprise Administrators

---

## 1. Overview & Architectural Motivation

In traditional 2-tier client-server architectures, client applications connect directly to the database. This exposes sensitive database connection strings, database port numbers, and requires direct network connectivity to the database cluster, creating severe security vulnerabilities and scalability bottlenecks.

BOS adopts the proven 3-tier enterprise ERP topology established by **Microsoft Dynamics NAV / Business Central** (NAV Server Tier / Navision Application Server) and **SAP NetSuite**:

```mermaid
flowchart TD
    subgraph Clients ["Client Layer (Zero Database Knowledge)"]
        Browser["Web Browser (Svelte 5 UI Studio)"]
        Mobile["Mobile / Barcode Scanner App"]
        API["3rd-Party Integration Client"]
    end

    subgraph NodeLayer ["BOS Server Node Layer (NST / NAS)"]
        Node1["Node: Interactive Web Server<br/>(Port 3000 / UI + API)"]
        Node2["Node: Dedicated NAS Background Worker<br/>(Headless Batch & Outbox Processor)"]
        Node3["Node: High-Throughput API Gateway<br/>(Port 3002 / External Integrations)"]
    end

    subgraph DBLayer ["Protected Database Tier (Private Subnet / VPC)"]
        DB[("PostgreSQL 16+<br/>(Multi-Tenant / Immutable Ledgers)")]
    end

    Browser -- "User ID & Password only" --> Node1
    Mobile -- "User ID & Password only" --> Node1
    API -- "OAuth / Service Credentials" --> Node3

    Node1 -->|Pool: 20 conns| DB
    Node2 -->|Pool: 5 conns| DB
    Node3 -->|Pool: 40 conns| DB
```

### 1.1. Core Invariants
1. **Client Isolation**: The client application *never* receives or requires database hostnames, ports, credentials, or SQL dialects. The client only provides **User ID** and **Password** to the designated server node endpoint.
2. **Node Encapsulation**: All infrastructure parameters (DB URL, pool sizing, tenant routing defaults, background NAS workers, static UI delivery) are configured declaratively within the server node profile (`bos-node.toml`).
3. **Arbitrary Scalability**: Administrators can instantiate an arbitrary number of server nodes on a single physical host, across distinct ports, or distributed across container clusters.
4. **Workload Segregation**: Interactive user operations (Web UI) and heavy asynchronous batch processing (NAS workers) can be physically decoupled onto distinct nodes sharing the same database.

---

## 2. Server Node Configuration Schema (`bos-node.toml`)

Every BOS server node instance is driven by a TOML configuration profile.

```toml
[node]
id = "node-main-01"
name = "BOS Enterprise Primary Node"
# Node roles:
# - "all_in_one": Serves Svelte 5 UI, REST/GraphQL API, and runs background NAS worker
# - "interactive": Serves Svelte 5 UI and REST/GraphQL API (no background batch worker)
# - "nas_worker": Headless background daemon (processes job queue & outbox, no UI)
# - "api_gateway": Headless API-only node optimized for external integrations
role = "all_in_one"

[network]
host = "0.0.0.0"
port = 3000
# Optional allowed CORS origins
cors_origins = ["*"]

[ui]
# If enabled, this node serves the compiled Svelte 5 single-page application
enabled = true
static_dir = "./ui/dist"
index_file = "index.html"

[database]
# Securely encapsulated on the node: client never sees this connection string
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
min_pool_size = 5
max_pool_size = 30
# Storage strategy: "shared_rls" | "schema_per_tenant" | "db_per_tenant"
storage_strategy = "shared_rls"
# Connection timeout in seconds
connection_timeout_seconds = 10

[defaults]
# When client logs in with user id & password, default to these tenants/companies if not specified
default_tenant = "DEFAULT_TENANT"
default_company = "MAIN_COMPANY"

[nas_worker]
# Background Application Server (NAS) Worker Configuration
enabled = true
poll_interval_ms = 1000
batch_size = 50
process_outbox = true
process_job_queue = true

[security]
# Maintenance & Emergency Fallback Controls
# If maintenance_mode is true, standard login is bypassed and super admin session is created immediately
maintenance_mode = false
emergency_username = "emergency_admin"
emergency_name = "Emergency Super Admin"
emergency_roles = ["SUPER", "Admin"]
# If true and the .bos Auth module is uncompiled or broken, the system grants an emergency super admin
# session so the developer can log in to Dev Studio (/studio) and fix the broken script.
auto_fallback_on_auth_failure = true
```

---

## 3. Node Operational Roles

### 3.1. Interactive Node (`interactive` / `all_in_one`)
- Serves the frontend single-page application (SPA) directly from memory/disk using high-performance static file serving (`tower_http::services::ServeDir`).
- Resolves all `/api/*` endpoints on the identical port, completely eliminating CORS preflight overhead.
- Fallback route returns `index.html` for client-side HTML5 history routing (`/app/customers`, `/app/orders`).

### 3.2. NAS Background Worker Node (`nas_worker`)
- Runs headlessly (daemon mode) without listening for public UI traffic.
- Spawns an internal Tokio background task loop that continuously monitors:
  1. `_bos_event_outbox`: Transactional outbox pattern for asynchronous event distribution and webhook dispatch.
  2. Scheduled codeunit tasks: Batch posting routines, recurring invoice generation, bank reconciliation sync.
- Guarantees that heavy batch computations never degrade sub-50ms interactive latency for front-line business users.

### 3.3. API Gateway Node (`api_gateway`)
- Optimized for machine-to-machine integrations (Shopify, CRM, automated EDI).
- Configured with separate connection pool limits and rate limiters to protect the transactional core.

---

## 4. Authentication Flow (User ID & Password Only)

### 4.1. 100% .BOS Module Authentication Topology
BOS authenticates users against the **`bos.auth`** package (`packages/bos.auth/package.bos`), which defines:
- `table User`: First-class user record with fields `username` (PK), `passwordHash`, `fullName`, `email`, `roles`, `isActive`.
- `table Role` & `table UserRole`: Dynamic role and permission assignments.
- `table Session`: Active sessions mapped with auto-expiring TTLs.
- `codeunit AuthManager`: Business logic for password hashing, credential verification, and session lifecycle.

### 4.2. Compile-Time Guard Rails (`bosc-analyzer`)
To prevent developers from accidentally authoring broken authentication modules, the compiler enforces compile-time semantic validation:
- **`BOS4001`**: Checks that any table designated as `User` defines a primary key (`username`).
- **`BOS4002`**: Checks that the `User` table defines credential storage (`passwordHash` or `password`).
- Compilation fails with actionable hints before broken code can ever reach production databases.

### 4.3. Resilient Emergency Fallback & Maintenance Mode
If a developer breaks the `.bos` Auth script during development, or if maintenance mode is enabled:
1. **Maintenance Mode (`maintenance_mode = true`)**:
   - The Server Node indicates maintenance mode via `GET /api/node/info`.
   - On login, authentication is bypassed and an emergency Super Admin session (`SUPER`, `Admin`) is issued immediately.
   - The UI surfaces a maintenance indicator and top banner with a direct link to **Dev Studio (`/studio`)** to correct the broken script.
2. **Auto-Fallback (`auto_fallback_on_auth_failure = true`)**:
   - If the `.bos` `user` table is missing, corrupt, or throws a database schema error during authentication, the node catches the error and issues an emergency Super Admin session with a persistent warning toast and banner.
   - Developers are never permanently locked out of their environment.
3. **Package Governance**:
   - Identity, credential verification, and session state are managed entirely within modular `.bos` packages (such as `bos.auth`), preserving a lean, unbloated Rust kernel with zero hardcoded auth tables or mock users.

### 4.4. Package-Governed Identity Lifecycle
1. User registration, authentication, and token management are orchestrated via the `bos.auth` package meta-objects (`table User`, `table UserSession`, `codeunit AuthManager`).
2. Verification and session state transition via standard JSON-RPC execution (`/api/v1/rpc/execute` or `/api/codeunits/AuthManager/run`).
3. Client requests pass tenant, company, and session context via headers (`x-bos-session`, `x-bos-tenant-id`, `x-bos-company-id`), eliminating kernel database auth lock-in.

---

## 5. CLI Node Management Commands

BOS CLI (`bosc`) provides full lifecycle management for server nodes:

```bash
# Generate a new default node configuration template
bosc node init --output ./config/nodes/node-prod.toml

# Start a server node with the given configuration profile
bosc node run --config ./config/nodes/node-prod.toml

# Validate node configuration and test database connectivity without starting the server
bosc node check --config ./config/nodes/node-prod.toml
```
