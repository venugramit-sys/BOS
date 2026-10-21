# Volume 1: Data Modeling — `table`, `tableextension`, Data Types, FlowFields & Storage Strategies

This volume documents every structural data-modeling feature in **BOS-Script**, how the Rust compiler (`bosc-syntax`, `bosc-analyzer`, `bosc-codegen-db`) lowers declarations into physical PostgreSQL DDL, and real-world enterprise patterns.

---

## 1. The `table` Meta-Object

A `table` defines a physical entity in the active PostgreSQL database. By architectural convention in BOS:
- **Table names** use singular `PascalCase` without module prefixes (e.g., `Customer`, `SalesHeader`, `SalesLine`, `GeneralLedgerEntry`, `Note`), which compile to singular `snake_case` SQL table names (`customer`, `sales_header`, `sales_line`, `general_ledger_entry`, `note`).
- **Structural purity**: Physical schema, keys, constraints, and declarative `flowField` formulas belong inside `table`. Procedural validations (`validate`) and lifecycle triggers (`onInsert`, `onModify`, `onDelete`) belong in a companion `tablelogic` object (though inline triggers inside `table` are also supported for backward compatibility, emitting compiler hint `BOS2008`).

### 1.1. Table Properties (`tableType`, `dataPerCompany`, `extensible`, `caption`, `onDelete`)

| Property | Accepted Values | Default | Behavior in Compiler & Database Engine |
| :--- | :--- | :--- | :--- |
| `tableType` | `Master`, `Document`, `Ledger`, `Setup`, `Supplemental`, `Register`, `Temporary` | `Master` (or `Normal`) | Controls semantic rules and DDL triggers. `Ledger` tables automatically receive an immutability trigger (`trg_immutable_<table>`) blocking `UPDATE` and `DELETE` unless a privileged statutory override is active. `Document` tables are checked for a `status` column (`BOS2005`). |
| `dataPerCompany` | `true` \| `false` | `true` | When `true`, injects `"_company_id" UUID NOT NULL` and scopes records per company according to the active storage strategy. When `false`, the table is shared across all companies within the tenant. |
| `extensible` | `true` \| `false` | `true` | Indicates whether downstream packages may declare a `tableextension` targeting this table. |
| `caption` | `"Human Label"` | Table name | Display name stored in `_bos_meta_objects.caption` and JSDoc in `/api/meta/ambient.d.ts`. |
| `onDelete` | `"RestrictIfLedgerExists"` \| `"Restrict"` \| `"Cascade"` | Inferred from FlowFields | Generates PostgreSQL `BEFORE DELETE` trigger `trg_guard_delete_<table>` preventing deletion of master records when child ledger rows exist. |

### 1.2. Automatically Injected System Columns

Every physical BOS table created by `PostgresGenerator::generate_table` automatically includes 7 system columns:

| System Column | PostgreSQL Type | Purpose |
| :--- | :--- | :--- |
| `_id` | `UUID PRIMARY KEY DEFAULT gen_random_uuid()` | Surrogate row identifier used by internal OCC updates, row locks, and REST APIs. |
| `_tenant_id` | `UUID NOT NULL` | Multi-tenant isolation key enforced by PostgreSQL Row-Level Security (`tenant_isolation_<table>`). |
| `_company_id` | `UUID NOT NULL` *(when `dataPerCompany: true`)* | Legal-entity / company isolation key within the tenant. |
| `_created_at` | `TIMESTAMPTZ NOT NULL DEFAULT NOW()` | Immutable creation timestamp. |
| `_created_by` | `UUID` | Authenticated user UUID (`ctx.userId`) who created the record. |
| `_updated_at` | `TIMESTAMPTZ NOT NULL DEFAULT NOW()` | Last modification timestamp automatically updated on `UPDATE`. |
| `_system_version` | `BIGINT NOT NULL DEFAULT 1` | Optimistic Concurrency Control (OCC) version counter incremented on every `UPDATE`. |

---

## 2. Complete BOS-Script Data Types Reference

BOS-Script supports both colon-style field declarations (`field_name: Type modifiers;`) and block-style property blocks (`field_name: Type { notNull = true; unique = true; };`).

| BOS-Script Type | Syntax Example | Compiled PostgreSQL Type | TypeScript (`ambient.d.ts`) Type | Real-World Use Case |
| :--- | :--- | :--- | :--- | :--- |
| `UUID` / `Guid` | `id: UUID notNull;` | `UUID` | `UUID` (`string`) | Primary keys, foreign keys (`customer_id`), correlation IDs. |
| `String(N)` | `name: String(100) notNull;` | `VARCHAR(N)` *(default `255`)* | `string` | Customer names, descriptions, email addresses. |
| `Code(N)` | `code: Code(20) notNull;` | `VARCHAR(N)` | `string` | Uppercase alphanumeric business identifiers (`CUST-0001`, `USD`, `WH-MAIN`). |
| `Text` | `notes: Text;` | `TEXT` | `string` | Unbounded long-form markdown, terms & conditions, JSON logs. |
| `Integer` / `Int` | `line_no: Integer default(10000);` | `INTEGER` | `Integer` (`number`) | Document line numbers (`10000`, `20000`), quantities, priority levels. |
| `BigInteger` / `BigInt` | `entry_no: BigInteger notNull;` | `BIGINT` | `BigInteger` (`bigint \| number`) | High-volume sequential ledger entry numbers. |
| `Decimal(p, s)` | `amount: Decimal(18, 2) default(0.00);` | `NUMERIC(p, s)` | `Decimal` (`BosDecimal \| number`) | Exact fixed-point currency amounts, unit prices (`Decimal(18, 4)`), tax rates. Never uses float! |
| `Boolean` / `Bool` | `is_blocked: Boolean default(false);` | `BOOLEAN` | `boolean` | Flags (`is_blocked`, `allow_post`, `tax_liable`). |
| `Date` | `posting_date: Date notNull;` | `DATE` | `string` (`YYYY-MM-DD`) | Accounting posting dates, due dates, shipment dates. |
| `DateTime` | `posted_at: DateTime;` | `TIMESTAMPTZ` | `string` (ISO-8601) | Exact UTC audit and event timestamps. |
| `Enum(Name)` or `<EnumName>` | `status: OrderStatus;` | `VARCHAR(50)` | `<EnumName>Enum` union | Strongly-typed status or category restricted to declared `enum` values. |
| `Json` | `metadata: Json;` | `JSONB` | `Record<string, any>` | Extensible attribute bags, webhook payloads, dynamic dimensions. |
| `Blob` | `attachment: Blob;` | `BYTEA` | `string` | Binary document attachments, PDFs, signatures. |
| `FlowFilter(Type)` | `date_filter: FlowFilter(Date);` | Virtual *(not persisted)* | `string` | Dynamic runtime filter parameter injected into FlowField subqueries. |

### 2.1. Field Modifiers & Inline Block Properties

Fields can be configured using concise inline keywords or `{ ... }` property blocks:

```bos
table Product {
    tableType: Master;

    // 1. Concise inline modifiers
    id: UUID notNull;
    sku: String(30) notNull unique caption("Stock Keeping Unit");
    unit_price: Decimal(18, 4) notNull default(0.0000) check("unit_price >= 0");
    unit_cost: Decimal(18, 4) notNull default(0.0000);

    // 2. Stored computed column. An expression becomes quoted snake_case SQL.
    //    A string is stored unchanged: formula("\"unit_price\" - \"unit_cost\"")
    margin_amount: Decimal(18, 4) { formula = unit_price - unit_cost; };

    // 3. Block-style field properties (as used in Visual Table Designer)
    barcode: String(50) {
        notNull = false;
        unique = true;
        caption = "EAN / UPC Barcode";
    };

    primaryKey(id);
    key(idx_product_sku, sku);
}
```

---

## 3. Keys, Secondary Indexes & SIFT Covering Indexes

1. **Primary Key (`primaryKey(col1, ...)`):**
   - Every table should declare `primaryKey(...)` (missing it triggers compiler warning `BOS2002` with a 1-click Quick Fix in Monaco).
   - Used by `QuickJsEngine::resolve_pk_col` (`_bos_meta_objects.definition->'primary_key'->>0`) and `DataEngine` referential integrity checks.
2. **Secondary B-Tree Indexes (`key(idx_name, col1, col2)`):**
   - Compiles to `CREATE INDEX IF NOT EXISTS "idx_name" ON "<table>" ("col1", "col2");`.
   - Under the `Partitioned` storage strategy, unique indexes automatically prepend `"_tenant_id"` so PostgreSQL partitioned unique constraints succeed.

---

## 4. Declarative FlowFields & SIFT (`flowField(...)`)

Inspired by Microsoft Dynamics NAV/BC, **FlowFields** eliminate N+1 query loops by declaring virtual calculated fields that compile into SQL subqueries, a `<table_name>_view` SQL view, and automatic **SIFT covering indexes** (`CREATE INDEX ... ON "<target_table>" ("_tenant_id") INCLUDE ("<target_col>")`).

### Supported Aggregation Formulas
- `sum(TargetTable.target_field where (fk_col == this.pk_col))`
- `count(TargetTable where (fk_col == this.pk_col))`
- `average(TargetTable.target_field where (...))`
- `min(TargetTable.target_field where (...))`
- `max(TargetTable.target_field where (...))`
- `lookup(TargetTable.target_field where (...))`
- `exist(TargetTable where (...))`

### Real-World Use Case: Customer Master with Live Ledger Balance & Open Order Count

```bos
table CustomerLedgerEntry {
    tableType: Ledger;
    dataPerCompany: true;

    id: UUID notNull;
    customer_id: UUID notNull;
    posting_date: Date notNull;
    document_no: String(30) notNull;
    amount: Decimal(18, 2) notNull;
    remaining_amount: Decimal(18, 2) notNull;
    is_open: Boolean default(true);

    primaryKey(id);
    key(idx_cle_customer, customer_id, posting_date);
}

table Customer {
    tableType: Master;
    dataPerCompany: true;
    caption: "Customer";
    onDelete: "RestrictIfLedgerExists";

    id: UUID notNull;
    code: String(20) notNull unique;
    name: String(150) notNull;
    credit_limit: Decimal(18, 2) default(10000.00);
    is_blocked: Boolean default(false);

    // Declarative FlowFields: Zero N+1 queries!
    balance_lcy: Decimal(18, 2) flowField(sum(CustomerLedgerEntry.amount where (customer_id == this.id)));
    open_entry_count: Integer flowField(count(CustomerLedgerEntry where (customer_id == this.id)));

    primaryKey(id);
    key(idx_customer_code, code);
}
```

**What `bosc-codegen-db` Generates for `Customer`:**
1. Physical table `"customer"` with RLS policy `"tenant_isolation_customer"`.
2. SIFT covering index `"idx_sift_customer_ledger_entry_amount"` on `"customer_ledger_entry" ("_tenant_id") INCLUDE ("amount")`.
3. Pushdown SQL view `"customer_view"` projecting `t.*` plus the `COALESCE(SUM("amount"), 0)` subquery.
4. Referential deletion guard trigger `"trg_guard_delete_customer"` blocking `DELETE FROM "customer"` if any row exists in `"customer_ledger_entry"` for that customer!

---

## 5. Non-Invasive Schema Evolution (`tableextension`)

When building vertical industry packages or tenant customizations (e.g., Healthcare, Retail Loyalty, Manufacturing), never modify the base `Customer` or `SalesHeader` source file. Instead, declare a `tableextension`:

### Real-World Use Case: Extending Base `Customer` for Cold-Chain Pharmaceuticals

```bos
tableextension PharmaCustomerExt extends Customer {
    dea_license_no: String(30) caption("DEA License Number");
    dea_expiry_date: Date caption("DEA License Expiration");
    cold_chain_certified: Boolean default(false);
    max_temp_celsius: Decimal(5, 2) default(8.00);

    key(idx_customer_dea, dea_license_no);
}
```

**How `MetaManager` & `PostgresGenerator` Apply `tableextension`:**
- Runs the schema synchronizer, which adds the extension columns (and their relations) to every physical copy of the table under the active storage strategy (`public."customer"`, `"comp_<code>"."customer"`, or `"<code>$customer"`). Redeploying the base table never drops extension columns. See [`27_`](../27_transactional_integrity_and_schema_migration.md) §5.
- Updates the live `ambient.d.ts` and Monaco catalog so `rec.dea_license_no` and `models.Customer.findMany({ where: { cold_chain_certified: true } })` immediately autocomplete across all `tablelogic` and `codeunit` files.
