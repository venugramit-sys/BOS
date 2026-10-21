# BOS (Business Operating System) — Core Engine Architectural Blueprint

> **Status:** Production Architecture & Complete Implementation  
> **Target Version:** BOS Core v2.0+  
> **Philosophy:** Modular, Modern, Metadata-Oriented, Blazing Fast, Secure, ERP+ Platform  

---

## 1. Executive Summary & Vision

**BOS (Business Operating System)** is next-generation enterprise application infrastructure designed to transcend the limitations of legacy ERPs (SAP, Microsoft Dynamics 365 Business Central / AL, Salesforce, Odoo) and boilerplate-heavy microservice frameworks.

At its core, BOS replaces brittle, hand-coded glue code with a **strict, compiled, metadata-driven architecture**:
- **Declarative & Intent-Driven:** Developers and enterprise architects define business models, logic, workflows, UI pages, and access rules in **BOS-Script** (a specialized TypeScript superset for business objects).
- **High-Speed Ahead-Of-Time Compilation:** The **Rust Compiler (`bosc`)** compiles BOS-Script with an integrated TypeScript Type Stripper into three specialized target artifacts:
  1. **Database Infrastructure:** Native PostgreSQL schemas, tables, stored generated columns (`GENERATED ALWAYS AS (...) STORED`), FlowField Pushdown SQL Views (`{table}_view`), Row-Level Security (RLS) policies, and in-database AMDP stored procedures.
  2. **Multi-Tier Execution Runtime:** 
     - **QuickJS Bytecode Sandbox:** 8 MB memory ceiling, 512 KB stack, exact 128-bit BigInt monetary math (`ctx.decimal`), dynamic TableModel proxies (`Customer.get()`, `Customer.create()`), and event dispatch.
     - **Sandboxed WebAssembly Host (`wasmi 2.0`):** Pure-Rust Wasm execution with instruction fuel metering (e.g. 50,000–100,000 units) to guarantee immunity against CPU exhaustion and infinite loops.
     - **Companion Sidecar Host:** Low-latency loopback IPC/HTTP bridge for out-of-process AI, OCR, or payment microservices (`ctx.sidecar.call()`).
     - **Decoupled Event Subscribers:** Prioritized publish-subscribe execution pipeline (`_bos_event_subscribers`) guaranteeing non-invasive extensions.
  3. **Frontend UI Components:** Reactive **Svelte 5** Studio application utilizing modern runes (`$state`, `$derived`, `$effect`) for instant API endpoint introspection and real-time execution.

```
                           ┌───────────────────────────────┐
                           │      BOS-Script Modules       │
                           │   (.bos Declarative Metadata  │
                           │    & Embedded Pure TypeScript)│
                           └───────────────┬───────────────┘
                                           │
                                           ▼
                           ┌───────────────────────────────┐
                           │     BOS Compiler (Rust)       │
                           │           `bosc`              │
                           │  - AST & Syntax Parser        │
                           │  - High-Speed TS Stripper     │
                           │  - Semantic Analyzer (IR)     │
                           │  - Code Generators            │
                           └───────┬───────┬───────┬───────┘
                                   │       │       │
             ┌─────────────────────┘       │       └─────────────────────┐
             ▼                             ▼                             ▼
   ┌────────────────────┐        ┌────────────────────┐        ┌────────────────────┐
   │ Database Artifacts │        │  Backend Artifacts │        │ Frontend Artifacts │
   ├────────────────────┤        ├────────────────────┤        ├────────────────────┤
   │ - PostgreSQL DDL   │        │ - QuickJS Sandboxes│        │ - Svelte 5 Studio  │
   │ - Multi-Tenant RLS │        │ - wasmi 2.0 Host   │        │ - Endpoint Tester  │
   │ - Generated Columns│        │ - Sidecar Host IPC │        │ - Schema Discovery │
   │ - FlowField Views  │        │ - Event Engine Bus │        │ - Real-time Trace  │
   │ - AMDP Procedures  │        │ - OCC Concurrency  │        │ - Monaco .d.ts     │
   └────────────────────┘        └────────────────────┘        └────────────────────┘
```

---

## 2. Core Pillars

| Pillar | Principle | Realization |
| :--- | :--- | :--- |
| **Metadata-Oriented** | Single Source of Truth | Every entity (Table, Page, Query, Codeunit, StateMachine, Role) is a typed metadata object describing layout, validation, security, and schema simultaneously in `_bos_meta_objects`. |
| **Non-Invasive Modularity** | Clean Extension Layering | Base application code is immutable; table extensions non-destructively add fields, and decoupled event subscribers hook into mutations without merge conflicts. |
| **3-Tier Execution Pipeline** | Compute Where It Belongs | Tier 1: AMDP / SQL Generated Columns in PostgreSQL (0ms IPC). Tier 2: QuickJS & wasmi Wasm for single-document interactive validations. Tier 3: Rust core kernel for concurrency, RLS, and pooling. |
| **Deterministic Security** | Defense-in-Depth | Multi-tenant context (`x-tenant-id`, `x-company-id`, `x-user-id`, `x-roles`) enforced at the HTTP boundary and pushed down to PostgreSQL Row-Level Security (`current_setting`). |
| **High Concurrency** | Non-Blocking OCC & Locks | Short-lived write transactions with Optimistic Concurrency Control (`_system_version`), plus Dynamics NAV pessimistic `FOR UPDATE` lock patterns. |

---

## 3. Core Engine Crates Topology

```
d:\Work Studio\Bos\
├── crates/
│   ├── bosc-syntax/      <-- Hand-written lexer and parser for .bos syntax
│   ├── bosc-ir/          <-- Intermediate Representation (TableIR, ColumnIR, CodeunitIR, ActionIR)
│   ├── bosc-analyzer/    <-- Lowering, Oxc syntax checks, line-preserving type eraser, script optimizer
│   ├── bosc-codegen-db/  <-- PostgreSQL DDL, Multi-tenant RLS, AMDP, Generated Columns, Views
│   ├── bosc-runtime/     <-- QuickJS VM, wasmi Wasm Host, Sidecar Host, Event Engine, Axum Server
│   └── bosc-cli/         <-- Command-line interface (`bosc build`, `bosc serve`)
├── docs/                 <-- Comprehensive architecture and implementation specifications
└── ui/                   <-- Svelte 5 + Tailwind CSS API Endpoint Tester & Studio Runner
```

---

## 4. Key Architectural Decisions

1. **TypeScript-First Business Logic Stripped at Compile Time:**
   - Full TypeScript IntelliSense via auto-generated ambient declarations (`/api/meta/ambient.d.ts`).
   - Pure Rust regex-based type stripper removes all interfaces, type assertions, and enums ahead-of-time, emitting pristine vanilla JavaScript to QuickJS.
2. **Pure-Rust WebAssembly (`wasmi 2.0`):**
   - Zero external C or LLVM dependencies, cross-compiles cleanly on Windows, Linux, and macOS.
   - Built-in fuel consumption metering ensures that infinite loops or CPU exhaustion are caught and terminated deterministically.
3. **Out-of-Process Sidecar Microservices:**
   - Companion services (Python AI models, Node.js gateways, Go payment bridges) communicate over internal loopback HTTP/IPC with automatic health probes.
4. **Pushdown FlowFields & Formulas:**
   - Formulas are lowered to PostgreSQL `GENERATED ALWAYS AS (...) STORED` columns.
   - SIFT-style aggregations are compiled into PostgreSQL views (`{table}_view`) with subqueries, avoiding application-level query overhead.
5. **High-Density Svelte 5 Studio:**
   - Runes-based reactive UI (`$state`, `$derived`, `$effect`) provides 1-click endpoint execution, live schema auto-fill, and multi-tenant context simulation.
