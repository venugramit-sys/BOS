# Volume 4: Workflows (`statemachine`), Security (`permissionset`), Analytical (`view`, `query`) & UI (`page`, `report`, `custommetadata`)

This volume documents the declarative workflow, security, analytical, UI, reporting, and configuration meta-objects in **BOS-Script** with real-world enterprise examples.

---

## 1. Deterministic Workflows (`statemachine` & `statemachineextension`)

A `statemachine` governs the lifecycle of a document or master record's status column (`for <Table>.<status_field>`).

### 1.1. Key Capabilities
- **States (`initial state <Name>`, `state <Name>`)**:
  - `allowModify: true | false`: Whether records in this state may be edited.
  - `allowDelete: true | false`: Whether records in this state may be deleted.
  - `timeout(duration, escalateTo: "<Role>", action: "<Transition>")`: SLA escalation timer.
- **Transitions (`transition <name> from <StateA> to <StateB>`)**:
  - `guard(rec) { ... }`: Boolean or throwing condition that must hold before transitioning.
  - `approvalMatrix { tier <Name> { condition: ...; roles: [...]; } }`: Multi-tier role approval rules.
  - `effect(rec) { ... }` / `onTransition(rec) { ... }`: Side-effect logic executed during transition.
- **Physical Audit Table**: `PostgresGenerator::generate_state_machine_infrastructure` automatically creates `"<table_name>_state_history"` (`id`, `entity_id`, `from_state`, `to_state`, `action`, `actor_id`, `comments`, `params`, `timestamp`).

### Real-World Use Case: Purchase Order Approval Workflow

```bos
statemachine PurchaseOrderWorkflow for PurchaseHeader.status {
    initial state Draft {
        allowModify: true;
        allowDelete: true;
    }

    state PendingApproval {
        allowModify: false;
        allowDelete: false;
    }

    state Released {
        allowModify: false;
        allowDelete: false;
    }

    state Closed {
        allowModify: false;
        allowDelete: false;
    }

    transition submitForApproval from Draft to PendingApproval {
        guard(rec) {
            return rec.total_amount > 0;
        }
    }

    transition approve from PendingApproval to Released {
        guard(rec) {
            return rec.total_amount <= 50000 || ctx.userId !== rec.created_by;
        }
    }

    transition reopen from Released to Draft {}
    transition closeOrder from Released to Closed {}
}
```

---

## 2. Role-Based Access Control & Row-Level Security (`permissionset`)

A `permissionset` combines table-level CRUD permissions (`RIMD` — `read`, `insert`, `modify`, `delete`), Codeunit `execute` permissions, and fine-grained PostgreSQL **Row-Level Security (`CREATE POLICY`)** filters (`where (...)`).

### Real-World Use Case: Regional Sales Representative Permission Set

```bos
permissionset RegionalSalesRepRole {
    caption: "Regional Sales Representative";

    permissions {
        Customer: read, insert, modify where (is_blocked == false);
        SalesHeader: read, insert, modify, delete where (status != "Posted");
        CustomerLedgerEntry: read;
    }

    execute: [SalesPostService];
}
```

**What `bosc-codegen-db` & `DataEngine` Do:**
1. `PostgresGenerator::generate_permission_set_ddl` emits `CREATE POLICY "rls_regionalsalesreprole_customer" ON "customer" FOR SELECT, INSERT, UPDATE USING (...)`.
2. `DataEngine::check_permission(ctx, entity, action)` inspects active `permissionset` objects in `_bos_meta_objects` to enforce `read`, `insert`, `modify`, `delete`, and `execute` rights at the API gateway.

---

## 3. Analytical Projections (`view` and `query`)

### 3.1. `view` (Standard & Materialized SQL Views)
A `view` compiles directly into a PostgreSQL `CREATE OR REPLACE VIEW` or `CREATE MATERIALIZED VIEW` (when `materialized: true;` is set, including a unique index on `"_id"` so `REFRESH MATERIALIZED VIEW CONCURRENTLY` works via `DataEngine::refresh_view`):

```bos
view ActiveCustomerSummaryView {
    materialized: true;
    sourceTable: customer;

    select {
        id as customer_id;
        code;
        name;
        credit_limit;
    }

    where (is_blocked == false);
}
```

### 3.2. `query` (Hierarchical Analytical Aggregations)
A `query` defines multi-table analytical joins, column aggregations (`sum`, `count`, `average`, `min`, `max`), `groupBy`, and `orderBy`, executable via `DataEngine::execute_named_query`:

```bos
query TopCustomersByRevenueQuery {
    caption: "Top Customers by Posted Ledger Revenue";

    elements {
        dataitem(cust: Customer) {
            column(customer_code, code);
            column(customer_name, name);

            dataitem(cle: CustomerLedgerEntry) {
                join: Inner on (cle.customer_id == cust.id);
                column(total_sales, amount, sum);
                column(invoice_count, id, count);
            }
        }
    }

    orderBy {
        total_sales: desc;
    }
}
```

---

## 4. Declarative UI, Reporting & Configuration (`page`, `pageextension`, `report`, `custommetadata`)

### 4.1. `page` & `pageextension`
Declares one screen with a single `source` and one `layout`. The page stores field overrides only. The full contract is [`docs/28_page_meta_object_design.md`](../28_page_meta_object_design.md). In Monaco Code Studio, typing `field(|)` inside `main` autocompletes columns of that source. The snippet inserts the field name. The caption stays on the table unless the author overrides it.

```bos
page NoteCard {
    source: Note;
    layout: record;
    caption: "Note Card";

    main {
        section(General) {
            field(code);
            field(content);
            field(content_2);
        }
    }

    commands {
        command(SyncNote) {
            caption: "Sync Note";
            run: NoteSync.run;
        }
    }
}

pageextension NoteCardAuditExt extends NoteCard {
    main {
        addLast(General) {
            field(created_at) { editable: false; }
        }
    }
}
```

### 4.2. `report` & `custommetadata`
```bos
report NoteSummaryReport {
    caption: "Notes Audit Report";
    dataset: Note;
}

custommetadata TaxProviderConfig {
    provider_code: String(30) notNull;
    endpoint_url: String(250) notNull;
    timeout_ms: Integer default(5000);
}
```
