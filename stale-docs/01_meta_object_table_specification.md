# Meta-Object Specification: `table` & `tableextension`

> **Status:** Final Frozen Specification  
> **Document:** `docs/01_meta_object_table_specification.md`  
> **References:** MS Dynamics NAV / Business Central table architecture, In-Table Draft State Lifecycle, PostgreSQL RLS & Schema Engine.  

---

## 1. Real-World Deep Dive: Enterprise Draft Handling (In-Table Status Pattern)

### 1.1 The Architecture: Why In-Table Drafts?
Rather than creating separate shadow tables (which introduce schema duplication, migration synchronization overhead, and foreign key drift), BOS uses the **In-Table Draft State Pattern**:
- The table contains a designated status column containing `'Draft'` as an option (or the compiler provisions a native `_record_state: Enum("Draft", "Active", "Archived") = "Active"`).
- **Single Source of Truth:** A single table schema, single set of columns, and single migration history.

### 1.2 Real-World Lifecycle Walkthrough
Imagine a procurement manager creating a **Purchase Order with 80 line items**:

```
[Keystroke / Line Added] ──► Auto-Save into PurchaseHeader with status = "Draft"
                                      │
                                      ├── Relaxed Validation (Non-blocking warnings)
                                      ├── Filtered out of Active GL & Warehouse Queries
                                      └── Available across browser sessions & devices
                                      │
                         User clicks "Submit" / "Post"
                                      │
                                      ▼
                      ┌────────────────────────────────┐
                      │    Activation Transaction      │
                      │ 1. Header & Line Validations   │
                      │ 2. Check Credit & Approvals    │
                      │ 3. Atomic Flip to "Released"   │
                      └───────────────┬────────────────┘
                                      │
                       Pass ──────────┴────────── Fail
                        │                          │
                        ▼                          ▼
               Status = "Released"        Status stays "Draft"
           (Triggers active workflows)   (Returns validation errors)
```

1. **Auto-Save as Draft:** Every edit saves directly to the table with `status = "Draft"`. Database constraints that require completion (e.g., total balance, vendor confirmation) are relaxed into draft-level warnings.
2. **Operational Isolation:** Active queries, automated warehouse pick-lists, and financial ledgers automatically filter out records where `status == "Draft"` using default view scopes or compiler-injected predicates.
3. **Activation Transaction:**
   - When the manager clicks **"Submit"** or **"Post"**, the engine executes `onDraftActivate()`:
     - Strict validation rules run across the header and lines.
     - Upon passing, the status flips atomically from `"Draft"` to `"Released"` or `"Open"`.
     - Downstream ledger posting and inventory allocation events are fired.
     - If validation fails, the status remains `"Draft"`, and line-by-line error messages are returned to the user without losing their input.
4. **Discard:** Calling `discard()` deletes the draft record cleanly before any active dependencies were created.

---

## 2. Immutable Ledgers with "Legal Superuser Powers"

### 2.1 The Accounting Invariant
In enterprise accounting and regulatory standards (SOX, IFRS, GAAP, statutory VAT), tables designated as `tableType = Ledger;` (e.g., `GeneralLedgerEntry`, `CustomerLedgerEntry`, `ItemLedgerEntry`) are **strictly write-once**:
- Normal business code and API requests are **physically blocked** from executing `UPDATE` or `DELETE` via native PostgreSQL triggers.
- Corrections must be made via **compensating transactions** (reversal entries, credit memos).

### 2.2 The "Privileged Audit Override" (The Legal Escape Hatch)
In exceptional circumstances (e.g., statutory court order, database repair, fixing severe posting metadata corruption), a certified system administrator must have a legal way to correct data.

BOS implements a **Cryptographically Verified Privileged Override**:
1. **Session-Scoped Elevation Token:**
   The admin initiates an elevated session providing a mandatory legal justification and external ticket ID:
   ```typescript
   Context.elevatePrivilegedOverride({
       reason: "Statutory court order #2026-992: Correct VAT registration ID",
       ticketRef: "INC-99412",
       authorizedBy: "ChiefComplianceOfficer"
   }, async () => {
       const entry = GeneralLedgerEntry.get(10482);
       entry.vatRegistrationNo = "DE123456789";
       entry.savePrivileged();
   });
   ```
2. **PostgreSQL Guard:** The DB trigger verifies the session token (`SET LOCAL bos.privileged_override = 'valid_token'`).
3. **Immutable Audit Shadow Log:** Every privileged update automatically records an unmodifiable entry in `_system_privileged_audit_log` capturing:
   - User UUID, IP address, and cryptographic certificate.
   - Exact timestamp.
   - Old values vs new values (JSON diff).
   - The mandatory legal justification text and ticket ID.

---

## 3. Consolidated `tableType` Architecture

Instead of separate disparate boolean flags, all core table operational behaviors are unified under the single, explicit **`tableType`** property:

```typescript
enum TableType {
    Normal,     // Standard mutable master data / setup (Customer, Vendor, Item, Terms)
    Document,   // Transactional document header/line with Draft-Active lifecycle (SalesOrder, PO)
    Ledger,     // Immutable write-once financial/inventory ledger (GLEntry, ItemLedgerEntry)
    Temporal,   // System-versioned historical tracking with asOf(timestamp) time-travel
    Temporary,  // In-memory / session-scoped scratch buffer (posting engines & complex math)
    Virtual     // Dynamic in-memory projection or external API / direct SQL view
}
```

### Behavioral Matrix by `tableType`:

| Feature | `Normal` | `Document` | `Ledger` | `Temporal` | `Temporary` | `Virtual` |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: |
| **Persisted to DB** | Yes | Yes | Yes | Yes | **No (Memory)** | **No** |
| **Draft State Support** | Optional | **Native** | No | No | No | No |
| **Immutable (Write-Once)**| No | No | **Yes (Trigger)**| No | No | No |
| **History Time-Travel** | No | No | By Design | **Yes (asOf)**| No | No |
| **Auditing (`_created_by`)| Yes | Yes | Yes | Yes | No | No |
| **PostgreSQL Target** | Base Table | Base Table | Base Table + Guard | System-Versioned | Memory Buffer | View/External |

---

## 4. Comprehensive Table & Field Property Matrix

### 4.1 Table-Level Properties

| Property | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `caption` | `String` / `i18n` | Table Name | User-facing display title |
| `description` | `String` | `""` | Technical documentation |
| `tableType` | `TableType` | `Normal` | Operational model (`Normal`, `Document`, `Ledger`, `Temporal`, `Temporary`, `Virtual`) |
| `dataPerCompany` | `Boolean` | `true` | Partition data per company or share across tenant |
| `lookupPage` | `PageReference` | Auto-List | Default list page for FK lookups |
| `drillDownPage` | `PageReference` | Auto-List | Default page opened when clicking records |
| `dataClassification` | `Enum` | `CustomerContent` | GDPR: `CustomerContent`, `EUII`, `SystemMetadata` |
| `obsoleteState` | `Enum` | `None` | `None`, `Pending`, `Removed` |
| `obsoleteReason` | `String` | `""` | Deprecation explanation for non-breaking upgrades |

### 4.2 Field-Level Properties

| Property | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `caption` | `String` / `i18n` | Field Name | Localized UI label |
| `notNull` | `Boolean` | `false` | Database NOT NULL constraint |
| `unique` | `Boolean` | `false` | Unique index constraint |
| `default` | `Expression` | `null` | Default value on creation |
| `editable` | `Boolean` / `Expr`| `true` | UI editability rule |
| `visible` | `Boolean` / `Expr` | `true` | UI visibility rule |
| `searchable` | `Boolean` | `false` | Included in global and quick search indexes |
| `mask` | `Enum` | `None` | Masking for sensitive data (`Password`, `SSN`) |
| `relation` | `Table.field` | `null` | Enforced foreign key to a unique field of another table (`relation = Customer.no;`). See [`27_`](./27_transactional_integrity_and_schema_migration.md) §4 |
| `onDelete` | `Enum` | `restrict` | Only with `relation`: `restrict`, `cascade`, `setNull` (not allowed on `notNull` fields), `noAction` |
| `renamedFrom` | `String` | `null` | Previous field name; the migration engine renames the column and keeps its data. Also valid as a table property |
| `computed` | `CalcFormula` | `null` | FlowField formula (`sum`, `avg`, `count`, `min`, `max`, `lookup`, `exist`) |
| `drillDownPage` | `PageReference` | `null` | Specific drill-down page for this FlowField |
| `dataClassification` | `Enum` | `CustomerContent` | Privacy classification |
| `validate(val)` | `Function` | `null` | Authored on `tablelogic` as `validate field(val)`. The parser still accepts an inline field block in existing packages. |
| `onChange(old, new)`| `Function` | `null` | Callback executed when field value changes |

---

## 5. Complete Catalog of Table Events & Triggers

These events are authored on `tablelogic` (`*.bl.bos`). The table file lists the fields and keys they protect.

```
                               ┌───────────────────────────────┐
                               │     Table Events & Triggers   │
                               └───────────────┬───────────────┘
                                               │
       ┌───────────────────────┬───────────────┴───────┬───────────────────────┐
       ▼                       ▼                       ▼                       ▼
1. Record Lifecycle     2. Field Lifecycle      3. Draft Lifecycle      4. Transaction / Post
───────────────────     ───────────────────     ───────────────────     ─────────────────────
• onBeforeInsert        • validate(val)         • onDraftSave           • onBeforePost
• onInsert              • onChange(old, new)    • onDraftActivate       • onAfterPost
• onAfterInsert         • onLookup()            • onDraftDiscard        • onBeforeCommit
• onBeforeModify                                                        • onAfterCommit
• onModify
• onAfterModify
• onBeforeDelete
• onDelete
• onAfterDelete
• onBeforeRename
• onRename
• onAfterRename
```

### 5.1 Record-Level Lifecycle Triggers
Every trigger executes within the transactional context and has access to:
- `this` (`Rec`): The active record state.
- `this.xRec`: The unmodified database snapshot.

1. **`onBeforeInsert()` / `onInsert()` / `onAfterInsert()`**
   - Executed during record insertion.
   - `onInsert()` is typically used to assign number series and default values.
2. **`onBeforeModify()` / `onModify()` / `onAfterModify()`**
   - Executed when an existing record is saved.
   - Ideal for delta checks: `if (this.creditLimit != this.xRec.creditLimit) { ... }`.
3. **`onBeforeDelete()` / `onDelete()` / `onAfterDelete()`**
   - Executed before/after row removal.
   - Used for referential integrity checks: `if (this.hasOpenOrders) error("Cannot delete");`.
4. **`onBeforeRename()` / `onRename()` / `onAfterRename()`**
   - Executed when a primary key field is updated.
   - Cascades the new key across all foreign keys atomically.

### 5.2 Field-Level Triggers
1. **`validate(value)`**
   - Triggered when assigning values via `record.field.validate(newValue)`.
   - Used to validate constraints and trigger dependent calculations (e.g. changing Item No recalculates Unit Price and Line Amount).
2. **`onChange(oldValue, newValue)`**
   - Triggered whenever the field value changes, whether via direct assignment or validation.
3. **`onLookup()`**
   - Custom lookup interceptor allowing programmatic filtering of the popup lookup list.

### 5.3 Draft Lifecycle Triggers (for `Document` Tables)
1. **`onDraftSave()`**
   - Hook executed when an auto-save occurs while `status == "Draft"`.
2. **`onDraftActivate()`**
   - Hook executed when moving from `"Draft"` to active state. Executes full business invariants.
3. **`onDraftDiscard()`**
   - Hook executed when a draft is discarded.

### 5.4 Transaction & Posting Triggers
1. **`onBeforePost()` / `onAfterPost()`**
   - Two-phase posting routine hooks for document posting to immutable ledgers.
2. **`onBeforeCommit()` / `onAfterCommit()`**
   - Hooks fired immediately before and after the database transaction commits.

### 5.5 Non-Invasive Event Subscribers (`[EventSubscriber]`)
Third-party extensions can subscribe to any trigger without altering base code:

```typescript
codeunit ComplianceCheckExtension {
    [EventSubscriber(Customer, "onBeforeDelete", priority = 10)]
    public static verifyNoPendingAudits(sender: Customer, args: CancelEventArgs) {
        if (sender.hasOpenTaxAudits()) {
            args.cancel("Cannot delete customer while statutory tax audits are active.");
        }
    }
}
```

---

## 6. Complete Syntax Example: `SalesHeader.bos`

```typescript
table SalesHeader {
    caption = "Sales Order";
    description = "Header entity for sales orders and invoices";
    tableType = Document;
    dataPerCompany = true;
    lookupPage = Page.SalesOrderList;
    drillDownPage = Page.SalesOrderCard;
    dataClassification = CustomerContent;

    // Primary Key
    id: UUID = auto();

    // Document Numbering
    no: String(30) {
        caption = "Order No.";
        notNull = true;
        unique = true;
    };

    // In-Table Draft Status
    status: Enum("Draft", "Open", "Released", "Invoiced", "Cancelled") {
        caption = "Status";
        default = "Draft";
    };

    customerId: UUID {
        caption = "Customer";
        notNull = true;
        relation = Customer.id { onDelete = Restrict; };
        validate(newCustId) {
            const cust = Customer.get(newCustId);
            cust.testField("blocked", false);
            this.paymentTermsCode = cust.paymentTermsCode;
            this.currencyCode = cust.currencyCode;
        }
    };

    orderDate: Date {
        caption = "Order Date";
        default = autoDate();
    };

    paymentTermsCode: String(10) {
        caption = "Payment Terms";
        relation = PaymentTerms.code;
    };

    currencyCode: String(10) {
        caption = "Currency";
        relation = Currency.code;
    };

    // FlowFields
    totalAmount: Decimal(18, 2) {
        caption = "Total Amount";
        computed = sum(SalesLine.lineAmount where { documentId == this.id });
        drillDownPage = Page.SalesOrderLines;
        editable = false;
    };

    totalQuantity: Decimal(18, 4) {
        caption = "Total Quantity";
        computed = sum(SalesLine.quantity where { documentId == this.id });
        editable = false;
    };

    // Keys
    primaryKey(id);
    key(no);
    key(customerId, orderDate);

    // Triggers
    onInsert() {
        if (!this.no) {
            this.no = NumberSeriesManagement.getNext("SALES_ORDER");
        }
    }

    onBeforeModify() {
        if (this.xRec.status == "Invoiced") {
            error("Cannot modify an already invoiced order.");
        }
    }

    onDraftActivate() {
        if (this.customerId == null) {
            error("Customer is required before activating order.");
        }
        this.calcFields("totalAmount");
        if (this.totalAmount <= 0) {
            error("Sales order must have at least one line with amount > 0.");
        }
    }
}
```

---

## 7. SQL Table Naming Conventions & Architectural Boundaries

### 7.1 Simple, Singular Table Naming Standard
BOS adheres to a strict **clean, singular naming convention** across all packages:
- **No Module Prefixes:** Do NOT prefix tables with module acronyms (use `User`, NOT `SecUser`; `Role`, NOT `SecRole`; `Customer`, NOT `CustCustomer`; `Account`, NOT `GLAccount`).
- **Namespacing via Packages:** Namespacing and isolation are handled at the package level (`bos.auth`, `bos.sales`, `bos.finance`) via `bos.json`, not by polluting entity names.
- **Singular PascalCase in BOS-Script:** Entity names represent a single record (`User`, `Role`, `UserRole`, `Session`, `OrderHeader`, `OrderLine`, `LedgerEntry`).
- **Direct Snake_Case Mapping in PostgreSQL:** The compiler automatically translates PascalCase entity names to singular `snake_case` in SQL:
  - `User` $\longrightarrow$ `"user"`
  - `UserRole` $\longrightarrow$ `"user_role"`
  - `Session` $\longrightarrow$ `"session"`
  - `UserSetup` $\longrightarrow$ `"user_setup"`
  - `SalesHeader` $\longrightarrow$ `"sales_header"`

### 7.2 Architectural Boundary: Tables vs Codeunits vs Events

| Capability | Meta-Object | Rationale & Architectural Rule |
| :--- | :--- | :--- |
| **Schema & Constraints** | `table` | Defines columns, datatypes, default values, primary/secondary keys, foreign key relations with `onDelete` cascades. |
| **Virtual Calculations** | `table` | Declarative FlowFields (`sum`, `count`, `avg`, `min`, `max`, `exist`) evaluated lazily or via bulk subqueries with zero stored column overhead. |
| **Field Validation** | `tablelogic` | `validate field(val)` and `validate field(val, old)` in `*.bl.bos`. Runs when that field is assigned. A single-parameter `validate(val)` inside a field remains legal for existing packages. |
| **Lifecycle Triggers** | `tablelogic` | Record scripts in `*.bl.bos`: `onInsert`, `onModify`, `onDelete`, `before*` / `after*`, and draft hooks (`onDraftSave`, `onDraftActivate`, `onDraftDiscard`). The table parser still accepts `on*` triggers so current packages keep compiling. |
| **Record methods** | `tablelogic` | Functions that read or shape one record of this table, such as a risk score or a display name. |
| **Custom Business Functions** | `codeunit` | Posting, multi-table workflows, pricing across documents, and API endpoints. These stay out of both the table and its logic companion. Codeunits provide isolated, testable services (`@transactional`, `@exposed`, `@job`). |
| **Domain Business Events** | `codeunit` | Table triggers represent low-level physical DB mutations (`INSERT`, `UPDATE`), not business milestones. High-level domain events (`@businessEvent` like `onOrderReleased`, `onUserLockedOut`) are declared and emitted from Codeunits, allowing clean, decoupled pub/sub extensibility via `[EventSubscriber]`. |

