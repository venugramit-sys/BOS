# 01. Meta-Objects & DSL Specification

## 1. Overview of BOS Meta-Objects

BOS defines applications through 18 first-class declarative **meta-objects**. Each meta-object represents a distinct architectural concern, separating raw data schemas from business logic, workflows, UI presentations, and security rules.

---

## 2. The 18 First-Class Meta-Objects

| # | Meta-Object | Primary File Extension | Architectural Purpose |
| :- | :--- | :--- | :--- |
| 1 | `table` | `.bos` | Declares relational tables, field definitions, primary/secondary keys, and FlowFields. |
| 2 | `tablelogic` | `.bl.bos` | Business validation logic, record triggers (`onInsert`, `onModify`, `onDelete`), and companion methods. |
| 3 | `tableextension`| `.bos` | Non-invasive extensions adding custom fields or keys to tables in lower packages. |
| 4 | `page` | `.bos` | Declarative UI views: Lists, Cards, Document Forms, Dashboards, and Worksheets. |
| 5 | `pageextension` | `.bos` | Non-invasive UI extensions inserting actions, controls, or tabs into existing pages. |
| 6 | `codeunit` | `.bos` | Encapsulated business service modules containing domain logic, posting engines, and API handlers. |
| 7 | `statemachine` | `.bos` | Formal document lifecycle transitions, status checks, and approval matrices. |
| 8 | `interface` | `.bos` | Polymorphic contract specifications for Strategy Pattern implementations. |
| 9 | `enum` | `.bos` | Extensible enumerated values and status flags. |
| 10| `enumextension` | `.bos` | Non-invasive enum extensions injecting new choices without editing core sources. |
| 11| `query` | `.bos` | Predefined multi-table join and aggregation views with pushdown optimization. |
| 12| `view` | `.bos` | Filtered, projected subset of a table or query. |
| 13| `menu` | `.bos` | Application navigation hierarchies, command palettes, and role centers. |
| 14| `permissionset`| `.bos` | Declarative security profiles specifying granular Read, Insert, Modify, and Delete rights. |
| 15| `report` | `.bos` | Structured document layouts, aggregation rules, and data output formatting. |
| 16| `job` | `.bos` | Asynchronous scheduled background jobs, cron expressions, and NAS batch tasks. |
| 17| `eventsubscriber`| `.bos` | Decoupled event listeners reacting to transactional `@businessEvent` triggers. |
| 18| `wasmplugin` | `.bos` | Sandboxed WebAssembly extension modules executed by `wasmi 2.0`. |

---

## 3. Detailed Syntax Examples

### 3.1. `table` Definition
```bos
table Customer {
    id: UUID [primaryKey, auto];
    customer_no: String(20) [indexed, unique];
    name: String(100) [indexed];
    credit_limit: Decimal(18, 4) [default: 0.0000];
    balance: FlowField(Sum("CustLedgerEntry".amount WHERE customer_no = field(customer_no)));
    status: Enum("CustomerStatus") [default: "Active"];
    created_at: DateTime [auto];
    updated_at: DateTime [auto];

    keys {
        key PK(id);
        key CustNo(customer_no);
        key NameIdx(name);
    }
}
```

### 3.2. `tablelogic` Companion
```typescript
tablelogic Customer {
    validate credit_limit(value: Decimal, xRec: Customer) {
        if (value < 0n) {
            throw new Error("Credit limit cannot be negative.");
        }
    }

    onInsert() {
        if (!this.customer_no) {
            this.customer_no = ctx.numberSeries.getNext("CUST");
        }
    }

    onModify() {
        // Enforce credit check if limit is reduced
        if (this.credit_limit < this.balance) {
            ctx.notifications.warn("Current balance exceeds newly assigned credit limit.");
        }
    }
}
```

### 3.3. `statemachine` Definition
```bos
statemachine PurchaseOrderWorkflow for PurchaseOrder {
    initial: Draft;

    state Draft {
        on submit -> PendingApproval {
            guard: "this.total_amount > 0";
        }
    }

    state PendingApproval {
        on approve -> Approved {
            roles: ["PurchasingManager", "SystemAdmin"];
        }
        on reject -> Draft {
            roles: ["PurchasingManager", "SystemAdmin"];
        }
    }

    state Approved {
        on post -> Posted {
            roles: ["PurchasingClerk", "PurchasingManager"];
            action: "PurchasePostCodeunit.post(this.id)";
        }
    }

    state Posted {
        // Immutable terminal state
    }
}
```

### 3.4. `page` Definition
```bos
page CustomerList for Customer {
    type: List;
    caption: "Customers";
    sourceTable: Customer;

    layout {
        area(Content) {
            repeater(Group) {
                field(customer_no; "No.") {}
                field(name; "Name") {}
                field(balance; "Outstanding Balance") {}
                field(credit_limit; "Credit Limit") {}
                field(status; "Status") {}
            }
        }
    }

    actions {
        action(ViewLedger) {
            caption: "Ledger Entries";
            runPage: "CustLedgerEntryList";
            runPageLink: "customer_no = field(customer_no)";
        }
    }
}
```

---

## 4. TypeScript OxC Type Stripping

BOS includes an ultra-fast type stripping engine powered by `oxc_parser` and `oxc_allocator`. When business logic triggers or codeunits are parsed:
1. Type annotations (`: Decimal`, `: Customer`, interfaces, type aliases) are analyzed for semantic consistency.
2. Type annotations are stripped at native memory speeds without launching Node.js or `tsc`.
3. The resulting pure JavaScript is compiled directly into QuickJS bytecode envelopes (`.qbc`) or cached in the Static Node runtime.
