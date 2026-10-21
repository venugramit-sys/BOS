# ADR 29: First-Run System Provisioning & Enterprise Authentication Architecture

## 1. Executive Summary & Problem Statement

### 1.1. Context
Previously, when launching BOS for the first time or connecting to a clean PostgreSQL instance, the frontend directly presented a simplistic "Create the first account" form (prompting for username, display name, email, and password). 

This approach presented several critical architectural deficiencies:
1. **Bypassed Database Architecture & Storage Strategy**: The administrator was unable to select the multi-tenancy storage model (`shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`) or target database.
2. **Ignored Base Package Provisioning**: The system did not compile or deploy the modular business packages (`bos.security.bospkg` and `bos.ledger.bospkg`). As a result, the rich BOS meta-objects (`User`, `Role`, `UserRole`, `SessionToken`, `UserManager`, `PermissionManager`) were not initialized in the database.
3. **No Organizational Hierarchy**: The initial user was created without an explicit Tenant partition or legal Company assignment.
4. **Poor Enterprise First-Impression**: Business users and power users were confused by an unbranded raw user creation prompt rather than an enterprise-grade guided provisioning wizard and clean subsequent login portal.

### 1.2. Decision
BOS adopts a **Two-Stage System Lifecycle Strategy**:
1. **Stage 1: First-Run Guided Provisioning Wizard (`InitialSetupWizard.svelte`)**:
   - Displayed **only** when the system is unprovisioned (`isProvisioned: false` or `needsSetup: true`).
   - Guides the Superuser through PostgreSQL cluster verification, storage strategy selection, automatic base package installation (`bos.security`, `bos.ledger`), organizational setup (Tenant & Company), and Superuser administrator creation.
   - Executes atomically via `POST /api/setup/provision`.
2. **Stage 2: Enterprise Sign-In Portal (`SignIn.svelte`)**:
   - Once provisioned, the system locks out the setup wizard.
   - On every subsequent visit or browser restart, users and power users are greeted exclusively by a polished enterprise Sign-In page.
   - Supports username/password authentication, tenant/company context, and session resumption.

---

## 2. Industry Benchmark Comparison

| Feature / Criterion | Microsoft Dynamics 365 BC | SAP S/4HANA | Salesforce | **BOS Architecture** |
| :--- | :--- | :--- | :--- | :--- |
| **First-Run Experience** | Admin Center / PowerShell Provisioning (`New-NAVTenant`, `Sync-NAVTenant`) | Initial Client 000 Setup & Post-Installation Customizing | Org Provisioning Wizard (`setup.salesforce.com`) | **`InitialSetupWizard.svelte` (Self-Guided 4-Step Provisioning)** |
| **Package / App Loading** | Installs `System.app` and `Base Application.app` into database | Imports Standard Customizing and SAP Basis Tables | Deploys Standard Objects (Account, Contact, User) | **Auto-compiles and imports `bos.security.bospkg` & `bos.ledger.bospkg`** |
| **Multi-Tenancy Setup** | Dedicated vs Shared database switch per tenant | Client-dependent (`MANDT`) data separation | Pod/Instance multi-tenant metadata architecture | **Choice of `shared_rls`, `partitioned`, `schema_per_company`, `table_per_company`** |
| **Superuser Role** | `SUPER` Role assigned to initial Windows/M365 Admin | `DDIC` / `SAP*` initial master user | System Administrator profile assigned to sign-up user | **`SUPER` + `Admin` role assigned to initial bootstrap user** |
| **Subsequent Loads** | Web Client Login (`/?tenant=...`) | SAP GUI / Fiori Launchpad Login Screen | Standard Login Portal (`login.salesforce.com`) | **Enterprise Sign-In Screen with Company Picker** |

---

## 3. System Lifecycle & Transition State Machine

```mermaid
stateDiagram-v2
    [*] --> ProbeStatus: App Bootstraps (fetch /api/auth/status)
    
    state ProbeStatus <<choice>>
    ProbeStatus --> FirstRunWizard: needsSetup == true (Unprovisioned / No Users)
    ProbeStatus --> EnterpriseLogin: needsSetup == false (Provisioned & Users Exist)
    
    state FirstRunWizard {
        [*] --> Step1_Cluster: 1. PostgreSQL Cluster & Database
        Step1_Cluster --> Step2_Strategy: 2. Storage Strategy & Base Packages (bos.security, bos.ledger)
        Step2_Strategy --> Step3_Org: 3. Initial Tenant & Legal Company
        Step3_Org --> Step4_SuperUser: 4. Superuser Account Credentials
        Step4_SuperUser --> ExecutingProvision: Submit to /api/setup/provision
        ExecutingProvision --> ProvisionSuccess: 201 Created & Auto-Session
    }
    
    ProvisionSuccess --> ActiveWorkspace: Auto-Login Superuser to Role Center
    
    state EnterpriseLogin {
        [*] --> CredentialsForm: Enter Username & Password
        CredentialsForm --> Validating: POST /api/auth/login
        Validating --> ActiveWorkspace: Session Granted (SUPER, Admin, User)
    }
    
    state ActiveWorkspace {
        [*] --> RoleCenter: Standard Users
        [*] --> DevStudio: Superusers navigating to /studio
    }
```

---

## 4. API Specifications & Contracts

### 4.1. `GET /api/auth/status` (Public)
Probes whether the system has been provisioned and is ready for login.

**Response (Unprovisioned)**:
```json
{
  "isProvisioned": false,
  "needsSetup": true,
  "activeDatabase": "postgres",
  "storageStrategy": "shared_rls",
  "hasSecurityPackage": false,
  "userCount": 0
}
```

**Response (Provisioned)**:
```json
{
  "isProvisioned": true,
  "needsSetup": false,
  "activeDatabase": "bos_primary",
  "storageStrategy": "shared_rls",
  "hasSecurityPackage": true,
  "userCount": 3
}
```

### 4.2. `POST /api/setup/provision` (Public on First Run, Locked After)
Executes atomic database provisioning, package compilation, tenant/company creation, and superuser generation.

**Request Payload**:
```json
{
  "database": "bos_enterprise",
  "connection_url": "postgresql://user:password@endpoint.neon.tech/neondb?sslmode=require",
  "storage_strategy": "shared_rls",
  "import_base_packages": true,
  "packages": ["bos.security", "bos.ledger"],
  "tenant_code": "DEFAULT",
  "tenant_name": "Default Enterprise Tenant",
  "company_code": "CRONUS",
  "company_name": "Cronus International Ltd",
  "admin_username": "admin",
  "admin_password": "SuperSecretPassword123!",
  "admin_display_name": "Global Administrator",
  "admin_email": "admin@cronus.corp"
}
```

> [!NOTE]
> If `connection_url` is provided, BOS connects directly to the specified cloud PostgreSQL provider (Neon, Supabase, AWS RDS, Azure, GCP). If omitted, BOS utilizes the local instance and creates/initializes the database identified by `database`.

**Backend Execution Pipeline**:
1. **Security Guard**: Verify that `user_count == 0`. If users already exist, reject immediately with `403 Forbidden` (`"System is already provisioned and initialized"`).
2. **Database Provisioning**: Ensure database exists; switch engine pool to target database.
3. **Kernel Initialization**: Run `MetaManager::provision_system_kernel` to create `_bos_sys_info`, `_bos_meta_objects`, `_bos_tenants`, `_bos_companies`, and system tables.
4. **Organizational Seeding**: Insert initial Tenant (`DEFAULT`) and Company (`CRONUS`).
5. **Base Package Deployment**:
   - If `import_base_packages` is true, locate `packages/bos.security.bospkg` and `packages/bos.ledger.bospkg` (or compile them).
   - Apply table DDL, register meta-objects, and apply tablelogic triggers into PostgreSQL.
6. **Superuser Creation**:
   - Hash password using real Argon2id (`$argon2id$v=19$...`).
   - Insert into `_bos_users` with `roles = ["SUPER", "Admin", "Global Admin"]`.
   - Also sync to `User` and `Role` meta-objects under `bos.security`.
7. **Session Issuance**:
   - Generate cryptographically secure session token via `__host_random_bytes(32)`.
   - Insert into `_bos_sessions`.
   - Return `{ "token": "...", "user": { ... }, "needsSetup": false }`.

---

## 5. UI/UX Architecture

### 5.1. Component Division
- **[`InitialSetupWizard.svelte`](file:///d:/Work%20Studio/Bos/ui/src/lib/components/shell/InitialSetupWizard.svelte)**:
  - Step 1: **Cluster & Database**: Detects PostgreSQL 5432, validates connection, inputs database name.
  - Step 2: **Architecture & Base Packages**: Interactive cards for `shared_rls`, `partitioned`, `schema_per_company`; auto-import toggles for `bos.security` and `bos.ledger`.
  - Step 3: **Organization Setup**: Tenant code/name and Company code/name.
  - Step 4: **Superuser Credentials**: Admin username, full name, email, password.
  - Step 5: **Progress & Verification**: Real-time progress bar with animated checkmarks, automated redirect on completion.
- **[`SignIn.svelte`](file:///d:/Work%20Studio/Bos/ui/src/lib/components/shell/SignIn.svelte)**:
  - Clean, modern enterprise login view.
  - Username & Password inputs with password reveal toggle.
  - Optional Company selector dropdown.
  - Direct login for standard users (`User`) and power users (`Admin`, `SUPER`).
- **[`App.svelte`](file:///d:/Work%20Studio/Bos/ui/src/App.svelte)**:
  - Top-level routing:
    ```svelte
    {#if needsSetup}
      <InitialSetupWizard onComplete={handleSetupComplete} />
    {:else if !shell.currentUser}
      <SignIn />
    {:else if isDevStudio}
      <StudioWorkspace theme={shell.theme} />
    {:else}
      <AppShell menus={liveMenus}>...</AppShell>
    {/if}
    ```

---

## 6. Zero-Drift & Quality Checklist

- [x] Strict zero-mock policy: connects to real PostgreSQL database and executes real DDL.
- [x] No `todo!()` or dummy returns in Rust runtime.
- [x] Exclusively Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`).
- [x] Full backward compatibility: existing databases with users continue directly to the Sign-In screen.
- [x] All base packages (`bos.security`, `bos.ledger`) compiled and validated before deployment.
