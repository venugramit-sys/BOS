# 05. Modular Packaging & Security System

## 1. Modular Packaging Hierarchy

BOS applications are modular packages identified by reverse domain notation (`bos.core`, `acme.sales`):

```
┌────────────────────────────────────────────────────────┐
│ Tenant Customization Layer (e.g. acme.custom)          │
├────────────────────────────────────────────────────────┤
│ Industry Vertical Layer (e.g. bos.manufacturing)       │
├────────────────────────────────────────────────────────┤
│ Regional Localization Layer (e.g. bos.in.gst)          │
├────────────────────────────────────────────────────────┤
│ Core Platform Layer (e.g. bos.core)          │
└────────────────────────────────────────────────────────┘
```

Lower layers cannot reference objects in upper layers. Upper layers extend lower objects non-invasively using `tableextension`, `pageextension`, and `enumextension`.

---

## 2. `.bospkg` Archive Format

A `.bospkg` bundle is a ZIP archive containing:
- `bos.json`: Manifest detailing package ID, version, publisher, dependencies, and SHA-256 integrity hash.
- `ir/`: Pre-compiled intermediate representation JSON envelopes.
- `bytecode/`: QuickJS compiled bytecode files (`.qbc`) with CRC32 envelopes.
- `sql/`: Deterministic DDL migration scripts.
- `signature.sig`: Detached Ed25519 digital signature verifying the publisher's cryptographic identity.

---

## 3. Role-Based Access Control (RBAC) & Security Profiles

Security and identity in BOS are managed via `bos.core`:

### Core Entities:
- **`Tenant`**: Top-level organization boundary.
- **`Company`**: Legal entity within a tenant.
- **`User`**: Account entity authenticated via argon2id hashed passwords or external OIDC/OAuth2.
- **`Role`**: Grouping of permissions assigned to users (e.g., `PurchasingClerk`).
- **`PermissionSet`**: Granular CRUD permissions per table or meta-object:
  ```bos
  permissionset PurchasingClerkPermissions {
      table "PurchaseHeader" = RIMD;
      table "PurchaseLine" = RIMD;
      table "Vendor" = R;
      table "VendorLedgerEntry" = R;
      codeunit "PurchasePost" = X;
  }
  ```
  *(R = Read, I = Insert, M = Modify, D = Delete, X = Execute)*

---

## 4. Immutable Audit Trails & Cryptographic Hash Chaining

For compliance with financial regulations and forensic audits:
1. Every write to critical business ledgers generates an audit record in `_bos_audit_log`.
2. Each audit row contains:
   - `id`: UUIDv7
   - `sequence_no`: Monotonically increasing BigInt
   - `timestamp`: UTC Timestamp
   - `user_id`: UUID
   - `action`: INSERT, UPDATE, or DELETE
   - `payload`: JSONB snapshot
   - `prev_hash`: SHA-256 hash of previous audit row
   - `hash`: SHA-256(`prev_hash` + `sequence_no` + `payload` + `timestamp`)
3. Tampering with any historical database row breaks the cryptographic chain, triggering immediate integrity alarms.
