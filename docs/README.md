# BOS (Business Operating System) Documentation Hub

Welcome to the **BOS (Business Operating System)** Documentation Hub. BOS is a next-generation, high-performance, metadata-driven ERP+ runtime and development platform engineered in 100% safe Rust and Svelte 5.

This directory serves as the unified single source of truth for both architectural planning specifications and user manuals for operational production deployments.

---

## 🏛️ Documentation Structure

The documentation is organized into two distinct sections:

```
docs/
├── README.md                      # This portal document
├── CODEMAP.md                     # Codebase index and symbols map
├── agents_full_reference.md       # Agent team operations and governance charter
│
├── planning/                      # 📐 Engineering Specifications & Architecture Blueprints
│   ├── README.md                  # Planning catalog and technical index
│   ├── 00_system_architecture_blueprint.md
│   ├── 01_meta_objects_and_dsl_specification.md
│   ├── 02_database_engine_and_storage_models.md
│   ├── 03_runtime_sandbox_and_execution_engine.md
│   ├── 04_ui_architecture_and_page_compiler.md
│   ├── 05_modular_packaging_and_security.md
│   ├── 06_roadmap_and_release_milestones.md
│   └── adr/                       # Architectural Decision Records (ADR 0001 - 0009)
│
└── production/                    # 📚 Production User Manual Library & Operations Guides
    ├── README.md                  # Master manual index, learning pathways & search guide
    ├── 01_getting_started/        # Fast onboarding, prerequisites, and first workspace
    │   ├── 01_quickstart_guide.md
    │   ├── 02_installation_and_system_requirements.md
    │   └── 03_first_workspace_and_project_setup.md
    ├── 02_end_user_manuals/       # Day-to-day business user and operator guides
    │   ├── 01_navigating_the_ui_studio.md
    │   ├── 02_working_with_data_records_and_lists.md
    │   ├── 03_document_editing_and_form_pages.md
    │   ├── 04_document_workflows_and_approval_states.md
    │   └── 05_notifications_and_audit_trails.md
    ├── 03_administrator_and_devops_manuals/  # System administration, DevOps & Cloud ops
    │   ├── 01_database_provisioning_and_clustering.md
    │   ├── 02_server_node_and_nas_management.md
    │   ├── 03_user_security_and_rbac.md
    │   ├── 04_backup_recovery_and_health_monitoring.md
    │   └── 05_cloud_deployment_and_docker_operations.md
    ├── 04_developer_manuals/      # Package authoring, DSL scripting, and CLI guide
    │   ├── 01_bosc_cli_complete_guide.md
    │   ├── 02_meta_objects_authoring_guide.md
    │   ├── 03_business_logic_and_quickjs_scripting.md
    │   ├── 04_building_and_distributing_packages.md
    │   └── 05_page_kit_and_custom_ui_development.md
    ├── 05_integration_and_api_manuals/      # Integration, REST APIs, WebAssembly & Events
    │   ├── 01_rest_api_reference_and_authentication.md
    │   ├── 02_event_driven_architecture_and_webhooks.md
    │   └── 03_wasm_plugins_and_companion_sidecars.md
    └── 06_troubleshooting_and_faqs/         # Diagnostics, error codes, and practical answers
        ├── 01_error_codes_and_diagnostic_guide.md
        └── 02_frequently_asked_questions.md
```

---

## 🧭 Which Section Should You Read?

### 1. I am a New User or Business Operator
Start with the **[Production User Manuals](production/README.md)**:
- **[5-Minute Quickstart Guide](production/01_getting_started/01_quickstart_guide.md)**: Spin up your first BOS instance in minutes.
- **[Navigating UI Studio](production/02_end_user_manuals/01_navigating_the_ui_studio.md)**: Learn the modern interface, keyboard shortcuts, and views.
- **[Document Workflows & Approvals](production/02_end_user_manuals/04_document_workflows_and_approval_states.md)**: How to submit, review, and post transactions.

### 2. I am a System Administrator or DevOps Engineer
Jump directly into the **[Admin & DevOps Section](production/03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md)**:
- **[Database Provisioning & Multi-Tenancy](production/03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md)**: PostgreSQL initialization and tenant partitioning models.
- **[Server Node & NAS Configuration](production/03_administrator_and_devops_manuals/02_server_node_and_nas_management.md)**: Configuring `bos-node.toml` and background daemon workers.
- **[Security & RBAC Management](production/03_administrator_and_devops_manuals/03_user_security_and_rbac.md)**: Managing tenants, users, roles, and cryptographic auditing.
- **[Cloud Deployment & Docker](production/03_administrator_and_devops_manuals/05_cloud_deployment_and_docker_operations.md)**: Hardened production deployment and Indian data sovereignty compliance.

### 3. I am a Developer or Package Creator
Consult the **[Developer Manuals](production/04_developer_manuals/01_bosc_cli_complete_guide.md)**:
- **[bosc CLI Guide](production/04_developer_manuals/01_bosc_cli_complete_guide.md)**: Command reference for `build`, `run`, `deploy`, and `package`.
- **[Meta-Objects Authoring](production/04_developer_manuals/02_meta_objects_authoring_guide.md)**: Declarative schemas for Tables, Pages, Codeunits, and Workflows.
- **[Business Logic in QuickJS](production/04_developer_manuals/03_business_logic_and_quickjs_scripting.md)**: Writing TypeScript business logic with exact 128-bit monetary math.
- **[Packaging System](production/04_developer_manuals/04_building_and_distributing_packages.md)**: Building and signing `.bospkg` packages.

### 4. I am an Architect or Core Engine Contributor
Explore the **[Planning & Architecture Specifications](planning/README.md)**:
- **[System Architecture Blueprint](planning/00_system_architecture_blueprint.md)**: Hybrid 3-tier execution engine, compiler AST/IR lowering.
- **[Architectural Decision Records (ADRs)](planning/adr/README.md)**: Detailed historical decisions on UUIDv7, uniform RLS, and hashing chains.

---

## ⚡ Key Architectural Facts

1. **Systems Core in 100% Rust**: Zero mocks, zero unhandled errors, memory-safe, thread-safe asynchronous execution with Axum and SQLx.
2. **Metadata-Driven Paradigm**: Applications are expressed as declarative meta-objects (`table`, `page`, `codeunit`, `statemachine`, etc.) compiled into database structures and bytecode envelopes.
3. **Hybrid 3-Tier Execution Runtime**:
   - Database layer pushdown (PostgreSQL views, PL/pgSQL, SIFT flowfields).
   - Sandboxed QuickJS VM (exact BigInt decimal math, memory quotas, fuel metering).
   - In-process WebAssembly (`wasmi 2.0`) and HTTP loopback IPC sidecars.
4. **Reactive Svelte 5 Frontend**: Modern Svelte 5 runes (`$state`, `$derived`, `$props`), responsive design, Lucide icons, Monaco editor, and `@bos/page-kit`.
