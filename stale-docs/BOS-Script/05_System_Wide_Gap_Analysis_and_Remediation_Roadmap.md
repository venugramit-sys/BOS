# Volume 5: System-Wide Gap Analysis & Remediation Roadmap (BOS-Script vs. Compiler, Runtime, Frontend & DB Management)

This volume provides a rigorous, truth-checked **System-Wide Gap Analysis** comparing the **BOS-Script Language & Studio Surface** against the actual implementation across:
1. **Compiler Crates** (`bosc-syntax`, `bosc-analyzer`, `bosc-ir`, `bosc-codegen-db`)
2. **Runtime & Execution Engine** (`bosc-runtime`: `QuickJsEngine`, `ExecutionEngine`, `DataEngine`, `MetaManager`)
3. **Frontend UI & Monaco Studio** (`ui/src/lib/components/studio/code-editor/monacoBosScript.js`, `monacoLspProvider.js`, `MonacoCodeStudio.svelte`, Visual Designers)
4. **Database & Schema Lifecycle Management** (`PostgresGenerator`, `_bos_meta_objects`, multi-tenant strategies)

---

## Executive Summary of System Gaps & Remediation Status

All **10 discovered system-wide gaps** across Compiler, Runtime, Database Codegen, and Monaco Studio have been **remediated and verified** (`✅ FIXED & VERIFIED`).

| Gap ID | Subsystem | Severity | Status | Summary of Discrepancy & Fix Applied |
| :--- | :--- | :--- | :--- | :--- |
| **GAP-01** | Runtime (`QuickJsEngine`) vs. Monaco Studio (`monacoBosScript.js`) | **CRITICAL** | **✅ FIXED** | **UI Client Bridge Mismatch (`ctx.ui.*`)**: Implemented `ctx.ui.confirm(...)`, `ctx.ui.openModal(...)`, `ctx.ui.navigate(...)`, `ctx.ui.setFieldVisible(...)`, `ctx.ui.setFieldEditable(...)`, and `ctx.ui.getDirectives()` in `QuickJsEngine` (`quickjs_engine.rs`) and added `message`, `success`, `warning`, `error`, `info`, `getMessages`, `getDirectives`, `clearMessages` to `monacoBosScript.js`. |
| **GAP-02** | Runtime (`QuickJsEngine`) vs. Monaco Studio (`monacoBosScript.js`) | **CRITICAL** | **✅ FIXED** | **`ctx.*` host surface**: `ctx.call`, `ctx.workflow`, `ctx.security`, `ctx.http`, `ctx.db`, `ctx.wasm.invoke`, and `ctx.sidecar.call` are real host bindings. `ctx.postLedger` and `ctx.dimension` are not on the host; ledger posting and dimensions belong in application packages. |
| **GAP-03** | Runtime (`QuickJsEngine` FFI) vs. `DataEngine` | **CRITICAL** | **✅ FIXED** | **Multi-Tenant `_tenant_id` Injection & Companion `tablelogic` Methods**: Updated `__host_db_create` and `__host_db_upsert` in `quickjs_engine.rs` to automatically bind `_tenant_id = ctx.tenant_id` when the physical table has a `_tenant_id` column, and updated `ExecutionEngine::dispatch_triggers` (`execution_engine.rs`) to inject companion `tablelogic` `validate<Field>` and record methods onto `__recProto` so `rec.validate("field", val)` and `rec.customMethod()` cascade inside triggers. |
| **GAP-04** | Compiler (`bosc-codegen-db`) vs. `table` AST (`primaryKey`) | **HIGH** | **✅ FIXED** | **Custom Primary Key Uniqueness & Named Secondary Keys**: `SemanticAnalyzer::lower_table` now automatically marks `primaryKey(...)` columns as `is_nullable = false` and `is_unique = true`, supports named `key(idx_name, col1, col2)` without false `BOS2004` errors, and `PostgresGenerator::generate_table` emits `CREATE UNIQUE INDEX IF NOT EXISTS "uq_pk_<table>_<cols>"` so `ON CONFLICT` and foreign keys always succeed. |
| **GAP-05** | Runtime (`DataEngine`) vs. `StateMachineIR` | **HIGH** | **✅ FIXED** | **State Machine Guards, Effects & State Lock Enforcement**: `bosc-ir` and `bosc-analyzer` now lower `allow_modify`, `allow_delete`, `guard_script`, `on_transition_script`, and `approval_tiers`. `DataEngine::update_record` and `delete_record` enforce `allow_modify: false` / `allow_delete: false`, and `DataEngine::transition_state` enforces `required_roles`, evaluates `guard_script` in QuickJS, and runs `on_transition_script`. |
| **GAP-06** | Compiler (`bosc-analyzer`) vs. `bosc-syntax` | **MEDIUM** | **✅ FIXED** | **Extension AST Lowering (`StateMachineExtension` & `EnumExtension`)**: Added `StateMachineExtensionIR` and `EnumExtensionIR` to `ModelIR`, implemented `lower_state_machine_extension` and `lower_enum_extension` in `bosc-analyzer`, and wired `MetaManager::compile_script` to persist and merge them into target `statemachine` / `enum` catalog definitions. |
| **GAP-07** | Compiler (`bosc-codegen-db`) vs. `@pushdown` (BOS-AMDP) | **MEDIUM** | **✅ FIXED** | **AMDP PL/pgSQL Pushdown for TypeScript/JS Bodies**: Enhanced `PostgresGenerator::generate_plpgsql_procedure` to lower common TypeScript statements in `method.script` (`rec.testField(...)`, `rec.<field> = <val>`, `ctx.audit.log(...)`) into native PL/pgSQL `RAISE EXCEPTION` and `jsonb_set` statements. |
| **GAP-08** | DB Codegen (`bosc-codegen-db`) vs. `_bos_meta_objects` Constraint | **MEDIUM** | **✅ FIXED** | **`ON CONFLICT` Target Alignment in `generate_permission_set_ddl` & `generate_query_catalog_ddl`**: Updated `bosc-codegen-db/src/lib.rs` to include `"package_name"` and use `ON CONFLICT ("object_type", "object_name", "package_name") DO UPDATE`. |
| **GAP-09** | Runtime Catalog (`MetaManager::compile_script`) | **MEDIUM** | **✅ FIXED** | **Preserving `source_script` & Registering `@subscribeEvent`**: `MetaManager::compile_script` now attaches `"source_script"` to `table`, `tableextension`, `tablelogic`, `tablelogicextension`, and `codeunit` definitions so multi-object `.bos` buffers (e.g., `table Note` + `tablelogic NoteLogic`) round-trip losslessly, and `ExecutionEngine::register_codeunit` automatically registers `@subscribeEvent(Publisher, "eventName")` methods into `_bos_event_subscribers`. |
| **GAP-10** | Parser & Analyzer Grammar Parity (`bosc-syntax`) | **MEDIUM** | **✅ FIXED** | **Full Grammar Parity Across All 14 Meta-Objects**: Added parser support for `Code(N)`, `Text`, `Guid`, `Int`, `BigInteger`, `Bool`, `Enum(Ident)`, inline `check(...)` / `formula(...)` / `relation(...)`, `tablelogicextension ... for`, `validate field(val: any)` type annotations, `transition` `from:` / `to:` colon syntax, `onTransition(rec)`, and `value(Name)` in `enumextension`. |

---

## Host surface that is not in the kernel

These calls are absent from `QuickJsEngine` and from `ambient.d.ts`. Application packages own them. Completions must not offer them.

- `ctx.postLedger`
- `ctx.dimension`
- A blocking `ctx.ui.confirm` dialog. `confirm(message, defaultAnswer)` records an info toast and returns `defaultAnswer`, or `true` when no default is passed. It does not wait for a person.
- A language server crate, a source-mapped debugger, or a persisted QuickJS bytecode cache.

`ctx.ui.openModal`, `navigate`, `setFieldVisible`, and `setFieldEditable` append a directive object the client can read with `getDirectives()`. They do not render a page by themselves.
