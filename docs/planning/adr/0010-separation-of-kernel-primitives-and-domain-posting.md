# 0010. Separation of Kernel Primitives and Domain Posting / Numbering Architecture

- **Status**: Accepted
- **Date**: 2026-10-28
- **Decider**: Supervisor & Product Owner

## Context
A major architectural dilemma arose regarding the responsibility boundaries between the core Rust systems engine (`crates/bosc-*`) and domain business packages (`packages/bos.core`, `packages/test.procurement`, etc.). Specifically:
1. Should document posting workflows and sequential number series allocation be implemented in the Rust systems kernel or in `.bos` domain meta-objects?
2. How do we ensure zero business logic leaks into Rust, while guaranteeing ACID transactional safety, concurrency locking, and ledger immutability?

## Decision
1. **The Iron Invariant: Zero Business Logic in Rust**:
   - The Rust engine is strictly domain-agnostic. It contains **no** accounting rules, no debit/credit validations, no number sequence mask parsing, and no business posting pipelines.
   - All domain logic, document lifecycles, and posting routines are written in declarative `.bos` meta-objects (`codeunit`, `statemachine`, `tablelogic`) in business packages.

2. **Rust Systems Primitives**:
   - Generic Pessimistic Row Locking (`Table.lock(pk)`): Executes SQL `SELECT ... FOR UPDATE` inside the active transaction.
   - Generic Atomic Scope (`ctx.tx`): Rolls back all changes if an unhandled exception or assertion fails.
   - Generic Table Immutability DDL: When a table metadata sets `tableType = Ledger;`, the database codegen automatically creates a PostgreSQL trigger (`guard_immutable_<table_name>`) rejecting `UPDATE` and `DELETE` queries at the database layer.
   - Generic Event Bus: Dispatches `@businessEvent` calls in topological dependency order.

3. **Domain Implementation in `.bos` Packages**:
   - **Number Series Engine**: Authored entirely in `packages/bos.core/06_number_series.bos` via `NoSeries`, `NoSeriesLine`, `NoSeriesSetup` tables, and `NoSeriesManagement` codeunit (using `Table.lock` to ensure gapless, collision-free allocation).
   - **Document Posting Pipeline**: Authored in domain codeunits (e.g. `PurchasePost`, `SalesPost`) adhering to the 3-phase lifecycle:
     - Phase 1: `Check & Lock` (validate mandatory fields, check dates, acquire row locks).
     - Phase 2: `Post` (fetch number from No. Series, insert immutable ledger entries, update master balances).
     - Phase 3: `Archive & Clean` (copy to posted history tables, delete working draft lines).

## Consequences
- **Positive**: Complete developer extensibility. Third-party packages can subscribe to posting events (`@businessEvent`) or customize number series formatting without touching or recompiling Rust binaries.
- **Positive**: 100% adherence to Dynamics 365 Business Central and SAP RAP industry standards.
- **Positive**: Safe multi-node clustering; concurrency locks and sequence state live securely in PostgreSQL rather than fragile in-memory Rust caches.
