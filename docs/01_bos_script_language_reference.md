# BOS-Script Language Reference

> **Document ID:** `BOS-DOC-01`  
> **Source Verification:** `crates/bosc-syntax/*`, `crates/bosc-analyzer/src/ts_stripper.rs`  
> **Target Audience:** Language Engineers, Business Developers, Solution Architects

---

## 1. Overview & Language Philosophy

**BOS-Script** is a domain-specific programming language designed specifically for enterprise business application development. It combines:
1. **Declarative Metadata Definitions**: High-level syntax for business constructs (`table`, `page`, `codeunit`, `statemachine`, `permissionset`, etc.).
2. **Modern Embedded JavaScript/TypeScript**: Business logic triggers and service procedures written in TypeScript/JavaScript, parsed and stripped of type annotations by a dedicated compiler pass, and executed inside an isolated QuickJS sandbox.

---

## 2. Lexical Structure & Tokens

Source files are UTF-8 encoded text with `.bos` (for metadata declarations) and `.bl.bos` (for companion business logic) extensions.

### 2.1. Comments
BOS-Script supports single-line and multi-line C-style comments:
```typescript
// Single line comment
/* Multi-line
   comment block */
```

### 2.2. Keywords
The lexer (`bosc-syntax/src/lexer.rs`) recognizes the following core keywords:
- **Object Declarations**: `table`, `tablelogic`, `tableextension`, `tablelogicextension`, `statemachine`, `statemachineextension`, `codeunit`, `interface`, `enum`, `enumextension`, `permissionset`, `view`, `query`, `page`, `pageextension`, `menu`, `report`, `custommetadata`.
- **Structural Keywords**: `for`, `extends`, `caption`, `primaryKey`, `key`, `fieldGroup`, `layout`, `actions`, `elements`, `dataitem`, `column`, `groupBy`, `orderBy`, `having`, `where`, `select`, `from`, `join`, `left`, `inner`, `transition`, `initial`, `states`, `guard`, `timeout`, `escalateTo`, `implements`, `extensible`.
- **Triggers & Lifecycle**: `validate`, `onInsert`, `beforeInsert`, `afterInsert`, `onModify`, `beforeModify`, `afterModify`, `onDelete`, `beforeDelete`, `afterDelete`, `onAction`.
- **FlowField Aggregators**: `sum`, `average`, `count`, `min`, `max`, `lookup`, `exist`.
- **Expressions & Logic**: `if`, `else`, `return`, `let`, `const`, `var`, `function`, `async`, `await`, `true`, `false`, `null`, `in`.

### 2.3. Literals & Identifiers
- **Identifiers**: Match `[a-zA-Z_][a-zA-Z0-9_]*`.
- **Strings**: Enclosed in double quotes (`"..."`) or single quotes (`'...'`), supporting standard escape sequences.
- **Numbers**: Integers (`12345`) and floating-point/decimal literals (`12.34`).
- **Booleans**: `true` or `false`.
- **Null**: `null`.

---

## 3. Data Types System

The BOS type system (`bosc-syntax/src/ast.rs::FieldType`) defines strongly typed database columns and variable types:

| BOS Type | SQL Representation (Postgres) | Description & Use Case |
| :--- | :--- | :--- |
| `UUID` | `UUID` | 128-bit universally unique identifier. Supports `default = auto()` for UUIDv4 generation. |
| `String(n)` | `VARCHAR(n)` | Variable-length string bounded to $n$ characters. Example: `code: String(30);` |
| `String` | `TEXT` | Unbounded string / text. |
| `Integer` | `INTEGER` | 32-bit signed integer. Example: `lineNo: Integer;` |
| `BigInt` | `BIGINT` | 64-bit signed integer. Example: `entryNo: BigInt;` |
| `Decimal(p, s)` | `NUMERIC(p, s)` | Exact fixed-point numeric with precision $p$ and scale $s$. Default is `Decimal(18, 2)`. |
| `Boolean` | `BOOLEAN` | Boolean true/false flag. |
| `Date` | `DATE` | Calendar date without time zone (`YYYY-MM-DD`). |
| `DateTime` | `TIMESTAMPTZ` | Timestamp with time zone (`UTC`). |
| `Enum(Name)` | `VARCHAR(50)` | Strongly typed extensible or frozen enumeration. |
| `Json` | `JSONB` | Binary JSON document for unstructured or dynamic attributes. |
| `Blob` | `BYTEA` | Binary large object for raw bytes or attachments. |
| `FlowFilter(T)` | *Virtual (None)* | Dynamic runtime filter parameter used in FlowField calculation matrices. |
| `Custom(Name)` | Custom SQL Type | User-defined custom data type. |

---

## 4. Expressions & Operators

BOS expressions (`ast.rs::Expr`) support full arithmetic, comparison, and logical evaluation:

### 4.1. Binary Operators (`BinaryOp`)
- **Arithmetic**: `+`, `-`, `*`, `/`
- **Comparison**: `==`, `!=` (or `<>`), `<`, `<=`, `>`, `>=`
- **Logical**: `AND`, `OR`
- **Set Membership**: `IN`

### 4.2. Unary Operators (`UnaryOp`)
- **Logical Negation**: `!` / `NOT`
- **Arithmetic Negation**: `-`

### 4.3. Member Access & Calls
- Property access: `Customer.balance` or `rec.email`
- Function invocations: `ctx.crypto.argon2Hash(password)` or `round(amount * taxRate, 2)`

---

## 5. Declarative FlowFields & Computed Formulas

FlowFields are declarative calculated fields that are never stored physically in the table; instead, they are computed on-the-fly or resolved via PostgreSQL pushdown subqueries.

### 5.1. Syntax
```typescript
balance: Decimal(18, 2) {
    computed = sum(CustomerLedgerEntry.amount where { customerId == id });
};

hasOpenOrders: Boolean {
    computed = exist(SalesHeader where { customerId == id && status == "Open" });
};

totalOrderCount: Integer {
    computed = count(SalesOrder where { customerId == id });
};
```

### 5.2. Aggregation Functions
- `sum(Table.field where { ... })`: Calculates the sum of a numeric column.
- `average(Table.field where { ... })`: Computes the arithmetic mean.
- `count(Table where { ... })`: Counts matching records.
- `min(Table.field where { ... })`: Determines the minimum value.
- `max(Table.field where { ... })`: Determines the maximum value.
- `lookup(Table.field where { ... })`: Retrieves a single related value.
- `exist(Table where { ... })`: Returns `true` if at least one matching record exists.

---

## 6. TypeScript Type Stripping (`ts_stripper.rs`)

BOS allows developers to write business logic triggers and codeunits using rich TypeScript types and modern JavaScript idioms. Before bytecode execution in QuickJS, the compiler runs `strip_typescript()` powered by the high-speed Rust `oxc_parser` engine.

### 6.1. Stripped Constructs
- Type annotations: `: string`, `: number`, `: Promise<boolean>`, `: void`.
- Generic type parameters: `<T, K>`.
- Type aliases: `type Status = "Draft" | "Posted";`.
- TypeScript interfaces: `interface OrderContext { ... }`.
- `import type { ... }` and `export type { ... }`.
- Type assertions: `as string`, `<number>val`.
- Decorators on stripped types.

### 6.2. Preserved Modern JavaScript Features
The stripper explicitly preserves all valid modern JavaScript features:
- Template literals with expressions: `` `Order ${rec.code} posted` ``.
- Regular expressions with curly braces: `const re = /}/;`.
- Nullish coalescing (`??`) and optional chaining (`?.`).
- Object and array destructuring: `const { code, amount } = rec;`.
- Arrow functions and `for...of` loops.
- Asynchronous functions: `async` and `await`.

### 6.3. Stripping Example
**Authored BOS-Script:**
```typescript
codeunit SalesPosting {
    postInvoice(orderId: UUID, notifyCustomer: boolean = true): Promise<boolean> {
        const order: SalesHeader = models.SalesHeader.get(orderId);
        if (!order) {
            throw new Error(`Order ${orderId} not found`);
        }
        ctx.audit.log(`Invoice posted for order ${order.code}`);
        return true;
    }
}
```

**Clean Stripped JavaScript (Executed in QuickJS):**
```javascript
codeunit SalesPosting {
    postInvoice(orderId, notifyCustomer = true) {
        const order = models.SalesHeader.get(orderId);
        if (!order) {
            throw new Error(`Order ${orderId} not found`);
        }
        ctx.audit.log(`Invoice posted for order ${order.code}`);
        return true;
    }
}
```

---

## 7. Meta-Object Separation Standard

To guarantee modularity, maintainability, and compile-time optimization, BOS enforces a strict separation standard:
1. **`table` (`*.bos`)**: Holds data structure only: fields, data types, default values, primary keys, secondary indexes, relations, and declarative FlowFields.
2. **`tablelogic` (`*.bl.bos`)**: Holds executable triggers and field validations for a table: `validate <field>`, `onInsert`, `onModify`, `onDelete`, and record instance methods.
3. **`codeunit` (`*.bos`)**: Holds encapsulated cross-record procedures, business transactions, posting routines, and domain events (`@businessEvent`).
