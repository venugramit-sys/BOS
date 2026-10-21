# BOS-Script Enterprise Language Reference Manual
## Official Meta-Object Grammar, Package Anatomy & Runtime Engine

> **Document ID:** `docs/23_bos_script_enterprise_reference_manual.md`  
> **Compiler Target:** `bosc-syntax` (Parser & Lexer) / `bosc-runtime` (QuickJS & Rust Native) / `bosc-codegen-db` (PostgreSQL)  
> **Grammar Standard:** Native BOS-Script (`.bos`) DSL with Embedded TypeScript Procedural Blocks  
> **Verified Against:** `crates/bosc-syntax/src/parser.rs`, `crates/bosc-syntax/src/ast.rs`, `packages/bos.auth/`, `packages/bos.typescript_test/`

---

# 1. Architectural Foundation & Why BOS-Script Differs from Generic TypeScript

In BOS (Business Operating System), **BOS-Script is not a generic TypeScript SDK or runtime class library**; it is a **domain-specialized compiled programming language (`.bos`)**.

The compiler (`bosc`) parses top-level enterprise keywords directly in Rust (`bosc-syntax`), validates relations and SIFT indexes (`bosc-analyzer`), generates PostgreSQL DDL migrations (`bosc-codegen-db`), and compiles procedural trigger/method blocks into sandboxed bytecode executed by QuickJS and the Rust native kernel (`bosc-runtime`).

### The Two Syntax Layers of BOS-Script
1. **Outer Declarative Layer (The Meta-Object DSL):** Top-level keywords (`table`, `tablelogic`, `tableextension`, `codeunit`, `statemachine`, `interface`, `enum`, `permissionset`, `view`, `query`) defining schemas, properties, relations, states, and permissions.
2. **Inner Procedural Layer (Embedded TypeScript):** Inside triggers (`onInsert`, `onModify`, `onDelete`), validation blocks (`validate(val)`), and codeunit methods, developers write standard, strongly typed TypeScript with access to runtime globals (`rec`, `xRec`, `ctx.crypto`, `ctx.audit`, `ctx.db`, `ctx.user`).

### JavaScript the event runner executes

The event body is modern JavaScript after TypeScript syntax is removed. These forms are kept intact, including when a type annotation sits on the same script:

- Template strings: `` ctx.ui.message(`date ${dt}`, "info") ``. Text outside `${ }` is a string. The hole is code, so completions and type removal apply there.
- Regular expressions: `/}/` is one literal. A brace inside it does not end the event.
- Optional chaining (`user?.name`), nullish coalescing (`name ?? "Guest"`), destructuring, arrow functions, `for...of`, and `async` functions.

Before QuickJS runs the body, the compiler removes type annotations, `import type` and `export type` lines, and decorator lines. A `using` binding is stored with `const`. The sandbox does not run `eval` or `new Function`, does not load `import` / `export` modules, and does not call disposal or decorator logic. An `await` inside an `async` function runs when that function is called; the event itself stays synchronous.

---

# 2. Package Anatomy & Manifest Specification (`bos.json`)

Every deployable BOS module is packaged as a `.bospkg` archive containing a root `bos.json` manifest.

```
packages/bos.auth/
├── bos.json                    # Package metadata & object registry
├── package.bos                 # Consolidated package source (or entry point)
├── tables/
│   ├── User.bos               # table User { ... }
│   ├── Role.bos               # table Role { ... }
│   ├── UserRole.bos           # table UserRole { ... }
│   └── SessionToken.bos       # table SessionToken { ... }
└── codeunits/
    └── AuthManager.bos        # codeunit AuthManager { ... }
```

### Manifest Schema (`bos.json`)
```json
{
  "package_id": "bos.auth",
  "name": "BOS Authentication & Identity Package",
  "version": "1.0.0",
  "publisher": "BOS Core Kernel",
  "description": "Identity management, RBAC roles, user role mappings, session tokens, and security codeunits.",
  "dependencies": [],
  "optional_features": [],
  "entry": "./package.bos",
  "objects": {
    "tables": [
      "tables/User.bos",
      "tables/Role.bos",
      "tables/UserRole.bos",
      "tables/SessionToken.bos"
    ],
    "codeunits": [
      "codeunits/AuthManager.bos"
    ]
  }
}
```

---

# 3. Complete Catalog: All First-Class BOS Meta-Objects

---

## 3.1 Meta-Object: `table` (Relational Entity)

Defines a relational database entity, column types, default values, primary/secondary keys, and SIFT FlowFields. Executable script for the entity lives in `tablelogic`.

### Table Properties
- `caption = "String"`: User-facing descriptive name.
- `tableType = Normal | Document | Ledger | Temporal | Temporary | Virtual`:
  - `Normal`: Standard mutable master data (Customer, Vendor, Item).
  - `Document`: Transactional document header/line with Draft-Active lifecycle.
  - `Ledger`: Strictly immutable write-once financial/inventory ledger entries.
  - `Temporal`: System-versioned historical tracking with `asOf(timestamp)` time travel.
  - `Temporary`: In-memory scratchpad buffer.
  - `Virtual`: Dynamic in-memory projection.
- `dataPerCompany = true | false`: Multi-company tenant data isolation flag.

### Supported Field Types
- `UUID`: Universally unique identifier.
- `String(length)`: Varchar with optional maximum length.
- `Integer`: 32-bit signed integer.
- `BigInt`: 64-bit integer.
- `Decimal(precision, scale)`: Fixed-point financial numeric (e.g. `Decimal(18, 2)`).
- `Boolean`: `true` or `false`.
- `Date`: Calendar date (`YYYY-MM-DD`).
- `DateTime`: UTC timestamp with millisecond precision.
- `Enum("EnumName")`: First-class enum reference.
- `Json`: Structured JSON document.
- `Blob`: Binary large object.

### Field Attributes
- `notNull = true;`: Non-nullable column.
- `unique = true;`: Unique constraint.
- `default = <value>;`: Default value.
- `searchable = true;`: Full-text searchable index.
- `validate` is not a field property. Write `validate field(val)` or `validate field(val, old)` in the `tablelogic` companion.
- `computed = <func>(<Table>.<field> where { <filter> });`: SIFT FlowField.

### Production Example: `Customer.bos`
```typescript
table Customer {
    caption = "Customer Master";
    tableType = Normal;
    dataPerCompany = true;

    id: UUID;
    code: String(30) {
        notNull = true;
        unique = true;
    };
    name: String(100) {
        notNull = true;
        searchable = true;
    };
    creditLimit: Decimal(18, 2) {
        default = 0.00;
    };
    balance: Decimal(18, 2) {
        // SIFT FlowField: Compiled directly to subquery or SIFT bucket
        computed = sum(CustomerLedgerEntry.amount where { customerNo == code && open == true });
    };
    isActive: Boolean {
        default = true;
    };

    primaryKey(code);
    key(name);
    key(isActive, creditLimit);
}
```

---

## 3.2 Meta-Object: `tableextension` (Non-Invasive Schema Extension)

Extends an existing table defined in another package without modifying the original source code.

```typescript
tableextension CustomerLoyalty extends Customer {
    loyaltyPoints: Integer {
        default = 0;
    };
    vipTier: String(20) {
        default = "Standard";
    };

    key(vipTier);

    onModify(rec, ctx) {
        if (rec.loyaltyPoints > 10000 && rec.vipTier === "Standard") {
            rec.vipTier = "Gold";
            ctx.audit.log(`Customer ${rec.code} promoted to Gold Tier`);
        }
    }
}
```

---

## 3.3 Meta-Object: `tablelogic` & `tablelogicextension` (Dedicated Business Rules Companion)

Keeps the table file strictly structural. Field validations, lifecycle scripts, and record methods belong in `Customer.bl.bos`.

```typescript
tablelogic CustomerLogic for Customer {
    validate creditLimit(val, oldVal) {
        if (val < 0) {
            throw new Error("Credit limit cannot be negative.");
        }
        if (val > 1000000 && !rec.hasExecutiveSignoff) {
            throw new Error("Credit limits exceeding $1,000,000 require executive signoff.");
        }
    }

    onInsert(rec, ctx) {
        if (!rec.code || rec.code.trim().length === 0) {
            throw new Error("Customer code is required.");
        }
        rec.code = rec.code.trim().toUpperCase();
        rec.createdAt = new Date().toISOString();
        ctx.audit.log(`Customer record ${rec.code} created.`);
    }

    onModify(rec, ctx) {
        if (!rec.code) {
            throw new Error("Customer code cannot be blank.");
        }
        ctx.audit.log(`Customer record ${rec.code} updated.`);
    }

    onDelete(rec, ctx) {
        if (rec.balance > 0) {
            throw new Error("Cannot delete customer with an open balance.");
        }
        ctx.audit.log(`Customer record ${rec.code} deleted.`);
    }

    public calculateRiskScore(rec: Customer): String {
        if (rec.creditLimit > 500000) {
            return "HIGH_EXPOSURE";
        }
        return "STANDARD";
    }
}
```

```typescript
tablelogicextension CustomerLoyaltyLogic for Customer {
    validate loyaltyPoints(val) {
        if (val < 0) {
            throw new Error("Loyalty points cannot be negative.");
        }
    }
}
```

---

## 3.4 Meta-Object: `codeunit` (Encapsulated Procedural Service)

Encapsulates transactional business logic, posting routines, calculation services, and RPC endpoints.

### Codeunit Attributes & Modifiers
- `caption = "String"`: Descriptive name.
- `tableNo = Table.<Entity>`: Record-bound codeunit automatically bound to a table type.
- Annotations:
  - `@exposed`: Exposes method as typed JSON-RPC / REST API endpoint.
  - `@transactional`: Enforces atomic transaction rollback on unhandled error.
  - `@job`: Asynchronous background worker dispatch.
  - `@test`: Test harness execution with auto-rollback.

### Production Example: `AuthManager.bos` / `SalesPostService.bos`
```typescript
codeunit AuthManager {
    caption = "Authentication & Session Security Service";

    @exposed
    public static ValidateUserRegistration(username: String, email: String) {
        if (!username || username.trim().length === 0) {
            throw new Error("Username must not be empty");
        }
        if (!email || email.trim().length === 0) {
            throw new Error("Email must not be empty");
        }
    }

    public AuthenticateUser(username: String, passwordHash: String) {
        if (!username || username.trim().length === 0) {
            throw new Error("Username cannot be empty");
        }
        if (!passwordHash || passwordHash.trim().length === 0) {
            throw new Error("Password hash cannot be empty");
        }
    }

    @transactional
    public LockUser(username: String, reason: String) {
        if (username === "admin") {
            throw new Error("Primary administrator account cannot be locked");
        }
        // In-sandbox SQL execution via host FFI
        ctx.db.execute("UPDATE \"user\" SET is_active = false WHERE username = $1", [username]);
        ctx.audit.log(`User ${username} locked. Reason: ${reason}`);
    }
}
```

---

## 3.5 Meta-Object: `interface` & `enum` / `enumextension` (Polymorphic Strategy Pattern)

Enables non-invasive polymorphic business rules (e.g., custom tax calculation engines, multiple payment processors) without `switch/case` bloat.

### Interface Definition
```typescript
interface IPaymentProcessor {
    ProcessPayment(orderNo: String, amount: Decimal): Boolean;
    RefundPayment(transactionId: String, amount: Decimal): Boolean;
}
```

### Enum Implementing Interface
```typescript
enum PaymentMethod implements IPaymentProcessor {
    extensible = true;

    CreditCard {
        bound_codeunit = StripePaymentService;
    }
    BankTransfer {
        bound_codeunit = ACHPaymentService;
    }
}
```

### Enum Extension (Injecting Third-Party Provider Without Modifying Core)
```typescript
enumextension CryptoPaymentExtension extends PaymentMethod {
    CryptoCurrency {
        bound_codeunit = BitcoinPaymentService;
    }
}
```

---

## 3.6 Meta-Object: `statemachine` & `statemachineextension` (FSM Workflows)

Defines document status transitions, approval matrices, role authorization, modal parameters, and transition triggers.

```typescript
statemachine OrderWorkflow for SalesHeader.status {
    caption = "Sales Order Lifecycle";
    initial = "Draft";

    states {
        state "Draft" {
            caption = "Draft";
        }
        state "PendingApproval" {
            caption = "Pending Approval";
        }
        state "Approved" {
            caption = "Approved";
        }
        state "Released" {
            caption = "Released";
        }
        state "Rejected" {
            caption = "Rejected";
        }
    }

    transition Submit {
        from = "Draft";
        to = "PendingApproval";
        guard(rec) {
            return rec.totalAmount > 0;
        }
        approvalMatrix {
            tier ExecutiveReview {
                condition = totalAmount > 100000;
                roles = ["CFO", "VPFinance"];
            }
            tier ManagerReview {
                condition = totalAmount <= 100000;
                roles = ["SalesManager"];
            }
        }
    }

    transition Approve {
        from = "PendingApproval";
        to = "Approved";
        roles = ["SalesManager", "CFO"];
        onTransition(rec, ctx) {
            ctx.audit.log(`Order ${rec.orderNo} approved by ${ctx.user.username}`);
        }
    }

    transition Reject {
        from = "PendingApproval";
        to = "Rejected";
        roles = ["SalesManager", "CFO"];
        params {
            rejectionReason: String(250) {
                notNull = true;
            };
        }
        onTransition(rec, ctx) {
            rec.notes = `Rejected: ${ctx.params.rejectionReason}`;
            ctx.audit.log(`Order ${rec.orderNo} rejected. Reason: ${ctx.params.rejectionReason}`);
        }
    }
}
```

---

## 3.7 Meta-Object: `permissionset` (RBAC & Row-Level Security)

Defines role permissions: Table CRUD actions (`R`ead, `I`nsert, `M`odify, `D`elete) with optional row filters, and Codeunit execution grants.

```typescript
permissionset SalesRepresentative {
    caption = "Sales Representative Access";

    permissions = {
        Customer = RIM where { country == "US" };
        SalesHeader = RIMD;
        SalesLine = RIMD;
        CustomerLedgerEntry = R;
    };

    execute = [
        AuthManager,
        PricingCalculator
    ];
}
```

---

## 3.8 Meta-Object: `view` (Relational Projections & Joins)

Defines declarative read-only projections combining multiple tables with SQL joins.

```typescript
view CustomerBalanceView {
    caption = "Customer Active Balance Summary";

    from Customer;

    leftjoin CustomerLedgerEntry as cle on (cle.customerNo == code);

    where {
        cle.open == true
    };

    select {
        code,
        name,
        creditLimit,
        cle.amount as openAmount,
        cle.postingDate as lastPostingDate
    };
}
```

---

## 3.9 Meta-Object: `query` (Aggregated Analytical SIFT Trees)

Defines multi-level analytical queries with hierarchical dataitems, grouping, and aggregations.

```typescript
query ItemSalesAnalytics {
    caption = "Item Sales Aggregation by Location";

    elements {
        dataitem ItemLedgerEntry {
            column itemNo;
            column locationCode;
            column quantity {
                aggregation = sum;
            };
            column costAmountActual {
                aggregation = sum;
            };
        }
    }

    groupBy(itemNo, locationCode);
    having {
        sum(quantity) > 0
    };
    orderBy(itemNo asc);
}
```

---

# 4. Embedded TypeScript Execution Model

Inside triggers (`onInsert`, `onModify`, `onDelete`), validation blocks (`validate(val)`), and codeunit methods, BOS-Script provides full TypeScript capabilities.

```typescript
onInsert(rec, ctx) {
    // 1. Strongly typed interfaces inside triggers
    interface RegistrationPayload {
        code: string;
        email: string;
        initialCredit?: number;
    }

    const payload: RegistrationPayload = rec as any;

    // 2. Cryptographic hashing via native kernel host FFI
    const salt = ctx.crypto.randomBytes(8).toHex();
    rec.securityHash = `$sha256$${salt}$${ctx.crypto.sha256(salt + payload.email)}`;

    // 3. User session context
    rec.createdBy = ctx.user.username;
    rec.createdDate = ctx.user.workDate || new Date().toISOString();

    // 4. Immutable audit logging
    ctx.audit.log(`Created entity ${rec.code}`);
}
```

### The Injected Runtime Context (`ctx`)
- `ctx.user.username`: Authenticated caller username.
- `ctx.user.roles`: Array of assigned security roles.
- `ctx.user.workDate`: User's effective ERP work date.
- `ctx.crypto.sha256(data)`: Fast native SHA-256 calculation.
- `ctx.crypto.randomBytes(len)`: Hardware-entropy CSPRNG random bytes.
- `ctx.audit.log(message)`: Direct append to the system audit trail.
- `ctx.db.query(sql, params)` / `ctx.db.execute(sql, params)`: Sandboxed database access.
