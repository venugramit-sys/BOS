---
trigger: glob
description: "BOS-Script and package rules for packages/ and .bos sources"
globs: "packages/**, examples/**/*.bos, **/*.bos, **/bos.json"
---
# BOS packages and BOS-Script
- Specs: docs/01 (language), docs/02 (12 meta-objects), docs/10 (packaging), docs/06 (QuickJS: 8 MB RAM cap, 100k fuel).
- packages/<package_id>/ with strict bos.json; non-invasive layering; singular PascalCase tables -> singular snake_case SQL.
- Deep research online (NAV/BC, SAP, NetSuite, Salesforce) before drafting; never assume, ask paired with Quick + Real-World Business Examples.
- Mandatory codebase pre-inspection: inspect bosc-syntax, -analyzer, -ir, -runtime before writing code to guarantee 100% sync.
- Mandatory dedicated Setup Table & Card Page: every module defines a singleton `<ModuleName>Setup` table and `<ModuleName>SetupCard` page with FastTabs; zero magic numbers in business logic.
- table = schema/constraints/FlowFields; tablelogic = validate/onInsert/onModify/onDelete/record methods; codeunit = cross-record procedures, @businessEvent.
- Zero N+1 (FlowFields/subqueries/pushdown), exact decimals, two-phase posting, immutable ledgers.
- **Zero-Defect Meta-Object Invariants**:
  - `this` is `undefined` in QuickJS codeunit executions: NEVER use `this.Method()`; use `ctx.call("Codeunit", "Method", params)` or direct `Table.get(...)`.
  - Page variables in triggers must be accessed via `ctx.page.variables` (never assume `vars` is a positional parameter).
  - Page action triggers must explicitly `return res;` so that UI handlers receive `res.return_value`.
  - Dialog pages (`StandardDialog`, `ConfirmationDialog`, `NavigatePage`) hide action bars: actions meant for the footer MUST be declared directly in `actions { ... }` with `inFooterBar = true;` (no `area(...)` wrapper).
  - Action names must strictly match UI routing contracts (e.g. `action(SignIn)`), with `primary = true;` on primary dialog actions for Enter-key QuickEntry.
- **Concise BOS-Script (Less Code for More Output)**: Use built-in FlowFields, declarative cascades (`onDelete = cascade;`), `ctx.crypto.*`, `ctx.ui.*`, and global Table proxies (`Table.get`, `Table.insert`, `Table.update`) instead of bloated imperative JS boilerplate.
- **Proactive Testing Against Delivery**: Always verify end-to-end: (1) `bosc package build`, (2) `bosc package install`, (3) run procedure/trigger tests with realistic payloads, (4) test critical UI paths headless. Catch and fix issues proactively before handoff.
- **Continuous Improvements Log**: Log all core engine bottlenecks, developer friction, or BOS-Script language improvement ideas in `docs/planning/bos_core_and_script_improvements.md` as requirements evolve.
