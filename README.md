# BOS (Business Operating System)

> **Modular, Modern, Metadata-Oriented, Blazing Fast, Secured, ERP+ Platform**

Welcome to the **BOS Core Engine** architectural blueprint, compiler, and hybrid execution runtime repository.

---

## 📚 Core Architecture Documentation

All specifications, architectural designs, and implementation plans are stored in the [`docs/`](file:///d:/Work%20Studio/Bos/docs) directory:

1. [**00. Architecture Overview**](file:///d:/Work%20Studio/Bos/docs/00_architecture_overview.md)
   - High-level vision, core philosophy, end-to-end compilation flow, and 3-tier hybrid execution pipeline (BOS-AMDP, QuickJS VM, wasmi 2.0 Wasm sandbox, loopback sidecar microservices).
2. [**01. BOS-Script Specification**](file:///d:/Work%20Studio/Bos/docs/01_bos_script_specification.md)
   - Domain-specific extensions to TypeScript, metadata objects (`table`, `page`, `view`, `query`, `menu`, `permissionset`, `codeunit`), non-invasive extensions (`tableextension`, `pageextension`), event triggers, and validations.
3. [**01B. BOS-Script Framework Deep Dive**](file:///d:/Work%20Studio/Bos/docs/01_bos_script_framework_deep_dive.md)
   - Deep dive into business logic architecture, record snapshots (`Rec`/`xRec`), field validation cascades, declarative FlowFields, state machine workflows, and immutable ledger patterns.
4. [**01C. Must-Have Enterprise Features & Meta-Object Deep Dive**](file:///d:/Work%20Studio/Bos/docs/01_bos_script_must_have_features_and_meta_objects.md)
   - Exhaustive feature analysis from SAP CAP, Business Central AL, and Salesforce Apex. Detailed 12-object catalog and step-by-step freezing protocol.
5. [**01D. Meta-Object Specification: `table` & `tableextension`**](file:///d:/Work%20Studio/Bos/docs/01_meta_object_table_specification.md)
   - Complete frozen specification for tables: draft handling, immutable ledgers, FlowField calculation matrix, dynamic FlowFilters, and NAV/BC-grade multi-tenancy (`dataPerCompany`).
6. [**01E. Meta-Object Specification: `statemachine` & Workflows**](file:///d:/Work%20Studio/Bos/docs/01_meta_object_statemachine_specification.md)
   - Declarative document state transitions, multi-tier enterprise approval matrices, parameterized transitions, atomic PostgreSQL transitions, and audit state history.
7. [**01F. Meta-Object Specification: `codeunit`, `interface`, `enum` & `enumextension`**](file:///d:/Work%20Studio/Bos/docs/01_meta_object_codeunit_interface_enum_specification.md)
   - Encapsulated business services, sandboxed execution, first-class polymorphic Strategy Pattern via extensible enums, transactional posting engines, and test auto-rollback.
8. [**02. Rust Compiler (`bosc`) Architecture**](file:///d:/Work%20Studio/Bos/docs/02_rust_compiler_architecture.md)
   - Compiler pipeline (Parser, High-Speed TypeScript Type Stripper, Semantic Analyzer, BOS-IR), crate breakdown, code generation targets for Database DDL, QuickJS bytecode, wasmi Wasm, and Svelte UI.
9. [**03. Database & Runtime Engine**](file:///d:/Work%20Studio/Bos/docs/03_database_and_runtime_engine.md)
   - PostgreSQL schema mapping, multi-tenancy models, Row-Level Security (RLS) enforcement, declarative migration engine, generated stored columns, and hybrid execution runtime.
10. [**04. Extension & Modular Packaging**](file:///d:/Work%20Studio/Bos/docs/04_extension_and_modular_sys.md)
    - Layering hierarchy (Base -> Localization -> Industry -> Tenant), `bos.json` manifest, `.bospkg` bundle format, and hot deployment lifecycle.
11. [**05. Implementation Roadmap & Milestones**](file:///d:/Work%20Studio/Bos/docs/05_implementation_roadmap.md)
    - Six phased development stages from AST/IR to live production ERP reference apps, benchmarks, and success criteria.
12. [**06. Live Infrastructure Verification Report**](file:///d:/Work%20Studio/Bos/docs/06_live_infrastructure_verification_report.md)
    - 100% verified live test execution report against PostgreSQL 18.4 validating DDL compilation, RLS, immutable ledger triggers, privileged audit logs, and atomic state transitions.
13. [**07. Database Provider Detector Utility**](file:///d:/Work%20Studio/Bos/docs/07_database_detector_utility.md)
    - Real-time detection and health probing utility for locally installed and running database engines (PostgreSQL, MySQL, SQL Server, Redis, ClickHouse, SQLite).
14. [**08. Live Backend API Server Verification Report**](file:///d:/Work%20Studio/Bos/docs/08_live_backend_api_server_verification_report.md)
    - 100% verified live test execution report for the Axum 0.8 / PostgreSQL 18.4 HTTP backend API server, verifying health check, database detector, CRUD with RLS, atomic state transitions, event subscribers, wasmi Wasm execution, and sidecar IPC calls.
15. [**09. Database Lifecycle Management & Svelte 5 Testing Studio**](file:///d:/Work%20Studio/Bos/docs/09_database_lifecycle_management_and_ui_studio.md)
    - Native database lifecycle engine architecture (`create`, `list`, `inspect`, `switch`, `manage`, `drop`), connection diagnostics, and interactive Svelte 5 API Studio.
16. [**10. System Kernel Provisioning & Meta-Objects Catalog**](file:///d:/Work%20Studio/Bos/docs/10_system_kernel_provisioning_and_meta_objects_catalog.md)
    - Automatic BOS System Kernel provisioning (`_bos_sys_info`, `_bos_meta_packages`, `_bos_meta_objects`, `_bos_tenants`, `_bos_companies`), dynamic compilation, and verification suite.
17. [**11. Adaptive Multi-Tenant & Multi-Company Storage Strategies**](file:///d:/Work%20Studio/Bos/docs/11_adaptive_multi_tenant_storage_strategies.md)
    - Enterprise data isolation topologies (`shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`), automatic DDL adaptation, and schema routing.
18. [**12. Lean Minimal System Tables Architecture & Single Source of Truth (ADR 12)**](file:///d:/Work%20Studio/Bos/docs/12_lean_minimal_system_tables_architecture.md)
    - Consolidated catalog in `_bos_meta_objects.definition` (JSONB with GIN index) for zero sprawl and maximum performance.
19. [**13. Enterprise Runtime Advanced Patterns**](file:///d:/Work%20Studio/Bos/docs/13_enterprise_runtime_advanced_patterns.md)
    - Number Series, Job Queue asynchronous batch worker, and SIFT pushdown aggregations.
20. [**14. BOS-Script Compiler & Hybrid Execution Engine Specification**](file:///d:/Work%20Studio/Bos/docs/14_bos_script_compiler_and_hybrid_runtime_specification.md)
    - Full technical blueprint for TypeScript type stripping, QuickJS sandbox integration, exact BigInt monetary math (`ctx.decimal`), and AMDP SQL pushdowns.
21. [**15. Enterprise Extensibility & Customization Guide**](file:///d:/Work%20Studio/Bos/docs/15_enterprise_extensibility_and_customization_guide.md)
    - Complete developer guide for the 5-pillar extension framework: Decoupled Event Subscribers, WebAssembly (`wasmi 2.0`) plugins, loopback sidecar microservices, modular `.bospkg` packages, and low-code formula columns.
22. [**18. QA Test Task Matrix & Feature Decomposition Catalog**](file:///d:/Work%20Studio/Bos/docs/18_qa_test_task_matrix.md)
    - Exhaustive decomposition of all BOS compiler crates, runtime engines, packages, and UI features into 36+ granular test tasks.
23. [**19. QA Tester Agent Architecture & Full-Stack Verification Specification**](file:///d:/Work%20Studio/Bos/docs/19_qa_tester_agent_architecture_and_verification_specification.md)
    - Autonomous test agent specifications covering feature decomposition, git blast-radius analysis, exhaustive API parameter fuzzing, and Playwright visual browser UI/UX verification.

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
cargo run -p bosc-cli -- detect-db

# Launch the live HTTP API server (port 3000)
cargo run -p bosc-cli -- serve --port 3000
```

### 3. Launch the Compact Svelte 5 API Endpoint Studio
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
- **Frontend Studio:** Compact Svelte 5 (Runes `$state`, `$derived`, `$effect`) + Tailwind CSS + Lucide Icons + Vite
