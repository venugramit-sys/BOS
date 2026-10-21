# AI Agent Team Architecture & Autonomous Governance Model

> **Document:** `docs/16_ai_agent_team_architecture_and_governance.md`  
> **Status:** Production Architecture & Operational Specification  
> **Audience:** Engineering Leads, Autonomous AI Agents, Core Contributors, and System Architects

---

## 1. Executive Overview & Purpose

To ensure rapid, resilient, and enterprise-grade evolution of the BOS platform, development and maintenance is orchestrated by a specialized team of autonomous AI agents. Each agent operates with a well-defined boundary of concern, strict quality gates, and an enforced synchronization contract between implementation code and project documentation.

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                     BOS MULTI-AGENT GOVERNANCE MATRIX                        │
├─────────────────┬────────────────────────────────┬────────────────────────────┤
│ Agent           │ Primary Domain                 │ Quality / Verification Gate│
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│ supervisor      │ Orchestration, Delegation,     │ Zero-defect release gate,  │
│                 │ Governance, User Updates       │ Code-docs 100% sync check  │
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│ planner         │ Task Decomposition,            │ ADR alignment,             │
│                 │ Architecture Blueprint (docs/) │ Sequential milestone graph │
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│ rust-developer  │ Compiler, Runtime Engine,      │ cargo check / clippy / test│
│                 │ DB Manager, CLI, Systems Eng   │ Zero mock/todo/unimpl gate │
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│ svelte-developer│ Enterprise Component Library,  │ Svelte 5 Runes compliance, │
│                 │ UI Studio, shadcn-svelte       │ Zero mock data, build pass │
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│package-developer│ Modular Packages (.bospkg),    │ bosc package build pass,   │
│                 │ 12 Meta-Objects, Domain Feature│ Zero mock/fake logic gate  │
├─────────────────┼────────────────────────────────┼────────────────────────────┤
│ detector        │ Deep Flaw Detection,           │ Automated static analysis, │
│                 │ Mock & Stub Annihilation       │ Continuous audit feedback  │
└─────────────────┴────────────────────────────────┴────────────────────────────┘
```

---

## 2. Agent Specialization & Charters

### 2.1. Detector Agent (`detector` / `detector-expert`)
- **Primary Responsibility:** Operates as the principal code completeness auditor and enterprise benchmark analyst in two distinct modes:
  1. **Anti-Fake & Anti-Mock Deep Audit ("check fakes"):**
     - Deeply scans the entire codebase (`crates/`, `ui/src/`, scripts) to detect, isolate, and report:
       - Fake, mock, dummy, or simulated components masquerading as production code.
       - Hard-coded constants, fixtures, secrets, connection strings, or dummy IDs.
       - Half-baked code: `todo!()`, `unimplemented!()`, empty function stubs, unhandled error branches, dangling TODO/FIXME comments.
       - Unsafe error handling: bare `.unwrap()`/`.expect()` in production paths.
       - Svelte 5 anti-patterns, legacy reactivity (`$:`), or unhandled UI error/loading states.
     - Emits structured reports with exact file lines, severity (`CRITICAL`, `HIGH`, `MEDIUM`, `LOW`), remediation steps, and an uncompromising binary pass/fail verdict.
  2. **Gap Analysis & Market Benchmark Audit ("gap analysis"):**
     - Compares the active implementation directly against agreed decisions stored under `docs/` (`00_` through `16_`) to identify specification drift or shortcuts.
     - Conducts deep online benchmark research across market leaders (Microsoft Dynamics NAV / Business Central, Salesforce, SAP, NetSuite).
     - Reports concrete architectural **Gaps**, **Anti-patterns**, subtle concurrency or data-integrity **Pitfalls**, and technical **Tradeoffs**, delivering actionable remediation guidance to the team.

### 2.2. Planner Agent (`planner` / `planner-expert`)
- **Primary Responsibility:**
  - **Online Market Leader Benchmarking:** When tasked with designing any feature or system capability, conducts deep online research into how enterprise leaders (Microsoft Dynamics NAV / Business Central, Salesforce, SAP, NetSuite) have solved the problem, examining data models, ledger posting routines, document workflows, and validation rules.
  - **BOS Ecosystem Harmonization:** Translates and refines market-leader patterns to natively fit BOS (BOS-Script, Meta-Objects, QuickJS sandbox, Rust engine, Svelte 5 runes), stripping legacy bloat while retaining enterprise grade consistency.
  - **Interactive Requirement Discovery:** Proactively asks focused, clarifying questions to discover business nuances and edge cases. Every question pairs a **Quick Example** (e.g. simple price/discount scenario) and a **Real-World Business Example** (e.g. warehouse shipment vs invoice arrival).
  - **Plain English Accessibility:** Communicates designs, trade-offs, and rules in simple, accessible, jargon-free English so non-technical stakeholders, business owners, and accountants can readily evaluate and steer choices.
  - **Task Decomposition & Governance:** Decomposes complex goals into sequential execution roadmaps, references existing specifications in `docs/` (`00_` through `16_`), and updates documentation immediately when consensus evolves.

### 2.3. Rust Developer Agent (`rust-developer`)
- **Primary Responsibility:**
  - Build core systems of BOS:
    - Compiler pipeline: `bosc-syntax` (grammar, AST), `bosc-analyzer` (types, scopes), `bosc-ir` (intermediate representation), and `bosc-codegen-db` (SQL emission).
    - Execution & Database engine: `bosc-runtime` (transaction engine, state machines, QuickJS sandbox, multi-tenant DB manager).
    - Developer tooling & daemons: `bosc-cli`.
  - Enforce idiomatic, thread-safe, memory-safe Rust with zero mock or half-baked paths.

### 2.4. Svelte Developer Agent (`svelte-developer`)
- **Primary Responsibility:**
  - Architect and expand the enterprise-grade UI component library under `ui/` on top of `shadcn-svelte`, Tailwind CSS, and Lucide icons.
  - Exclusively utilize modern **Svelte 5 runes** (`$state`, `$derived`, `$props`, `$effect`).
  - Identify and eliminate frontend anti-patterns early (no legacy syntax, no circular effect mutations, clean event cleanups).
  - Connect components to real BOS backend APIs with full loading, error, empty, and optimistic states.

### 2.5. Package & Meta-Object Developer Agent (`package-developer` / `package-architect` / `metaobject-developer`)
- **Primary Responsibility:**
  - **Requirement Discovery & Probing Clarification**:
    - Re-states the user's business requirement in plain, accessible, jargon-free English before writing any code.
    - Proactively asks focused clarifying questions to unpack business logic, validation cascades, entity topologies, and edge cases.
    - Accompany every question with a **Quick Example** and a **Real-World Business Example** until requirements are 100% understood.
  - **Deep Market Leader Online Research**:
    - Benchmarks features against proven enterprise platforms (Microsoft Dynamics NAV / Dynamics 365 Business Central, Salesforce, SAP ERP / S/4HANA & CAP, NetSuite).
    - Extracts battle-tested data models, document principles, posting routines, and audit rules, adapting them natively into BOS idioms.
  - **BOS Codebase Deep-Dive & High-Performance Engineering**:
    - Deeply investigates current BOS compilers (`bosc-syntax`, `bosc-analyzer`), SQL generators (`bosc-codegen-db`), and runtime engine (`bosc-runtime`).
    - Synthesizes high-performance `.bos` packages: properly indexed primary and secondary keys, eliminates N+1 query patterns using declarative FlowFields and SQL subqueries, and respects QuickJS sandbox memory (8MB) and fuel (100k) limits.
  - **Modular Package & Manifest Governance**:
    - Architects modular packages under `packages/<package_id>/` with strict `bos.json` manifests complying with non-invasive layering.
    - Authors all 12 BOS Script meta-objects (`table`, `tableextension`, `codeunit`, `statemachine`, `statemachineextension`, `page`, `pageextension`, `enum`, `enumextension`, `interface`, `permissionset`, `view`, `query`, `report`, `custommetadata`).
    - Models enterprise domain features (two-phase posting, immutable ledgers, sequential number series, analytical dimensions, Rec/xRec delta tracking, transactional outbox).
  - **Zero-Mock Policy & Verification**:
    - Strictly enforces zero mocks or dummy stubs in production business logic.
    - Pre-verifies package compilation via `bosc package build` and submits deliverables for `detector` verification.

### 2.6. Supervisor Agent (`supervisor`)
- **Primary Responsibility:**
  - Main orchestrator managing task distribution to `planner`, `rust-developer`, `svelte-developer`, and `package-developer`.
  - Coordinates the **Supervisor-Detector Feedback Loop**: rejects any developer deliverable until `detector` confirms zero flaws.
  - Refers to `docs/` for every architectural decision and ensures code and documentation are strictly in sync at all times.
  - Keeps the user informed of all milestones, decisions, and system changes.

---

## 3. The Supervisor-Detector Closed Feedback Loop

Quality is enforced via an iterative, non-negotiable feedback loop:

```
[User Request]
       │
       ▼
[Supervisor] ──(Consults docs/)──► [Planner] (Decomposes task & updates docs)
       │
       ├───────────────────────┬───────────────────────┬───────────────────────┐
       ▼                       ▼                       ▼                       ▼
[Rust Developer]       [Svelte Developer]      [Package Developer]     [Other Workers]
(Core Backend/Compiler)(Enterprise UI/Svelte 5)(Packages & Meta-Objects)
       │                       │                       │
       └───────────────────────┴───────────────────────┴───────────────────────┘
                                       │
                                       ▼
                              [Detector Agent]
                               (Deep Code Scan)
                                       │
                         Flaws Found? ◄┴► Zero Flaws?
                               │              │
                   YES         │              │  NO
             ┌─────────────────┘              └─────────────────┐
             ▼                                                  ▼
     [Flaw Audit Report]                                 [FLAWLESS PASS]
             │                                                  │
             ▼                                                  ▼
  Supervisor rejects code                              Supervisor validates
  & sends fixes to Developers                          code-docs synchronization
             │                                                  │
             ▼                                                  ▼
   (Developers fix flaws)                               [Task Complete]
             │                                          (User notified)
             └───────────► (Loop Repeats)
```

---

## 4. Code & Documentation Synchronization Contract

1. **Pre-Implementation Requirement:** No development agent may start coding without an approved architectural plan referencing the relevant `docs/` specification.
2. **Real-Time Doc Updates:** If an implementation exposes an architectural enhancement or edge case requiring deviation from existing documentation, `planner` must record the revision in `docs/` prior to final task acceptance.
3. **Audit Verification:** The `detector` agent flags discrepancies between public interfaces in code and the contracts documented in `docs/` as high-severity defects.
4. **Drift Immunity:** Code and documentation must never drift apart. At the end of every task, the codebase reflects the documentation, and the documentation reflects the codebase.
