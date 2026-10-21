# BOS UI Messaging & Toast Notification Specification

## 1. Executive Summary & Market Leader Benchmarking

Enterprise ERP and CRM platforms require non-modal, immediate contextual feedback for asynchronous operations, business trigger side-effects, validation warnings, and long-running codeunit execution. Traditional web applications often rely on disruptive modal dialogs (`alert()`, `confirm()`), which block user workflows and degrade operational efficiency.

BOS incorporates a full-stack, enterprise-grade **UI Messaging and Toast Notification System** inspired by market leaders:

| Enterprise Platform | Mechanism | Strengths | BOS Adaptation |
| :--- | :--- | :--- | :--- |
| **Microsoft Dynamics 365 Business Central (AL)** | `Message()`, `Error()`, `Notification.Send()` | Non-modal notification bar with action links (`AddAction`) | `ctx.ui.toast(...)` with optional action buttons (`action: { label, url }`) and non-blocking execution |
| **SAP Fiori / UI5** | `MessageToast.show()`, `MessageBox` | Transient floating toasts with customizable duration, auto-dismiss, and theme adherence | Stackable floating cards (top-right), pause-on-hover, custom auto-dismiss durations |
| **Salesforce Lightning (LWC)** | `ShowToastEvent` (`variant`, `mode: dismissible/pester/sticky`) | Standardized severity levels (`info`, `success`, `warning`, `error`) and structured titles | Four standard levels, dismissible flags, unread counter badge, persistent Notification Center history |
| **Modern Web (Sonner / Radix UI / Svelte 5)** | Reactive multi-toast stack, reactive runes, ARIA compliance | Smooth transitions, stacking order, accessibility | Svelte 5 `$state` and `$derived` runes, keyboard shortcuts, notification drawer |

---

## 2. BOS-Script Ambient API Specification

Within any BOS-Script context (Table Triggers e.g. `onInsert`, `onModify`, `onDelete`, `onValidate`, or Codeunit procedures), the runtime injects `ctx.ui` into scope, along with global aliases `ui`, `toast`, and `message`.

### 2.1. API Surface

```typescript
declare interface BosUiToastAction {
    label: string;
    action?: string;
    url?: string;
    codeunit?: string;
    method?: string;
}

declare interface BosUiToastOptions {
    title?: string;
    message?: string;
    text?: string;
    type?: "info" | "success" | "warning" | "error";
    duration?: number;      // In milliseconds (default: 5000ms, error: 7000ms, 0: sticky)
    dismissible?: boolean;  // Whether close button is displayed (default: true)
    action?: BosUiToastAction;
}

declare interface BosUiMessage {
    id: string;
    type: "info" | "success" | "warning" | "error";
    title: string;
    message: string;
    duration: number;
    dismissible: boolean;
    timestamp: string;
    action: BosUiToastAction | null;
}

declare interface BosUiContext {
    toast(options: BosUiToastOptions): BosUiMessage;
    toast(message: string, optionsOrTitle?: BosUiToastOptions | string): BosUiMessage;
    message(message: string, type?: "info" | "success" | "warning" | "error"): BosUiMessage;
    success(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    warning(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    error(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    info(message: string | BosUiToastOptions, titleOrOpts?: string | BosUiToastOptions): BosUiMessage;
    getMessages(): BosUiMessage[];
    clearMessages(): void;
}
```

### 2.2. BOS-Script Usage Examples

#### Quick Informational Message
```typescript
ctx.ui.info("Tax recalculation applied using state rate of 8.25%");
```

#### Success Toast with Custom Header
```typescript
ctx.ui.success("Sales order SO-1002 posted to general ledger", "Posting Succeeded");
```

#### Warning Toast with Action Button
```typescript
ctx.ui.warning({
    title: "Credit Limit Exceeded",
    message: `Customer ${rec.customerName} has reached 95% of approved limit ($${rec.balance}/$${rec.creditLimit})`,
    duration: 8000,
    action: {
        label: "Review Credit Risk",
        url: "/finance/credit-review?cust=" + rec.customerNo
    }
});
```

#### Global Shortcut Syntax
```typescript
// Shorthand syntax available directly in global scope
toast("Order released", "success");
message("Background sync scheduled");
```

---

## 3. End-to-End Architectural Pipeline

```
  +---------------------------------------------------------------+
  | QuickJS Sandboxed Engine (crates/bosc-runtime)                |
  |   ctx.ui.toast(...)                                           |
  |     -> __host_ui_toast (FFI to Rust tracing log)              |
  |     -> rec._system_messages.push(toastObj)                    |
  +---------------------------------------------------------------+
                                 |
                                 v
  +---------------------------------------------------------------+
  | PostgreSQL Isolation Layer (crates/bosc-runtime/src/lib.rs)  |
  |   insert_record_with_messages / update_record                 |
  |   - Strips _system_messages from physical SQL column lists    |
  |   - Returns (id, messages) to server callers                  |
  +---------------------------------------------------------------+
                                 |
                                 v
  +---------------------------------------------------------------+
  | Axum REST Engine (crates/bosc-runtime/src/server.rs)          |
  |   create_entity / update_entity / execute_codeunit_handler     |
  |   - Serializes response with "messages": [ ... ]              |
  +---------------------------------------------------------------+
                                 |
                                 v
  +---------------------------------------------------------------+
  | Svelte 5 Frontend Studio (ui/src/App.svelte)                  |
  |   executeRequest() auto-unpacks res.messages                  |
  |   - addToast(msg) pushes to floating top-right stack (max 5)  |
  |   - notificationHistory records past session alerts (max 60)  |
  |   - Header Bell badge displays unread notification counter    |
  |   - Notification Center Drawer filters by severity level      |
  +---------------------------------------------------------------+
```

---

## 4. Database Safety Invariants

1. **Physical Schema Non-Invasiveness**: `_system_messages` and `_messages` are transient UI metadata properties and are **never** stored into database column tables. Both `insert_record` and `update_record` strictly filter these identifiers before generating SQL `INSERT INTO` or `UPDATE SET` statements.
2. **Deterministic OCC Isolation**: Optimistic Concurrency Control (`_system_version`) operates solely on actual table fields; UI messages do not alter OCC version checks.
3. **Rollback Resilience**: In transactional codeunit failures or trigger validation errors (`throw new Error(...)`), database writes are rolled back while error messages are cleanly surfaced to the client toast system.
