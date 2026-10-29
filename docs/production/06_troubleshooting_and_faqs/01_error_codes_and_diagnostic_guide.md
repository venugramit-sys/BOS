# Volume 6: Troubleshooting & FAQs
## 01. Error Codes & Diagnostic Guide

This guide catalogs common BOS compiler diagnostics, runtime exception codes, and practical steps to resolve them.

---

## 🛑 BOS Compiler Diagnostics Catalog

Compiler diagnostics follow the format `BOS<CATEGORY><CODE>`:

| Error Code | Category | Description | Remediation |
| :--- | :--- | :--- | :--- |
| `BOS1001` | Syntax | Unexpected token in `.bos` declaration | Check for missing semicolons, unmatched braces `{}`, or invalid field keywords. |
| `BOS1002` | Syntax | Invalid meta-object keyword | Ensure the meta-object is one of the 18 valid keywords (`table`, `codeunit`, `page`, etc.). |
| `BOS2001` | Semantic | Unknown field reference in FlowField | Verify that the target table and field name exist in the current or imported package. |
| `BOS2002` | Semantic | Duplicate primary key definition | A table may only declare a single primary key `key PK(...)`. |
| `BOS2003` | Semantic | Type mismatch in assignment | Ensure assigned value matches target field type (e.g., cannot assign String to Decimal). |
| `BOS2004` | Semantic | Cyclic package dependency detected | Check `bos.json` manifests; package A cannot depend on package B if package B depends on A. |
| `BOS3001` | Page IR | Unknown sourceTable in page definition | Ensure the table specified in `sourceTable: TableName` is defined and imported. |
| `BOS3002` | Page IR | Invalid action target | Verify that `runPage` or `runCodeunit` references a valid meta-object. |

---

## ⚡ Runtime Exception Codes

When an Axum API endpoint or QuickJS runtime hook fails, it returns a structured JSON error payload:

```json
{
  "error": "RESOURCE_CONFLICT",
  "code": "BOS_ERR_409",
  "message": "Record was modified by another session (system version mismatch).",
  "details": {
    "table": "Customer",
    "expected_version": 4,
    "current_version": 5
  }
}
```

### Common Runtime Error Codes:
- **`BOS_ERR_401 (UNAUTHORIZED)`**: Missing, invalid, or expired session token. Re-authenticate via `POST /api/auth/login`.
- **`BOS_ERR_403 (FORBIDDEN)`**: Active user's assigned PermissionSets do not grant the required table/codeunit privilege (`RIMDX`).
- **`BOS_ERR_404 (NOT_FOUND)`**: Target record or meta-object does not exist.
- **`BOS_ERR_409 (RESOURCE_CONFLICT)`**: Optimistic Concurrency Control violation (`_system_version` mismatch). Reload latest record state.
- **`BOS_ERR_422 (UNPROCESSABLE_ENTITY)`**: Field validation trigger threw an error. Inspect the `message` string for the exact business constraint.
- **`BOS_ERR_500 (SANDBOX_LIMIT_EXCEEDED)`**: QuickJS VM exceeded 8 MB memory cap or 100,000 instruction fuel limit. Optimize loops or push heavy logic to database FlowFields.

---

## 🔍 Diagnostic Probing Commands

### 1. Database Connection & System Kernel Check
```powershell
bosc node check --config nodes/dev.toml
```

### 2. Live Verification Suite
```powershell
bosc test-live --db postgres://postgres:postgres@localhost:5432/bos_dev
```

### 3. Dry-Run Schema Verification
```powershell
bosc deploy --input package.bos --db postgres://postgres:postgres@localhost:5432/bos_dev --dry-run
```
