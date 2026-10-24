# 18. BOS QA Test Task Matrix & Feature Decomposition Catalog

> Automatically generated and maintained by the **QA Tester Agent (`tester`)**.

## 📊 Executive Summary

- **Total Granular Test Tasks:** `36`
- **By Category:** **UNIT:** 12 | **INTEGRATION:** 10 | **API_CONTRACT:** 4 | **SECURITY_RLS:** 1 | **CONCURRENCY_OCC:** 2 | **UX_VISUAL:** 3 | **UI_INTERACTION:** 4
- **By Priority:** **CRITICAL:** 15 | **HIGH:** 19 | **MEDIUM:** 2

---

## 🧩 Modules & Granular Test Tasks Breakdown

### bosc-syntax (AST & Lexer) (Compiler DSL)
*Directory: `crates/bosc-syntax`*

#### DSL Meta-Object Grammar Parsing

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`SYN-01`** | `CRITICAL` | `UNIT` | Parse 12 Meta-Object Declarations | `cargo test -p bosc-syntax test_meta_object_parsing` |
| **`SYN-02`** | `HIGH` | `UNIT` | Parse Non-Invasive Table & Page Extensions | `cargo test -p bosc-syntax test_extension_parsing` |
| **`SYN-03`** | `HIGH` | `UNIT` | Parse Field Validation Blocks validate(val) | `cargo test -p bosc-syntax test_field_validation_syntax` |
| **`SYN-04`** | `HIGH` | `UNIT` | Parse Declarative State Machines & Transitions | `cargo test -p bosc-syntax test_statemachine_syntax` |

### bosc-analyzer (Semantic Analyzer) (Compiler DSL)
*Directory: `crates/bosc-analyzer`*

#### Semantic Type & Symbol Checking

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`ANA-01`** | `CRITICAL` | `UNIT` | Validate Table Primary Key Constraints | `cargo test -p bosc-analyzer test_pk_constraints` |
| **`ANA-02`** | `CRITICAL` | `UNIT` | Validate FlowField Formula References & Cycle Detection | `cargo test -p bosc-analyzer test_flowfield_cycles` |
| **`ANA-03`** | `HIGH` | `UNIT` | Validate Extensible Enum Strategy Mapping | `cargo test -p bosc-analyzer test_enum_extension_bindings` |
| **`ANA-04`** | `HIGH` | `UNIT` | Validate Non-Invasive Field Conflict Protection | `cargo test -p bosc-analyzer test_field_name_collisions` |

### bosc-ir (IR & Type Stripper) (Compiler DSL)
*Directory: `crates/bosc-ir`*

#### TypeScript Zero-Overhead Type Stripping

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`IR-01`** | `CRITICAL` | `UNIT` | Strip TypeScript Types from BOS-Script to JS | `cargo test -p bosc-ir test_typescript_stripping` |
| **`IR-02`** | `CRITICAL` | `UNIT` | Compile Declarative AST to Relational IR Schema | `cargo test -p bosc-ir test_ir_lowering` |

### bosc-codegen-db (Database Generator) (Database Engine)
*Directory: `crates/bosc-codegen-db`*

#### PostgreSQL DDL & FlowField Views

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`DB-01`** | `CRITICAL` | `INTEGRATION` | Generate PostgreSQL Tables with RLS Policies | `cargo test -p bosc-codegen-db test_ddl_generation` |
| **`DB-02`** | `HIGH` | `INTEGRATION` | Generate Stored Generated Formula Columns | `cargo test -p bosc-codegen-db test_generated_columns` |
| **`DB-03`** | `HIGH` | `INTEGRATION` | Generate SIFT Pushdown Aggregation Views | `cargo test -p bosc-codegen-db test_sift_views` |
| **`DB-04`** | `CRITICAL` | `INTEGRATION` | Generate Immutable Ledger Reversal Triggers | `cargo test -p bosc-codegen-db test_immutable_ledger_triggers` |

### bosc-runtime (Execution Engine & API Server) (Core Runtime)
*Directory: `crates/bosc-runtime`*

#### Axum HTTP REST API & Security Context

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`API-01`** | `CRITICAL` | `API_CONTRACT` | Health & Database Detector Endpoints (/api/health, /api/detect-db) | `curl http://localhost:3000/api/health` |
| **`API-02`** | `CRITICAL` | `API_CONTRACT` | Dynamic Database Lifecycle (list, create, switch, manage, test-connection) | `powershell scripts/test_new_db_install.ps1` |
| **`API-03`** | `CRITICAL` | `API_CONTRACT` | System Provisioning & Meta Catalog (/api/meta/provision, /api/meta/objects) | `curl -X POST http://localhost:3000/api/meta/provision` |
| **`API-04`** | `CRITICAL` | `SECURITY_RLS` | Entities Generic CRUD with Multi-Tenant RLS (/api/entities/{table}) | `powershell test_security_api.ps1` |
| **`API-05`** | `CRITICAL` | `CONCURRENCY_OCC` | Mandatory Optimistic Concurrency Control (missing `_system_version` → 428, stale → 409) | `cargo test -p bosc-runtime --test transactional_integrity updates_require_the_current_version` |
| **`API-06`** | `HIGH` | `CONCURRENCY_OCC` | Table & Record Locking (/api/entities/{table}/lock) | `curl -X POST http://localhost:3000/api/entities/customer/lock` |
| **`API-07`** | `HIGH` | `INTEGRATION` | Declarative State Transitions & Audit History (/api/entities/{table}/{id}/transition) | `curl -X POST http://localhost:3000/api/entities/order/UUID/transition` |
| **`API-08`** | `HIGH` | `API_CONTRACT` | GraphQL-Like Query Protocol with Relation Inclusions (/api/query) | `curl -X POST http://localhost:3000/api/query` |
| **`API-09`** | `HIGH` | `INTEGRATION` | Transactional Outbox & Worker Dispatcher (/api/events/outbox) | `curl http://localhost:3000/api/events/outbox` |
| **`API-10`** | `HIGH` | `INTEGRATION` | QuickJS Sandboxed Codeunit Runner (/api/codeunits/{name}/run) | `cargo test -p bosc-runtime test_quickjs_sandbox` |
| **`API-11`** | `HIGH` | `INTEGRATION` | WebAssembly wasmi 2.0 Plugin Runner (/api/wasm/{module}/{func}/invoke) | `cargo test -p bosc-runtime test_wasm_execution` |
| **`API-12`** | `MEDIUM` | `INTEGRATION` | Loopback IPC Sidecar Gateway (/api/sidecars/{name}/call) | `cargo test -p bosc-runtime test_sidecar_ipc` |

#### Transactional Integrity & Schema Migration (real PostgreSQL)

These tests need `BOS_TEST_DATABASE_URL` (a PostgreSQL URL with `CREATEDB` rights); each creates and drops its own `bos_it_<uuid>` database. Design: [`27_transactional_integrity_and_schema_migration.md`](./27_transactional_integrity_and_schema_migration.md).

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`TXN-01`** | `CRITICAL` | `INTEGRATION` | Failing codeunit rolls back every write it made | `cargo test -p bosc-runtime --test transactional_integrity codeunit_failure_rolls_back_every_write` |
| **`TXN-02`** | `CRITICAL` | `INTEGRATION` | Failing trigger rolls back the insert and the trigger's own writes | `cargo test -p bosc-runtime --test transactional_integrity trigger_failure_rolls_back_insert_and_trigger_writes` |
| **`TXN-03`** | `HIGH` | `INTEGRATION` | `@test` procedures leave no data behind | `cargo test -p bosc-runtime --test transactional_integrity test_runner_rolls_back_test_writes` |
| **`TXN-04`** | `CRITICAL` | `INTEGRATION` | `relation` / `onDelete` become enforced FKs; redeploy is idempotent; rule changes are applied | `cargo test -p bosc-runtime --test transactional_integrity relations_become_enforced_foreign_keys` |
| **`TXN-05`** | `CRITICAL` | `INTEGRATION` | Schema sync renames keep data, safe changes apply, destructive changes are blocked unless allowed, runs are journaled | `cargo test -p bosc-runtime --test transactional_integrity schema_changes_migrate_data_and_guard_destructive_steps` |
| **`TXN-06`** | `HIGH` | `INTEGRATION` | Fuel budget stops runaway scripts and rolls back their writes | `cargo test -p bosc-runtime --test transactional_integrity fuel_budget_stops_runaway_scripts_and_rolls_back` |
| **`TXN-07`** | `HIGH` | `UNIT` | Migration planner classifies safe vs destructive steps | `cargo test -p bosc-codegen-db migration` |
| **`TXN-08`** | `MEDIUM` | `UNIT` | Number-series host API stays removed (number series live in BOS packages) | `cargo test -p bosc-runtime test_number_series_host_api_removed` |

### bosc-cli (Unified CLI) (Developer Tooling)
*Directory: `crates/bosc-cli`*

#### Command-Line Operations

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`CLI-01`** | `HIGH` | `UNIT` | Database Auto-Detection (bosc-cli detect-db) | `cargo run -p bosc-cli -- detect-db` |
| **`CLI-02`** | `CRITICAL` | `INTEGRATION` | HTTP API Server Launch (bosc-cli node run) | `cargo run -p bosc-cli -- node run --help` |
| **`CLI-03`** | `HIGH` | `UNIT` | Package Build & Validation (bosc-cli package build) | `cargo run -p bosc-cli -- package --help` |

### ui (Compact Svelte 5 API Studio) (Frontend Studio)
*Directory: `ui`*

#### Interactive Testing Studio & Developer UX

| ID | Priority | Type | Task Title | Verification Command |
| :--- | :--- | :--- | :--- | :--- |
| **`UI-01`** | `CRITICAL` | `UX_VISUAL` | Responsive 3-Column Studio Layout (Desktop, Tablet, Mobile) | `node scripts/test-agent/visual_browser_test.mjs --viewport=all` |
| **`UI-02`** | `HIGH` | `UX_VISUAL` | Dark and Light Theme Toggle Contrast & Aesthetics | `node scripts/test-agent/visual_browser_test.mjs --theme=all` |
| **`UI-03`** | `HIGH` | `UI_INTERACTION` | Interactive Multi-Tenant Security Context Drawer | `node scripts/test-agent/visual_browser_test.mjs --test=drawer` |
| **`UI-04`** | `CRITICAL` | `UI_INTERACTION` | 1-Click Auto-Fill Preset Endpoint Execution | `node scripts/test-agent/visual_browser_test.mjs --test=presets` |
| **`UI-05`** | `HIGH` | `UI_INTERACTION` | Real-Time Database Cluster Switcher Dropdown | `node scripts/test-agent/visual_browser_test.mjs --test=db-switcher` |
| **`UI-06`** | `HIGH` | `UI_INTERACTION` | Dynamic JSON Request Builder & Response Inspector Tabs | `node scripts/test-agent/visual_browser_test.mjs --test=response-tabs` |
| **`UI-07`** | `MEDIUM` | `UX_VISUAL` | Error & Validation Toast Notifications Dismissal | `node scripts/test-agent/visual_browser_test.mjs --test=toasts` |

