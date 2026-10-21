# Extension Architecture, Modular Packaging & Ecosystem

> **Status:** Draft / Specification  
> **Document:** `docs/04_extension_and_modular_sys.md`  

---

## 1. Modular Hierarchy & Layering Model

BOS adheres to a strict non-invasive layered architecture. Upstream packages cannot be modified directly; they are enhanced via extensions.

```
┌────────────────────────────────────────────────────────┐
│           Tenant Customization Layer                   │
│           (Company-specific fields, bespoke workflows) │
└───────────────────────────┬────────────────────────────┘
                            │ depends on
                            ▼
┌────────────────────────────────────────────────────────┐
│           Industry / ISV Solution Layer               │
│           (e.g., Manufacturing, Real Estate, Retail)   │
└───────────────────────────┬────────────────────────────┘
                            │ depends on
                            ▼
┌────────────────────────────────────────────────────────┐
│           Country / Localization Layer                 │
│           (e.g., Tax Engines, E-Invoicing, Statutory)  │
└───────────────────────────┬────────────────────────────┘
                            │ depends on
                            ▼
┌────────────────────────────────────────────────────────┐
│           Core Base Application                        │
│           (General Ledger, Sales, Purchases, Inventory)│
└───────────────────────────┬────────────────────────────┘
                            │ depends on
                            ▼
┌────────────────────────────────────────────────────────┐
│           BOS System Runtime & Standard Library        │
│           (Security, Users, NumberSeries, Workflows)   │
└────────────────────────────────────────────────────────┘
```

---

## 2. Package Manifest (`bos.json`)

Every modular extension contains a `bos.json` manifest at its root:

```json
{
  "name": "acme-manufacturing",
  "publisher": "Acme Solutions",
  "version": "1.4.0",
  "brief": "Advanced MRP & Production Routing for BOS",
  "compatibility": {
    "bosEngineVersion": "^2.0.0",
    "dbTarget": "postgres"
  },
  "dependencies": [
    {
      "name": "bos-core-base",
      "publisher": "BOS Foundation",
      "version": "^2.0.0"
    },
    {
      "name": "bos-inventory-advanced",
      "publisher": "BOS Foundation",
      "version": "^2.0.0"
    }
  ],
  "idRanges": [
    { "from": 50000, "to": 59999 }
  ],
  "entry": "./src/index.bos"
}
```

---

## 3. Extension Rules & Non-Invasive Augmentation

> **Full Specification:** See [`docs/14_bos_script_compiler_and_hybrid_runtime_specification.md`](file:///d:/Work%20Studio/Bos/docs/14_bos_script_compiler_and_hybrid_runtime_specification.md).

### 3.1 The 5-Pillar Enterprise Extension Framework
1. **Publish-Subscribe Event Pattern:** Base code exposes `@businessEvent` and triggers; extensions attach `[EventSubscriber(Table::SalesHeader, "onBeforeInsert")]` without editing base code.
2. **WebAssembly (Wasm) plugin host:** Third parties compile Rust, Go, or C++ to `.wasm`. `bosc-runtime` loads the module with `wasmi` and fuel. A script calls `ctx.wasm.invoke(module, function, a, b)` with two numbers.
3. **Out-of-Process Sidecar Microservices:** Heavy integrations (Stripe, OpenAI, Avalara Tax, EDI) run in container sidecars communicating via internal mTLS gRPC / Unix sockets, keeping the core ERP database runtime isolated.
4. **Modular Namespaced Packages (`.bospkg`):** Self-contained, cryptographically signed extension bundles.
5. **Low-Code Customizations:** Excel-like formula fields (`GENERATED ALWAYS AS ... STORED`) and declarative validation rules for business analysts.

### 3.2 Table Extensions (`tableextension`)
- **Add Fields:** Append new fields to existing tables without affecting base column definitions.
- **Add Keys / Indexes:** Define new single or composite indexes on combinations of base and custom fields.
- **Event Hooks:** Attach triggers (`onBeforeInsert`, `onAfterInsert`, `onBeforeModify`, etc.) that execute in deterministic order based on dependency topology.
- **Constraint Overrides:** Tighten validations (cannot loosen base constraints).

### 3.3 Page Extensions (`pageextension`)
- **Layout Injections:**
  - `addFirst(ContainerId) { ... }`
  - `addLast(ContainerId) { ... }`
  - `addBefore(ControlId) { ... }`
  - `addAfter(ControlId) { ... }`
- **Action Injections:** Insert custom action buttons, menus, and shortcuts into established toolbars.
- **Property Overrides:** Dynamically adjust field visibility, editable states, and conditional formatting.

---

## 4. Package Artifact (`.bospkg`)

When compiled with `bosc package`, the output is an immutable signed archive (`.bospkg`) containing:
1. `manifest.json`: Verified package metadata and cryptographic signature.
2. `schema_ir.json`: Canonical schema metadata for database migration.
3. `logic.wasm`: Compiled backend business logic bytecode.
4. `ui_bundle/`: Compiled Svelte components, asset manifests, and client hydration scripts.
5. `symbols.sym`: Symbol exports for downstream dependency resolution and IDE autocompletion.

---

## 5. Hot Deployment & Sandboxed Lifecycle

```
Draft / Development ──► Staging Validation ──► Compile & Sign ──► Tenant Installation
                                                                          │
                                                                          ▼
                                                               ┌──────────────────────┐
                                                               │ 1. Apply DB Migr.    │
                                                               │ 2. Verify Invariants │
                                                               │ 3. Mount WASM Logic  │
                                                               │ 4. Activate UI Pages │
                                                               └──────────────────────┘
```

- **Zero-Downtime:** Adding columns or views does not block read/write transactions in PostgreSQL.
- **Rollback Protection:** If any stage of installation fails validation, transactional migrations roll back completely.

---

## 6. Pre-Installed Core Packages: Authentication & Identity (`bos.auth`)

BOS includes pre-compiled, kernel-grade packages that provide identity, role-based access control (RBAC), and session management:

### 6.1 Package Specification: `bos.auth`
- **Package ID:** `bos.auth`
- **Name:** `BOS Authentication & Identity Package`
- **Version:** `1.0.0`
- **Publisher:** `BOS Core Kernel`
- **Source Definition:** [`packages/bos.auth.bos`](file:///d:/Work%20Studio/Bos/packages/bos.auth.bos)
- **Compiled Package Artifact:** [`packages/bos.auth.bospkg`](file:///d:/Work%20Studio/Bos/packages/bos.auth.bospkg)

### 6.2 Bundled Meta-Objects
1. **`User`** (`user`): Identity master record with unique `username`, password hash (`password_hash`), `email`, `is_active`, and lockout tracking (`failed_attempts`).
2. **`Role`** (`role`): RBAC role definitions with unique role codes.
3. **`UserRole`** (`user_role`): Composite key relation mapping users to roles.
4. **`SessionToken`** (`session_token`): Token lifecycle table tracking bearer session identifiers, client IP addresses, and revocation status.
5. **`AuthManager`**: Native business logic codeunit with methods (`ValidateUserRegistration`, `CheckTokenStatus`) executing parameter validations and audit trail entries.

### 6.3 Compilation & Package Management CLI Commands

```bash
# 1. Compile BOS Script into a verified .bospkg package bundle
cargo run --bin bosc-cli -- package build \
  --input packages/bos.auth.bos \
  --output packages/bos.auth.bospkg \
  --package-id bos.auth \
  --name "BOS Authentication & Identity Package" \
  --version 1.0.0 \
  --publisher "BOS Core Kernel"

# 2. Install and activate package bundle into PostgreSQL cluster
cargo run --bin bosc-cli -- package install \
  --input packages/bos.auth.bospkg \
  --db postgres://postgres:postgres@localhost:5432/bos_dev

# 3. List installed packages in the database
cargo run --bin bosc-cli -- package list
```
