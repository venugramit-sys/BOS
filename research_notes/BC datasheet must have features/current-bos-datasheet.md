# Current BOS Datasheet Feature Inventory

Read-only inventory of `ui/src/lib/bos-ui/datasheet` and the editors it mounts (`Input`, `Select`, `Combobox`, `DatePicker`), plus what `docs/28_page_meta_object_design.md` and `docs/22_bos_complete_ui_element_taxonomy_and_shadcn_mapping.md` already say. No files were changed. The component was not executed.

Files read: `Datasheet.svelte`, `model.js`, `model.test.js`, `index.js`, `date-picker/evaluator.js`, `date-picker/DatePicker.svelte` (props and `executeFormulaEvaluation`), `input/evaluator.js` (`evaluateMath`), `input/Input.svelte` (`executeMathEvaluation`, `handleKeyDown`).

## Which keyboard actions are implemented in Datasheet.svelte?

### Takeaway

`onGridKeydown` and `onEditorKey` implement arrows, Enter, Tab, F2, F4, Alt+Down, F8, Escape, Delete/Backspace, Ctrl/Cmd+V, and Ctrl/Cmd+Shift+C. Cell copy uses the browser `copy` event (`onCopy`), not a Ctrl+C branch. Fill is a pointer drag (`startFill`), not a key.

### Cited Findings

- File header states the intended keys: "Enter walks Quick Entry fields. F8 copies the cell above. Locked cells are not typed over." — [Datasheet.svelte](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- `onGridKeydown` returns immediately when the event target is inside `input, textarea, select, [data-sheet-editor]`. Grid keys therefore apply only when the grid itself is focused. — [Datasheet.svelte `onGridKeydown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- ArrowUp, ArrowLeft, and ArrowRight call `moveBy` with `event.shiftKey` so Shift extends the range. ArrowDown does the same only when `!event.altKey`, so Alt+ArrowDown is left for the editor shortcut. — [Datasheet.svelte `onGridKeydown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Tab calls `moveEntry(..., "any")`. Enter calls `moveEntry(..., "entry")`. Shift reverses the step. `moveCell` documents the split: "Enter walks Quick Entry fields. Tab walks every column." Eligible Quick Entry columns are those with `editable !== false` and `quickEntry !== false`. At the end of a row, both wrap to the next or previous row. — [model.js `moveCell`, `stepColumn`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- F2 calls `toggleCheck` on a checkbox column and `beginEdit` on every other editor. — [Datasheet.svelte `onGridKeydown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- F4, and Alt+ArrowDown, call `beginEdit` from the grid. `beginEdit` opens the menu for `select` and `lookup` always, and for `date` when no typed character was passed (`menuOpen = column.editor === "date" && initial == null`). While an editor is open, the same chord toggles `menuOpen` in `onEditorKey`. — [Datasheet.svelte `onGridKeydown`, `onEditorKey`, `beginEdit`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- F8 calls `copyAbove` from both `onGridKeydown` and `onEditorKey`. `copyAbove` reads the same column on the previous view row, writes it through `commitActive`, and does nothing on row 0, a missing column, or `editable === false`. — [Datasheet.svelte `copyAbove`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Escape sets `cancelEdit`, clears `editing`, `menuOpen`, typeahead, and `cellError`, then `focusGrid`. Inside the editor, Escape only closes `menuOpen` when the menu is already open. — [Datasheet.svelte `onGridKeydown`, `onEditorKey`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Delete and Backspace both call `clearRange`. That writes `false` into checkbox cells and `""` into other editable cells across the selected range, and calls `onCommitCell` for existing rows. It does not remove the row. Row removal is the context-menu action `removeRow`. — [Datasheet.svelte `clearRange`, `removeRow`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Copy of the selected cells is `onCopy`: if not editing, it `preventDefault`s and sets `text/plain` to `toTsv(matrix)`. There is no `key === "c"` branch for plain Ctrl+C. Ctrl/Cmd+Shift+C is explicit and copies full rows via `currentMatrix(true)`. — [Datasheet.svelte `onCopy`, `onGridKeydown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Paste is both `onPaste` (clipboard event text) and Ctrl/Cmd+V (`navigator.clipboard.readText` then `applyClipboard`). `applyClipboard` uses `pastePlan` + `parseTsv`, skips `editable === false`, stops on the first `coerceCell` or `checkCell` error, and calls `onCommitCell` per changed existing cell. — [Datasheet.svelte `applyClipboard`; model.js `pastePlan`, `parseTsv`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Fill is `startFill` on pointerdown of the "Fill down" handle, shown only when the cell is active, `editable` is true, and the row is not new. On pointerup it fills only downward (`target <= range.r2` returns). `extendFill` copies a single value or text; two or more numbers continue the last numeric step. New rows are skipped (`entry.isNew`). — [Datasheet.svelte `startFill`; model.js `extendFill`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Also implemented, outside the asked list: Home and Ctrl+Home, End and Ctrl+End (Shift extends), Space toggles a checkbox, and a single printable character with no Ctrl/Cmd/Alt calls `beginEdit` with that character. `model.test.js` covers fill, paste TSV, and typeahead matching. — [Datasheet.svelte `onGridKeydown`; model.test.js](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.test.js)

### Inferences

- Ctrl+C works only if the browser fires `copy` on the focused grid. The context menu labels that action "Ctrl+C" and calls `writeClipboard` directly, so the menu path does not depend on the copy event.
- Because `onEditorKey` is `onkeydowncapture` and stops F8, Enter, Tab, Escape, F4, and Alt+Down, those keys are handled by the sheet before the inner editor's own key handler.

### Gaps

- Page Up, Page Down, Ctrl+X, and Ctrl+Z do not appear in `onGridKeydown`. Whether the browser still performs a native cut was not tested.
- No runtime check confirmed that Ctrl+C reaches `onCopy` in this browser.

## Which editors exist, and what do date formulas and number math already do?

### Takeaway

The sheet editor snippet mounts `Select`, `DatePicker`, `Combobox`, and `Input`, plus `Checkbox` outside that snippet. Date text is resolved by `evaluateDateFormula`. Number text is resolved by `evaluateMath` (`+ - * /` and parentheses). `columnsFromFields` only assigns checkbox, date, number, and text; select, lookup, and code are extra editor names the component already switches on.

### Cited Findings

- Imports: `Input`, `Select`, `Checkbox`, `DatePicker`, `Combobox`. The `editor` snippet branches on `column.editor`: `"select"` → `Select`, `"date"` → `DatePicker`, `"lookup"` → `Combobox` with `showFactBox={false}`, everything else → `Input`. `Input` gets `type="code"` only when `column.editor === "code"`, and `evalMath={column.editor === "number"}`. `Checkbox` is rendered in the desktop cell and the phone card, not inside the snippet. — [Datasheet.svelte `editor` snippet](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- `editorForType` maps boolean/bool → `"checkbox"`, date/datetime → `"date"`, integer/bigint/decimal/number → `"number"`, and every other type → `"text"`. It does not return `"select"`, `"lookup"`, or `"code"`. — [model.js `editorForType`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- `columnsFromFields` hides `visible === false`, locks primary keys, FlowFields, computed fields, and `editable === false`, sets `quickEntry` to the same unlocked flag, sets `total: "sum"` for number columns, and reads decimal scale from `decimal(p,s)`. — [model.js `columnsFromFields`, `decimalsOf`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- `coerceCell` is the commit parser. Checkbox accepts boolean or `1/true/yes/y` and `0/false/no/n/""`. Date calls `evaluateDateFormula(text)` and stores `formatISODate`, or `C` plus that ISO when `result.isClosing`. Number strips commas and calls `evaluateMath`; failure returns "Enter a number". Select and lookup match option value or case-insensitive label. Other editors become strings. — [model.js `coerceCell`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- `evaluateDateFormula` accepts ISO `YYYY-MM-DD`, a day number (`15` = that day of the base month), `MMDD` (`1215`), `t`/`today`, `w`/`work`, `-cm`/`cm`, `-cq`/`cq`, `-cy`/`cy`, relative `D/W/M/Q/Y` terms such as `+30d` and `cm+10d`, and a leading `C` closing-date flag (`C123126`). `W`/`WORK` copies `baseDate`. The function default for `baseDate` is `new Date()`. `coerceCell` calls it with one argument, so the sheet commit path uses that default. Unrecognized leftovers fall through to `new Date(raw)` and otherwise return null. — [date-picker/evaluator.js `evaluateDateFormula`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/date-picker/evaluator.js)
- `getAccountingPresets` returns Today, Yesterday, This Week, Month to Date, Full Month, Quarter to Date, Year to Date, and Full Fiscal Year. `DatePicker` builds that list and evaluates typed formulas in `executeFormulaEvaluation`, including `..` ranges when `range` is true. Closing dates are prefixed `C` only when `allowClosingDates` is true (default true). — [date-picker/evaluator.js `getAccountingPresets`; DatePicker.svelte `executeFormulaEvaluation`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/date-picker/DatePicker.svelte)
- The sheet mounts `DatePicker` with `density="sm"`, `clearable={false}`, bound `open` and `value`, and `onChange`. It does not pass `range`, `showTime`, `closedPeriods`, `workDate`, or `allowClosingDates`. Those stay at the picker defaults (`range` false, `showTime` false, `closedPeriods` empty, `evalFormula` true, `allowClosingDates` true). — [Datasheet.svelte date branch; DatePicker.svelte props](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/date-picker/DatePicker.svelte)
- `evaluateMath` is a shunting-yard parser for `+`, `-`, `*`, `/`, parentheses, decimals, and unary minus. It rejects other characters, rejects division by zero, and rounds to 6 decimal places. A string with no operator is parsed as a plain number. The file comment says it does not use `eval` or `Function`. — [input/evaluator.js `evaluateMath`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/input/evaluator.js)
- `Input.executeMathEvaluation` runs on blur, and on Enter inside `Input.handleKeyDown`, when `evalMath` is true or `type === "currency"`. It rewrites the field to the numeric result and can show a calculation badge. Datasheet does not pass `onCopyAbove`, `masked`, `maskType`, or `pattern`. `Input.handleKeyDown` still has its own F8 → `onCopyAbove` branch, but the sheet captures F8 first. — [Input.svelte `executeMathEvaluation`, `handleKeyDown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/input/Input.svelte)
- `checkCell` rejects empty values when `column.required` is set, and runs `column.validate(value, row)` when that property is a function. — [model.js `checkCell`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Select typeahead uses `nextTypeQuery` (800 ms window) and `matchOption` (label prefix, then value prefix). Lookup typeahead writes the typed text into `draft` and opens the combobox. — [Datasheet.svelte `pushTyped`, `acceptTypeahead`; model.js `nextTypeQuery`, `matchOption`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)

### Inferences

- Number math on commit does not depend on `Input`'s blur handler. `commitActive` always runs `coerceCell`, which calls `evaluateMath` for number columns.
- A date formula typed in the cell is evaluated twice in the design of the two modules: once inside `DatePicker.executeFormulaEvaluation` when that picker runs it, and again in `coerceCell` on commit. Both call the same `evaluateDateFormula`.
- `w` / `work` on the sheet commit path is today, because no work-date argument is passed. Doc 28 says work date comes from `themeStore.workDate` on `onNewRecord`, not as a header. That store is not read in `coerceCell`.

### Gaps

- The exact key or blur inside `DatePicker` that calls `executeFormulaEvaluation` was not fully traced. The function exists and the sheet commit path is independent of it.
- No call site in the datasheet passes a company work date into `evaluateDateFormula`.

## What layout features exist: freeze, filter, sort, search, formula bar, new row, phone cards, windowing, totals, context menu?

### Takeaway

All ten are present in `Datasheet.svelte`. Freeze is a column-count prop. Filter is contains-text or a value list. Sort is one column. The formula bar is a read-only label. Phone cards replace the grid below 768 px. Windowing starts at 48 rows. Totals are a sticky sum footer plus a selection status bar.

### Cited Findings

- Props that turn features on: `showNewRow = true`, `freezeCount = 0`, `showFilter = true`, `showSearch = true`, `showFormulaBar = true`, `showStatusBar = true`, `totalCount`, `offset`, `onPreviousPage`, `onNextPage`. — [Datasheet.svelte props](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Freeze: `frozenLeft` returns a pixel `left` for `colIndex < freezeCount`, starting after the 36 px gutter, and those header and body cells get `sticky`. The gutter column is always `sticky left-0`. Default `freezeCount` is 0, so nothing but the gutter sticks until the caller passes a count. — [Datasheet.svelte `frozenLeft`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Filter: each header has a `Popover` when `showFilter` is true. Select and lookup columns list `column.options` and store `option.label`. Other columns use an input whose placeholder is `Contains…`. `rowMatches` keeps a row only when every active filter is a case-insensitive substring of `displayCell`. Active filters render as removable chips; `clearFilters` clears both column filters and the search query. — [Datasheet.svelte `setFilter`; model.js `rowMatches`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Sort: header click calls `toggleSort`: first click ascending, second descending, third clears `sortKey`. One `sortKey` only. `compareCells` sorts numbers numerically (empty is negative infinity) and other values with `localeCompare` numeric, base sensitivity. — [Datasheet.svelte `toggleSort`; model.js `compareCells`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Search: a text input bound to `query`, placeholder "Search records", shown when `showSearch`. `rowMatches` treats it as a substring across any column display text. — [Datasheet.svelte search input; model.js `rowMatches`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Formula bar: when `showFormulaBar`, an `<output>` shows `formulaText`. That derived value is the select typeahead, the edit `draft`, or `displayCell` of the focused cell. The field name and optional `hint` sit beside it. It is not an `<input>`. — [Datasheet.svelte `formulaText`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- New row: if `showNewRow && editable`, `view` appends `{ key: NEW_ROW_KEY, isNew: true }` where `NEW_ROW_KEY` is `"__bos_new__"`. The gutter shows `*`. Leaving that row calls `finishNewRow`, which drops empty draft fields via `draftPayload`, checks required columns, then `onInsertRow(payload)` or pushes a local row with `crypto.randomUUID()`. — [Datasheet.svelte `finishNewRow`; model.js `draftPayload`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Phone cards: a `matchMedia("(max-width: 767px)")` effect sets `narrow`. The narrow branch renders one `<article>` per view row, caption "New record" or `Record ${key}`, and a label/value stack. Tap calls `beginEdit`. The desktop grid, context menu, freeze, and totals footer are not in that branch. — [Datasheet.svelte narrow branch](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Windowing: `windowed` renders every row when `view.length < 48`. At 48 or more, it slices from `scrollTop / 32` with 6 rows of lead and about one viewport plus 12 rows, and pads top and bottom with spacer `<tr>` elements. `rowHeight` is 32. `reveal` scrolls the focused row into view. — [Datasheet.svelte `windowed`, `reveal`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Totals: a sticky `<tfoot>` calls `columnTotal` per column. That returns a sum only when `column.total === "sum"` and `column.editor === "number"`; the new row is excluded. The status bar shows `Record N of total`, the text "F8 copy above", and `selectionStats` (count, and sum/average when the selection contains numbers). Optional previous/next buttons call `onPreviousPage` / `onNextPage`. — [Datasheet.svelte status bar; model.js `columnTotal`, `selectionStats`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Context menu: desktop grid is wrapped in `ContextMenu` with Copy cell, Copy field above (F8), Copy rows (Ctrl+Shift+C), Paste (Ctrl+V), and Delete row. Delete row is disabled on the new row. — [Datasheet.svelte `menuItems`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Also present beside that list: column resize (`startResize`, minimum 72 px), a row gutter (`*` / dirty pencil / `▶` / row number), `EmptyState` when there are no rows, and a `Banner` for `cellError`. Header cells are `sticky top-0`. — [Datasheet.svelte `startResize`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)

### Inferences

- Filter, sort, and search run on the in-memory `rows` array inside `view`. They do not call a server. `totalCount` and the page buttons are the only hooks for a larger record set.
- Phone layout keeps editors and the new-row entry, and drops freeze, column filters, the fill handle, the totals footer, and the context menu.

### Gaps

- No screenshot or viewport run confirmed the 767 px card switch.
- `offset` is only added into the gutter number and the status-bar record label. How a parent should page was not traced beyond the two callbacks.

## What do docs/28 and docs/22 already require of Datasheet?

### Takeaway

Doc 28 names `Datasheet` once: the desktop journal and studio record sheet, with a row gutter, Quick Entry, F8 copy-above, frozen columns, and the same field editors. It assigns list grids to `DataTable` and simple cell editing to `DataGrid`. Doc 22 does not contain the word Datasheet. Its grid requirements are written against `DataTable`, `PolymorphicSubform`, `FilterBar`, and `DataGridTotalsBar`.

### Cited Findings

- Doc 28, section 14, lists `Datasheet` among components already in bos-ui, then defines it: "Datasheet is the desktop journal and studio record sheet: row gutter, Quick Entry, F8 copy-above, frozen columns, and the same field editors." The next sentences assign lookup to `Combobox`, date formulas to `DatePicker`, and "math evaluation, masks, and F8 copy-above" to `Input`. — [docs/28_page_meta_object_design.md §14](file:///d:/Work%20Studio/Bos/docs/28_page_meta_object_design.md)
- The same section's `Repeater` row composes `DataTable` and `DataGrid`, not `Datasheet`. Props listed there include `editable`, `delayedInsert`, `editing`, `totals`, `onCommitCell`, `onDraftRow`, `onDeleteRow`. "Phone cards inside Repeater use Field and the same editors." Column filters "stay on DataTable." — [docs/28_page_meta_object_design.md §14 Repeater and FilterBar](file:///d:/Work%20Studio/Bos/docs/28_page_meta_object_design.md)
- Feature 29: a repeater is a grid on a desktop and one card per line on a phone. Acceptance names `DataTable` at 1920 px and cards at 375 px. Feature 35: cell editing includes F8 copy-above because `DataGrid` and `Input` already do that, and editing quantity issues `PATCH` for that field only. Feature 27 and §7.1 map Date → `DatePicker` ("Date formulas stay inside DatePicker"), Integer/Decimal → `Input` with in-field math, Enum/Option → `Select`, Relation → `Combobox`, Boolean default → `Switch` with a checkbox style. — [docs/28_page_meta_object_design.md features 27, 29, 35 and §7.1](file:///d:/Work%20Studio/Bos/docs/28_page_meta_object_design.md)
- Feature 21 puts related facts in the `side` slot (`FactBox`), not inside the sheet. Feature 30's drill is on a tile (`Cue`), which opens a `drill` page. Feature 37's card open is `layout: split` plus a nested card page. — [docs/28_page_meta_object_design.md features 21, 30, 37](file:///d:/Work%20Studio/Bos/docs/28_page_meta_object_design.md)
- A search of doc 22 for `Datasheet` and `datasheet` returned no matches. Tier 4 specifies `DataTable` (multi-sort, global search, column filters such as `>100` and `10..20`, grouping, pagination, row selection, column visibility, order, resize, pinning, expand, aggregation footer, CSV export; keyboard arrows, Enter, F8), `PolymorphicSubform` (desktop spreadsheet, mobile touch cards), `FilterBar` (wildcards `..`, `*`, `>1000`, `Draft|Approved`), and `DataGridTotalsBar` (sums, record counts, VAT/tax breakdowns). — [docs/22_bos_complete_ui_element_taxonomy_and_shadcn_mapping.md Tier 4](file:///d:/Work%20Studio/Bos/docs/22_bos_complete_ui_element_taxonomy_and_shadcn_mapping.md)
- Doc 22's `Input` row requires shunting-yard math (`evalMath`, example `120 * 1.15 = 138`), masking, pattern masks, and F8 `onCopyAbove`. Its `DatePicker` row requires formulas `t`, `cm`, `-cm`, `cm+10d`, `cq`, `cy`, accounting presets, closing date `C123126`, closed-period lock, and optional 15-minute time. Its `Combobox` row requires F4, arrow, Enter, a live FactBox preview, and "+ New" quick create. — [docs/22 Tier 3 Input, Combobox, DatePicker](file:///d:/Work%20Studio/Bos/docs/22_bos_complete_ui_element_taxonomy_and_shadcn_mapping.md)

### Inferences

- Doc 28's direct Datasheet requirements are the four behaviors in that one sentence (gutter, Quick Entry, F8, frozen columns) plus shared field editors. List-page behaviors in the same document are specified on `DataTable` / `Repeater`, not on `Datasheet`.
- Doc 22 cannot be cited as a Datasheet specification. The nearest written grid contract is the `DataTable` row, and several of its items (multi-sort, filter operators, column visibility, CSV, grouping) are not what `Datasheet.svelte` implements.

### Gaps

- Doc 28 does not say whether the studio record sheet and the journal must be the same component instance, or when `PageHost` should mount `Datasheet` instead of `DataTable` / `DataGrid`.
- Doc 22 was not searched beyond the Datasheet name and the Tier 3–4 grid rows for a later alias.

## What is explicitly absent in the component?

### Takeaway

Searches of `ui/src/lib/bos-ui/datasheet` found no undo stack, no column chooser, no filter-operator parser, no FactBox, no dimensions, no card-open action, and no drill-down. The only FactBox token is `showFactBox={false}` on the lookup `Combobox`. The only "drill" string is sample item text in `model.test.js`.

### Cited Findings

- Undo: `Datasheet.svelte` and `model.js` have no `undo` identifier, no history array, and no Ctrl+Z branch in `onGridKeydown`. Escape cancels the current edit only (`cancelEdit`). — [Datasheet.svelte `onGridKeydown`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Column chooser: headers can be resized and the first `freezeCount` columns stick. There is no control that hides, shows, or reorders columns. `columnsFromFields` only drops fields already marked `visible === false` before the sheet is built. — [model.js `columnsFromFields`; Datasheet.svelte header](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Filter operators: `rowMatches` uses `String.includes` on the display text. The popover placeholder is `Contains…`. There is no parser for `>`, `..`, `*`, or `|`. — [model.js `rowMatches`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- FactBox: the lookup branch sets `showFactBox={false}`. The file does not import `FactBox`. — [Datasheet.svelte lookup branch](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Dimensions: a search of the datasheet directory for dimension tagging, dimension sets, or a dimension editor returned no feature. Number columns are plain sums. — [datasheet/model.js `columnTotal`](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.js)
- Card open: props are commit, insert, and delete callbacks plus optional page buttons. There is no `onOpen`, no double-click handler (`dblclick` does not appear), and no navigation to a card page. Phone `<article>` blocks are the narrow layout, not an open-card command. — [Datasheet.svelte props and narrow branch](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/Datasheet.svelte)
- Drill-down: no drill callback or link renderer. `model.test.js` uses the strings `"drill"` and `"DRILL-10"` as a search/typeahead fixture. — [datasheet/model.test.js](file:///d:/Work%20Studio/Bos/ui/src/lib/bos-ui/datasheet/model.test.js)

### Inferences

- Absence here means "not in this directory's source." Doc 28 still requires FactBox, tile drill, and split card open as other components (`FactBox`, `Cue`, `SplitPane` / nested `PageHost`).
- Masks and `Input`'s own F8 callback are implemented in `Input.svelte` and are not wired by the sheet props that were read.

### Gaps

- Other bos-ui grids (`DataTable`, `DataGrid`) were not inventoried, so this absence list does not say those features are missing from the whole UI kit.
