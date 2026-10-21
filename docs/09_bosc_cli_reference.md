# BOS Compiler CLI (`bosc`) Manual

> **Document ID:** `BOS-DOC-09`  
> **Source Verification:** `crates/bosc-cli/src/main.rs`  
> **Target Audience:** DevOps Engineers, CLI Developers, System Administrators

---

## 1. Overview & Installation

The **`bosc`** command-line interface is the primary tool for compiling BOS-Script sources, packaging business applications, synchronizing live PostgreSQL database schemas, and running the HTTP backend API server.

### Building & Running the Binary
```powershell
# Build optimized release binary
cargo build --release -p bosc-cli

# Run via Cargo
cargo run -p bosc-cli -- --help
```

---

## 2. Command Hierarchy & Subcommands

```
bosc [COMMAND]

Commands:
  build        Compile BOS script files into PostgreSQL DDL or JSON Bytecode IR
  run          Execute a Codeunit procedure bytecode against PostgreSQL database
  deploy       Compile and synchronize the schema of a BOS script into PostgreSQL (one transaction)
  test-live    Run verification tests against live PostgreSQL infrastructure
  detect-db    Detect locally installed and running database providers
  serve        Start the live HTTP & JSON-RPC Backend API Server
  provision    Provision BOS System Kernel on a PostgreSQL database
  create-db    Create a new database in PostgreSQL cluster with specified options
  package      Manage BOS Packages (.bospkg bundles)
  help         Print this message or the help of the given subcommand(s)
```

---

## 3. Detailed Command Reference

### 3.1. `bosc build`
Compiles BOS-Script files (`.bos`, `.bl.bos`) into PostgreSQL DDL or intermediate JSON representation.
- **Flags & Options:**
  - `-i, --input <PATH>`: Source file to compile.
  - `-o, --output <PATH>`: Target output file (default: stdout).
  - `--emit-ir`: Emits JSON `ModelIR` instead of SQL DDL.
- **Examples:**
  ```powershell
  # Compile to SQL DDL
  bosc build -i examples/dispatch_validator.bos -o dispatch.sql

  # Emit Intermediate Representation (IR)
  bosc build -i examples/dispatch_validator.bos --emit-ir -o dispatch.ir.json
  ```

---

### 3.2. `bosc deploy`
Compiles and applies schema updates to a live database in a single atomic transaction.
- **Flags & Options:**
  - `-i, --input <PATH>`: Source BOS script file.
  - `-d, --db <URL>`: Target PostgreSQL connection URL (default: `postgres://postgres:postgres@localhost:5432/bos_dev`).
  - `--package <ID>`: Owning package ID (default: `bos.app`).
  - `--allow-destructive`: Permits destructive schema changes (dropping columns, narrowing data types).
  - `--dry-run`: Computes and logs planned migration steps, then rolls back.
- **Examples:**
  ```powershell
  # Preview migration without applying changes
  bosc deploy -i schema.bos --dry-run

  # Apply safe schema changes
  bosc deploy -i schema.bos -d postgres://postgres:secret@localhost:5432/production_db

  # Force schema changes including column drops
  bosc deploy -i schema.bos --allow-destructive
  ```

---

### 3.3. `bosc serve`
Starts the Axum 0.8 HTTP API Server.
- **Flags & Options:**
  - `-d, --db <URL>`: Connection string to PostgreSQL database.
  - `-p, --port <PORT>`: HTTP port to listen on (default: `3000`).
- **Example:**
  ```powershell
  bosc serve --port 3000 --db postgres://postgres:postgres@localhost:5432/bos_dev
  ```

---

### 3.4. `bosc run`
Executes an exposed codeunit procedure against the database.
- **Flags & Options:**
  - `-c, --codeunit <NAME>`: Target codeunit identifier.
  - `-m, --method <NAME>`: Method name to execute.
  - `-p, --params <JSON>`: JSON arguments object (default: `{}`).
  - `-f, --file <PATH>`: Optional BOS source file defining the codeunit if not yet in database.
  - `-d, --db <URL>`: Target database URL.
- **Example:**
  ```powershell
  bosc run -c FleetValidator -m ValidateFleetDispatch -p '{"vin":"1HGCR2F83HA000000","capacityTons":15}' -f examples/dispatch_validator.bos
  ```

---

### 3.5. `bosc detect-db`
Probes local ports and returns all running database server instances on the host:
```powershell
bosc detect-db
```
- **Sample Output:**
  ```
  Found running PostgreSQL cluster on port 5432
  Detected Redis instance on port 6379
  ```

---

### 3.6. `bosc provision`
Provisions the BOS System Kernel catalogs (`_bos_sys_info`, `_bos_meta_packages`, `_bos_meta_objects`, etc.) on a fresh database.
- **Flags & Options:**
  - `-d, --db <URL>`: Target PostgreSQL connection string.
  - `-s, --strategy <STRATEGY>`: Storage strategy (`shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`). Default: `shared_rls`.
- **Example:**
  ```powershell
  bosc provision -d postgres://postgres:postgres@localhost:5432/fresh_db --strategy shared_rls
  ```

---

### 3.7. `bosc create-db`
Creates a brand-new database in the PostgreSQL cluster, initializes UUID/pgcrypto extensions, provisions system kernel tables, and configures default tenant and company records.
- **Flags & Options:**
  - `-n, --name <NAME>`: Name of the database to create.
  - `-c, --cluster-url <URL>`: Cluster administrative URL (default: `postgres://postgres:postgres@localhost:5432/postgres`).
  - `-s, --strategy <STRATEGY>`: Storage strategy (`shared_rls`, etc.).
  - `--tenant <CODE>`: Default tenant code (default: `DEFAULT_TENANT`).
  - `--company <CODE>`: Default company code (default: `MAIN_COMPANY`).
  - `--no-base-packages`: Skips importing default packages (`bos.security`).
  - `-p, --packages <LIST>`: Additional package IDs or files to import.
- **Example:**
  ```powershell
  bosc create-db -n enterprise_prod --strategy shared_rls --tenant ACME_HOLDINGS --company ACME_USA
  ```

---

### 3.8. `bosc package` (Modular Packaging Tooling)

#### `bosc package build`
Compiles a `.bos` script into a verified `.bospkg` JSON package bundle with SHA-256 integrity checksums:
```powershell
bosc package build -i examples/security_module.bos -o packages/bos.auth.bospkg --package-id bos.auth --name "Authentication & RBAC" --version 1.0.0 --publisher "BOS Core Engineering"
```

#### `bosc package install`
Installs a `.bospkg` bundle into a target database:
```powershell
bosc package install -i packages/bos.auth.bospkg -d postgres://postgres:postgres@localhost:5432/bos_dev
```

#### `bosc package list`
Lists all installed packages registered in the `_bos_meta_packages` catalog:
```powershell
bosc package list -d postgres://postgres:postgres@localhost:5432/bos_dev
```
