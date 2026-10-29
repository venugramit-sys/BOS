# Modular Packaging System & Distribution

> **Document ID:** `BOS-DOC-10`  
> **Source Verification:** `crates/bosc-ir/src/lib.rs`, `crates/bosc-runtime/src/package_manager.rs`  
> **Target Audience:** Solution Architects, Package Maintainers, DevOps Engineers

---

## 1. Overview & Non-Invasive Layering

BOS features a modular package architecture that allows business modules (e.g., Core Financials, Inventory, CRM, Custom Extensions) to be authored, versioned, distributed, and installed as self-contained bundles (`.bospkg`).

```mermaid
graph TD
    TenantLayer["Tenant Customization Layer\n(e.g., tenant.acme.custom)"]
    IndustryLayer["Industry Vertical Layer\n(e.g., bos.manufacturing, bos.retail)"]
    LocLayer["Localization Layer\n(e.g., bos.tax.de, bos.tax.in)"]
    BaseLayer["Base Application Layer\n(e.g., bos.core, bos.auth, bos.financials)"]

    TenantLayer --> IndustryLayer
    IndustryLayer --> LocLayer
    LocLayer --> BaseLayer
```

Packages stack non-invasively: higher layers augment lower layers via `tableextension`, `tablelogicextension`, `pageextension`, and event subscribers without modifying base package source code.

---

## 2. Package Manifest Specification (`bos.json`)

Every package defines a manifest matching `PackageManifestIR` (`crates/bosc-ir/src/lib.rs`):

```json
{
    "package_id": "bos.auth",
    "name": "BOS Enterprise Authentication & Identity",
    "version": "1.0.0",
    "publisher": "BOS Core Engineering Team",
    "dependencies": [
        "bos.core"
    ],
    "optional_features": [
        "mfa_authenticator",
        "saml_sso"
    ]
}
```

### Manifest Fields
- **`package_id`**: Globally unique dot-notated identifier (e.g. `bos.financials`, `acme.logistics`).
- **`name`**: Human-readable title.
- **`version`**: Semantic version string (`MAJOR.MINOR.PATCH`).
- **`publisher`**: Organization or author name.
- **`dependencies`**: Array of package IDs that must be installed and active in the target database before this package can be installed.
- **`optional_features`**: Feature flags declared by the package.

---

## 3. Package Bundle Format (`.bospkg`)

A compiled `.bospkg` file is a verified JSON bundle (`PackageBundleIR`):

```rust
pub struct PackageBundleIR {
    pub manifest: PackageManifestIR,
    pub objects: Vec<serde_json::Value>,
    pub checksum: String,
}
```

### 3.1. Cryptographic Integrity Checksum
During package creation (`bosc package build`), the compiler calculates a SHA-256 digest of the serialized objects array:
- When installing, the runtime validates that the recalculated hash matches the bundle's `checksum`.
- Any manual tampering or corruption results in immediate installation rejection.

---

## 4. Singular Table Naming Standard

BOS enforces clean, singular PascalCase table naming without redundant module acronym prefixes:

| Good (Standard) | Bad (Rejected Anti-Pattern) |
| :--- | :--- |
| `table User` | `table SecUser` |
| `table Role` | `table SecRole` |
| `table SessionToken` | `table SecSessionToken` |
| `table OrderHeader` | `table SalesOrderHeader` |
| `table Account` | `table GLAccount` |

### Why?
- **Package Namespacing**: Isolation is achieved at the metadata layer via the package identifier (`_bos_meta_objects.package_name`), preventing collision between packages.
- **Direct SQL Mapping**: PascalCase `User` maps directly to clean PostgreSQL table `"user"`.
- **Elegance & Readability**: Scripts remain concise, readable, and free from awkward enterprise acronyms.

---

## 5. Package Installation & Lifecycle Protocols

Packages can be installed into target databases via multiple verified mechanisms:

### 5.1. CLI Installation
```powershell
bosc package install -i packages/bos.auth.bospkg -d postgres://postgres:postgres@localhost:5432/bos_dev
```

### 5.2. Dedicated Database Route (`POST /api/databases/:name/packages/install`)
Used when targeting a specific database from an administrative management session:
```http
POST /api/databases/enterprise_tenant_01/packages/install
Content-Type: application/json

<Raw .bospkg JSON Body>
```

### 5.3. Query Parameter Route (`POST /api/packages/install?db=:name`)
Used by automation pipelines:
```http
POST /api/packages/install?db=enterprise_tenant_01
Content-Type: application/json

<Raw .bospkg JSON Body>
```

### 5.4. Package Export (`GET /api/packages/:id/export`)
Retrieves all active metadata objects belonging to a package from the database catalog and serializes them into a verified `.bospkg` bundle for distribution.

---

## 6. Ahead-of-Time (AOT) Bytecode & Static Distribution Architecture

BOS compiles package logic and schema metadata ahead-of-time into optimized static artifacts:

```mermaid
flowchart LR
    Source["package.bos\n(Tables, Logic, Codeunits)"] --> Compiler["bosc package build\n(AOT Bytecode Compiler)"]
    Compiler --> QBC["bytecode/*.qbc\n(QuickJS Bytecode Envelopes)"]
    Compiler --> SQL["schema/tables.sql\n& prepared_queries.json"]
    Compiler --> Manifest["manifest.json\n(Merkle Root & Signatures)"]
    Compiler --> Vault["_bos_sys_package_vault\n(Database Self-Healing Source)"]

    QBC --> NodeCache["Node In-Memory BytecodeCache (< 0.12ms)"]
    SQL --> DBPipeline["PostgreSQL Binary Connection Pipelining"]
    Vault -.->|Auto-Restore on Corruption| NodeCache
```

### 6.1. Binary Bytecode Envelope (`BytecodeArtifactIR`)
All codeunits, tablelogics, and statemachines are compiled directly into QuickJS bytecode envelopes:
- **Magic Header**: `0xBC 0x42 0x4F 0x53` (`BCBOS`)
- **Version**: `1`
- **Integrity**: 32-bit CRC32 checksum over bytecode payload
- **Pre-linked Bundle**: All package routines compiled into `package.bundle.qbc` for instant single-read memory loading.

### 6.2. Authoritative Database Package Vault (`_bos_sys_package_vault`)
PostgreSQL stores the pristine package bundle and its compiled artifacts. If local disk files are ever deleted, corrupted by bit-rot, or locked:
- The node initiates automated self-healing in **< 80 milliseconds**.
- Restores pristine artifacts to disk atomically using temporary staging and atomic directory replacement.

---

## 7. Language Server Protocol (`bosc lsp`)

BOS integrates a native, zero-dependency JSON-RPC 2.0 Language Server Protocol (`bosc lsp`) engine:
- Runs directly on `stdio` for VS Code, Zed, Sublime, Neovim, and browser Monaco editor integration.
- Supported LSP capabilities:
  - `textDocument/didOpen`, `textDocument/didChange`: Real-time incremental syntax checking and AST compilation.
  - `textDocument/publishDiagnostics`: Live error reporting with visual error squiggles, accurate line/column spans, and actionable compiler error messages.
  - `textDocument/completion`: Context-aware keyword, meta-object (`table`, `codeunit`, `page`, `statemachine`, `permissionset`), and field completions.
  - `textDocument/hover`: Instant documentation and type information on hover.
  - `textDocument/documentSymbol`: Breadcrumbs and outline tree symbols for meta-objects.

### Launching the LSP Server
```powershell
bosc lsp
```

---

## 8. Fine-Grained Field Redaction in PermissionSets

BOS supports column-level data security and field masking directly in `permissionset` declarations:

```typescript
permissionset "FinanceClerk" {
    caption = "Accounts Payable Clerk";
    assignable = true;

    tabledata Vendor = RIMD {
        exclude: [contactEmail, taxIdentificationNumber];
        where (tier == "Enterprise");
    };
}
```

### 8.1. Compiler & Runtime Execution
- **Parser (`bosc-syntax`)**: Parses `exclude: [field1, field2]` within `TablePermissionDef`.
- **Semantic Analyzer (`bosc-analyzer`)**: Normalizes field names into canonical snake_case columns.
- **Runtime Masking (`bosc-runtime`)**: When non-admin sessions query records (`get_record` / `query_records`), excluded fields are automatically stripped or masked as `null`/`[REDACTED]` prior to JSON serialization, guaranteeing zero unauthorized data egress at the kernel layer.
