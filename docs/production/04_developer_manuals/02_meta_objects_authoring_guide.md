# Volume 4: Developer Manuals
## 02. Meta-Objects Authoring Guide

This guide explains how to author the core declarative meta-objects in `.bos` files with real-world examples and zero mocks.

---

## 1. Naming Conventions & Rules

- **Object Names**: PascalCase (e.g., `Customer`, `SalesHeader`, `SalesPost`).
- **Table Names**: Singular PascalCase (`Customer`, not `Customers` or `tbl_customer`).
- **Field Names**: snake_case (`customer_no`, `credit_limit`, `posting_date`).
- **SQL Table Output**: Automatically lowered to singular snake_case (`customer`, `sales_header`).

---

## 2. Creating Tables (`table`)

Tables define the physical database schema, data types, indexes, and FlowFields.

```bos
table Item {
    id: UUID [primaryKey, auto];
    item_no: String(30) [indexed, unique];
    description: String(100);
    unit_price: Decimal(18, 4) [default: 0.0000];
    inventory_qty: FlowField(Sum("ItemLedgerEntry".quantity WHERE item_no = field(item_no)));
    unit_of_measure: Enum("UnitOfMeasure") [default: "PCS"];
    blocked: Boolean [default: false];
    created_at: DateTime [auto];
    updated_at: DateTime [auto];

    keys {
        key PK(id);
        key ItemNoIdx(item_no);
        key DescIdx(description);
    }
}
```

### Supported Field Data Types:
- `UUID`: 128-bit time-ordered UUIDv7.
- `String(n)`: Varchar with maximum character length.
- `Text`: Unbounded text field.
- `Integer`: 32-bit signed integer.
- `BigInt`: 64-bit signed integer.
- `Decimal(p, s)`: Fixed-point exact decimal with precision `p` and scale `s`.
- `Boolean`: `true` or `false`.
- `Date`: Calendar date (`YYYY-MM-DD`).
- `DateTime`: UTC timestamp with timezone.
- `Enum("Name")`: Enumerated set of values.
- `FlowField(...)`: Declarative virtual calculation field.

---

## 3. Creating State Machines (`statemachine`)

State machines define document lifecycle workflows and role-based transition authorization.

```bos
statemachine SalesOrderLifecycle for SalesHeader {
    initial: Draft;

    state Draft {
        on release -> Released {
            guard: "this.total_amount > 0";
        }
    }

    state Released {
        on reopen -> Draft {
            roles: ["SalesManager", "Admin"];
        }
        on post -> Posted {
            roles: ["SalesClerk", "SalesManager"];
            action: "SalesPostCodeunit.post(this.id)";
        }
    }

    state Posted {
        // Immutable terminal state
    }
}
```

---

## 4. Creating Codeunits (`codeunit`)

Codeunits encapsulate business logic services, batch processing, and transactional posting.

```bos
codeunit SalesPost {
    procedure post(orderId: UUID): Boolean {
        // 1. Fetch order header
        const header = models.SalesHeader.findById(orderId);
        if (!header) {
            throw new Error("Order not found: " + orderId);
        }

        // 2. Fetch lines
        const lines = models.SalesLine.find({ sales_header_id: orderId });
        if (lines.length === 0) {
            throw new Error("Cannot post empty sales order.");
        }

        // 3. Post to immutable General Ledger
        for (const line of lines) {
            models.ItemLedgerEntry.insert({
                item_no: line.item_no,
                quantity: line.quantity.negate(),
                document_no: header.order_no,
                posting_date: header.posting_date
            });
        }

        // 4. Update status
        header.status = "Posted";
        header.save();

        return true;
    }
}
```

---

## 5. Non-Invasive Extensions (`tableextension`)

Extend core tables from vendor packages without touching their source code:

```bos
tableextension CustomerLoyalty for Customer {
    loyalty_tier: Enum("LoyaltyTier") [default: "Bronze"];
    reward_points: Integer [default: 0];

    keys {
        key LoyaltyIdx(loyalty_tier);
    }
}
```
The compiler automatically merges the extension fields into the database table and IR symbol table.
