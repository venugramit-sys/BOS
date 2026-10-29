# BOS Enterprise Visual Test Suite & Feature Matrix

**Status**: Active Governance Document  
**Specification Reference**: `docs/04_ui_architecture_and_page_compiler.md`, `docs/11_svelte5_ui_studio_and_component_library.md`  
**Target Viewports**: 
- **High-DPI Desktop**: 1920 × 1080
- **Standard Enterprise Laptop**: 1366 × 768
- **Tablet (iPad / Surface)**: 1024 × 768
- **Mobile Handheld**: 375 × 812
**Theme Coverage**: Light Theme & Dark Theme (OLED Black / Zinc)

---

## Feature 1: Enterprise Authentication & System Shell

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-AUTH-01** | `SignIn` / `LoginPage` | Initial landing on unauthenticated route `#/studio`. | 1920x1080, 375x812 (Light & Dark) | Branded hero backdrop renders with zero layout shift; autofocus sits on username input; password reveal toggle operates smoothly; Enter key triggers QuickEntry submission without full-page reload. |
| **VT-AUTH-02** | Login Failure & Error Shake | Enter invalid credentials (`admin` / `wrongpass`). | 1366x768 (Dark) | Red border accent with subtle error vibration; structured toast notification renders with Lucide alert-triangle icon; password field clears and retains focus. |
| **VT-AUTH-03** | Shell Command Palette (`Ctrl+K`) | Trigger `Ctrl+K` from any view. | 1920x1080, 1024x768 | Blurred backdrop overlay renders; fuzzy search input filters pages, actions, and records instantaneously (<16ms); ArrowUp / ArrowDown navigation with active highlight; Escape cleanly closes palette. |
| **VT-AUTH-04** | Theme Switcher & Accent Sync | Toggle between Dark and Light mode from NavUser. | 1920x1080 | Instant CSS variable swap with zero flashing; chart axes, borders, and badge contrasts maintain WAI-ARIA WCAG AAA contrast ratio (>7:1); local storage preserves theme preference across refresh. |
| **VT-AUTH-05** | FastTab Collapse & Expansion | Click header chevrons on General, Policies, and Defaults FastTabs. | 1366x768 (Light) | Smooth height tweening; summary badges appear in collapsed state (e.g. "Status: Active • Currency: USD"); state persists during tab navigation. |

---

## Feature 2: Page Surface & Dynamic Metadata Renderers

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-PAGE-01** | `CustomerList` (List Page) | Open Customer List view. | 1920x1080, 1024x768 (Light) | Sticky table headers with column sorting indicators; numeric columns right-aligned with monospace font; status badges with semantic colors (Green = Active, Slate = Inactive); multi-select checkboxes in leftmost column. |
| **VT-PAGE-02** | `CustomerCard` (Card Page) | Drill down into customer record `CUST-10001`. | 1920x1080, 1366x768 (Dark) | FastTabs layout properly; action bar renders promoted actions (`Post`, `Release`, `Dimensions`) with primary styling; dirty state tracker displays yellow pill when fields mutate; `Ctrl+S` triggers atomic PATCH. |
| **VT-PAGE-03** | FactBox Inspector Pane | Open right-hand FactBox pane on Customer Card. | 1920x1080 | Side pane smoothly docks to the right; calculates real-time customer balance and credit limit FlowFields; zero N+1 queries; collapsing pane expands form area gracefully. |
| **VT-PAGE-04** | Responsive Form Stacking | Resize viewport to 375px mobile width. | 375x812 (Light) | Multi-column field grids collapse into a clean single-column vertical flow; action bar transitions into a bottom sheet drawer with thumb-reachable primary action buttons. |
| **VT-PAGE-05** | StandardDialog Modal | Trigger a dialog action (e.g. `ChangePassword` or `ConfirmPosting`). | 1024x768 (Dark) | Modal centers perfectly with backdrop blur; footer actions promote primary action with auto-focus; Enter key confirms; Escape cancels. |

---

## Feature 3: Number Series Architecture (`NoSeries`)

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-NUM-01** | `NoSeriesList` View | Navigate to Number Series administration page. | 1920x1080 (Dark) | Displays all system series (`CUST`, `VEND`, `SO`, `SI_POST`, `PO`, `PI_POST`); shows active status, starting/ending numbers, and visual progress bar of sequence utilization. |
| **VT-NUM-02** | Sequence Allocation & Lock UI | Open Order Entry and trigger automatic number series allocation. | 1366x768 (Light) | Displays allocated number (e.g. `SO-2026-00042`); shows green checkmark indicating pessimistic lock was cleanly acquired and committed without sequence gap. |
| **VT-NUM-03** | Warning Threshold Badge | Allocate number past the 90% threshold. | 1366x768 (Dark) | Amber warning badge displays on `NoSeriesCard`; toast alerts user that sequence is nearing exhaustion; document creation proceeds unhindered. |

---

## Feature 4: Dimensions & Analytical Combinations

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-DIM-01** | `DimensionList` & Hierarchy | Navigate to Dimensions Setup. | 1920x1080 (Light) | Displays dimensions (`DEPARTMENT`, `PROJECT`, `AREA`, `COSTCENTER`); tree view expands dimension value nodes with clean indentation levels (0, 1, 2). |
| **VT-DIM-02** | Document Line Dimension Picker | Click "Dimensions" action on a Sales / Purchase Order Line. | 1366x768 (Dark) | Flyout modal allows multi-dimension key-value tagging; auto-deduplicates conflicting dimension codes; displays calculated canonical SHA-256 set hash. |
| **VT-DIM-03** | Blocked Dimension Warning | Assign a blocked dimension value to a line. | 1366x768 (Light) | Visual red warning icon appears next to the input; submit button disables with tooltip explaining that the dimension value is blocked for posting. |

---

## Feature 5: Document Posting & Ledger Exploration

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-POST-01** | Purchase Order Posting Action | Click "Post Invoice" on an Approved Purchase Order. | 1920x1080 (Dark) | Full 3-phase posting progress indicator renders; displays deferred number series allocation; creates `PurchInvHeader`, lines, and `VendorLedgerEntry`; redirects to Posted Invoice with success toast. |
| **VT-POST-02** | Immutable Ledger Grid | Open `VendorLedgerEntries` list page. | 1920x1080 (Light) | Displays posted financial entries; inline editing and delete buttons are disabled; immutable padlock badge appears in header; audit hash chain link is inspectable. |
| **VT-POST-03** | SIFT Balance FlowField Dynamic Filter | Filter ledger entries by date range in FactBox. | 1366x768 (Dark) | FlowField instantly recalculates balance using Index-Only SIFT scan; displays exact decimal formatting with thousands separators and two decimals. |

---

## Feature 6: Visual Studio Designers & DX Powerhouse

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-DSGN-01** | Visual Page Designer | Open Page Designer for `CustomerCard`. | 1920x1080 (Dark) | WYSIWYG canvas displays interactive drag-and-drop hierarchy; left panel lists fields & controls; right property panel displays live AST property inspector with instant canvas synchronization. |
| **VT-DSGN-02** | State Machine Visual Canvas | Open State Machine Designer for `DocumentWorkflow`. | 1920x1080 (Dark) | Directed graph renders states (`Draft`, `Pending`, `Approved`, `Posted`, `Closed`) with bezier curves; transitions display required role pills; clicking a node selects it in the properties pane. |
| **VT-DSGN-03** | Monaco BOS-Script IDE | Open Monaco code editor on `PurchasePost` codeunit. | 1920x1080 (Dark) | Full syntax highlighting; IntelliSense auto-completes `ctx.crypto.uuidv7()`, `Table.lock()`, and `NoSeriesManagement`; diagnostics underline syntax errors in real-time. |
| **VT-DSGN-04** | Live Schema Drift Inspector | Open Schema Drift modal. | 1366x768 (Light) | Compares AST definitions vs live PostgreSQL catalog; displays green additions, yellow modifications, red warnings; 1-click "Synchronize" previews DDL before execution. |

---

## Feature 7: Multi-Company & Tenant Switching

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-TEN-01** | Company Switcher Dropdown | Click company selector in top navigation bar. | 1920x1080 (Light) | Lists accessible companies; selecting a new company triggers seamless RLS context switch; active tab records refresh without session termination. |
| **VT-TEN-02** | Cross-Company Data Isolation | Verify customer list in Company A vs Company B. | 1920x1080 (Dark) | Zero cross-company record leakage; company-specific number series and ledger accounts remain strictly isolated. |

---

## Feature 8: Error UX & Concurrency Conflict Resolution

| Test ID | Feature / Component | Use Case & Action Flow | Target Viewports | Acceptance Criteria |
| :--- | :--- | :--- | :--- | :--- |
| **VT-ERR-01** | Concurrency Conflict Modal | Simulate simultaneous modification on the same record version. | 1366x768 (Light) | Friendly diff dialog opens showing "Your changes" vs "Server version"; allows user to merge, overwrite, or discard without data loss. |
| **VT-ERR-02** | Offline / Disconnected Toast | Disconnect network or stop backend node. | 1366x768 (Dark) | Top reconnection banner appears with pulsing red dot; queued offline changes are preserved locally; auto-reconnects when network restores. |
