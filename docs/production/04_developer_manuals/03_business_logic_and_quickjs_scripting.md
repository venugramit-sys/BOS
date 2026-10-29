# Volume 4: Developer Manuals
## 03. Business Logic & QuickJS Scripting

Business validations, entity triggers, and calculations in BOS are authored in TypeScript and executed within the high-speed sandboxed QuickJS virtual machine.

---

## 🏗️ Structure of a `tablelogic` File

Companion logic files (`.bl.bos`) accompany table definitions and implement record lifecycle triggers:

```typescript
tablelogic SalesLine {
    // 1. Field Validation Triggers
    validate quantity(value: Decimal, xRec: SalesLine) {
        if (value <= 0n) {
            throw new Error("Quantity must be greater than zero.");
        }
        this.recalculateAmount();
    }

    validate unit_price(value: Decimal, xRec: SalesLine) {
        if (value < 0n) {
            throw new Error("Unit price cannot be negative.");
        }
        this.recalculateAmount();
    }

    // 2. Lifecycle Triggers
    onInsert() {
        if (!this.line_no) {
            this.line_no = ctx.numberSeries.getNextLineNo(this.sales_header_id);
        }
    }

    onModify() {
        // Automatically touch parent header updated_at
        models.SalesHeader.touch(this.sales_header_id);
    }

    onDelete() {
        // Prevent deleting line if header is locked
        const header = models.SalesHeader.findById(this.sales_header_id);
        if (header.status === "Posted") {
            throw new Error("Cannot delete lines from a posted document.");
        }
    }

    // 3. Helper Methods
    recalculateAmount() {
        const qty = ctx.decimal.from(this.quantity);
        const price = ctx.decimal.from(this.unit_price);
        const discountRate = ctx.decimal.from(this.discount_pct || "0");

        const gross = qty.mul(price);
        const discount = gross.mul(discountRate).div(ctx.decimal.from("100"));
        this.amount = gross.sub(discount).toString();
    }
}
```

---

## 🔢 Exact 128-Bit Monetary Decimal Math

Never use JavaScript floating-point numbers (`number`) for financial amounts. Always use the injected `ctx.decimal` API:

| Operation | Method Syntax | Example |
| :--- | :--- | :--- |
| **Creation** | `ctx.decimal.from(val)` | `const a = ctx.decimal.from("150.2500")` |
| **Addition** | `a.add(b)` | `const total = a.add(b)` |
| **Subtraction** | `a.sub(b)` | `const net = gross.sub(discount)` |
| **Multiplication**| `a.mul(b)` | `const lineTotal = qty.mul(unitPrice)` |
| **Division** | `a.div(b)` | `const unitCost = totalCost.div(qty)` |
| **Comparison** | `a.cmp(b)` | `if (a.cmp(b) > 0) { ... }` |
| **Rounding** | `a.round(scale, mode)` | `a.round(2, "HalfEven")` *(Bankers' Rounding)* |

---

## 🛡️ Sandbox Limits & Best Practices

1. **Memory Budget**: Each execution context has an 8 MB heap quota. Avoid instantiating massive arrays in memory; process records in paginated batches (`find({ limit: 100, offset })`).
2. **Instruction Fuel**: 100,000 instructions are permitted per trigger. Heavy aggregation should be pushed to declarative FlowFields rather than calculated in JavaScript loops.
3. **No Side Effects in Validations**: Field validations should only validate and calculate local record state. Do not trigger external network calls or write to unrelated tables inside a field `validate` hook.
