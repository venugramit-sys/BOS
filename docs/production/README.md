# BOS Production User Manual Library

Welcome to the **BOS Production User Manual Library**. This library is designed to guide new users, enterprise business operators, system administrators, and application developers step-by-step through installing, running, operating, configuring, and extending BOS.

---

## 🏛️ Library Organization & Catalog

The user manuals are organized into six structured library volumes:

```
docs/production/
├── README.md                                    # Master Library Catalog & User Learning Tracks
│
├── 01_getting_started/                         # Volume 1: Fast Onboarding & Setup
│   ├── 01_quickstart_guide.md                  # 5-minute fast track from zero to running system
│   ├── 02_installation_and_system_requirements.md # Hardware, OS, Rust, Node, and PostgreSQL setup
│   └── 03_first_workspace_and_project_setup.md # Directory structures, bos-node.toml, initial boot
│
├── 02_end_user_manuals/                        # Volume 2: Day-to-Day Business Operators
│   ├── 01_navigating_the_ui_studio.md          # Workspace layout, navigation sidebar, theme & shortcuts
│   ├── 02_working_with_data_records_and_lists.md # Sorting, filtering, searching, and exporting lists
│   ├── 03_document_editing_and_form_pages.md   # Field inputs, validations, dirty states, and draft saves
│   ├── 04_document_workflows_and_approval_states.md # State machines, approvals, posting, and ledgers
│   └── 05_notifications_and_audit_trails.md   # Notification center, toasts, and cryptographic audit logs
│
├── 03_administrator_and_devops_manuals/       # Volume 3: Systems Administration & Cloud Ops
│   ├── 01_database_provisioning_and_clustering.md # detect-db, create-db, provision, and storage topologies
│   ├── 02_server_node_and_nas_management.md   # Standalone server nodes, bos-node.toml, and NAS worker daemons
│   ├── 03_user_security_and_rbac.md           # Tenants, companies, users, roles, and permission sets
│   ├── 04_backup_recovery_and_health_monitoring.md # Automated backups, WAL streaming, health checks, and DR
│   └── 05_cloud_deployment_and_docker_operations.md # Docker, Compose, Caddy TLS, and Indian cloud compliance
│
├── 04_developer_manuals/                       # Volume 4: Package Developers & Business Engineers
│   ├── 01_bosc_cli_complete_guide.md           # Complete CLI reference (build, run, deploy, package, node)
│   ├── 02_meta_objects_authoring_guide.md      # Writing tables, pages, codeunits, workflows, and queries
│   ├── 03_business_logic_and_quickjs_scripting.md # TypeScript business logic, Rec/xRec, and 128-bit math
│   ├── 04_building_and_distributing_packages.md # bos.json manifests, .bospkg archives, and Ed25519 signing
│   └── 05_page_kit_and_custom_ui_development.md # Svelte 5 Page Kit, Monaco editor, and dynamic UI forms
│
├── 05_integration_and_api_manuals/            # Volume 5: Integrators & API Engineers
│   ├── 01_rest_api_reference_and_authentication.md # Axum REST APIs, session tokens, and optimistic concurrency
│   ├── 02_event_driven_architecture_and_webhooks.md # Business events, transactional outbox, and event listeners
│   └── 03_wasm_plugins_and_companion_sidecars.md # wasmi 2.0 WebAssembly plugins and HTTP loopback sidecars
│
└── 06_troubleshooting_and_faqs/                # Volume 6: Support, Diagnostics & FAQs
    ├── 01_error_codes_and_diagnostic_guide.md  # BOS compiler error catalog (BOS10xx-BOS30xx) & runtime fixes
    └── 02_frequently_asked_questions.md        # Comprehensive FAQ for beginners, admins, and developers
```

---

## 🎯 Guided Learning Pathways

To help you get productive quickly, follow the learning pathway tailored to your role:

### 🌟 Pathway A: Complete Beginner (0 to Running App in 15 Minutes)
1. Read **[Volume 1: 5-Minute Quickstart Guide](01_getting_started/01_quickstart_guide.md)** to install and launch BOS.
2. Read **[Volume 1: First Workspace & Project Setup](01_getting_started/03_first_workspace_and_project_setup.md)** to initialize your first database.
3. Open **[Volume 2: Navigating UI Studio](02_end_user_manuals/01_navigating_the_ui_studio.md)** to tour the user interface.

### 🏢 Pathway B: Business User & Operations Clerk
1. **[Navigating UI Studio](02_end_user_manuals/01_navigating_the_ui_studio.md)**
2. **[Working with Records & Lists](02_end_user_manuals/02_working_with_data_records_and_lists.md)**
3. **[Document Editing & Form Pages](02_end_user_manuals/03_document_editing_and_form_pages.md)**
4. **[Document Workflows & Approvals](02_end_user_manuals/04_document_workflows_and_approval_states.md)**
5. **[Notifications & Audit Logs](02_end_user_manuals/05_notifications_and_audit_trails.md)**

### 🛡️ Pathway C: System Administrator & DevOps Engineer
1. **[Installation & System Requirements](01_getting_started/02_installation_and_system_requirements.md)**
2. **[Database Provisioning & Multi-Tenancy](03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md)**
3. **[Server Node & NAS Daemons](03_administrator_and_devops_manuals/02_server_node_and_nas_management.md)**
4. **[User Security & RBAC Configuration](03_administrator_and_devops_manuals/03_user_security_and_rbac.md)**
5. **[Cloud Deployment & Docker Operations](03_administrator_and_devops_manuals/05_cloud_deployment_and_docker_operations.md)**
6. **[Backup, Recovery & Health Monitoring](03_administrator_and_devops_manuals/04_backup_recovery_and_health_monitoring.md)**

### 💻 Pathway D: Enterprise Developer & Package Author
1. **[BOS Compiler CLI Complete Guide](04_developer_manuals/01_bosc_cli_complete_guide.md)**
2. **[Meta-Objects Authoring Guide](04_developer_manuals/02_meta_objects_authoring_guide.md)**
3. **[Business Logic in QuickJS](04_developer_manuals/03_business_logic_and_quickjs_scripting.md)**
4. **[Building & Distributing Packages](04_developer_manuals/04_building_and_distributing_packages.md)**
5. **[Dynamic Page Kit UI Development](04_developer_manuals/05_page_kit_and_custom_ui_development.md)**

### 🔌 Pathway E: Integration Engineer
1. **[REST API Reference & Auth](05_integration_and_api_manuals/01_rest_api_reference_and_authentication.md)**
2. **[Transactional Outbox & Webhooks](05_integration_and_api_manuals/02_event_driven_architecture_and_webhooks.md)**
3. **[WebAssembly Plugins & IPC Sidecars](05_integration_and_api_manuals/03_wasm_plugins_and_companion_sidecars.md)**

---

## 🔎 Quick Topic Search Index

- **Adding a new table**: See [04_developer_manuals/02_meta_objects_authoring_guide.md](04_developer_manuals/02_meta_objects_authoring_guide.md#2-creating-tables).
- **Posting an invoice**: See [02_end_user_manuals/04_document_workflows_and_approval_states.md](02_end_user_manuals/04_document_workflows_and_approval_states.md#3-posting-to-immutable-ledgers).
- **Resetting a password**: See [03_administrator_and_devops_manuals/03_user_security_and_rbac.md](03_administrator_and_devops_manuals/03_user_security_and_rbac.md#3-managing-users).
- **Creating a database**: See [03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md](03_administrator_and_devops_manuals/01_database_provisioning_and_clustering.md#2-creating-a-new-database).
- **Fixing compiler errors**: See [06_troubleshooting_and_faqs/01_error_codes_and_diagnostic_guide.md](06_troubleshooting_and_faqs/01_error_codes_and_diagnostic_guide.md).
