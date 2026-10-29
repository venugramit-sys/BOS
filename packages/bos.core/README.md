# BOS Enterprise Core Platform Mega-Module (`bos.core`)

## Overview
`bos.core` is the foundational mega-module for the Business Operating System (BOS). It consolidates all enterprise environment governance, identity management, multi-factor authentication, security hardening, and machine-to-machine integrations into a unified, modular package.

The package is partitioned into five distinct feature domain definitions:

```
packages/bos.core/
├── bos.json                       # Package Manifest
├── 01_app_management.bos          # Multi-Tenant, Multi-Company, Regional & Theming
├── 02_user_management.bos         # User Master, Roles, User Groups & Personalization
├── 03_user_auth.bos               # Sessions, Lockout, Argon2, TOTP MFA, Google SSO & VITS Override
├── 04_third_party_integration.bos # Scoped API Keys, OAuth IdP Providers & Service Accounts
├── 05_rbac_and_menus.bos          # Role Centers, Permissionsets & Navigation Trees
└── README.md
```

---

## Feature Domains

### 1. App & Environment Management (`01_app_management.bos`)
- **Singleton Configuration (`CoreSetup` & `CoreSetupCard`)**:
  - Application identity (branding, title, logo, favicon, support contact).
  - Global color palettes (`CorporateBlue`, `Slate`, `Emerald`, `Violet`, `Amber`, `Rose`, `Zinc`) and UI mode (`Light`, `Dark`, `SystemAuto`).
  - Regional localization (ISO date formats, 24h/12h time formats, number separators, base currency, currency symbols).
  - Global posting date boundaries (`allowPostingFrom`, `allowPostingTo`) preventing accidental transactions into closed fiscal periods.
  - Planned maintenance windows with broadcast message banners.
- **Multi-Tenant Administration (`Tenant`, `TenantList`, `TenantCard`)**:
  - Tenant organization master with subscription tier quotas, storage allocations, and custom domain slugs.
- **Multi-Company Management (`Company`, `CompanyList`, `CompanyCard`)**:
  - Legal business entities and subsidiaries with tax/VAT registration numbers, addresses, base reporting currencies, and fiscal year configurations.
- **Currencies & Countries (`Currency`, `CountryRegion`)**:
  - ISO currency definitions with decimal precision and base reporting indicators.
  - ISO 3166-1 country and region master records with address formatting standards.
- **Procedural Engine (`CoreManager`)**:
  - Central operational services for setup initialization, company/tenant provisioning, and formatting utilities.

### 2. User & Profile Management (`02_user_management.bos`)
- **User Master (`User`, `UserList`, `UserCard`)**:
  - Clean, singular PascalCase naming (`User`).
  - Active session counts, lockout timestamps, last login tracking.
- **Roles & RBAC (`Role`, `RoleList`, `RoleCard`, `UserRole`)**:
  - Fine-grained role definitions and user mappings.
- **User Groups & Teams (`UserGroup`, `UserGroupMember`)**:
  - Hierarchical user groups (e.g. `Finance Department`, `Operations`) with default role inheritance.
- **User Personalization (`UserPersonalization`, `UserPersonalizationCard`)**:
  - Per-user overrides for theme palette, UI mode, locale, timezone, and default active company.
- **User Security Overrides (`UserSecuritySetting`)**:
  - Per-user MFA settings, session concurrency caps, and password expiry policies.
- **Procedural Engine (`UserManager`)**:
  - User group membership management and profile aggregation.

### 3. Authentication, Security & Hardening (`03_user_auth.bos`)
- **Dynamic Policy Setup (`AuthSetup` & `AuthSetupCard`)**:
  - Singleton table with FastTabs for Password Policy, Account Lockout, Session Policy, MFA Settings, and Google Federation.
  - Zero magic numbers: `AuthManager` dynamically reads all thresholds from `AuthSetup`.
- **Blowfish bcrypt Password Hashing**:
  - Ultra-efficient, industry-standard 4 KB RAM footprint per hash resisting GPU brute force while preventing memory exhaustion during high-concurrency login bursts. Multi-format verifier maintains backward compatibility for existing Argon2id and SHA-256 hashes.
- **Production-Hardened VITS Developer Override**:
  - Emergency super-developer access requires non-default keys and is automatically disabled in production environments.
- **Strict Linear Audit Hash Chain**:
  - PostgreSQL advisory transaction locking guarantees fork-free, tamper-evident cryptographic chaining for all security events.
- **RFC 6238 TOTP Multi-Factor Authentication**:
  - Compatible with Google Authenticator, Microsoft Authenticator, 1Password, Bitwarden.
  - Standard `otpauth://totp/` URI and 160-bit shared secret generation (`GenerateTotpSecret`).
  - Rolling 6-digit verification with drift tolerance (`VerifyTotp`).
- **Google & External Identity Federation**:
  - OpenID Connect (OIDC) authentication flow (`AuthenticateGoogle`).
  - Just-In-Time (JIT) provisioning for verified corporate Google emails.
- **Cryptographic Session Lifecycle (`UserSession`)**:
  - Automatic expiration, heartbeat activity tracking, and token revocation.
- **Immutable Security Audit Log (`SecurityAuditLog`)**:
  - Append-only audit trail with hash chaining (ADR 0007).
- **VITS Super Developer & Emergency Override (`EmergencyDeveloperOverride`)**:
  - Provides a secure, dedicated bypass for developers and automated test suites.
  - Prevents development lockouts during active coding, test automation, and package builds.
  - Verifies emergency override key, auto-unlocks root administrator, generates full `SUPER` session, and writes an explicit audit record (`auth.vits_override.invoked`).

### 4. Third Party Integration & M2M Access (`04_third_party_integration.bos`)
- **Scoped API Keys (`ApiKey`, `ApiKeyList`, `ApiKeyCard`)**:
  - Cryptographic keys prefixed with `bos_live_...`.
  - Stored strictly as SHA-256 hashes at rest; raw keys displayed only once upon generation.
  - Expiration dates, last-used timestamps, and permission scopes (e.g. `api:read`, `api:write`).
- **External Identity Providers (`ExternalAuthProvider`)**:
  - Configuration store for Google, Microsoft Entra ID, Okta, and SAML 2.0 endpoints.
- **Service Accounts (`ServiceAccount`)**:
  - Non-human workload accounts for background Daemons and cron integrations.
- **Procedural Engine (`IntegrationManager`)**:
  - Key generation, constant-time hash verification, and key revocation.

### 5. RBAC & Navigation Menus (`05_rbac_and_menus.bos`)
- **Permission Sets**:
  - `CoreAdmin`: Complete platform entitlements and codeunit execution privileges.
  - `SecurityAdmin`: Identity, MFA, and integration configuration privileges.
  - `CoreUser`: Standard business workspace read entitlements.
- **Role-Center Menus**:
  - `CorePlatformMenu`: Comprehensive platform administration sidebar.
  - `SuperAdminMenu`, `SystemAdminMenu`, `BusinessUserMenu`.

---

## Compilation & Installation
```powershell
# Compile the unified mega-package to AOT QuickJS bytecode and Svelte 5 page modules
cargo run -p bosc-cli -- package build packages/bos.core

# Install into local PostgreSQL dev database
cargo run -p bosc-cli -- package install -i packages/bos.core/bos.core.bospkg -d postgres://postgres:postgres@localhost:5432/bos_dev
```
