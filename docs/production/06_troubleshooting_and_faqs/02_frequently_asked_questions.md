# Volume 6: Troubleshooting & FAQs
## 02. Frequently Asked Questions (FAQ)

Answers to frequently asked questions across beginners, operators, administrators, and developers.

---

## 🟢 General & Beginner Questions

### Q1: What makes BOS different from traditional ERPs like SAP or Microsoft Dynamics Business Central?
**A**: Traditional ERP systems carry decades of legacy runtime bloat, multi-gigabyte virtual machines, proprietary relational database locks, and slow development cycles. BOS is engineered in modern, memory-safe Rust and reactive Svelte 5 with zero legacy baggage. It provides enterprise features (immutable ledgers, FlowFields, state machine workflows, two-phase posting, Row-Level Security) with sub-50ms cold starts, native TypeScript/QuickJS logic, and zero-mock testing.

### Q2: Can I run BOS locally on a lightweight laptop without Docker?
**A**: Yes! A local PostgreSQL 14+ instance is the only external service required. The Rust binary (`bosc`) is a self-contained executable with an embedded QuickJS engine and wasmi interpreter.

---

## 🏢 End-User & Business Questions

### Q3: Why is my document locked and not allowing edits?
**A**: Documents in `PendingApproval` or `Posted` state are locked by design to guarantee financial compliance and auditability. If you need to make changes to a pending document, an authorized approver must click **Reject**, which reverts the document to `Draft` mode.

### Q4: How do I find a record if I don't know the exact document number?
**A**: Press `Ctrl + K` (or `Cmd + K`) to open the Universal Search Palette, or use the instant search bar on any list page. The search engine performs indexed searches across document numbers, customer names, dates, and amounts simultaneously.

---

## 🛡️ Administration & DevOps Questions

### Q5: How do I choose between `shared_rls` and `schema_per_company`?
**A**:
- Use **`shared_rls`** if you are hosting a multi-tenant cloud application where operational simplicity, automatic schema synchronization, and cost-effective database resource sharing are paramount.
- Use **`schema_per_company`** if individual companies have statutory data residency requirements, separate backup retention schedules, or require distinct database access policies.

### Q6: What happens if the database connection drops while the node is running?
**A**: The SQLx connection pool automatically detects dropped connections and initiates asynchronous background reconnections with exponential backoff. Incoming HTTP requests that arrive during the outage receive structured 503 Service Unavailable responses, and retry automatically once the connection recovers.

---

## 💻 Developer & Scripting Questions

### Q7: Why does the compiler throw an error when I use standard JavaScript `Math.round()`?
**A**: Standard JavaScript `number` uses IEEE 754 floating-point arithmetic, which leads to precision loss and fractional penny rounding discrepancies. BOS requires all currency and financial calculations to use the exact 128-bit fixed-point decimal engine `ctx.decimal` (e.g. `amount.round(2, "HalfEven")`).

### Q8: How do I handle large batch processing without hitting QuickJS fuel limits?
**A**:
1. Push aggregations and filter sets down to PostgreSQL using declarative **FlowFields** or queries.
2. Delegate large batch jobs to asynchronous NAS worker daemons that process records in chunks (e.g., 50 to 100 rows per batch) rather than running unbounded loops inside a single trigger.
