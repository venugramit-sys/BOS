# BOS (Business Operating System)

> **Modular, Modern, Metadata-Oriented, Blazing Fast, Secured, ERP+ Platform**

Welcome to the **BOS Core Engine** architectural blueprint, compiler, and hybrid execution runtime repository.

---

## 📚 Documentation Hub

All specifications, architectural designs, implementation plans, and production user manuals are stored in the [`docs/`](docs/README.md) directory, cleanly separated into **Planning** and **Production**:

### 📐 [Planning & Architectural Specifications](docs/planning/README.md)
1. [**00. System Architecture Blueprint**](docs/planning/00_system_architecture_blueprint.md) - Vision, compiler flow, and 3-tier hybrid execution pipeline.
2. [**01. Meta-Objects & DSL Specification**](docs/planning/01_meta_objects_and_dsl_specification.md) - Formal specification of 18 meta-objects, TypeScript OxC type stripping, and grammar.
3. [**02. Database Engine & Storage Models**](docs/planning/02_database_engine_and_storage_models.md) - PostgreSQL DDL, RLS policies, FlowFields, SIFT pushdown, and declarative schema migrations.
4. [**03. Runtime Sandbox & Execution Engine**](docs/planning/03_runtime_sandbox_and_execution_engine.md) - Sandboxed QuickJS VM (8MB/100k fuel), exact 128-bit monetary math, wasmi 2.0 Wasm engine, and HTTP IPC sidecars.
5. [**04. UI Architecture & Page Compiler**](docs/planning/04_ui_architecture_and_page_compiler.md) - Svelte 5 runes architecture, Page IR v2, Monaco editor, and `@bos/page-kit`.
6. [**05. Modular Packaging & Security**](docs/planning/05_modular_packaging_and_security.md) - `.bospkg` package format, Ed25519 digital signatures, RBAC (`bos.core`), and hash-chain audit logging.
7. [**06. Roadmap & Release Milestones**](docs/planning/06_roadmap_and_release_milestones.md) - Phased delivery milestones, benchmarks, and enterprise cloud distribution.
8. [**Architectural Decision Records (ADR)**](docs/planning/adr/README.md) - ADR 0001 through 0009.

### 📚 [Production User Manual Library](docs/production/README.md)
- **Volume 1: Getting Started**
  - [01. 5-Minute Quickstart Guide](docs/production/01_getting_started/01_quickstart_guide.md)
  - [02. Installation & System Requirements](docs/production/01_getting_started/02_installation_and_system_requirements.md)
  - [03. First Workspace & Project Setup](docs/production/01_getting_started/03_first_workspace_and_project_setup.md)
- **Volume 2: End-User & Operator Manuals**
  - [01. Navigating UI Studio](docs/production/02_end_user_manuals/01_navigating_the_ui_studio.md)
  - [02. Working with Data Records & Lists](docs/production/02_end_user_manuals/02_working_with_data_records_and_lists.md)
  - [03. Document Editing & Form Pages](docs/production/02_end_user_manuals/03_document_editing_and_form_pages.md)
  - [04. Document Workflows & Approval States](docs/production/02_end_user_manuals/04_document_workflows_and_approval_states.md)
  - [05. Notifications & Audit Trails](docs/production/02_end_user_manuals/05_notifications_and_audit_trails.md)
- **Volume 3: Administrator & DevOps Manuals**
  - [01. Database Provisioning & Storage Strategies](docs/production/03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md)
  - [02. Server Node & NAS Management](docs/production/03_administrator_and_devops_manuals/02_server_node_and_nas_management.md)
  - [03. User Security & RBAC Configuration](docs/production/03_administrator_and_devops_manuals/03_user_security_and_rbac.md)
  - [04. Backup, Recovery & Health Monitoring](docs/production/03_administrator_and_devops_manuals/04_backup_recovery_and_health_monitoring.md)
  - [05. Cloud Deployment & Docker Operations](docs/production/03_administrator_and_devops_manuals/05_cloud_deployment_and_docker_operations.md)
- **Volume 4: Developer Manuals**
  - [01. bosc CLI Complete Guide](docs/production/04_developer_manuals/01_bosc_cli_complete_guide.md)
  - [02. Meta-Objects Authoring Guide](docs/production/04_developer_manuals/02_meta_objects_authoring_guide.md)
  - [03. Business Logic & QuickJS Scripting](docs/production/04_developer_manuals/03_business_logic_and_quickjs_scripting.md)
  - [04. Building & Distributing Packages](docs/production/04_developer_manuals/04_building_and_distributing_packages.md)
  - [05. Page Kit & Custom UI Development](docs/production/04_developer_manuals/05_page_kit_and_custom_ui_development.md)
- **Volume 5: Integration & API Manuals**
  - [01. REST API Reference & Authentication](docs/production/05_integration_and_api_manuals/01_rest_api_reference_and_authentication.md)
  - [02. Event-Driven Architecture & Webhooks](docs/production/05_integration_and_api_manuals/02_event_driven_architecture_and_webhooks.md)
  - [03. WebAssembly Plugins & Companion Sidecars](docs/production/05_integration_and_api_manuals/03_wasm_plugins_and_companion_sidecars.md)
- **Volume 6: Troubleshooting & FAQs**
  - [01. Error Codes & Diagnostic Guide](docs/production/06_troubleshooting_and_faqs/01_error_codes_and_diagnostic_guide.md)
  - [02. Frequently Asked Questions](docs/production/06_troubleshooting_and_faqs/02_frequently_asked_questions.md)

---

## 🚀 Quickstart & Interactive Testing Studio

### 1. Run Workspace Verification Suite
```powershell
cargo test --workspace
```
All unit and integration tests across all crates (`bosc-syntax`, `bosc-ir`, `bosc-analyzer`, `bosc-codegen-db`, and `bosc-runtime`) execute with 0 failures and 0 warnings.

### 2. Launch the BOS Core Backend Engine
```powershell
# Check local database cluster
bosc detect-db

# Launch the BOS node (API on port 3000) from the local dev config
bosc node run --config nodes/dev.toml
```

Or start the node and the UI (hot reload) together: `npm run dev` from the repo root.

### 3. Launch the Svelte 5 Developer Studio
```powershell
cd ui
npm run dev
```
Open `http://localhost:5173` in your browser to interactively test all backend APIs, introspect live entity schemas with 1-click auto-fill, register and publish business events, execute sandboxed WebAssembly functions, probe companion sidecars, and view execution logs in real time.

---

## 🛠️ Technology Stack Summary

- **Language / DSL:** BOS-Script (TypeScript business superset with zero-overhead type stripping)
- **Compiler:** Rust (`bosc`) with AST parser, semantic analyzer, and multi-target code generators
- **Backend API Server:** Rust Axum 0.8 + SQLx
- **Multi-Tier Runtime:** 
  - QuickJS Sandboxed VM (8 MB memory cap, exact 128-bit monetary math, TableModel proxies)
  - Pure-Rust WebAssembly Interpreter (`wasmi 2.0` with instruction fuel metering)
  - Loopback IPC Companion Microservice Host (`reqwest` with pure Rust `rustls`)
  - Decoupled Event-Subscriber Engine (`_bos_event_subscribers`)
- **Database:** PostgreSQL (with lifecycle manager, RLS, stored generated columns, and FlowField pushdown views)
- **Frontend Studio:** Modern Svelte 5 (Runes `$state`, `$derived`, `$effect`) + Tailwind CSS + Lucide Icons + Monaco Editor + Vite
