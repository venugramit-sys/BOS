# Meta-Object Specification: `statemachine` & Business Workflows

> **Status:** Final Frozen Specification  
> **Document:** `docs/01_meta_object_statemachine_specification.md`  
> **References:** Finite State Machine (FSM) theory, Enterprise Approval Patterns, Multi-Level Matrix Routing, Svelte 5 Action Generators, PostgreSQL Concurrency.  

---

## 1. Why a Dedicated `statemachine` Meta-Object?

In enterprise ERP systems, document lifecycles (Purchase Orders, Sales Invoices, Expense Reports, Quality Inspections) are governed by strict state transitions.

### The Problem in Traditional Frameworks:
- **Scattered `if/else` checks:** Transition rules are fragmented across API controllers, database triggers, Svelte/React buttons, and background cron jobs.
- **Race conditions & double-clicks:** Two managers clicking "Approve" simultaneously can cause duplicate approvals or ledger entries.
- **Manual UI synchronization:** Developers must repeatedly write logic to decide if the "Approve" button should be visible, disabled, or prompt for a rejection comment.
- **Missing audit history:** Building dedicated history tables to record who transitioned what, when, and with what comment is tedious and error-prone.

### The BOS Script Solution:
In BOS Script, **`statemachine`** is a first-class, declarative meta-object that compiles into:
1. **PostgreSQL Database:** Atomic state transition procedures with optimistic locking and an automatic `<Table>_StateHistory` audit table.
2. **WASM Backend:** Strict transition guards, dynamic approval matrix routing, and transaction hooks.
3. **Svelte 5 Frontend:** Auto-generated action buttons, permission-gated disabled states, and automatic modal prompts for parameterized actions (e.g., Rejection Reasons).

---

## 2. Architecture & Lifecycle Overview

```
                      ┌───────────────────────────────────────┐
                      │       BOS State Machine Lifecycle     │
                      └───────────────────┬───────────────────┘
                                          │
                  Action Clicked in UI (e.g., "Submit" / "Approve")
                                          │
                                          ▼
                      ┌───────────────────────────────────────┐
                      │ 1. Parameter Prompt (if params exist) │
                      │    - Svelte auto-renders modal dialog │
                      └───────────────────┬───────────────────┘
                                          │
                                          ▼
                      ┌───────────────────────────────────────┐
                      │ 2. Backend Guard & Role Verification  │
                      │    - Current user has required role?  │
                      │    - Dynamic approval matrix check?   │
                      │    - Guard boolean function passes?   │
                      └───────────────────┬───────────────────┘
                                          │
                                          ▼
                      ┌───────────────────────────────────────┐
                      │ 3. Atomic DB Execution (PostgreSQL)   │
                      │    - UPDATE ... WHERE status = :from  │
                      │      AND _version = :expected_version │
                      │    - Insert into _StateHistory log    │
                      │    - Execute `onTransition` block     │
                      └───────────────────┬───────────────────┘
                                          │
                                          ▼
                      ┌───────────────────────────────────────┐
                      │ 4. Post-Commit Side Effects           │
                      │    - Outbox domain events emitted     │
                      │    - Email / Push notifications       │
                      │    - UI real-time WebSocket refresh   │
                      └───────────────────────────────────────┘
```

---

## 3. Syntax & Structure

### 3.1 Declarative Definition (`PurchaseOrderWorkflow.bos`)

```typescript
statemachine PurchaseOrderWorkflow for PurchaseOrder.status {
    caption = "Purchase Order Lifecycle";
    initial = "Draft";
    terminal = ["Invoiced", "Cancelled"];

    // States configuration
    states {
        state "Draft" {
            caption = "Draft";
            badge = "neutral";
        }
        state "PendingApproval" {
            caption = "Pending Approval";
            badge = "warning";
        }
        state "Approved" {
            caption = "Approved";
            badge = "success";
        }
        state "Ordered" {
            caption = "Ordered";
            badge = "info";
        }
        state "Invoiced" {
            caption = "Invoiced";
            badge = "success";
        }
        state "Cancelled" {
            caption = "Cancelled";
            badge = "danger";
        }
    }

    // Transitions
    transition Submit {
        caption = "Submit for Approval";
        icon = "Send";
        from = "Draft";
        to = "PendingApproval";

        // Guard: Business rule validation before transition
        guard(doc) {
            return doc.lines.length > 0 && doc.totalAmount > 0;
        }

        onTransition(doc) {
            Notification.sendToManager(doc.createdBy, `PO ${doc.no} submitted for approval`);
        }
    }

    transition Approve {
        caption = "Approve";
        icon = "CheckCircle";
        from = "PendingApproval";
        to = "Approved";

        // Enterprise Approval Matrix Routing
        approvalMatrix {
            tier Manager {
                condition = doc.totalAmount <= 5_000;
                roles = ["PurchasingManager"];
            }
            tier Director {
                condition = doc.totalAmount <= 50_000;
                roles = ["DepartmentDirector"];
            }
            tier CFO {
                condition = doc.totalAmount > 50_000;
                roles = ["CFO", "Executive"];
            }
        }

        onTransition(doc) {
            doc.approvedBy = context.user.id;
            doc.approvedAt = context.now;
        }
    }

    transition Reject {
        caption = "Reject";
        icon = "XCircle";
        from = "PendingApproval";
        to = "Draft";
        roles = ["PurchasingManager", "DepartmentDirector", "CFO"];

        // Parameterized transition: Prompts user in Svelte modal
        params {
            reason: String(250) {
                caption = "Reason for Rejection";
                notNull = true;
            };
        }

        onTransition(doc, params) {
            doc.rejectionNotes = params.reason;
            Notification.send(doc.createdBy, `PO ${doc.no} rejected: ${params.reason}`);
        }
    }

    transition Cancel {
        caption = "Cancel Order";
        icon = "Slash";
        confirm = "Are you sure you want to cancel this Purchase Order? This cannot be undone.";
        from = ["Draft", "PendingApproval", "Approved"];
        to = "Cancelled";
        roles = ["PurchasingManager", "Admin"];

        guard(doc) {
            return doc.receivedQuantity == 0;
        }
    }
}
```

---

## 4. Key World-Class Features

### 4.1 Enterprise Multi-Tier Approval Matrix
In enterprise ERPs, approval thresholds depend on amounts, departments, or custom dimensions:
- `approvalMatrix` blocks let architects define tiered routing without hardcoded user IDs.
- The framework resolves the current user's roles and limits dynamically at runtime.
- If a user lacks the limit, the UI automatically hides or disables the button and displays a tooltip: *"Requires Department Director approval (Amount > $5,000)"*.

### 4.2 Parameterized Transitions (Automatic Svelte Dialogs)
Transitions can declare a `params` block:
- When a transition with `params` is triggered in the Svelte UI, the framework automatically renders a modal dialog with full type validation (e.g. required rejection comments, shipment tracking numbers, return codes).
- The parameters are passed to `onTransition(doc, params)` and automatically persisted in the `_StateHistory` audit log.

### 4.3 Atomic PostgreSQL State Execution & Concurrency
To guarantee zero double-approvals in high-concurrency environments:
```sql
-- Generated PostgreSQL atomic transition statement
UPDATE purchase_order
SET status = 'Approved',
    _system_version = _system_version + 1,
    _updated_at = NOW(),
    _updated_by = current_setting('bos.user_id')::uuid
WHERE id = :order_id
  AND status = 'PendingApproval'
  AND _system_version = :expected_version;
```
If another manager approved the document a millisecond prior, the query returns 0 rows modified and rolls back cleanly with a descriptive concurrency error: *"Record was already transitioned by another user."*

### 4.4 Automated State History Audit Table
The compiler automatically generates and populates a shadow history table `<Entity>_StateHistory`:
- `id`: UUID (Primary Key)
- `entity_id`: UUID (Target document ID)
- `from_state`: String
- `to_state`: String
- `action`: String (Name of the transition)
- `actor_id`: UUID (User who executed the transition)
- `timestamp`: TIMESTAMPTZ (Exact time)
- `params`: JSONB (Any transition parameters such as rejection reason)
- `ip_address`: INET

### 4.5 Timeout & Auto-Escalation Engine
In enterprise procurement, documents cannot stay stuck indefinitely if an approver is on leave:
- States can declare an optional `timeout` block:
```typescript
state "PendingApproval" {
    caption = "Pending Approval";
    badge = "warning";
    timeout(48.hours) {
        escalateTo = "DepartmentDirector";
        notify = [doc.createdBy, "ApproverManager"];
        action = AutoEscalate;
    }
}
```
- The BOS host runtime engine schedules background timer evaluations and triggers the escalation transition automatically.

---

## 5. Non-Invasive Workflow Extensibility (`statemachineextension`)

Downstream extensions and ISVs can modify or augment state machines without editing upstream code:

```typescript
statemachineextension PurchaseOrderWorkflowQualityExt extends PurchaseOrderWorkflow {
    // Add a new state
    states {
        state "PendingQualityInspection" {
            caption = "Quality Check";
            badge = "warning";
        }
    }

    // Intercept transition: redirect Approved -> PendingQualityInspection
    modifyTransition Approve {
        to = "PendingQualityInspection";
    }

    // Add new transition from inspection to Released
    transition PassInspection {
        caption = "Pass Quality Inspection";
        icon = "ShieldCheck";
        from = "PendingQualityInspection";
        to = "Approved";
        roles = ["QualityInspector"];
    }
}
```

---

## 6. Svelte 5 UI Auto-Synthesis

The BOS UI compiler reads the `statemachine` and automatically binds:
1. **Status Badges:** Dynamic color badges in the header card corresponding to state definitions (`neutral`, `warning`, `success`, `danger`, `info`).
2. **Action Bar Buttons:** Action buttons are dynamically generated in the page action bar with corresponding captions, icons, and keyboard shortcuts.
3. **Reactivity with Runes:** Buttons automatically toggle their enabled/visible state reactively when form fields or document totals change:
   ```svelte
   <!-- Concept of synthesized Svelte 5 rune binding -->
   <Button
       variant="primary"
       disabled={!canApprove($doc)}
       onclick={() => executeTransition('Approve')}>
       Approve
   </Button>
   ```
