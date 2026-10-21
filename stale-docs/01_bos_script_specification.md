# BOS Script Language Specification & Meta-Object Framework

> **Status:** Draft / Specification  
> **Document:** `docs/01_bos_script_specification.md`  

---

## 1. Design Rationale & Core Concepts

**BOS Script** is a domain-specialized dialect and extension of TypeScript. It combines:
1. **Declarative Object Declarations:** Top-level keywords (`table`, `page`, `view`, `query`, `menu`, `permissionset`, `codeunit`, `report`) defining first-class enterprise objects.
2. **Procedural Logic Engine:** Standard, strongly typed TypeScript statements and expressions within triggers, methods, and validation rules.
3. **Non-Invasive Extensibility:** Extension keywords (`tableextension`, `pageextension`, `menuextension`) allowing modifications to base objects without editing base source code.

---

## 2. Meta-Object Categories

| Meta-Object | Purpose | Output Targets |
| :--- | :--- | :--- |
| **`table`** | Relational entity definition, keys, fields, relations, triggers, validation, audit logs | SQL DDL, Migration AST, WASM ORM Stubs, TS Types |
| **`tableextension`** | Adds fields, keys, triggers to an existing table from another package | SQL `ALTER TABLE` / delta migration, WASM proxy |
| **`page`** | One screen: a single `source`, one layout, and field/command overrides. See `docs/28_page_meta_object_design.md` | Compact page document rendered by one Svelte host |
| **`pageextension`** | Injects controls, actions, or views into an existing page | Svelte slot/fragment injects, patch metadata |
| **`view`** | Materialized or virtual aggregated relational views | SQL `CREATE VIEW`, Svelte Query Read Model |
| **`query`** | Strongly typed multi-table analytical joins and aggregations | SQL queries, WASM query executor |
| **`menu`** | Navigation trees, search keywords, command palette items | Svelte navigation layout, Permission gate |
| **`permissionset`** | Role-based & attribute-based access control (CRUD + Execute) | PostgreSQL RLS policies, WASM guards |
| **`codeunit`** | Pure or transactional business service, API endpoint, or workflow | Sandboxed WASM service module, OpenAPI route |

---

## 3. Syntax & Structure

### 3.1 Table Definition Example (`Customer.bos`)

```typescript
table Customer {
    id: UUID = auto();
    code: String(20) {
        unique = true;
        notNull = true;
        caption = "Customer Code";
    };
    name: String(120) {
        notNull = true;
        searchable = true;
    };
    creditLimit: Decimal(18, 2) {
        default = 0.00;
        validate(value) {
            if (value < 0) {
                error("Credit limit cannot be negative");
            }
        }
    };
    balance: Decimal(18, 2) {
        // FlowField / Computed field from Ledger Entries
        computed = sum(CustomerLedgerEntry.amount where { customerId == id });
        editable = false;
    };
    status: Enum("Active", "Blocked", "Prospect") {
        default = "Prospect";
    };
    createdAt: DateTime = autoNow();
    updatedAt: DateTime = autoUpdate();

    // Primary & Secondary Keys
    primaryKey(id);
    key(code);
    key(status, name);

    // Relational Foreign Keys
    relation paymentTerms: PaymentTerms.code;

    // Triggers
    onInsert() {
        if (!this.code) {
            this.code = NumberSeriesManagement.getNext("CUSTOMER");
        }
    }

    onDelete() {
        if (this.balance > 0) {
            error("Cannot delete customer with open balance");
        }
    }
}
```

### 3.2 Table Extension Example (`CustomerLoyalty.bos`)

```typescript
// Extending base customer table without altering base package
tableextension CustomerLoyalty extends Customer {
    loyaltyPoints: Integer {
        default = 0;
        caption = "Loyalty Points";
    };
    tier: Enum("Standard", "Silver", "Gold", "Platinum") {
        default = "Standard";
    };

    onAfterInsert() {
        AuditLogger.log("Customer created with tier: " + this.tier);
    }
}
```

### 3.3 Page Definition Example (`CustomerCard.bos`)

```bos
page CustomerCard {
    source: Customer;
    layout: record;
    caption: "Customer Details";

    main {
        section(General) {
            field(code);
            field(name);
            field(status);
        }
        section(Financials) {
            field(creditLimit);
            field(balance) { editable: false; }
        }
        part(LedgerLines) {
            source: CustomerLedger;
            link: customerId = id;
        }
    }

    commands {
        command(PostInvoice) {
            caption: "Create Invoice";
            primary: true;
            visible: status == "Active";
            run: SalesInvoiceService.createForCustomer;
        }
    }
}
```

### 3.4 Page Extension Example (`CustomerCardExtension.bos`)

```bos
pageextension CustomerCardLoyalty extends CustomerCard {
    main {
        addAfter(Financials) {
            section(Loyalty) {
                caption: "Loyalty";
                field(loyaltyPoints);
                field(tier);
            }
        }
    }
    commands {
        addLast(commands) {
            command(UpgradeTier) {
                caption: "Evaluate Tier";
                run: LoyaltyService.evaluate;
            }
        }
    }
}
```

### 3.5 PermissionSet Definition Example (`SalesRepresentative.bos`)

```typescript
permissionset SalesRepresentative {
    caption = "Sales Representative Access";

    // Entity Permissions: [Read, Insert, Modify, Delete]
    permissions = {
        Customer: [Read, Insert, Modify],
        SalesOrder: [Read, Insert, Modify],
        CustomerLedgerEntry: [Read],
        PaymentTerms: [Read]
    };

    // Row-level Security / Filter rules
    rowFilters = {
        SalesOrder: {
            assignedRepId: session.userId
        }
    };

    // Stored codeunit execution
    execute = [
        SalesInvoiceService,
        NumberSeriesManagement
    ];
}
```

---

## 4. Type System & Field Primitives

BOS Script enforces strict ERP-grade primitive types that map deterministically to database columns and TypeScript interfaces:

1. `UUID` (PostgreSQL `uuid`)
2. `String(N)` (PostgreSQL `varchar(N)` or text)
3. `Integer` (PostgreSQL `int4` / `int8`)
4. `Decimal(P, S)` (PostgreSQL `numeric(P, S)`) - essential for currency and ledger precision
5. `Boolean` (PostgreSQL `boolean`)
6. `Date` / `Time` / `DateTime` (PostgreSQL `date`, `time`, `timestamptz`)
7. `Enum(A, B, C)` (PostgreSQL typed `enum` or validated domain)
8. `Blob` / `Media` (Object store reference or byte array)
9. `Json` (PostgreSQL `jsonb` with schema validation)
10. `Computed` (FlowField virtual calculated values)

---

## 5. Event and Hook Architecture

- **Table Triggers:** `onInsert`, `onModify`, `onDelete`, `onRename` (pre/post execution).
- **Field Triggers:** `validate(value)`, `onChange(oldVal, newVal)`.
- **Global Event Subscribers:**
  ```typescript
  [EventSubscriber(Customer, "onAfterInsert")]
  function notifySalesDepartment(record: Customer) {
      EmailQueue.enqueue("sales@company.com", `New customer: ${record.name}`);
  }
  ```
