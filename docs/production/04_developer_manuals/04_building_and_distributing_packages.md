# Volume 4: Developer Manuals
## 04. Building & Distributing Packages

BOS modular packages are self-contained, versioned distribution units (`.bospkg`) containing metadata schemas, compiled bytecode, database DDL, and cryptographic signatures.

---

## 📦 Anatomy of a Package Directory

A package source folder contains:

```
packages/acme.procurement/
├── bos.json                # Package Manifest
├── tables/
│   ├── purchase_order.bos
│   └── vendor.bos
├── logic/
│   ├── purchase_order.bl.bos
│   └── vendor.bl.bos
├── codeunits/
│   └── purchase_post.bos
└── pages/
    ├── purchase_order_list.bos
    └── purchase_order_card.bos
```

### The Package Manifest (`bos.json`)
```json
{
  "id": "acme.procurement",
  "name": "Acme Advanced Procurement",
  "version": "1.0.0",
  "publisher": "Acme Corporation",
  "dependencies": [
    {
      "id": "bos.core",
      "version": "^0.1.0"
    }
  ],
  "entry": "tables/purchase_order.bos"
}
```

---

## 🔨 Compiling a Package (`bosc package build`)

To compile the source directory into a `.bospkg` bundle:

```powershell
bosc package build packages/acme.procurement/ --output dist/acme.procurement.bospkg
```

### What Happens During Build?
1. **Source Discovery**: Recursively discovers all `.bos` and `.bl.bos` files.
2. **Grammar & AST Parsing**: Validates syntax across all meta-objects.
3. **OxC Type Stripping**: Strips TypeScript types and verifies semantic consistency.
4. **Intermediate Representation**: Serializes AST to `ir/` JSON envelopes.
5. **QuickJS AOT Compilation**: Compiles JavaScript logic into `.qbc` bytecode envelopes with CRC32 integrity hashes.
6. **PostgreSQL DDL Generation**: Emits idempotent database table creation scripts.
7. **Bundle Packaging**: Archives all assets into a compressed ZIP container (`.bospkg`).

---

## 📥 Installing a Package (`bosc package install`)

Install the compiled package into your target PostgreSQL database:

```powershell
bosc package install dist/acme.procurement.bospkg --db postgres://postgres:postgres@localhost:5432/bos_dev
```

### Installation Steps Executed by Runtime:
1. Manifest and dependency tree validation.
2. Declarative schema synchronization and table creation.
3. Bytecode registration in `_bos_meta_objects`.
4. Package registration in `_bos_meta_packages`.

---

## 📋 Listing Installed Packages

Verify installed packages and versions:

```powershell
bosc package list --db postgres://postgres:postgres@localhost:5432/bos_dev
```

**Output:**
```
Installed Packages (2):
- bos.core (v1.0.0) by BOS Systems [Core Kernel]
- acme.procurement (v1.0.0) by Acme Corporation [Active]
```
