# Volume 1: Getting Started
## 03. First Workspace & Project Setup

This guide explains how to structure your project workspace, configure your first server node profile, and prepare your development environment for building enterprise business applications.

---

## 📁 Repository & Directory Layout

A standard BOS development workspace contains the following top-level directories:

```
my-bos-workspace/
├── .bos-node.toml          # Default server node profile
├── nodes/                  # Named server node configuration profiles
│   ├── dev.toml            # Local development node profile
│   └── prod.toml           # Production deployment node profile
├── packages/               # Modular business packages (.bos and bos.json)
│   ├── bos.core/ # Unified Core Platform, Identity & RBAC package           # Identity, RBAC and Security package
│   └── acme.sales/         # Custom business package
├── crates/                 # Rust core engines and compiler
├── ui/                     # Svelte 5 Developer Studio & Page Kit
└── docs/                   # Documentation and architectural blueprints
```

---

## ⚙️ Configuring the Server Node (`bos-node.toml`)

The BOS Server Node acts as the unified application server (equivalent to Microsoft Dynamics NAV NST/NAS). Its behavior is controlled by declarative TOML configuration files.

### Generating a New Node Configuration
Generate a clean template with the `bosc` CLI:

```powershell
bosc node init --output nodes/dev.toml --force
```

### Inspecting `nodes/dev.toml`
Open `nodes/dev.toml` to customize your node settings:

```toml
[node]
id = "dev-primary-01"
name = "Local Development Node"
environment = "development"

[database]
# PostgreSQL connection string
url = "postgres://postgres:postgres@localhost:5432/bos_dev"
# Pool configuration
max_connections = 10
min_connections = 2
idle_timeout_seconds = 300
storage_strategy = "shared_rls"

[server]
# Bind address and port
bind_address = "0.0.0.0"
port = 3000
enable_cors = true
allowed_origins = ["http://localhost:5173", "http://127.0.0.1:5173"]

[nas_worker]
# Background asynchronous worker daemon
enabled = true
poll_interval_ms = 1000
batch_size = 50
max_concurrent_jobs = 4

[quickjs]
# Sandboxed JavaScript VM limits
memory_limit_mb = 8
max_fuel_instructions = 100000
```

---

## 🧪 Validating the Node Profile

Before running the server, test that the configuration is syntactically valid and that the database connection succeeds:

```powershell
bosc node check --config nodes/dev.toml
```

**Output:**
```
✓ Node configuration loaded successfully: dev-primary-01
✓ Database connection to postgres://... verified (PostgreSQL 16.2)
✓ Storage strategy validated: shared_rls
✓ System Kernel tables verified: _bos_meta_objects, _bos_sys_info
Node check passed!
```

---

## 🏁 Starting the Server Node

Run the server node in foreground mode:

```powershell
bosc node run --config nodes/dev.toml
```

The server is now listening for incoming HTTP requests on `http://localhost:3000` and polling for background NAS tasks.
