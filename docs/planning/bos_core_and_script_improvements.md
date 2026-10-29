# BOS Core & BOS-Script Architectural Improvements Log

## 1. Purpose & Operating Charter

As business requirements change, new modules are engineered, and enterprise packages (`.bospkg`) are authored, developers actively interface with the compiler (`bosc-syntax`, `bosc-analyzer`), execution runtime (`bosc-runtime`), code generation, and client page kit (`page-kit`).

This document serves as the **mandatory living register** where the package building agent and lead architects log concrete engine improvements, language syntax proposals, and developer experience (DX) enhancements discovered during package authoring and debugging.

---

## 2. Requirement Change & Friction Logging Protocol

Whenever a package developer encounters a runtime trap, silent layout failure, or repetitive boilerplate while implementing a requirement:
1. **Never implement fragile workarounds silently**: Immediately record the underlying platform limitation here.
2. **Structure every proposal**:
   - **ID**: `IMP-XXX` (sequential)
   - **Trigger / Context**: Package, module, or user requirement that exposed the issue.
   - **Affected Subsystem**: Compiler, Runtime, Codegen, Page Kit, or BOS-Script DSL.
   - **Root Cause & Failure Mode**: Why the issue occurred (e.g., scoping, AST limitation, missing warning).
   - **Concrete Architectural Proposal**: Exact syntax, compiler pass, or runtime enhancement proposed.
   - **Priority**: `P0` (Critical runtime trap) | `P1` (High DX / safety) | `P2` (Optimization / Sugar).

---

## 3. Active Improvement Proposals

| ID | Subsystem | Title | Priority | Status |
| :--- | :--- | :--- | :--- | :--- |
| **IMP-001** | `bosc-analyzer` | Compile-time diagnostic for `this.` calls inside codeunits | **P0** | **Implemented & Verified** |
| **IMP-002** | `bosc-analyzer` | Dialog action footer promotion & visibility warnings | **P0** | **Implemented & Verified** |
| **IMP-003** | `bosc-runtime` | Automatic closure injection of `vars` in `wrap_page_script` | **P1** | **Implemented & Verified** |
| **IMP-004** | `page-kit` | Enterprise primary fallback for dialog confirmation handlers | **P1** | **Implemented & Verified** |
| **IMP-005** | `bosc-syntax` / `bosc-analyzer` | First-class `self.Procedure()` syntax sugar in BOS-Script | **P2** | **Implemented & Verified** |
| **IMP-006** | `bosc-runtime` | Zero-mock crypto primitives (`uuidv7`, `hmacSha256`), SSRF protection, state machine bypass guard & outbox backoff | **P0** | **Implemented & Verified** |

---

### IMP-001: Compile-time Diagnostic for `this.` in Codeunits
- **Trigger**: Authentication failure during `AuthManager.Authenticate` calling `this.GetAuthSetup()`.
- **Affected Subsystem**: `crates/bosc-analyzer/src/lib.rs` (`scan_codeunit_this`).
- **Status**: **Implemented & Verified**.
- **Implementation**:
  - Implemented `scan_codeunit_this` in `crates/bosc-analyzer/src/lib.rs`.
  - When any codeunit procedure contains `this.<ident>`, the compiler intercepts it and reports a compiler error `BOS1003`:
    ```
    error[BOS1003]: 'this.GetAuthSetup' is not valid inside codeunit 'AuthManager': 'this' is not bound inside QuickJS codeunits.
    hint: Use 'self.GetAuthSetup(...)' or 'ctx.call("AuthManager", "GetAuthSetup", params)' or direct table operations.
    ```
  - Eliminates silent runtime crashes before packages are deployed.

---

### IMP-002: Dialog Action Footer Promotion & Visibility Warnings
- **Trigger**: Sign-In dialog rendering generic "Cancel" and "OK" fallback buttons because `SignInAction` was placed inside `area(Processing)` without `inFooterBar = true;`.
- **Affected Subsystem**: `crates/bosc-analyzer/src/page_resolver.rs`.
- **Status**: **Implemented & Verified**.
- **Implementation**:
  1. **Automatic Footer Promotion**: If `ctx.page_type.is_dialog()` and an action is designated `primary = true`, `in_footer_bar` automatically infers `true` regardless of whether the action was enclosed in an area.
  2. **Visibility Diagnostic**: If an action on a dialog page is not marked for the footer bar and is not primary, the compiler issues diagnostic warning `BOS3015`:
     ```
     warning[BOS3015]: Action 'X' is inside an area on dialog page 'LoginPage' and will not be displayed; add 'inFooterBar = true' or declare outside an area.
     ```

---

### IMP-003: Automatic Closure Injection of `vars` in `wrap_page_script`
- **Trigger**: `LoginPage.action.SignInAction` script accessing `vars.username` throwing `TypeError: cannot read property 'username' of undefined`.
- **Affected Subsystem**: `crates/bosc-runtime/src/page_artifacts.rs` (`wrap_page_script`).
- **Status**: **Implemented & Verified**.
- **Implementation**:
  - `wrap_page_script` now automatically injects `const vars = (ctx.page && ctx.page.variables) || {};` into the top of the trigger closure:
    ```javascript
    ctx.page = { name: {name}, trigger: {trigger}, variables: {vars} };
    const __bosPageResult = (() => {
        const vars = (ctx.page && ctx.page.variables) || {};
        {body}
    })();
    rec._page_variables = ctx.page.variables;
    rec._ui_directives = ctx.ui.getDirectives();
    return __bosPageResult;
    ```
  - Both `vars.<field>` and `ctx.page.variables.<field>` now resolve natively and reliably.

---

### IMP-004: Enterprise Primary Fallback for Dialog Confirmation Handlers
- **Trigger**: Clicking "OK" in a StandardDialog with no footer actions resulted in a no-op save on unbound forms.
- **Affected Subsystem**: `ui/src/lib/page-kit/PageFrame.svelte`.
- **Status**: **Implemented & Verified**.
- **Implementation**:
  - `confirmDialog()` and `onPrimary` in `PageFrame.svelte` check `footer.find(a => a.primary) || actions.find(a => a.primary)`.
  - When Enter is pressed or a primary action exists, dialog submission always triggers the primary action instead of attempting an unbound table save.

---

### IMP-005: First-Class `self.Procedure()` Syntax Sugar in BOS-Script
- **Trigger**: Repetitive, stringly-typed `ctx.call("AuthManager", "Procedure", params)` boilerplate within the same codeunit.
- **Affected Subsystem**: `crates/bosc-analyzer/src/lib.rs` (`rewrite_codeunit_self`, `rewrite_self_in_script`).
- **Status**: **Implemented & Verified**.
- **Implementation**:
  - Added AST rewriting pass in `bosc-analyzer`:
    - `self.ProcedureName()` lowers to `ctx.call("<CodeunitName>", "ProcedureName")`.
    - `self.ProcedureName(arg1, ...)` lowers to `ctx.call("<CodeunitName>", "ProcedureName", arg1, ...)`.
  - Gives package developers clean, concise, idiomatic OOP syntax sugar with zero runtime overhead.

---

### IMP-006: Runtime Cryptographic Primitives, SSRF Guard, State Machine Bypass Guard & Outbox Backoff
- **Trigger**: Multi-tenant security audit and procurement posting verification.
- **Affected Subsystem**: `crates/bosc-runtime` (`quickjs_engine.rs`, `server.rs`, `auth_guard.rs`, `event_engine.rs`).
- **Status**: **Implemented & Verified**.
- **Implementation**:
  - **Host Crypto Primitives**: Exposed `ctx.crypto.uuidv7()`, `ctx.crypto.randomUUID()`, and `ctx.crypto.hmacSha256(key, data)` to QuickJS sandbox, backed by Rust `uuid` v7 and `sha2` RFC 2104 engine.
  - **SSRF Hardening**: Implemented `is_reserved_or_private_http_host` refusing Loopback (`127.0.0.0/8`, `::1`), Cloud IMDS (`169.254.0.0/16`, `169.254.169.254`, `metadata.google.internal`, `fd00:ec2::254`), and private networks (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fe80::/10`).
  - **State Machine Protection**: Added guard in `Table.update` preventing direct mutations to state-machine-governed `status` fields, enforcing `transition()` execution.
  - **Emergency Super Admin Session Registry**: Added thread-safe in-memory session registry with 15-minute TTL on `BosEngine`, verified automatically by `auth_guard` during maintenance mode without broken database lookups.
  - **Event Outbox Exponential Backoff**: Added `next_retry_at TIMESTAMPTZ` column and exponential backoff ($2^{\text{retry}}$ seconds) in `process_outbox_in` to prevent retry exhaustion during downstream outages.
