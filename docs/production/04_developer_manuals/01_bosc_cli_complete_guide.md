# Volume 4: Developer Manuals
## 01. BOS Compiler CLI (`bosc`) Complete Guide

The `bosc` binary is the unified command-line tool for compiling scripts, validating schemas, executing procedures, building packages, running server nodes, and provisioning database kernels.

---

## 🛠️ Global Syntax

```bash
bosc <COMMAND> [OPTIONS]
```

### Top-Level Commands Overview

| Command | Description |
| :--- | :--- |
| `build` | Compiles `.bos` source files into PostgreSQL DDL or JSON Bytecode IR. |
| `run` | Executes a Codeunit procedure directly against a PostgreSQL database. |
| `deploy` | Compiles and synchronizes the schema of a `.bos` script into PostgreSQL. |
| `test-live` | Executes verification tests against live PostgreSQL infrastructure. |
| `detect-db` | Probes locally installed and running database providers. |
| `provision` | Provisions the BOS System Kernel onto an existing PostgreSQL database. |
| `create-db` | Creates a new PostgreSQL database and initializes the kernel and base packages. |
| `package` | Manages `.bospkg` package archives (`build`, `install`, `list`). |
| `node` | Manages server node profiles (`init`, `check`, `run`). |
| `lsp` | Launches the Language Server Protocol (LSP) daemon for IDE autocomplete. |

---

## 📖 Command Deep-Dive

### 1. `bosc build`
Compile `.bos` files into PostgreSQL DDL or Intermediate Representation:
```powershell
# Compile to DDL SQL script
bosc build --input packages/acme.sales/tables.bos --output dist/schema.sql

# Emit Intermediate Representation (JSON AST / IR)
bosc build --input packages/acme.sales/tables.bos --output dist/ir.json --emit-ir
```

### 2. `bosc deploy`
Synchronize declarative schemas directly into a target database in a single atomic transaction:
```powershell
# Safe deployment (additive changes only)
bosc deploy --input schema.bos --db postgres://postgres:postgres@localhost:5432/bos_dev --package "acme.sales"

# Dry run: prints planned schema migrations without applying them
bosc deploy --input schema.bos --db postgres://postgres:postgres@localhost:5432/bos_dev --dry-run

# Permitting destructive changes (dropped columns, narrowed types)
bosc deploy --input schema.bos --db postgres://postgres:postgres@localhost:5432/bos_dev --allow-destructive
```

### 3. `bosc run`
Execute a specific Codeunit procedure directly from the CLI:
```powershell
bosc run \
  --codeunit "PurchasePost" \
  --method "postOrder" \
  --params '{"orderId": "018f4a12-8800-7214-9bf2-b430c5e7b411"}' \
  --db postgres://postgres:postgres@localhost:5432/bos_dev
```

### 4. `bosc package` Subcommands
```powershell
# Build a .bospkg package archive from directory containing bos.json
bosc package build packages/acme.sales/ --output dist/acme.sales.bospkg

# Install a .bospkg package into database
bosc package install dist/acme.sales.bospkg --db postgres://postgres:postgres@localhost:5432/bos_dev

# List installed packages in the database catalog
bosc package list --db postgres://postgres:postgres@localhost:5432/bos_dev
```

### 5. `bosc node` Subcommands
```powershell
# Initialize new configuration profile
bosc node init --output nodes/staging.toml

# Validate configuration profile and DB connectivity
bosc node check --config nodes/staging.toml

# Start the server node daemon
bosc node run --config nodes/staging.toml
```
