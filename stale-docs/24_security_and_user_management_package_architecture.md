# BOS Architectural Specification: Enterprise Security & User Management Package (`bos.security`)

**Document Version:** 1.0.0  
**Status:** PROPOSED & APPROVED FOR IMPLEMENTATION  
**Date:** October 2026  
**Authors:** DeepMind Antigravity Team  
**Scope:** Identity Lifecycle, RBAC, Permissions, Operational Guardrails, Profiles, Personalization, Session Management  

---

## 1. Architectural Principles & Non-Negotiable Standards

1. **Lean Minimal System Kernel Compliance ([ADR 12](file:///d:/Work%20Studio/Bos/docs/12_lean_minimal_system_tables_architecture.md)):**
   - The PostgreSQL core kernel maintains only 5 system tables.
   - All security, authentication, permissions, profiles, and personalization entities are authored as first-class meta-objects inside a native BOS package (`packages/bos.security` or enhanced `packages/bos.auth`).
2. **Clean Singular Table Naming Standard ([ADR 01](file:///d:/Work%20Studio/Bos/docs/01_meta_object_table_specification.md) & [AGENTS.md](file:///d:/Work%20Studio/Bos/AGENTS.md)):**
   - Table names must be simple, singular PascalCase identifiers without module acronym prefixes: `User`, `Role`, `UserRole`, `UserSetup`, `PermissionSet`, `Permission`, `SecurityFilter`, `Profile`, `UserPersonalization`, `PagePersonalization`, `UserGroup`, `UserGroupMember`, `SessionToken`, `SecurityAuditLog`.
   - SQL tables map directly to singular snake_case (`user`, `role`, `user_setup`, etc.). Package manifests (`bos.json`) provide modular isolation.
3. **Strict Meta-Object Boundary Separation:**
   - **Tables:** Data schemas, field constraints, defaults, primary keys, indexes, and relations. Field validations and lifecycle triggers live in the table's `tablelogic` companion.
   - **Codeunits:** Procedural workflows, password hashing, session tokens, permission checking, security filter injection, and domain events (`@businessEvent`).
4. **Zero-Mock Policy:**
   - Production meta-objects must contain zero placeholder mocks, dummy passwords, hard-coded permission arrays, or stub implementations.

---

## 2. Market Leader Comparative Analysis

| Dimension | Dynamics 365 Business Central | Salesforce CRM | SAP S/4HANA | BOS Design Architecture |
| :--- | :--- | :--- | :--- | :--- |
| **User Identity** | Table 2000000120 `User` | Standard `User` Object | `SU01` User Master | `table User` with argon2/sha256 hashing, lockout counters, status. |
| **Operational Limits**| Table 91 `User Setup` | Custom Settings / Validation Rules | User Parameter IDs (`PID`) | `table UserSetup` with posting windows and approval monetary limits. |
| **Permission Sets** | System & User Permission Sets (RIMDX)| Permission Sets & Permission Set Groups | PFCG Authorization Roles & Objects | `table PermissionSet` + `table Permission` (CRUDX + Field-Level Security). |
| **Exclusion / Muting**| Exclude Permission Sets | Muting Permission Sets | Explicit Exclusion Profiles | `PermissionSet.setType = Muting` to subtract rights cleanly. |
| **Record Filtering** | Security Filters (XML/JSON expressions)| Sharing Rules, Role Hierarchy, OWD | Org Element Restrictions (Company, Plant)| `table SecurityFilter` injecting dynamic SQL predicates into AST queries. |
| **Profiles & Roles** | Table 2000000072 `User Profile` | User Profiles (UI layouts & default app) | Fiori Launchpad Spaces & Roles | `table Profile` defining Role Center dashboard and UI defaults. |
| **Personalization** | Table 2000000178 `User Personalization` | Pinned Apps, Custom List Views | `SU3` User Defaults & Personalization | `table UserPersonalization` (locale/theme) + `table PagePersonalization` (deltas). |

---

## 3. Package Structure: `packages/bos.security`

```
packages/bos.security/
├── bos.json                       # Package manifest
├── package.bos                    # Single-file manifest bundle
├── tables/
│   ├── User.bos                   # Master user identity
│   ├── UserSetup.bos              # Operational posting and approval boundaries
│   ├── Role.bos                   # Organizational job roles
│   ├── UserRole.bos               # User-to-role assignment (with company scoping)
│   ├── PermissionSet.bos          # Entitlement set definitions
│   ├── RolePermissionSet.bos      # Mapping permission sets into roles
│   ├── Permission.bos             # Granular CRUDX + FLS rules
│   ├── SecurityFilter.bos         # Record-level row filters
│   ├── UserGroup.bos              # Departmental/functional user groups
│   ├── UserGroupMember.bos        # Group membership
│   ├── UserGroupRole.bos          # Group role assignment
│   ├── Profile.bos                # Role center and layout templates
│   ├── UserPersonalization.bos    # User preferences and locale
│   ├── PagePersonalization.bos    # Per-page UI customization JSON deltas
│   ├── SessionToken.bos           # Active sessions and client details
│   └── SecurityAuditLog.bos       # Immutable security audit trail
└── codeunits/
    ├── UserManager.bos            # User provisioning, lockouts, credential security
    ├── PermissionManager.bos      # Effective permission resolution, muting, checking
    ├── SecurityFilterEngine.bos   # Dynamic row filter injection
    ├── ProfileManager.bos         # Profile provisioning and Role Center dispatch
    ├── PersonalizationManager.bos # Preference persistence and delta management
    └── AuthManager.bos            # Login, session generation, token revocation
```

---

## 4. Entity Schema Specifications

### 4.1. `User` (Identity Master)
```bos
table User {
    caption = "User Security Master";
    tableType = Normal;
    dataPerCompany = false;

    username: String(50) {
        notNull = true;
        unique = true;
    };
    email: String(100) {
        notNull = true;
        unique = true;
    };
    passwordHash: String(255) {
        notNull = true;
    };
    fullName: String(100);
    isActive: Boolean {
        default = true;
    };
    userType: Option("Standard", "Administrator", "External", "Service") {
        default = "Standard";
    };
    lockedUntil: DateTime;
    failedAttempts: Integer {
        default = 0;
    };
    mfaEnabled: Boolean {
        default = false;
    };
    mfaSecret: String(64);
    expiryDate: Date;

    primaryKey(username);
}
```

### 4.2. `UserSetup` (Operational Boundaries)
```bos
table UserSetup {
    caption = "User Operational Setup";
    tableType = Normal;
    dataPerCompany = true;

    username: String(50) {
        notNull = true;
    };
    allowPostingFrom: Date;
    allowPostingTo: Date;
    salesApprovalLimit: Decimal {
        default = 0.00;
    };
    purchaseApprovalLimit: Decimal {
        default = 0.00;
    };
    approverUsername: String(50);
    responsibilityCenter: String(20);
    canEditPostingDate: Boolean {
        default = false;
    };

    primaryKey(username);
}
```

### 4.3. `PermissionSet` & `Permission`
```bos
table PermissionSet {
    caption = "Permission Set Definition";
    tableType = Normal;
    dataPerCompany = false;

    code: String(30) {
        notNull = true;
        unique = true;
    };
    name: String(100) {
        notNull = true;
    };
    setType: Option("Normal", "Muting") {
        default = "Normal";
    };
    isSystem: Boolean {
        default = false;
    };

    primaryKey(code);
}

table Permission {
    caption = "Granular Object & Field Permission";
    tableType = Normal;
    dataPerCompany = false;

    id: String(64) {
        notNull = true;
        unique = true;
    };
    permissionSetCode: String(30) {
        notNull = true;
    };
    objectType: Option("Table", "Codeunit", "Page", "Report", "System") {
        notNull = true;
    };
    objectName: String(100) {
        notNull = true;
    };
    fieldName: String(100); // Empty for entire object, specified for FLS
    canRead: Boolean { default = false; };
    canInsert: Boolean { default = false; };
    canModify: Boolean { default = false; };
    canDelete: Boolean { default = false; };
    canExecute: Boolean { default = false; };
    isIndirect: Boolean { default = false; }; // Allows execution via trusted codeunit

    primaryKey(id);
    index(permissionSetCode, objectType, objectName);
}
```

### 4.4. `Profile` & `UserPersonalization`
```bos
table Profile {
    caption = "User Profile & Role Center";
    tableType = Normal;
    dataPerCompany = false;

    code: String(30) {
        notNull = true;
        unique = true;
    };
    name: String(100) {
        notNull = true;
    };
    roleCenterPage: String(100) {
        notNull = true;
    };
    isDefault: Boolean {
        default = false;
    };
    isEnabled: Boolean {
        default = true;
    };

    primaryKey(code);
}

table UserPersonalization {
    caption = "User Environment Personalization";
    tableType = Normal;
    dataPerCompany = false;

    username: String(50) {
        notNull = true;
    };
    profileCode: String(30);
    defaultCompany: String(50);
    language: String(10) { default = "en-US"; };
    locale: String(10) { default = "en-US"; };
    timeZone: String(50) { default = "UTC"; };
    dateFormat: String(20) { default = "YYYY-MM-DD"; };
    numberFormat: String(20) { default = "#,##0.00"; };
    theme: Option("light", "dark", "system") { default = "system"; };

    primaryKey(username);
}
```

---

## 5. Execution Pipeline & Effective Permission Evaluation

1. **Permission Resolution Algorithm:**
   - **Step 1 (Collection):** Aggregate all Permission Sets assigned to the user directly (`UserRole` -> `RolePermissionSet`) and via groups (`UserGroupMember` -> `UserGroupRole` -> `RolePermissionSet`).
   - **Step 2 (Additive Union):** For each object/field, calculate `Union(NormalSets)`: any permission with `canRead = true` sets effective read to true.
   - **Step 3 (Muting Subtraction):** For all assigned Muting Sets, apply subtractive masks: if any active muting set specifies `muteRead = true`, effective read is forced to false.
   - **Step 4 (Indirect Privileges):** When executing codeunits marked `@trustedTransaction`, user temporarily gains indirect write access to dependent ledger tables.

---

## 6. Svelte 5 Visual Studio Component Plan

- **`UserListPage` & `UserCard`:** Filterable grid with avatar badges, lockout status indicators, role tags, and multi-factor status.
- **`UserSetupList`:** Operational table view showing posting dates and approval limits with instant cell editing.
- **`PermissionMatrixGrid`:** Interactive grid mapping objects (rows) against permissions (CRUDX columns) with instant save and diff previews.
- **`EffectivePermissionsViewer`:** Diagnostic tool allowing admins to simulate effective rights for any user in any company.
- **`PersonalizationEditor`:** Real-time visual customizer in Studio allowing users to drag columns, adjust spacing, freeze headers, and preview date formats.

---

## 7. Verification & Quality Assurance Gates

1. **Compilation Validation:** Package bundle must compile via `bosc-cli package build` with 0 warnings/errors.
2. **Detector Scan:** Mode 1 anti-mock scan must return **`VERDICT: FLAWLESS (PASSED)`** (no `todo!()`, mock passwords, or dummy permissions).
3. **Tester Fuzzing Matrix:**
   - Brute-force lockout tests (verify account locks after configured failed attempts).
   - Password hash verification (verify hash format and resistance to timing attacks).
   - Muting permission set precedence tests.
   - Cross-company permission isolation tests.
