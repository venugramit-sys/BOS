# Volume 2: End-User & Operator Manuals
## 03. Document Editing & Form Pages

Card and Document pages allow operators to inspect, create, and update detailed records with instant field-level validation, draft persistence, and dirty state management.

---

## 📝 Document Page Structure

A typical business document (e.g., Sales Order, Purchase Invoice) contains two main tiers:
1. **Header Area (General & Posting Details)**: Document Number, Posting Date, Customer/Vendor ID, Currency Code, Payment Terms.
2. **Lines Grid (Subform)**: Tabular lines detailing items, quantities, unit prices, discount rates, and line totals.

```
┌────────────────────────────────────────────────────────────────────────┐
│ Sales Order: SO-2026-0042 [Draft]        [Save Ctrl+S] [Submit] [Close]│
├────────────────────────────────────────────────────────────────────────┤
│ ▼ General                                                              │
│ Customer No.: [ CUST-00101      🔍 ] Name: Acme Industrial Corp       │
│ Posting Date: [ 2026-10-26      📅 ] Due Date: 2026-11-25             │
│ Currency:     [ USD             ▾ ] Status: Draft                     │
├────────────────────────────────────────────────────────────────────────┤
│ ▼ Lines Grid                                                  [+ Line] │
│ Line No. | Type | Item No. | Description    | Qty | Price  | Amount    │
│ 10       | Item | ITM-1001 | Industrial Pump|  2  | 500.00 | 1,000.00  │
│ 20       | Item | ITM-1045 | High-Press Hose|  4  |  25.00 |   100.00  │
├────────────────────────────────────────────────────────────────────────┤
│ Total Excl. Tax: $1,100.00 | Tax (18%): $198.00 | Total Incl.: $1,298.00│
└────────────────────────────────────────────────────────────────────────┘
```

---

## ⚡ Field Validations & Instant Feedback

When typing into input fields, BOS runs immediate validations:

### 1. In-Field Validation Badges
If an entered value violates business rules (e.g., credit limit exceeded, quantity less than zero, invalid postal code), the field is immediately highlighted in red with an explanatory tooltip badge:
```
Quantity: [ -5 ] ⚠️ Quantity must be greater than zero.
```

### 2. Cascading Field Updates
Changing a header field (e.g., switching `Customer No.`) automatically cascades to related fields:
- Payment terms are updated to the customer's default terms.
- Currency code and exchange rates are refreshed.
- Existing lines have customer-specific pricing and discounts recalculated.

### 3. Dirty State Indicator (`*`)
When you modify any field, a small dot or asterisk appears next to the document title:
- `SO-2026-0042 • (Unsaved Changes)`
- Press `Ctrl + S` or click **Save** to commit changes to the backend.
- If you attempt to navigate away with unsaved changes, the application prompts you to confirm discarding edits.

---

## 🔒 Concurrency Protection (Optimistic Concurrency Control)

BOS protects your edits against concurrent overwrites by other operators:
- Every record maintains a hidden `_system_version` sequence.
- If another operator saves changes to the same order while you are viewing it, attempting to save your changes will trigger a **Conflict Alert**:
  ```
  ⚠️ Conflict Detected: This document was updated by user 'sarah_ops' at 14:32.
  [ Review Changes ]   [ Reload Document ]
  ```
- Your uncommitted edits are preserved in local memory so you do not lose your typed input.
