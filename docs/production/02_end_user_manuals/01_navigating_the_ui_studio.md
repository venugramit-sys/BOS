# Volume 2: End-User & Operator Manuals
## 01. Navigating the UI Studio

The BOS User Interface is engineered in **Svelte 5** to provide a fast, responsive, and intuitive workspace for day-to-day business operations, data management, and application testing.

---

## 🖥️ Screen Layout Overview (shadcn `sidebar-08` Inset Architecture)

When you open `http://localhost:5173` (or port `3000` in production), the screen features a modern enterprise **Inset Sidebar (`sidebar-08`)** layout engineered in Svelte 5 runes:

```
┌──────────────────────────────────────────────────────────────────────────────────────────────────┐
│ [Company Header]         │ [ ⊞ Sidebar Toggle ]                     [ Search (⌘K) ] [Bell]       │
│                          ├───────────────────────────────────────────────────────────────────────┤
│ [NavMain: Role Menu]     │ [Tab: Role Center] [Tab: Users ✕]                                     │
│   PLATFORM               ├───────────────────────────────────────────────────────────────────────┤
│   - Role Center          │                                                                       │
│   - Databases            │                                                                       │
│   IDENTITY & ACCESS      │                      Main Content Viewport                            │
│   - Users                │                  (Dynamic Page Kit View: List/Card)                   │
│   - Roles                │                                                                       │
│                          │                                                                       │
│ [NavSecondary: Help]     │                                                                       │
│   - Documentation        │                                                                       │
│                          │                                                                       │
│ [NavUser: Avatar ▾]      │                                                                       │
└──────────────────────────┴───────────────────────────────────────────────────────────────────────┘
```

### 1. Inset Navigation Sidebar (`SidebarNav.svelte`)
Adheres to the official `sidebar-08` design pattern with zero-mock role menus and off-canvas collapsing:
- **`NavSwitcher` (Header)**: Displays the active company name and enterprise subtitle. The interactive company selector appears **only when multiple companies exist** in the system; with a single company, it renders a clean, static header without redundant dropdown controls.
- **`NavMain` (Role-Tailored Navigation)**: Dynamically resolves and groups primary menu entries based on the authenticated user's assigned roles (`UserRole` -> `SUPER`, `Admin`, `User`). Menus are compiled directly from declarative `menu` meta-objects registered in `_bos_meta_objects` without hardcoded dummy navigation trees.
- **`NavSecondary` (Utility Rails)**: Pinned lower utility navigation providing rapid access to system documentation, diagnostic monitors, and support tickets.
- **`NavUser` (Footer User Profile)**: Displays the active user avatar, initials, username, email, and security role badge (`SUPER`, `Admin`, `User`). Features a popover menu with:
  - **Complete Theme Selector**: Immediate 3-state switching between Light, Dark, and System modes.
  - **Dev Studio Navigation**: One-click navigation to `/studio` for all object designers, database management, and development tools.
  - **Log Out**: Secure session invalidation.
- **Full Hide on Collapse**: When collapsed (`Ctrl + B` or toggle button), the sidebar transitions completely off-canvas to `w-0` with zero footprint, freeing 100% of the screen width for workspace operations.

### 2. Elevated Main Inset Canvas (`SidebarInset`)
- **Inset Header & Sidebar Trigger**: Features a minimalist header with the `PanelLeft` toggle button (`Ctrl + B`), universal search (`⌘K`), and notification center. (Telemetry, health pills, and debug status bars are stripped from the business application shell).
- **Multi-Tab Workspace (`WorkspaceTabs`)**: Keeps multiple operational views, cards, and reports open simultaneously with unsaved state tracking and close buttons.
- **Dynamic Page Kit Viewport**: Renders compiled declarative pages (`List`, `Card`, `Document`, `Dashboard`) driven directly by BOS meta-object schemas with real-time REST backend synchronization. When no documents are open, a distraction-free clean canvas is shown.

---

## ⌨️ Essential Keyboard Shortcuts

Increase your daily efficiency with enterprise productivity shortcuts:

| Shortcut | Action | Where Applicable |
| :--- | :--- | :--- |
| `Ctrl + K` / `Cmd + K` | Universal Command Palette & Page Search | Anywhere in the application |
| `Ctrl + S` / `Cmd + S` | Save current record / Save draft | Inside Card or Document forms |
| `Alt + N` | Create a new record (Insert) | On any List page |
| `Ctrl + B` | Toggle Navigation Sidebar | Anywhere in the application |
| `Escape` | Close active modal, drawer, or cancel edit | Anywhere in the application |
| `F5` / `Ctrl + R` | Refresh live data grid without page reload | Data Lists and Grids |
| `Enter` | Open selected row into Card view | Data Lists |
| `Tab` / `Shift + Tab` | Move to next / previous input field | Data Entry Forms |

---

## 🎨 Theme Customization

BOS supports two primary enterprise themes:
- **Dark Mode (Default)**: Engineered with slate and zinc backgrounds to minimize eye strain during long operational shifts.
- **Light Mode**: High-contrast, clean white and neutral gray palette ideal for daylight office environments and printing.

Click the Sun/Moon icon in the top right header to toggle your preference instantly. Preferences are remembered across sessions in browser local storage.
