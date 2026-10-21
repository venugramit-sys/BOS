# Page and Page Extension Design

> **Status:** Design revised — awaiting product-owner approval  
> **Document:** `docs/28_page_meta_object_design.md`  
> **Meta-objects:** `page`, `pageextension`  
> **Freeze step:** 05 in `docs/01_bos_script_must_have_features_and_meta_objects.md`  
> **Audience:** Store owners, accountants, and the developers who will build the renderer  
> **This document is a design.** It freezes the language, the stored shape, and the runtime contract. It does not contain production Rust or Svelte implementation code.

A page is the contract for one screen. It names **one source**, picks **one layout**, and then stores only what the table does not already know: which fields to show, which sections to put them in, and which buttons to offer. One renderer, `PageHost`, draws every screen from that contract using the existing `bos-ui` library.

This revision replaces the earlier draft that copied the Business Central page object (`sourceTable` / `sourceView` / `sourceQuery`, `pageType`, `area(Content)`, `area(FactBoxes)`, and a second Promoted action tree). Those ideas are mixed into a shorter document. The capabilities stay. The duplicate storage goes.

---

## 1. What a page is, in plain English

Think of a page as a form that already knows its filing cabinet.

- **Where the data comes from.** One `source` line. `Customer` is a file you can write. `query.SalesByCustomer` is a question you can only read.
- **How it is arranged.** One `layout` word: a list, a single record, a home screen, a set of steps, a board, a split, an API field list, or a free arrangement.
- **What is stored.** The field's type, default caption, and lookup columns already live on the table. The page stores the field name and only the exceptions (a different caption, locked, hidden, badge).
- **What the user can do.** One command list. The important button is marked `primary`. The command calls a codeunit or a status change. The posting math stays in the codeunit.

A page extension is a sticker: an extra field, an extra button, a hidden fax number. The sticker never rewrites the original form.

Names stay singular PascalCase: `Customer`, `SalesHeader`, `CustomerCard`, `SalesOrder`. The package is the namespace.

---

## 2. Why the stored page is shorter

Business Central stores the same fact several times. A document page has a page type that already means "header plus lines," a content area that wraps the only content, a line part that is a second page of type ListPart, and a Post button that is declared once under Processing and again as a promoted reference.

BOS keeps the useful habits and stores each fact once.

| Habit worth keeping | Where it came from | How BOS stores it |
| :--- | :--- | :--- |
| Sections that collapse to a one-line summary | Business Central FastTabs | `section` inside `main` |
| A side panel of related facts | Business Central FactBox, Salesforce sidebar | the `side` slot |
| Lines that belong to a header | Business Central document lines, Salesforce related list | an inline `part` with its own `source` and `link` |
| A few important buttons on the top strip | Business Central promoted actions, Fiori footer | `primary: true` on a command |
| A home screen of counts | Business Central Role Center cues, Fiori overview cards | `layout: home` and `tile` |
| Named saved filters | Business Central views, Fiori filter bar | a `views` block |
| A partner adds a field without editing the base file | Business Central page extension, Salesforce dynamic forms | `pageextension` anchors |
| The table already knows the caption and the type | Salesforce dynamic forms | the page stores overrides only |
| Drag a card, a calendar, a history, a tree | Modern work boards plus Fiori lists | `layout: board` plus one `arrange` word |
| A list beside the open record | Fiori flexible column, Salesforce console | `layout: split` plus `card` |
| A field list for another program, with no screen | Business Central API page | `layout: api` |

What is no longer stored:

- Three source properties. There is one `source`.
- A page type plus a layout tree that repeats it. There is one `layout`.
- `ListPart`, `CardPart`, and `HeadlinePart`. A part is a nested fragment or a reference to a normal page.
- `area(Content)`, `area(FactBoxes)`, `area(Processing)`, `area(Navigation)`, `area(Reporting)`, and `area(Promoted)`.
- A second copy of a button just to promote it.
- A caption copied from the table when the author did not change it.

A sales order that took a page type, two areas, a child ListPart page, and a promoted action reference now stores one record layout, one inline line part, and one primary command.

---

## 3. Design principles

1. **One source property.** A page, a tile, and an inline part each have at most one `source`. The compiler records the kind (`table`, `view`, or `query`) inside that value. Authors do not set `sourceTable`, `sourceView`, or `sourceQuery`.
2. **Store the delta.** Unchanged captions, types, relations, and DropDown columns are read from the table, view, or query at render time. The page file and the saved meta-object keep only the overrides.
3. **Eight layouts, composed.** A document is a record with a line part. A journal is a list with cell editing. A dialog is a step list with one step. A kanban, calendar, timeline, or tree is a board with an `arrange` word.
4. **One component kit.** The renderer uses `ui/src/lib/bos-ui/`. Missing pieces are added there in Svelte 5 runes and compose the controls that already exist. `PageHost` does not import `ui/src/lib/components/enterprise/`.
5. **One renderer.** `PageHost` reads the merged compact document and mounts bos-ui controls. There is no hand-written Svelte page per customer card.
6. **Extensions never edit the base page.** Anchors name a section, a field, a part, or a command.
7. **Read models stay read-only.** A view or a query can fill a screen. Insert, modify, and delete go only to tables, through the existing save path, which runs `tablelogic`.
8. **Commands call codeunits.** Page triggers may warn and may call a codeunit. They do not post ledgers.
9. **The runtime you already have.** Rows, saves, codeunits, and transitions use the Axum routes in `bosc-runtime`. Page scripts run in the QuickJS sandbox (8 MB RAM, 100k fuel).
10. **Permissions still decide.** A button does not grant a right.

---

## 4. The single `source` property

```bos
source: Customer;
source: table.Customer;
source: view.CustomerBalance;
source: query.SalesByCustomer;
```

| Written form | Rule |
| :--- | :--- |
| `Customer` | The compiler looks for a table, then a view, then a query with that name. One match wins. |
| `table.Customer` | The kind is explicit. The name must be a table. |
| `view.CustomerBalance` | Read-only. |
| `query.SalesByCustomer` | Read-only. |

If `Customer` is both a table and a view, the compiler stops and asks the author to write `table.Customer` or `view.Customer`. That is still one property.

The saved document stores the resolved value once:

```json
"source": { "name": "Customer", "kind": "table" }
```

`sourceTable`, `sourceView`, and `sourceQuery` are not properties of the language. If a file contains them, the compiler reports: use `source`. The header `page CustomerCard for Customer` is accepted only as a fold into that same `source`. It is not a second stored field. New pages write `source` inside the body. The designer emits `source` only.

A node has zero or one `source`.

- A list, record, board, split, sheet-style list, and API page name the page `source`.
- `layout: home` may omit the page `source`. Each `tile` and each `part` names its own.
- An inline `part` names the child `source`. A part that references another page does not name a source; the referenced page already has one.

Setting two sources on the same node is impossible because there is only one property. Qualifying two different objects on one page is a compile error.

### Writes

Writes exist only when all of these are true:

- The resolved kind is `table`.
- The layout may write, and the author has not turned the verb off.
- The user has that right on the table.
- The field is editable. FlowFields and masked-as-read-only fields refuse keystrokes.

| Layout | Default writes |
| :--- | :--- |
| `list` | Insert, modify, and delete when `editable: true`. Default `editable` is false for a plain list and true when `editing: cell`. |
| `record` | Insert, modify, delete. |
| `board` with `arrange: kanban` | Modify (moving a card). Insert and delete stay off unless the author turns them on. |
| `board` with `arrange: calendar` | Modify (moving to another day). |
| `board` with `arrange: timeline` or `tree` | Timeline is read-only. Tree allows modify. |
| `split` | The list follows the list rules. The card page on the right follows that page's own rules. |
| `steps` | Modify when the dialog edits an existing row. Finish calls the command. Insert and delete stay off. |
| `home`, `api` | No writes from the screen. An API client uses the entity routes under the same rights, and only for fields listed on the page. |
| `free` | Modify only, until the author sets `insertAllowed` or `deleteAllowed`. |

An author may tighten a default (`modifyAllowed: false` on a posted document). An author may not turn on writes when the kind is `view` or `query`.

### Delayed insert

A blank row is not stored because the user opened New.

- **Record.** The form stays in memory until Save. Closing it leaves the table unchanged.
- **Inline lines and `editing: cell`.** `delayedInsert: true` is the default. The row is stored when the user leaves the first field that has a value. An empty extra row is discarded.

The first real save is `POST /api/entities/{table}`. Later edits are `PATCH`. `tablelogic` runs on that save.

### Links

A line part filters the child source with equality pairs. Each pair is one `link` line:

```bos
part(Lines) {
    source: SalesLine;
    link: documentId = id;
    link: lineType = lineType;
}
```

Meaning: show `SalesLine` rows whose `documentId` equals this header's `id`, and whose `lineType` equals this header's `lineType`. The pairs are AND. The renderer adds them to the existing list filter. It does not invent a join API.

`link` on a side part means the same thing: reload and filter when the parent field changes.

### FlowFields, FlowFilters, and lookups

A FlowField is calculated by the database and is always read-only on the page. The renderer asks for it on load and again after a successful save or command.

A FlowFilter is a question in the margin. "Balance as of 31 March" must not be written onto the customer. The page declares it, the user sets it in the filter bar, and it travels as `flowFilter` on the entity read. Clearing it shows the unfiltered total again.

```bos
filters {
    filter(asOf) {
        caption: "Balance as of";
        appliesTo: balance;
    }
}
```

A relation field uses `Combobox`. The dropdown columns are the table's `DropDown` field group. Search calls the lookup projection in section 15. Quick-create is allowed only when the user can insert into that table.

---

## 5. Canonical syntax

Properties use `name: value` or `name = value`. Examples use the colon form. Both store the same property.

The page body has these blocks. Each is optional. Empty blocks are omitted from the saved document.

| Block | Stores |
| :--- | :--- |
| `main` | Sections, fields, grids, and parts of the working area |
| `side` | Parts shown in the factbox |
| `commands` | The button list |
| `tiles` | Count and amount tiles. Used by `layout: home` |
| `steps` | Ordered steps. Used by `layout: steps` |
| `regions` | Named regions. Used by `layout: free` |
| `filters` | FlowFilters |
| `views` | Named filter and sort presets |
| `onOpen`, `onAfterGetRecord`, `onNewRecord`, `onQueryClose` | Page scripts |

A `layout: list` may place `field` lines directly on the page. They belong to the single repeater. That avoids wrapping them in a section.

### 5.1 List

```bos
page CustomerList {
    source: Customer;
    layout: list;
    caption: "Customers";
    card: CustomerCard;

    field(code);
    field(name);
    field(city);
    field(balance) { editable: false; }
    field(status) { style: badge; }

    views {
        view(Active) {
            caption: "Active customers";
            filter: "status=Active";
            sort: "name";
        }
    }

    commands {
        command(New) {
            caption: "New";
            primary: true;
            shortcut: "Ctrl+N";
            open: CustomerCard;
            mode: create;
        }
    }
}
```

`field(balance)` stores `{ "name": "balance", "editable": false }`. The caption "Balance" comes from the table. `style: badge` is stored because it is an override.

### 5.2 Record, including a document

A document is not its own layout. It is a record whose `main` block contains an inline part.

```bos
page SalesOrder {
    source: SalesHeader;
    layout: record;
    caption: "Sales Order";

    main {
        section(General) {
            field(orderNo) { editable: false; }
            field(customerId) { caption: "Customer"; }
            field(orderDate);
            field(status) { style: badge; }
        }
        part(Lines) {
            source: SalesLine;
            link: documentId = id;
            field(itemId);
            field(quantity);
            field(amount) { editable: false; }
        }
    }

    side {
        part(Customer; CustomerFact);
    }

    commands {
        command(Post) {
            caption: "Post";
            group: Posting;
            primary: true;
            shortcut: "Alt+P";
            visible: status == "Released";
            confirm: "Post this sales order?";
            run: SalesPost.post;
        }
        command(OpenCustomer) {
            caption: "Customer";
            group: Go;
            open: CustomerCard;
            link: id = customerId;
        }
    }
}
```

`part(Customer; CustomerFact)` stores a reference, `{ "kind": "part", "name": "Customer", "page": "CustomerFact" }`, because that factbox is reused. The lines are inline because they belong to this document only. Reuse them later by moving the part into its own `layout: list` page and switching the document to `part(Lines; SalesLineList)`.

`columns: 2` on a section places fields side by side on a desktop and stacks them under 768px. The saved section stores `columns` only when it is not 1.

### 5.3 Journal

A journal is a list that edits cells. It is not a worksheet page type.

```bos
page ItemJournal {
    source: ItemJournalLine;
    layout: list;
    editing: cell;
    caption: "Item Journal";

    field(itemId);
    field(quantity);
    field(locationCode);
}
```

### 5.4 Home

```bos
page SalesHome {
    layout: home;
    caption: "Sales";

    tiles {
        tile(OpenOrders) {
            caption: "Open Orders";
            source: query.OpenSalesOrders;
            tone: warning;
            drill: SalesOrderList;
        }
    }

    main {
        part(Recent; SalesOrderList);
    }
}
```

The page has no `source`. The tile has one. The embedded list page has its own.

### 5.5 Steps

Several steps are a wizard. One step is a dialog. The renderer uses `Wizard` for more than one step and `Dialog` or `ConfirmDialog` for one step.

```bos
page ShipConfirm {
    source: SalesHeader;
    layout: steps;
    caption: "Ship";

    steps {
        step(Check) {
            caption: "Check";
            field(shipDate);
            field(locationCode);
        }
        step(Confirm) {
            caption: "Confirm";
            field(orderNo) { editable: false; }
        }
    }

    commands {
        command(Finish) {
            caption: "Post Shipment";
            primary: true;
            run: SalesPost.ship;
        }
    }
}
```

### 5.6 Board

`arrange` picks the bos-ui component. The rows still come from the one page `source`.

```bos
page OrderBoard {
    source: SalesHeader;
    layout: board;
    arrange: kanban;
    columnField: status;
    card: SalesOrder;

    field(orderNo);
    field(customerId);
    field(amount);
}
```

| `arrange` | Required field | Component |
| :--- | :--- | :--- |
| `kanban` | `columnField` | `Kanban` |
| `calendar` | `dateField` | `Calendar` |
| `timeline` | `dateField` | `Timeline` |
| `tree` | `parentField` | `Tree` |

If the `columnField` is owned by a state machine, a drag is a transition. Otherwise a drag saves that field. That rule is open decision 3, and the recommendation is to follow the state machine when one exists.

### 5.7 Split, API, and free

```bos
page CustomerWorkspace {
    source: Customer;
    layout: split;
    card: CustomerCard;

    field(code);
    field(name);
    field(city);
}
```

The left pane is the list. The right pane is `CustomerCard` for the selected row. `CustomerCard.source` must resolve to the same table. The analyzer rejects a card page pointed at a different table.

```bos
page CustomerApi {
    source: Customer;
    layout: api;

    field(code);
    field(name);
    field(city);
}
```

Nothing is drawn. A caller can read and write only these fields, and only through the entity routes.

```bos
page WarehouseFloor {
    source: Shipment;
    layout: free;
    caption: "Warehouse Floor";

    regions {
        region(Map) {
            control(DockMap) { component: "WarehouseMap"; }
        }
        region(Queue) {
            part(Waiting) {
                source: ShipmentLine;
                link: shipmentId = id;
                field(dock);
                field(status);
            }
        }
    }
}
```

`control` names a component shipped with a package. An unknown name renders `EmptyState`. The page does not accept raw HTML. That is open decision 7, and it is the recommendation.

### 5.8 A view or a query

```bos
page TopCustomers {
    source: query.SalesByCustomer;
    layout: list;
    caption: "Sales by Customer";

    field(customerName);
    field(amount);
}
```

Cells are locked. A command may still call a codeunit that writes tables.

### 5.9 Saved shape

The compiler stores this document and drops every default. Scripts are stored on the trigger, not copied into a second action tree.

```json
{
  "name": "SalesOrder",
  "source": { "name": "SalesHeader", "kind": "table" },
  "layout": "record",
  "caption": "Sales Order",
  "main": [
    {
      "kind": "section",
      "name": "General",
      "fields": [
        { "name": "orderNo", "editable": false },
        { "name": "customerId", "caption": "Customer" },
        { "name": "orderDate" },
        { "name": "status", "style": "badge" }
      ]
    },
    {
      "kind": "part",
      "name": "Lines",
      "source": { "name": "SalesLine", "kind": "table" },
      "link": [["documentId", "id"]],
      "fields": [
        { "name": "itemId" },
        { "name": "quantity" },
        { "name": "amount", "editable": false }
      ]
    }
  ],
  "side": [
    { "kind": "part", "name": "Customer", "page": "CustomerFact" }
  ],
  "commands": [
    {
      "name": "Post",
      "caption": "Post",
      "group": "Posting",
      "primary": true,
      "shortcut": "Alt+P",
      "visible": "status == \"Released\"",
      "confirm": "Post this sales order?",
      "run": { "codeunit": "SalesPost", "method": "post" }
    }
  ]
}
```

The merged route in section 15 returns this shape after extensions and after the permission set has removed fields the caller cannot read.

---

## 6. Layouts

Eight values of `layout`. New pages use these words. The renderer maps each value to bos-ui components.

| `layout` | What the user sees | bos-ui | Composes into |
| :--- | :--- | :--- | :--- |
| `list` | Many rows, search, sort, filter. `card` opens one row. `editing: cell` is a journal | `Repeater` (`DataTable`, or `DataGrid` when editing cells), `FilterBar`, `ActionBar` | The line list inside a record, by using the same repeater on an inline part |
| `record` | One row, sections, optional lines, optional side facts | `Hero`, `FastTab`, field editors, `Repeater`, `FactBox`, `Path` | A document, when `main` contains a part. A factbox body, when this page is referenced from `side` |
| `home` | Tiles plus embedded pages | `Cue`, `Hero`, `part` | The role home. A headline is the hero, not a separate page |
| `steps` | One or more steps, then a finish command | `Wizard`, or `Dialog` / `ConfirmDialog` when there is one step | A wizard and a prompt |
| `board` | The same rows arranged by a field | `Kanban`, `Calendar`, `Timeline`, or `Tree`, chosen by `arrange` | Kanban, calendar, history, and chart-of-accounts |
| `split` | The list on the left, the `card` page on the right | `SplitPane`, `DataTable`, nested `PageHost` | Master and detail |
| `api` | No screen. The field list is the contract | Nothing drawn | Integrations |
| `free` | Named regions | `PageHost` regions, `control`, `part`, fields | A dock map, a custom floor |

`mobileBreakpoint` and `tabletBreakpoint` default to 768 and 1024, from `docs/21`. A page stores them only when it overrides them.

| Width | What changes |
| :--- | :--- |
| Under 768px | One column. Repeaters become cards. The `primary` command is the bottom button. `side` stays closed until the user taps its badge, then it opens as a sheet. Sections are one scrolling column with jump links. |
| 768px to 1024px | Sections in two columns. `side` opens as a slide-over. Commands stay in a toolbar. |
| Above 1024px | Sections flow in up to three columns. `side` is a docked panel about 300px wide. `split` uses `SplitPane`. |

Density comes from `themeStore` and applies at every width.

A third column (list, document, and lines at once) is out of this freeze. `split` is two panes.

---

## 7. Controls

Every control's properties sit in four groups: Identity, Data, Behavior, and Events. The page designer does not add a fifth group.

| Group | What the person is deciding |
| :--- | :--- |
| **Identity** | The name, the caption, and the kind of control |
| **Data** | Which source field, link, or command outcome it uses |
| **Behavior** | Whether it can be seen, edited, required, or emphasized |
| **Events** | The small scripts in section 9 |

### 7.1 Fields, by table type

The table type chooses the bos-ui control. The page stores `style` only when the author changes the default.

| Table field type | Default control | `style` the page may store | Notes |
| :--- | :--- | :--- | :--- |
| `String`, `Code` | `Input` | `text` | `Code` sets uppercase. `assistEdit: true` adds a `Button` for number series |
| Long `String`, or `style: memo` | `Textarea` | `memo` | Notes |
| `Integer`, `Decimal` | `Input` (numeric, in-field math) | `number`, `currency` | Currency formats on blur. No second currency widget |
| `Boolean` | `Switch` | `switch`, `checkbox` | Checkbox uses bos-ui `Checkbox` |
| `Date`, `Time`, `DateTime` | `DatePicker` | `date`, `datetime` | Date formulas stay inside `DatePicker` |
| `Enum` / `Option` | `Select` | `select`, `badge` | `badge` is the read-only color on a list. Editing still uses `Select` |
| Relation | `Combobox` | `lookup` | Columns from the `DropDown` field group |
| `FlowField` | The control of the result type, locked | same as the result | Never editable |
| `Media` / `Blob` | `Attachment` | `file` | One file. Galleries are later |
| `Json` | `Textarea` | `json` | A structured editor is later |

`importance: promoted` pulls the field into the hero and into the collapsed section summary. The default importance is not stored.

`visible` and `editable` accept a boolean or an expression (`status == "Active"`, `balance > creditLimit`, combined with `and` / `or`). Expressions are a screen convenience. The server permission check still runs on save.

### 7.2 Layout nodes

| Node | bos-ui | Stored when |
| :--- | :--- | :--- |
| `section` | `FastTab` | Always, with `name`. `caption` only if it differs from the name. `columns` only if it is not 1 |
| `column` | A stack inside a section | The two sides are uneven |
| `part` inline | Nested `Repeater` or nested `PageHost` | It has its own `source`, `link` lines, and field overrides |
| `part` reference | Nested `PageHost` | It has `page`. It must not also have `source` |
| `field` | Section 7.1 | The name, plus overrides |
| `tile` | `Cue` | `caption`, its own `source`, `drill`, and `tone` when the tone is not neutral |
| `step` | One page of `Wizard` | `caption` and fields |
| `region` | A named slot in `free` | Its children |
| `control` | The registered component, or `EmptyState` | `component` name |
| `filter` | `FilterBar` | `appliesTo` and a caption |
| `view` | A `FilterBar` preset | `caption`, `filter`, `sort` |

The hero is not a node the author must write. `PageHost` builds it from the record title, the status field, and fields marked `importance: promoted`. An author who needs a different title field sets `titleField` and `statusField` on the page. Those two properties are stored only when set.

### 7.3 Commands

A command has one outcome. Setting two of `run`, `transition`, `open`, `query`, and `report` is a compile error.

| Group | Properties |
| :--- | :--- |
| Identity | `name`, `caption`, `image`, `tooltip` |
| Data | `run` (codeunit.procedure), `transition`, `open` (a page), `link` lines, `mode` (`navigate`, `modal`, `create`), `query`, `report` |
| Behavior | `group`, `primary`, `shortcut`, `visible`, `enabled`, `confirm`, `scope` (`record`, `selection`, `page`) |
| Events | `onAction` |

`group` is the menu label, such as Posting or Go. It is not a second page and not an action area. Commands with the same group sit together in the menu. `primary: true` places that command on the toolbar. The first primary command is the phone's main button. There is no Promoted block and no `actionRef`.

---

## 8. Commands on the screen

```bos
commands {
    command(Post) {
        caption: "Post";
        group: Posting;
        primary: true;
        shortcut: "Alt+P";
        visible: status == "Released";
        confirm: "Post this sales order?";
        scope: record;
        run: SalesPost.post;
    }
    command(Print) {
        caption: "Print";
        group: Report;
        report: SalesOrderReport;
    }
}
```

| Concern | Rule |
| :--- | :--- |
| Menu | `ActionBar` clusters commands by `group` inside `Menubar` |
| Toolbar | Every `primary` command is a `Toolbar` `Button`. One of them uses the filled style |
| Phone | The first `primary` command is the bottom button. The rest sit in a More sheet built with `Dialog` |
| Shortcut | Shown with `Kbd`. A shortcut that collides inside one page is a compile error |
| Hidden and disabled | Hidden means absent. Disabled means shown and grey, with the reason in `tooltip` |
| Confirmation | `confirm` opens `ConfirmDialog` before the call |
| Codeunit | `run: SalesPost.post` becomes `POST /api/codeunits/SalesPost/run` with `method: "post"` and the current record id |
| State machine | `transition: Release` uses the edge whose action is `Release` and whose `from` is the current status. The page does not restate the from/to matrix |
| Open | `open` plus `link` lines. `mode: modal` uses `Dialog`. `mode: create` opens the card unsaved |
| Selection | `scope: selection` receives the checked ids. The list shows the batch bar when two or more rows are checked and at least one command has that scope |
| Palette | `CommandPalette` lists the same commands. It does not keep a second list |

`runReport` stores the report name. Printing itself belongs to the report meta-object, freeze step 08.

---

## 9. Page triggers that are allowed

Page scripts are for the screen. They run in the QuickJS sandbox. They may call codeunits. They may read the current record. They do not post ledgers.

| Trigger | Where | Allowed to | Stays out |
| :--- | :--- | :--- | :--- |
| `onOpen` | Page | Set a starting filter, such as "this month" | Inserting rows |
| `onAfterGetRecord` | Page | Adjust a caption from the row just read | Writing the row |
| `onNewRecord` | Page | Propose values on a blank card, such as order date = work date | Inserting the row |
| `onQueryClose` | Page | Ask when the card is dirty | Silent discard of a posted document |
| `onValidate` | Field | A fast message when the user leaves the cell | Replacing `tablelogic`. The server result wins |
| `onLookup` | Field | A custom chooser when `Combobox` is not enough | Bypassing permission on the lookup table |
| `onAssistEdit` | Field | Ask the number-series codeunit for the next number | Inventing a number in the page |
| `onDrillDown` | Field | Open the list behind a FlowField | Editing the ledger from the drill |
| `onAction` | Command | Extra UI around the click. The lasting work is still `run` or `transition` | Posting math |

`onInsert`, `onModify`, and `onDelete` are not page triggers. Those words belong to `tablelogic`. The compiler rejects them on a page and points the author at `tablelogic` or a codeunit.

The page's `onValidate` on discount can say "over 20% needs a manager" as soon as the clerk tabs out. On Save, `tablelogic` checks the rule again. A caller who skips the page and uses the API is still refused. Posting the invoice is the Post command, which calls `SalesPost.post`.

---

## 10. Page extension

Anchors name a section, a field, a part, a command, a tile, or a step. The caption is not the anchor.

| Anchor | Use |
| :--- | :--- |
| `addFirst(Place)` / `addLast(Place)` | Insert at the start or end of `main`, `side`, `commands`, a section, or a step |
| `addBefore(Name)` / `addAfter(Name)` | Insert beside a field, section, part, or command |
| `moveFirst` / `moveLast` / `moveBefore` / `moveAfter` | Rearrange by name, including a control an earlier extension added |
| `modify(Name)` | Change stored overrides: caption, tooltip, visible, editable, importance |
| `hide(Name)` | Sugar for `modify(Name) { visible: false; }` |

An extension that drops a field into `main` with no anchor is a compile error. Extensions apply in package dependency order. Two extensions must not add a control with the same name.

```bos
pageextension CustomerCardLoyalty extends CustomerCard {
    main {
        addAfter(Financials) {
            section(Loyalty) {
                caption: "Loyalty";
                field(loyaltyPoints) { editable: false; }
                field(tier) { style: badge; }
            }
        }
        modify(balance) {
            tooltip: "Includes open invoices and unapplied payments.";
        }
    }

    commands {
        addLast(commands) {
            command(EvaluateTier) {
                caption: "Evaluate Tier";
                group: Posting;
                run: LoyaltyService.evaluate;
            }
        }
    }
}
```

The base file is unchanged. Uninstalling the package removes the section and the button.

```bos
pageextension CustomerCardHideFax extends CustomerCard {
    main {
        hide(faxNo);
    }
}
```

The column remains on the table. It is absent from this screen for everyone who runs this package. One user's choice to hide a column is personalization, not this.

`for` is accepted as an alias of `extends` on the extension header. The stored target name is the same either way.

---

## 11. Personalization and design-time customization

| Who | What they change | Where it is stored | Survives a package upgrade? |
| :--- | :--- | :--- | :--- |
| Developer or partner | Fields, commands, captions, visibility for every user | `pageextension` in a package | Yes. It is reapplied onto the new base page |
| Profile administrator | The shared starting view for a role | A `view` block, or a published view record if open decision 1 allows it | Yes, when it is in the package. Admin-published views are data |
| End user | Column order, hidden columns, width, pinning, page size, personal filters | `PagePersonalization`, cached in the browser | Yes. Keyed by user, page, and control |
| End user | Density, surface, radius, light or dark, work date | `themeStore`, hydrated from `UserPersonalization` | Yes |

`DataTable` already caches column layout under `bos.table.{stateKey}`. For a page, `stateKey` is `{pageName}.{controlName}`, for example `CustomerList.rows`. The source of truth is `PagePersonalization` so the same clerk sees the same columns on the next computer.

Cascade:

1. Base page, already in the compact shape.
2. Page extensions in package order, applied to that shape.
3. The shared view the user picked.
4. That user's `PagePersonalization`.
5. `themeStore` tokens on every bos-ui control.

A user cannot personalize their way into a field an extension hid, or a field their permission set cannot read.

---

## 12. Security

The page is a view onto rights that already exist.

| Check | When | What the user sees if it fails |
| :--- | :--- | :--- |
| Execute on the page | Opening the page | The page does not open |
| Read on the source | Loading rows | An empty state. No row payload is sent |
| Insert, modify, delete | New, edit, delete | The button is absent. A direct API call still returns `{ "error": "..." }` |
| Field read | Building the field list | The field is omitted from the merged document and from the JSON |
| Field modify false, read true | Drawing the field | The control is visible and locked |
| Field masked | Drawing the field | bos-ui `Input` masking. The raw value is not in the payload |
| Execute on the codeunit, or the transition's role guard | Drawing and running the command | The command is omitted. The route checks again |
| Row filter | Every list, part, and lookup | Added on the server. The page cannot remove it |

`layout: api` uses the same field list and the same rights. An integration cannot read a field left off the page, and it cannot read a field the caller's permission set excludes.

Client-side `visible` expressions are not a security check.

---

## 13. Must-have feature list

Fifty-two features are in this freeze. Each one is a sentence a business user can approve, then the check that proves it.

### Data

1. A page, a tile, and an inline part each have at most one `source`. **Acceptance:** the analyzer rejects `sourceTable`, `sourceView`, `sourceQuery`, and a second `source` on the same node.
2. An unqualified name resolves to one object. A name that matches two kinds must be written `table.Name`, `view.Name`, or `query.Name`. **Acceptance:** a colliding name is an error that names both kinds; `query.SalesByCustomer` stores `{ "name": "SalesByCustomer", "kind": "query" }`.
3. When the kind is `table`, the user can add, change, and delete only if the layout allows it and their permission set allows it. **Acceptance:** a user with read-only on `Customer` can open the record and cannot save.
4. When the kind is `view` or `query`, the screen is read-only. **Acceptance:** `PATCH` is not called from that page, and the cells render locked.
5. A new record or a new line is not stored until the user has typed something and saved or left the row. **Acceptance:** opening New and closing it inserts zero rows.
6. An inline part shows only the child rows that match its `link` lines. **Acceptance:** sales order 1001 does not show lines whose `documentId` is 1002.
7. A calculated total is shown and cannot be typed over. **Acceptance:** customer balance is present after load and the input is read-only.
8. A FlowFilter changes the total without writing the filter onto the record. **Acceptance:** setting `asOf` changes the displayed balance and `GET` of the customer shows the same stored columns as before.
9. A lookup shows the columns in the DropDown field group. **Acceptance:** the customer combobox headers match that field group.
10. Lists, parts, and lookups page, sort, and filter through the existing entity, view, and query routes, with limit clamped from 1 to 1000 and the `x-total-count`, `x-page`, `x-limit`, and `x-has-more` headers on entity lists. **Acceptance:** a list request uses those routes and those headers.
11. When two people save the same row, the second person sees that the row changed and their save is refused. **Acceptance:** the existing optimistic-concurrency response is shown in a toast, and the form reloads the current row.
12. A list can ship named views. **Acceptance:** choosing "Active customers" sends that filter and sort on the next read.

### Storage

13. The saved page is the compact document in section 5.9: one `source`, one `layout`, and the slots that are actually used. **Acceptance:** the meta-object JSON has no `pageType`, no `sourceTable`, and no `area` nodes.
14. A field stores its name and only the overrides. Caption, type, and relation come from the source at render time. **Acceptance:** `field(name)` with no body is stored as `{ "name": "name" }`, and the label on screen is the table caption.
15. A document is a `record` layout with an inline part, not a separate page type and not a second ListPart page. **Acceptance:** `SalesOrder` has `layout: "record"` and a `main` part whose `source` is `SalesLine`.
16. A journal is a `list` with `editing: cell`, not a worksheet type. **Acceptance:** `ItemJournal` stores `layout: "list"` and `editing: "cell"`.
17. A home screen omits the page `source` when every tile and part names its own. **Acceptance:** `SalesHome` has no page-level `source`, and the Open Orders tile stores one `source`.
18. A kanban, calendar, timeline, and tree share `layout: board` and differ by `arrange`. **Acceptance:** four screens store the same layout value and four arrange values.
19. A wizard and a one-step dialog share `layout: steps`. **Acceptance:** two steps render `Wizard`; one step renders `Dialog`.
20. A reused factbox stores a page reference. A one-off line list stores an inline part. **Acceptance:** `part(Customer; CustomerFact)` has `page` and no `source`; the line part has `source` and no `page`.

### Layout and controls

21. `main`, `side`, and `commands` are the three slots a record uses. **Acceptance:** the sales order factbox is the `side` slot, and there is no FactBoxes area in the file.
22. A section renders as a FastTab that can collapse to a one-line summary of its promoted fields. **Acceptance:** collapsing General hides the inputs and still shows the customer name.
23. `side` docks on the right on a desktop and becomes a sheet on a phone. **Acceptance:** at 1920px the panel is visible; at 375px it is closed until the badge is tapped.
24. An empty list shows an empty state with the primary command. **Acceptance:** zero rows render bos-ui `EmptyState` and the New button.
25. A record shows a hero with the title, the status, and the promoted fields. **Acceptance:** the sales order hero shows the order number and the status badge without an author-written hero node.
26. `columns: 2` places fields side by side at desktop width and stacks them under 768px. **Acceptance:** city sits beside post code at 1920px and stacks at 375px.
27. Each table field type uses the bos-ui control in section 7.1. **Acceptance:** a date field mounts `DatePicker`, a relation mounts `Combobox`, a boolean mounts `Switch`.
28. Visibility and editability can depend on the current record. **Acceptance:** `visible: status == "Released"` hides Post when the status is Draft.
29. A repeater is a grid on a desktop and one card per line on a phone. **Acceptance:** the same sales lines render `DataTable` at 1920px and cards at 375px.
30. A tile shows a count or amount and opens its `drill` page. **Acceptance:** Open Orders shows the query result and navigates to `SalesOrderList`.
31. A kanban board shows one column per status and moves a card by the state machine or by saving that field. **Acceptance:** dragging an order to Released calls the transition route when a state machine owns `columnField`.
32. A calendar places records on `dateField`. **Acceptance:** an order dated 15 October appears on that day.
33. A timeline shows events in time order. **Acceptance:** approval history for one document renders as `Timeline`.
34. A tree shows parent and child rows from `parentField`. **Acceptance:** expanding an account group reveals its children.
35. Cell editing includes F8 copy-above, because `DataGrid` and `Input` already do that. **Acceptance:** editing quantity issues `PATCH` for that field only.
36. When the source table has a state machine, a record shows the status path. **Acceptance:** `Path` steps match the machine's states.
37. `layout: split` shows the list and the `card` page of the selected row. **Acceptance:** choosing a customer shows `CustomerCard` in the right pane, and a card page whose source is a different table is a compile error.
38. `layout: api` publishes the listed fields and draws no screen. **Acceptance:** there is no `PageHost` view, and a client can read the field contract from the merged document.
39. `layout: free` renders named regions, parts, and a named user control. **Acceptance:** an unknown `component` shows `EmptyState`.

### Commands

40. Commands live in one list. `group` labels the menu, and `primary` pins the toolbar. **Acceptance:** the saved command has `primary: true` and there is no Promoted area and no `actionRef`.
41. The first primary command is the phone's main button. **Acceptance:** Post is a toolbar button at 1920px and the bottom button at 375px.
42. A command can require a confirmation sentence before it runs. **Acceptance:** Post opens `ConfirmDialog` and does not call the codeunit if the user cancels.
43. A command runs a codeunit, a transition, or opens a page, query, or report. **Acceptance:** the request hits `/api/codeunits/{name}/run` or `/api/entities/{table}/{id}/transition`, and the page source contains no ledger writes. Two outcomes on one command are a compile error.
44. A shortcut is shown with `Kbd`. **Acceptance:** Alt+P runs Post when the page is focused and no dialog is open.
45. The command palette lists the same commands. **Acceptance:** searching "Post" runs the same codeunit call.
46. Checking two or more rows surfaces selection-scoped commands. **Acceptance:** a `scope: selection` command receives the selected ids and is hidden when nothing is selected.

### Extensibility

47. A page extension can add controls and commands at the start, the end, before, or after a named anchor. **Acceptance:** the loyalty section renders after Financials and the base file is unchanged.
48. A page extension can move or modify an existing control, or hide it. **Acceptance:** `hide(faxNo)` omits the field from the merged document and the table column remains.
49. Extensions apply in package order and address controls by name. **Acceptance:** a second package can move a control the first package added.

### Responsive behavior, theme, and security

50. Layout shifts at 768px and 1024px unless the page stores other breakpoints. Density, surface, radius, theme, and work date come from `themeStore`. Column order is stored per page and per control, separate from the theme. **Acceptance:** changing density does not reset `CustomerList.rows`.
51. Opening a page checks execute permission. Reading rows checks read permission. A field the user cannot read is omitted. A field they cannot change is locked. A masked field stays masked. A command is omitted when the user cannot run it. Row filters apply to every list, part, and lookup. **Acceptance:** a muting set that removes read on `creditLimit` drops that field from the merged document and from the JSON.

### Studio and runtime

52. The page designer reads and writes the compact syntax. The property sheet for a page control has exactly Identity, Data, Behavior, and Events. Choosing an event opens that trigger in the editor. Preview loads real metadata and live rows through the routes in section 15. **Acceptance:** a round trip of `SalesOrder` emits `source`, `layout`, `main`, `side`, and `commands`, and `parse_page` accepts it. Preview of `CustomerList` shows database rows.

---

## 14. Component gap list

### Already in bos-ui

`Menubar`, `MenuItem`, `Input`, `Combobox`, `DatePicker`, `themeStore` / `surface`, `Button`, `Badge`, `Checkbox`, `Switch`, `Select`, `Textarea`, `Field`, `Tabs`, `Dialog`, `ConfirmDialog`, `Banner`, `Toast`, `Tooltip`, `Kbd`, `Spinner`, `EmptyState`, `Toolbar`, `StatusBar`, `ContextMenu`, `CommandPalette`, `SplitPane`, `Tree`, `DataGrid`, `DataTable`, `Datasheet`.

`DataTable` covers list grids. `DataGrid` covers simple cell editing. `Datasheet` is the desktop journal and studio record sheet, including the table designer's field list. One row is one record. Enter is Quick Entry, Tab visits every column, and F8 copies the cell above. F4 and Alt+Down open a lookup. Alt+N inserts a line, Ctrl+Delete deletes one, and Page Up and Page Down move a screen. Date words use the page work date (`w` is that date, `t` is today). Each clerk can hide, reorder, freeze, and choose which columns Enter stops on by right-clicking a heading; that layout is remembered for them. A column marked keep-visible, such as a posting date that differs by line, cannot be hidden. The sheet has no formula bar. F3 opens the current column’s filter, and Alt+F3 filters to the current value. `editable` may be a function of the row, so a cell such as string length stays locked on a decimal field. The table designer uses that sheet for its field list and keeps field order fixed. Filters accept `10..20`, `<>`, and `A|B`. Required fields and attention rules show as issues and do not trap the cursor. Alt+Up reads the reason. The sheet tells the page which row is selected, and a calculated column can open the records behind it. `Combobox` is the lookup. `DatePicker` keeps date formulas. `Input` keeps math evaluation, masks, and F8 copy-above. `SplitPane` is the split layout. `Tree` is `arrange: tree`. `Dialog` and `ConfirmDialog` are the one-step layout. `CommandPalette`, `Menubar`, `Toolbar`, and `Button` are the command chrome.

### New components, all under `ui/src/lib/bos-ui/`

Each one is Svelte 5 runes (`$state`, `$derived`, `$props`, `$effect`) and composes the pieces named here.

| Component | Path | Composes | Props | Used by |
| :--- | :--- | :--- | :--- | :--- |
| `FastTab` | `bos-ui/fast-tab/FastTab.svelte` | `Button`, `Badge` | `caption`, `summary`, `open`, `children` | `section` |
| `FactBox` | `bos-ui/fact-box/FactBox.svelte` | `Button`, `Badge`, `Dialog`, `EmptyState` | `title`, `count`, `open`, `children` | `side` |
| `Repeater` | `bos-ui/repeater/Repeater.svelte` | `DataTable`, `DataGrid`, `Field`, `Button`, `EmptyState`, `Dialog` | `columns`, `rows`, `editable`, `delayedInsert`, `editing`, `totals`, `onCommitCell`, `onDraftRow`, `onDeleteRow` | `list`, inline parts |
| `Cue` | `bos-ui/cue/Cue.svelte` | `Button`, `Badge` | `caption`, `value`, `unit`, `tone`, `onDrill` | `tile` |
| `Wizard` | `bos-ui/wizard/Wizard.svelte` | `Dialog`, `Button`, `Banner`, `Kbd` | `steps`, `current`, `onBack`, `onNext`, `onFinish`, `finishLabel` | `steps` |
| `Hero` | `bos-ui/hero/Hero.svelte` | `Badge`, `Toolbar`, `Button` | `title`, `subtitle`, `status`, `statusTone`, `facts` | record and home, built by `PageHost` |
| `FilterBar` | `bos-ui/filter-bar/FilterBar.svelte` | `Toolbar`, `Field`, `Input`, `Select`, `DatePicker`, `Combobox`, `Badge`, `Button` | `filters`, `activeView`, `views`, `onChange` | Page FlowFilters and views. Column filters stay on `DataTable` |
| `Kanban` | `bos-ui/kanban/Kanban.svelte` | `Badge`, `Button`, `EmptyState`, `Dialog` | `columns`, `cards`, `onMove` | `arrange: kanban` |
| `Calendar` | `bos-ui/calendar/Calendar.svelte` | `Button`, `Badge`, `Dialog`, `EmptyState` | `events`, `view`, `onOpen`, `onRangeChange` | `arrange: calendar` |
| `Timeline` | `bos-ui/timeline/Timeline.svelte` | `Badge`, `Button` | `items` | `arrange: timeline` |
| `Path` | `bos-ui/path/Path.svelte` | `Button`, `Badge` | `steps`, `onAdvance` | Record whose table has a state machine |
| `Attachment` | `bos-ui/attachment/Attachment.svelte` | `Button`, `Banner`, `EmptyState` | `fileName`, `accept`, `maxBytes`, `onUpload`, `onClear` | `Media` and `Blob` fields |
| `ActionBar` | `bos-ui/action-bar/ActionBar.svelte` | `Menubar`, `Toolbar`, `Button`, `Dialog`, `Kbd`, `Badge` | `commands`, `onRun` | Every layout except `api` |

`PageHost` is the renderer at `ui/src/lib/page-runtime/PageHost.svelte`. It is not a second kit. Phone cards inside `Repeater` use `Field` and the same editors.

---

## 15. Runtime contract

All row and command traffic uses routes that already exist, plus the four additions marked **Add**. Additions extend the entity query, the schema payload, and the meta routes. They do not add a second query language.

### 15.1 Load the screen

`GET /api/meta/pages/{name}` **Add.** Returns the merged compact document for the current tenant and the caller's packages. The shape is section 5.9 plus:

- `insertAllowed`, `modifyAllowed`, `deleteAllowed`, and `delayedInsert` after the layout defaults and the author's overrides
- `extensions`: the package names that were applied
- `stateMachine`: the machine name when the source table has one
- `card`, `arrange`, `editing`, `columnField`, `dateField`, or `parentField` only when the page set them

Scripts are not inlined as posting code. A command carries `run`, `transition`, `open`, `query`, or `report`.

`GET /api/meta/objects/{name}` still returns one stored object. Studio opens the base page and each extension separately. The renderer calls the merged route.

`GET /api/entities/{table}/schema` **Add** `canRead`, `canModify`, and `masked` on each column for the current user.

Personalization uses the existing entity list: `GET /api/entities/PagePersonalization?filter=...`. The client also reads `bos.table.{stateKey}` and reconciles. Until that table exists, the browser cache still works. Columns are `userName`, `pageName`, `controlName`, and `layoutDelta` (hidden columns, order, pinning, widths, page size, personal filter).

### 15.2 Fetch rows

| Source kind | Existing route | Response |
| :--- | :--- | :--- |
| `table` | `GET /api/entities/{table}?fields&limit&offset&page&sort&filter` | `PaginatedRecords` and the `x-total-count`, `x-page`, `x-limit`, `x-has-more` headers |
| One row | `GET /api/entities/{table}/{id}` | The record |
| `view` | `GET /api/views/{name}?filter&order_by&limit&offset` | `{ success, view, count, data }` |
| `query` | `POST /api/queries/{name}/run` with `{ "params": {} }` | `{ success, query, count, data }` |
| Lookup | `GET /api/entities/{table}/lookup?q&limit&sort` **Add** | `{ items, total_count, field_group }`. Same query engine as the entity list. Default limit 50, maximum 200 |

**Add** `flowFilter` on the entity query, a JSON object in the same shape as `filter`. The runtime passes it into FlowField evaluation and does not persist it.

A part's `link` lines become equalities inside `filter`. A permission row filter is applied on the server before the page filter. View calls keep the existing `order_by` parameter name.

### 15.3 Save a field

1. The user leaves the cell or presses Save.
2. A new row uses `POST /api/entities/{table}`.
3. An existing row uses `PATCH /api/entities/{table}/{id}` with the changed fields only.
4. The runtime runs `tablelogic` and returns the saved row or `{ "error": "..." }`.
5. The renderer shows a field error or a toast (`docs/19_ui_messaging_and_toast_spec.md`). On success it refreshes FlowFields.
6. Delete is `DELETE /api/entities/{table}/{id}`.
7. A blank new row that the user discards never calls POST.

### 15.4 Run a command

| Outcome | Route | Body |
| :--- | :--- | :--- |
| `run` | `POST /api/codeunits/{name}/run` | `{ "method": "post", "params": { "id": "<record id>", "ids": [] } }` |
| `transition` | `POST /api/entities/{table}/{id}/transition` | The existing `TransitionRequest`. The renderer fills `from_state` and `to_state` from the machine and the current status |
| `open` | Client navigation | The target page is loaded through 15.1 |
| `query` | The query route above | |
| `report` | Stored only. Freeze step 08 performs the print | |

Permission is checked again inside those handlers. A fuel or script error becomes a toast, not a partial post.

### 15.5 Session context

Every call already sends the tenant, company, user, and roles headers the engine reads in `extract_context`. The work date travels as a field default from `themeStore.workDate` on `onNewRecord`. It is not a new header in this freeze.

---

## 16. Phased milestones

No step here is implemented by this design task.

### M1. Parser and analyzer — `rust-developer`

**Depends on:** the current `parse_page` and `PageDef` in `crates/bosc-syntax`.

**Files:** `crates/bosc-syntax/src/parser.rs`, `crates/bosc-syntax/src/ast.rs`, `crates/bosc-analyzer`, `crates/bosc-ir`.

**Work:**

- Parse `source` as one property. Accept `Name`, `table.Name`, `view.Name`, and `query.Name`. Store `{ name, kind }` after the catalog lookup.
- Reject `sourceTable`, `sourceView`, and `sourceQuery` with a diagnostic that names `source`.
- Fold `page X for Name` into that same `source`. Do not store a second field.
- Parse `layout` as one of the eight values. Reject `pageType` with a diagnostic that names `layout`.
- Parse `main`, `side`, `commands`, `tiles`, `steps`, `regions`, `filters`, and `views`.
- Parse inline parts (`source` plus `link` lines plus fields) and page references (`part(Name; Page)`).
- Parse `command` with one outcome. Reject a second outcome.
- Store field overrides only.
- Reject page-level `onInsert`, `onModify`, and `onDelete`.
- Reject an extension child that has no anchor.
- Reject writes when the resolved kind is `view` or `query`.
- Reject a `split` card page whose source table differs from the list.

**Acceptance:** the examples in section 5 parse to the compact tree, and the negative cases are errors with a line number.

**Verification:** `cargo test` in `bosc-syntax` and `bosc-analyzer`.

### M2. Merged page and lookup — `rust-developer`

**Depends on:** M1 and the existing meta and entity routes in `crates/bosc-runtime`.

**Files:** `crates/bosc-runtime/src/server.rs`, `crates/bosc-runtime/src/lib.rs`.

**Work:**

- `GET /api/meta/pages/{name}` returns the merged compact document.
- Add `flowFilter` and thread it into FlowField evaluation.
- Add `GET /api/entities/{table}/lookup`.
- Add `canRead`, `canModify`, and `masked` on schema columns.
- Do not add a second CRUD API.

**Acceptance:** a page with one extension returns the extension's fields in order; lookup returns only DropDown columns; a FlowFilter changes a FlowField and not the stored row.

**Verification:** `cargo test` for merge order, lookup limits (default 50, max 200), and a forbidden user receiving `{ "error": "..." }`.

### M3. bos-ui components — `svelte-developer`

**Depends on:** the current `ui/src/lib/bos-ui` exports. Can proceed in parallel with M1 using fixture props.

**Files:** the paths in section 14, plus `ui/src/lib/bos-ui/index.js`.

**Work:** build the thirteen components as Svelte 5 runes components that compose the named bos-ui pieces.

**Acceptance:** each component renders from props with no hard-coded business rows, no legacy `$:` reactivity, and a clean effect teardown.

**Verification:** the component showcase mounts each one. Check `Repeater`, `FactBox`, and `ActionBar` at 375px and 1920px.

### M4. The one renderer — `svelte-developer`

**Depends on:** M2 and M3.

**Files:** `ui/src/lib/page-runtime/PageHost.svelte` and a loader beside it. Studio preview reuses this host from `PageDesigner.svelte`.

**Work:**

- Map the eight layouts to section 6.
- Bind fields with section 7, filling captions from the schema.
- Wire load, save, and commands to section 15.
- Apply the breakpoints and `themeStore`.
- Keep property-sheet groups as Identity, Data, Behavior, and Events.
- Teach the designer to emit the compact syntax.

**Acceptance:** `CustomerList`, `SalesOrder`, `ItemJournal`, `SalesHome`, `TopCustomers`, and `ShipConfirm` render from metadata only.

**Verification:** browser pass at 375, 768, and 1920, including save, a refused validation, a confirmed command, and a factbox sheet at phone width. No production path may ship a fixture dataset.

### M5. Sample package — `package-developer`

**Depends on:** M1. Full visual proof waits on M4.

**Files:** a package under `packages/` containing the pages in section 5, the loyalty `pageextension`, and table `PagePersonalization`.

**Work:** author the `.bos` sources only. No mock rows.

**Acceptance:** `bosc package build` succeeds, table names are singular PascalCase, and the extension file does not modify the base page file.

**Verification:** the build registers the pages through the existing package install route.

### M6. Proof — `tester`, after M4 and M5

**Depends on:** M4 and M5.

**Work:** add the page cases to `docs/18_qa_test_task_matrix.md` and run the API and browser checks named in M2 and M4.

**Acceptance:** the fifty-two checks in section 13 have a test or an explicit browser note. Failures return `{ "error": "..." }` and do not panic.

**Verification:** the tester's report. This design does not mark that report done.

---

## 17. Conflicts this revision resolves

| Earlier text | Decision in this revision |
| :--- | :--- |
| The first draft of this file, and `docs/01_bos_script_specification.md`, used `sourceTable`, `sourceView`, and `sourceQuery` | One property, `source`. The kind is stored inside the resolved value |
| Those docs used `pageType` with seventeen Business Central names (`Card`, `ListPart`, `RoleCenter`, `Worksheet`, and the rest) | Eight layouts. Documents, journals, wizards, dialogs, and boards are compositions |
| Samples used `layout { area(Content) ... }`, `actions { area(Processing) }`, and `area(Promoted)` plus `actionRef` | `main`, `side`, and `commands`. `primary: true` replaces the promoted copy |
| `part Lines: Page.X` and `subPageLink: { documentId: id }` | `part(Lines) { source: SalesLine; link: documentId = id; }` or `part(Lines; SalesLineList)` |
| `docs/22` specifies `GET /api/v1/data/:tableName/lookup` | Lookup is `GET /api/entities/{table}/lookup` |
| `docs/21` places FastTab and factbox under `ui/src/lib/components/enterprise/` | New controls are created in `bos-ui` |
| `docs/20` lists property tabs General, Data, Layout, Appearance for some designers | A page control uses Identity, Data, Behavior, and Events |
| Volume 4 drops `field(created_at)` into an extension with no anchor | An anchor is required |
| `pageextension ... for Target` versus `extends` | `extends` is canonical. `for` is the same stored target |

Older samples in `docs/01_bos_script_specification.md`, `docs/01_bos_script_must_have_features_and_meta_objects.md`, and `docs/BOS-Script/04_Workflows_Security_Analytical_and_UI_MetaObjects.md` are updated to this syntax so the catalog matches the freeze.

### Open decisions that need a product-owner answer

The recommendation under each decision is what M1–M5 will use if you answer "go with the recommendation."

**1. Who may publish a view for the whole company?**

Recommendation: any user may save a personal view into `PagePersonalization`. Only a profile administrator may publish a view that other people see, and that published view is data, not a page extension.

Quick Example: A clerk saves "My overdue invoices" for themselves. The office manager publishes "Overdue over $500" for the sales team.

Real-World Business Example: At a hardware store, the cashier wants the list to open on today's walk-in orders, while the owner wants every salesperson to start on unpaid invoices for the current month.

**2. On a phone, does the side panel open by itself?**

Recommendation: it stays closed. A badge shows how many facts are waiting. The user taps the badge.

Quick Example: Opening a customer on a phone shows the name and the phone number. Tapping "Balance" opens the side panel.

Real-World Business Example: A delivery driver checking a customer on a phone needs the address first. A credit panel that covers the address would slow the drop-off.

**3. What are the columns of a kanban board?**

Recommendation: if the table has a state machine on `columnField`, the columns are that machine's states and a drag is a transition. Otherwise a drag saves the field.

Quick Example: A sales-order board uses Draft, Released, and Posted from the state machine. A small task board with no state machine uses High, Normal, and Low.

Real-World Business Example: A warehouse board follows the pick-status state machine, so a card cannot skip a required check. A repair shop's "waiting / in progress / done" board is just a status list, because there is no approval path.

**4. Which fields does an API layout expose?**

Recommendation: only the fields written on the page. A column that exists on the table and is absent from the page is absent from the payload.

Quick Example: The customer API lists code, name, and city. Credit limit exists on the table and is not in the payload.

Real-World Business Example: A web shop should receive the item name and the selling price. It should not receive the vendor cost.

**5. When is a brand-new record actually inserted?**

Recommendation: the record waits for Save. Lines and cell-edited rows are stored when the user leaves the first filled cell. Opening New and then closing it writes nothing.

Quick Example: New Customer shows a blank card. Nothing is stored until Save.

Real-World Business Example: A salesperson opens New Customer, gets a phone call, and closes the tab. The customer list must not gain a blank customer number.

**6. If the page and the table logic both check a field, which answer wins?**

Recommendation: the page `onValidate` may warn immediately. On save, `tablelogic` runs on the server and that result wins.

Quick Example: The page warns that a credit limit over 50,000 looks high. The table logic refuses anything over the company policy of 25,000 when Save is pressed.

Real-World Business Example: A cashier sees at once that a discount over 20% needs a manager. If someone bypasses the screen and calls the API, the server still refuses the discount.

**7. What may a custom control contain?**

Recommendation: a named component shipped with a package. An unknown name renders `EmptyState`. The page does not accept raw HTML.

Quick Example: `control(DockMap) { component: "WarehouseMap"; }` shows that component. A typo shows "This section is not available."

Real-World Business Example: A logistics company wants a dock-door map on the shipment screen. The map is a packaged component, so a faulty add-on cannot rewrite the rest of the shipment screen.

---

## 18. Explicit non-goals

- A second component library, or page rendering through `ui/src/lib/components/enterprise/`.
- Hand-written Svelte pages for individual business screens.
- Copying the Business Central page object: three source properties, seventeen page types, content and factbox areas, or a promoted action tree.
- Page triggers that post ledgers, create journal lines, or replace `tablelogic`.
- Writable views or writable queries.
- Editing the base page file from an extension.
- Business Central analysis mode, matrix pages, and report-preview page types. Grouping inside `DataTable` covers "slice this list."
- Report printing itself. A command may name `report`. Freeze step 08 performs the print.
- A three-pane split. Two panes are in scope.
- Sparklines, micro charts, and a free-form chart designer.
- Formula columns typed onto a page at runtime.
- Copying a parent field onto a child page beyond what a FlowField, a view, or a side part already shows.
- Multi-language caption packs. This freeze stores one `caption` override. `UserPersonalization.language` continues to drive formats.
- A new data protocol or a `/api/v1/data` alias.
- Raw HTML regions.
- Replacing `themeStore` with a per-control theme.
- Personalization that reveals a field an extension hid or a permission set forbids.
- Mock rows in the renderer or the sample package.

---

## Approval checklist

Mark yes or no. "Go with the recommendation" on the seven open decisions counts as yes for those items.

| # | Decision | Yes | No |
| :---: | :--- | :---: | :---: |
| 1 | A page stores one `source`, one `layout`, and only the field and command overrides. | | |
| 2 | `sourceTable`, `sourceView`, `sourceQuery`, and `pageType` are not properties. The kind lives inside `source`. | | |
| 3 | The eight layouts are list, record, home, steps, board, split, api, and free. A document, a journal, a wizard, and a dialog are compositions of those layouts. | | |
| 4 | Views and queries are read-only. Writes go to tables through the existing save path and `tablelogic`. | | |
| 5 | Commands call codeunits and state-machine transitions. `primary` replaces a second promoted action tree. | | |
| 6 | Page extensions use anchors and never edit the base page. | | |
| 7 | Personal column layout is separate from developer extensions and from `themeStore`. | | |
| 8 | New visual pieces are the thirteen bos-ui components in section 14. | | |
| 9 | Runtime traffic stays on the existing entity, view, query, codeunit, and transition routes, plus the merged page route, lookup, `flowFilter`, and schema rights. | | |
| 10 | The fifty-two must-have features in section 13 are the catalog. Section 18 is out of scope. | | |
| 11 | Open decision 1: users save personal views; a profile administrator publishes a shared view. | | |
| 12 | Open decision 2: on a phone the side panel stays closed until the user taps it. | | |
| 13 | Open decision 3: kanban columns follow the state machine when `columnField` has one. | | |
| 14 | Open decision 4: an API layout exposes only the fields listed on the page. | | |
| 15 | Open decision 5: a new record is inserted on Save; a blank close writes nothing. | | |
| 16 | Open decision 6: page `onValidate` may warn; `tablelogic` on the server wins. | | |
| 17 | Open decision 7: a user control is a named packaged component. | | |

When this checklist is approved, freeze step 05 can move from "Design revised" to a build freeze and M1 can start.
