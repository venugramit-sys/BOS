<script>
  /**
   * Datasheet — Business Central edit-list for BOS.
   * One row is one record. Editors are Input, Combobox, DatePicker, Select, and Checkbox.
   * Enter walks Quick Entry fields. F8 copies the cell above. Locked cells are not typed over.
   */
  import { tick, untrack } from "svelte";
  import { cn } from "$lib/utils.js";
  import { Popover } from "bits-ui";
  import { ChevronDown, ChevronLeft, ChevronRight, ListFilter, Pencil } from "lucide-svelte";
  import Input from "../input/Input.svelte";
  import Select from "../select/Select.svelte";
  import Checkbox from "../checkbox/Checkbox.svelte";
  import DatePicker from "../date-picker/DatePicker.svelte";
  import Combobox from "../combobox/Combobox.svelte";
  import EmptyState from "../empty-state/EmptyState.svelte";
  import StatusBar from "../status-bar/StatusBar.svelte";
  import Button from "../button/Button.svelte";
  import Banner from "../banner/Banner.svelte";
  import ContextMenu from "../context-menu/ContextMenu.svelte";
  import {
    NEW_ROW_KEY,
    applyColumnLayout,
    columnLocked,
    checkCell,
    coerceCell,
    columnTotal,
    compareCells,
    displayCell,
    draftPayload,
    extendFill,
    fieldProblem,
    fieldProblems,
    formatNumber,
    hiddenSharedNote,
    inRange,
    matchOption,
    moveCell,
    nextTypeQuery,
    normalizeRange,
    pageStep,
    parseTsv,
    revealColumnScroll,
    pastePlan,
    rowMatches,
    selectionStats,
    toTsv
  } from "./model.js";
  import { formatISODate } from "../date-picker/evaluator.js";

  let {
    columns: sourceColumns = [],
    rows = $bindable([]),
    rowKey = "id",
    label = "Datasheet",
    layoutKey = "",
    editable = true,
    showNewRow = true,
    freezeCount = 0,
    showFilter = true,
    showStatusBar = true,
    sortable = true,
    totalCount = null,
    offset = 0,
    emptyTitle = "No records",
    emptyDescription = "New rows start on the star line.",
    workDate = "",
    onCommitCell = null,
    onInsertRow = null,
    onDeleteRow = null,
    onPreviousPage = null,
    onNextPage = null,
    onActiveRow = null,
    onOpenCard = null,
    onDrillDown = null,
    selectedKey = $bindable(null),
    focusKey = $bindable(""),
    class: className = ""
  } = $props();

  const rowHeight = 32;
  const sheetEditorClass =
    "h-full min-h-0 w-full max-w-full [&_button.group]:h-full [&_button.group]:w-full [&_button]:min-h-0 [&_button]:rounded-none [&_button]:border-0 [&_button]:bg-transparent [&_button]:shadow-none [&_.group]:h-full [&_.group]:w-full [&_.group]:rounded-none [&_.group]:border-0 [&_.group]:bg-transparent [&_.group]:px-1 [&_.group]:shadow-none";
  let menuOpen = $state(false);
  let typeQuery = $state("");
  let selectActive = $state("");
  let typedAt = 0;
  let filterOpen = $state("");
  let headerKey = $state("");
  let scrollEl = $state(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(360);
  let narrow = $state(false);
  let query = $state("");
  let filters = $state({});
  let sortKey = $state("");
  let sortDir = $state("asc");
  let widths = $state({});
  let focusCol = $state(0);
  let anchorKey = $state("");
  let anchorCol = $state(0);
  let editing = $state(null);
  let draft = $state("");
  let cellError = $state("");
  let cancelEdit = false;
  let newDraft = $state({});
  let dirty = $state({});
  let fillTarget = $state(null);
  let fillListeners = null;
  let layout = $state(null);
  let issuesOnly = $state(false);
  let layoutLoaded = false;
  let storageKey = $derived(`bos.datasheet.layout.${layoutKey || label}`);
  let columns = $derived(applyColumnLayout(sourceColumns, layout));
  let pinned = $derived(Number.isInteger(layout?.freezeCount) ? layout.freezeCount : freezeCount);
  let sharedNote = $derived(hiddenSharedNote(sourceColumns, layout));
  let workNote = $derived(workDate && workDate !== formatISODate(new Date()) ? `Work date is ${workDate}. It is not today.` : "");
  let problemByKey = $derived.by(() => {
    const map = new Map();
    for (const row of rows) {
      const problems = fieldProblems(row, sourceColumns);
      const id = row?.[rowKey];
      if (id != null && id !== "" && problems.length) map.set(String(id), problems);
    }
    return map;
  });
  let issueCount = $derived([...problemByKey.values()].reduce((total, list) => total + list.length, 0));

  let view = $derived.by(() => {
    const filtered = rows.filter((row) => {
      if (issuesOnly) {
        const id = row?.[rowKey];
        if (id == null || id === "" || !problemByKey.get(String(id))?.length) return false;
      }
      return rowMatches(row, columns, filters, query);
    });
    const column = columns.find((item) => item.key === sortKey);
    const sorted = column ? [...filtered].sort((a, b) => compareCells(a, b, column, sortDir)) : filtered;
    const entries = sorted.map((record, index) => ({
      record,
      key: identity(record, index),
      isNew: false
    }));
    if (showNewRow && editable) entries.push({ record: null, key: NEW_ROW_KEY, isNew: true });
    return entries;
  });

  let focusIndex = $derived.by(() => {
    const index = view.findIndex((entry) => entry.key === focusKey);
    return index < 0 ? 0 : index;
  });

  let range = $derived.by(() => {
    const anchor = view.findIndex((entry) => entry.key === anchorKey);
    const focus = view.findIndex((entry) => entry.key === focusKey);
    if (anchor < 0 || focus < 0) return { r1: focusIndex, r2: focusIndex, c1: focusCol, c2: focusCol };
    return normalizeRange({ row: anchor, col: anchorCol }, { row: focus, col: focusCol });
  });

  let windowed = $derived.by(() => {
    if (view.length < 48) return { start: 0, end: view.length, padTop: 0, padBottom: 0 };
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 6);
    const count = Math.ceil(viewportHeight / rowHeight) + 12;
    const end = Math.min(view.length, start + count);
    return {
      start,
      end,
      padTop: start * rowHeight,
      padBottom: Math.max(0, (view.length - end) * rowHeight)
    };
  });

  let activeEntry = $derived(view[focusIndex] || null);
  let matrix = $derived.by(() => {
    const lines = [];
    for (let row = range.r1; row <= range.r2; row += 1) {
      const entry = view[row];
      if (!entry) continue;
      const line = [];
      for (let col = range.c1; col <= range.c2; col += 1) {
        const column = columns[col];
        if (!column) continue;
        line.push(readValue(entry, column));
      }
      lines.push(line);
    }
    return lines;
  });

  let stats = $derived(selectionStats(matrix));
  let recordTotal = $derived(totalCount == null ? rows.length : totalCount);
  let tableWidth = $derived(36 + columns.reduce((sum, column) => sum + widthOf(column), 0));
  let activeFilters = $derived(columns.filter((column) => String(filters[column.key] || "").trim()));
  let showTotals = $derived(columns.some((column) => column.total === "sum" && column.editor === "number"));

  $effect(() => {
    const media = window.matchMedia("(max-width: 767px)");
    const apply = () => (narrow = media.matches);
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  });

  $effect(() => {
    const el = scrollEl;
    if (!el) return;
    viewportHeight = el.clientHeight;
    const observer = new ResizeObserver(() => {
      viewportHeight = el.clientHeight;
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  $effect(() => {
    if (!view.length) return;
    if (!view.some((entry) => entry.key === focusKey)) {
      focusKey = view[0].key;
      anchorKey = view[0].key;
    }
  });

  $effect(() => {
    const key = storageKey;
    if (layoutLoaded) return;
    layoutLoaded = true;
    try {
      const raw = localStorage.getItem(key);
      layout = raw ? JSON.parse(raw) : null;
    } catch {
      layout = null;
    }
  });

  $effect(() => {
    const entry = view.find((item) => item.key === focusKey);
    const record = entry && !entry.isNew ? entry.record : null;
    untrack(() => onActiveRow)?.(record);
  });

  $effect(() => {
    if (!editing || !typeQuery) return;
    if (columns[editing.col]?.editor !== "lookup") return;
    draft = typeQuery;
  });

  function rememberLayout(next) {
    layout = next;
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
    } catch {
      /* The sheet still works when the browser refuses storage. */
    }
  }

  function layoutBase() {
    const order = layout?.order?.length ? [...layout.order] : sourceColumns.map((column) => column.key);
    for (const column of sourceColumns) {
      if (!order.includes(column.key)) order.push(column.key);
    }
    return {
      order,
      hidden: [...(layout?.hidden || [])],
      freezeCount: Number.isInteger(layout?.freezeCount) ? layout.freezeCount : freezeCount,
      quickEntry: { ...(layout?.quickEntry || {}) }
    };
  }

  function toggleColumn(key) {
    const column = sourceColumns.find((item) => item.key === key);
    if (!column || column.keepVisible) return;
    const next = layoutBase();
    next.hidden = next.hidden.includes(key) ? next.hidden.filter((item) => item !== key) : [...next.hidden, key];
    rememberLayout(next);
  }

  function moveColumn(key, step) {
    const next = layoutBase();
    const index = next.order.indexOf(key);
    const target = index + step;
    if (index < 0 || target < 0 || target >= next.order.length) return;
    const [item] = next.order.splice(index, 1);
    next.order.splice(target, 0, item);
    rememberLayout(next);
  }

  function toggleQuick(key) {
    const column = sourceColumns.find((item) => item.key === key);
    if (!column) return;
    const next = layoutBase();
    const current = next.quickEntry[key] != null ? next.quickEntry[key] : column.quickEntry !== false;
    next.quickEntry[key] = !current;
    rememberLayout(next);
  }

  function freezeThrough(index) {
    const next = layoutBase();
    next.freezeCount = index + 1;
    rememberLayout(next);
  }

  function resetLayout() {
    layout = null;
    try {
      localStorage.removeItem(storageKey);
    } catch {
      /* ignore */
    }
  }

  function problemOn(entry, column) {
    if (!entry || entry.isNew || !column) return "";
    return problemByKey.get(entry.key)?.find((item) => item.key === column.key)?.message || "";
  }

  function identity(record, index) {
    const id = record?._id ?? record?.[rowKey];
    if (id != null && id !== "") return String(id);
    return `row-${index}`;
  }

  function readValue(entry, column) {
    if (!entry || !column) return "";
    if (entry.isNew) return newDraft[column.key] ?? "";
    const value = entry.record?.[column.key];
    return value == null ? "" : value;
  }

  function writeValue(entry, column, value) {
    if (entry.isNew) {
      newDraft = { ...newDraft, [column.key]: value };
      return;
    }
    entry.record[column.key] = value;
  }

  function same(left, right) {
    return String(left ?? "") === String(right ?? "");
  }

  function widthOf(column) {
    return widths[column.key] || column.width || 148;
  }

  function frozenLeft(colIndex) {
    if (colIndex >= pinned) return null;
    let left = 36;
    for (let index = 0; index < colIndex; index += 1) left += widthOf(columns[index]);
    return left;
  }

  async function focusGrid() {
    await tick();
    if (editing) return;
    scrollEl?.focus({ preventScroll: true });
  }

  function reveal(key) {
    const index = view.findIndex((entry) => entry.key === key);
    if (index >= 0 && scrollEl) {
      const top = index * rowHeight;
      if (top < scrollEl.scrollTop) scrollEl.scrollTop = top;
      const bottom = top + rowHeight;
      if (bottom > scrollEl.scrollTop + scrollEl.clientHeight - 32) {
        scrollEl.scrollTop = bottom - scrollEl.clientHeight + 32;
      }
    }
    revealFocusedColumn();
  }

  function revealFocusedColumn() {
    const el = scrollEl;
    const colIndex = focusCol;
    if (!el || colIndex < pinned || colIndex >= columns.length) return;
    const key = columns[colIndex]?.key;
    if (!key) return;
    const cell = el.querySelector(`thead th[data-column-key="${CSS.escape(key)}"]`);
    if (!(cell instanceof HTMLElement)) return;
    const grid = el.getBoundingClientRect();
    let frozen = 0;
    const gutter = el.querySelector("thead th");
    if (gutter instanceof HTMLElement) frozen = gutter.getBoundingClientRect().right - grid.left;
    for (let index = 0; index < pinned; index += 1) {
      const pinnedKey = columns[index]?.key;
      if (!pinnedKey) continue;
      const pinnedCell = el.querySelector(`thead th[data-column-key="${CSS.escape(pinnedKey)}"]`);
      if (pinnedCell instanceof HTMLElement) {
        frozen = Math.max(frozen, pinnedCell.getBoundingClientRect().right - grid.left);
      }
    }
    const box = cell.getBoundingClientRect();
    const columnLeft = box.left - grid.left + el.scrollLeft;
    const next = revealColumnScroll(el.scrollLeft, el.clientWidth, columnLeft, box.width, frozen, 12);
    if (Math.abs(next - el.scrollLeft) > 0.5) el.scrollLeft = next;
  }

  function clearTypeahead() {
    typeQuery = "";
    selectActive = "";
    typedAt = 0;
  }

  function rememberType(char, fresh) {
    const now = Date.now();
    typeQuery = fresh ? String(char) : nextTypeQuery(typeQuery, String(char), now - typedAt);
    typedAt = now;
  }

  async function focusEditor(selectAll = false) {
    await tick();
    const input = document.querySelector("[data-sheet-editor] input, [data-sheet-editor] textarea");
    if (!(input instanceof HTMLInputElement || input instanceof HTMLTextAreaElement)) return;
    input.focus();
    if (selectAll) input.setSelectionRange(0, input.value.length);
    else {
      const end = input.value.length;
      input.setSelectionRange?.(end, end);
    }
  }

  async function focusChooser() {
    await tick();
    const trigger = document.querySelector("[data-sheet-editor] button");
    if (trigger instanceof HTMLElement) trigger.focus({ preventScroll: true });
  }

  function stepSelect(step) {
    const column = columns[editing?.col ?? focusCol];
    const options = column?.options || [];
    if (!column || column.editor !== "select" || !options.length) return;
    const current = selectActive || draft;
    let index = options.findIndex((option) => String(option.value) === String(current));
    if (index < 0) index = step > 0 ? -1 : 0;
    index = Math.max(0, Math.min(options.length - 1, index + step));
    selectActive = String(options[index].value);
  }

  function pushTyped(column, char) {
    if (column.editor === "select") {
      rememberType(char, false);
      selectActive = matchOption(column.options, typeQuery);
      menuOpen = true;
      return;
    }
    if (column.editor === "lookup") {
      rememberType(char, false);
      draft = typeQuery;
      menuOpen = true;
      focusEditor();
      return;
    }
    draft = `${draft ?? ""}${char}`;
    typeQuery = String(draft);
    typedAt = Date.now();
    focusEditor();
  }

  function beginEdit(entry, colIndex, initial) {
    const column = columns[colIndex];
    if (!editable || !entry || !column || columnLocked(column, entry.isNew ? newDraft : entry.record) || column.editor === "checkbox") return;
    cancelEdit = false;
    if (editing?.key === entry.key && editing?.col === colIndex) {
      if (initial == null) return;
      pushTyped(column, initial);
      return;
    }
    const current = readValue(entry, column);
    const currentText = current == null ? "" : String(current);
    editing = { key: entry.key, col: colIndex, original: current ?? "" };
    focusKey = entry.key;
    focusCol = colIndex;
    cellError = "";
    if (column.editor === "select") {
      draft = currentText;
      if (initial == null) {
        clearTypeahead();
        selectActive = currentText;
        menuOpen = true;
      } else {
        rememberType(initial, true);
        selectActive = matchOption(column.options, typeQuery);
        menuOpen = true;
      }
      focusChooser();
      return;
    }
    if (column.editor === "lookup") {
      if (initial == null) {
        clearTypeahead();
        draft = currentText;
      } else {
        rememberType(initial, true);
        draft = typeQuery;
      }
      menuOpen = true;
      focusEditor();
      return;
    }
    draft = initial != null ? String(initial) : currentText;
    typeQuery = initial != null ? String(initial) : "";
    typedAt = initial != null ? Date.now() : 0;
    menuOpen = column.editor === "date" && initial == null;
    focusEditor(initial == null);
  }

  function acceptTypeahead(column) {
    if (column?.editor === "select" && menuOpen && selectActive) {
      draft = selectActive;
      return true;
    }
    if (!column || !typeQuery) return true;
    if (column.editor === "select") {
      if (selectActive) draft = selectActive;
      return true;
    }
    if (column.editor === "lookup") {
      const match = matchOption(column.options, typeQuery);
      if (!match) {
        cellError = "No matching record";
        return false;
      }
      draft = match;
      return true;
    }
    return true;
  }

  async function commitActive() {
    if (cancelEdit) {
      cancelEdit = false;
      editing = null;
      menuOpen = false;
      clearTypeahead();
      return true;
    }
    if (!editing) return true;
    const pendingColumn = columns[editing.col];
    if (pendingColumn && !acceptTypeahead(pendingColumn)) return false;
    const entry = view.find((item) => item.key === editing.key);
    const column = columns[editing.col];
    const original = editing.original;
    if (column?.editor === "date") {
      const live = document.querySelector("[data-sheet-editor] input");
      if (live instanceof HTMLInputElement) {
        if (live.dataset.calendarMoved === "1" && live.dataset.cursorIso) draft = live.dataset.cursorIso;
        else if (live.value.trim()) draft = live.value;
      }
    }
    const typed = draft;
    if (!entry || !column) {
      editing = null;
      menuOpen = false;
      clearTypeahead();
      return true;
    }
    const coerced = coerceCell(column, typed, workDate);
    if (coerced.error) {
      cellError = coerced.error;
      return false;
    }
    const message = checkCell(column, coerced.value, entry.isNew ? newDraft : entry.record);
    if (message) {
      cellError = message;
      return false;
    }
    if (same(original, coerced.value)) {
      editing = null;
      menuOpen = false;
      clearTypeahead();
      cellError = "";
      return true;
    }
    if (!entry.isNew && onCommitCell) {
      try {
        const result = await onCommitCell(entry.record, column.key, coerced.value, original);
        if (result && result.ok === false) {
          cellError = result.message || "This value was not saved";
          return false;
        }
      } catch (error) {
        cellError = error?.message || "This value was not saved";
        return false;
      }
    }
    writeValue(entry, column, coerced.value);
    if (!entry.isNew) dirty = { ...dirty, [entry.key]: true };
    editing = null;
    menuOpen = false;
    clearTypeahead();
    cellError = "";
    return true;
  }

  async function finishNewRow(nextKey) {
    if (focusKey !== NEW_ROW_KEY || nextKey === NEW_ROW_KEY) return true;
    const payload = draftPayload(newDraft);
    if (!Object.keys(payload).length) return true;
    for (const column of columns) {
      if (!(column.key in payload)) continue;
      const message = checkCell(column, payload[column.key], payload);
      if (message) {
        cellError = message;
        return false;
      }
    }
    try {
      if (onInsertRow) await onInsertRow(payload);
      else rows = [...rows, { [rowKey]: crypto.randomUUID(), ...payload }];
      newDraft = {};
      return true;
    } catch (error) {
      cellError = error?.message || "The new row was not saved";
      return false;
    }
  }

  function setFilter(key, value) {
    filters = { ...filters, [key]: value };
  }

  function clearFilters() {
    filters = {};
    query = "";
  }

  async function selectCell(entry, colIndex, extend = false) {
    if (!entry) return;
    if (!extend && editing?.key === entry.key && editing?.col === colIndex) return;
    if (extend) {
      focusKey = entry.key;
      focusCol = colIndex;
      return;
    }
    const saved = await commitActive();
    if (!saved) return;
    const inserted = await finishNewRow(entry.key);
    if (!inserted) return;
    focusKey = entry.key;
    focusCol = colIndex;
    anchorKey = entry.key;
    anchorCol = colIndex;
    selectedKey = entry.isNew ? null : entry.key;
    if (columns[colIndex]?.editor !== "checkbox") beginEdit(entry, colIndex);
  }

  async function moveTo(key, col, extend = false) {
    const saved = await commitActive();
    if (!saved) return;
    const inserted = await finishNewRow(key);
    if (!inserted) return;
    focusKey = key;
    focusCol = Math.max(0, Math.min(columns.length - 1, col));
    if (!extend) {
      anchorKey = key;
      anchorCol = focusCol;
    }
    const entry = view.find((item) => item.key === key);
    selectedKey = entry && !entry.isNew ? entry.key : null;
    reveal(key);
    await focusGrid();
  }

  async function moveBy(rowStep, colStep, extend = false) {
    if (!view.length || !columns.length) return;
    const saved = await commitActive();
    if (!saved) return;
    const row = Math.max(0, Math.min(view.length - 1, focusIndex + rowStep));
    const col = Math.max(0, Math.min(columns.length - 1, focusCol + colStep));
    await moveTo(view[row].key, col, extend);
  }

  async function moveEntry(step, mode) {
    if (!view.length || !columns.length) return;
    const saved = await commitActive();
    if (!saved) return;
    const current = view[focusIndex];
    const currentRow = current?.isNew ? newDraft : current?.record;
    const next = moveCell(columns, focusIndex, focusCol, view.length, step, mode, (column) => !columnLocked(column, currentRow) && column.quickEntry !== false);
    await moveTo(view[next.row].key, next.col, false);
  }

  async function toggleCheck(entry, column) {
    if (!editable || columnLocked(column, entry.isNew ? newDraft : entry.record)) return;
    const next = !readValue(entry, column);
    if (!entry.isNew && onCommitCell) {
      try {
        const result = await onCommitCell(entry.record, column.key, next, !next);
        if (result && result.ok === false) {
          cellError = result.message || "This value was not saved";
          return;
        }
      } catch (error) {
        cellError = error?.message || "This value was not saved";
        return;
      }
    }
    writeValue(entry, column, next);
    if (!entry.isNew) dirty = { ...dirty, [entry.key]: true };
    cellError = "";
  }

  async function copyAbove() {
    if (focusIndex <= 0) return;
    const column = columns[focusCol];
    const entry = view[focusIndex];
    if (!column || !entry || columnLocked(column, entry.isNew ? newDraft : entry.record)) return;
    const source = readValue(view[focusIndex - 1], column);
    draft = source == null ? "" : String(source);
    editing = { key: entry.key, col: focusCol, original: readValue(entry, column) ?? "" };
    await commitActive();
  }

  function currentMatrix(fullRows = false) {
    if (!fullRows) return matrix;
    const rowLines = [];
    for (let row = range.r1; row <= range.r2; row += 1) {
      const entry = view[row];
      if (!entry) continue;
      rowLines.push(columns.map((column) => readValue(entry, column)));
    }
    return rowLines;
  }

  async function writeClipboard(text) {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      cellError = "The browser blocked clipboard access";
    }
  }

  async function applyClipboard(text) {
    const plan = pastePlan(view.length, columns, { row: focusIndex, col: focusCol }, parseTsv(text));
    for (const step of plan) {
      const entry = view[step.rowIndex];
      const column = columns[step.colIndex];
      if (!entry || !column || columnLocked(column, entry.isNew ? newDraft : entry.record)) continue;
      const coerced = coerceCell(column, step.raw, workDate);
      if (coerced.error) {
        cellError = `${column.label}: ${coerced.error}`;
        await moveTo(entry.key, step.colIndex);
        return;
      }
      const message = checkCell(column, coerced.value, entry.isNew ? newDraft : entry.record);
      if (message) {
        cellError = `${column.label}: ${message}`;
        await moveTo(entry.key, step.colIndex);
        return;
      }
      const original = readValue(entry, column);
      if (same(original, coerced.value)) continue;
      if (!entry.isNew && onCommitCell) {
        try {
          const result = await onCommitCell(entry.record, column.key, coerced.value, original);
          if (result && result.ok === false) {
            cellError = result.message || "This value was not saved";
            return;
          }
        } catch (error) {
          cellError = error?.message || "This value was not saved";
          return;
        }
      }
      writeValue(entry, column, coerced.value);
      if (!entry.isNew) dirty = { ...dirty, [entry.key]: true };
    }
    cellError = "";
  }

  async function clearRange() {
    for (let row = range.r1; row <= range.r2; row += 1) {
      const entry = view[row];
      if (!entry) continue;
      for (let col = range.c1; col <= range.c2; col += 1) {
        const column = columns[col];
        if (!column || columnLocked(column, entry.isNew ? newDraft : entry.record)) continue;
        const original = readValue(entry, column);
        const next = column.editor === "checkbox" ? false : "";
        if (same(original, next)) continue;
        if (!entry.isNew && onCommitCell) {
          try {
            await onCommitCell(entry.record, column.key, next, original);
          } catch (error) {
            cellError = error?.message || "This value was not saved";
            return;
          }
        }
        writeValue(entry, column, next);
      }
    }
  }

  function stopFill() {
    fillListeners?.();
    fillListeners = null;
    fillTarget = null;
  }

  function startFill(event) {
    event.preventDefault();
    event.stopPropagation();
    fillTarget = range.r2;
    const move = (pointer) => {
      const hit = document.elementFromPoint(pointer.clientX, pointer.clientY);
      const row = hit?.closest?.("[data-sheet-row]");
      if (row) fillTarget = Number(row.dataset.sheetRow);
    };
    const up = async () => {
      const target = fillTarget;
      stopFill();
      if (target == null || target <= range.r2) return;
      const extra = target - range.r2;
      for (let col = range.c1; col <= range.c2; col += 1) {
        const column = columns[col];
        if (!column || column.editable === false) continue;
        const source = [];
        for (let row = range.r1; row <= range.r2; row += 1) source.push(readValue(view[row], column));
        const filled = extendFill(source, extra);
        for (let index = 0; index < filled.length; index += 1) {
          const entry = view[range.r2 + 1 + index];
          if (!entry || entry.isNew || columnLocked(column, entry.record)) continue;
          const original = readValue(entry, column);
          const coerced = coerceCell(column, filled[index], workDate);
          if (coerced.error) {
            cellError = coerced.error;
            return;
          }
          if (same(original, coerced.value)) continue;
          if (onCommitCell) {
            try {
              await onCommitCell(entry.record, column.key, coerced.value, original);
            } catch (error) {
              cellError = error?.message || "This value was not saved";
              return;
            }
          }
          writeValue(entry, column, coerced.value);
          dirty = { ...dirty, [entry.key]: true };
        }
      }
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    fillListeners = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }

  function startResize(event, column) {
    event.preventDefault();
    event.stopPropagation();
    const startX = event.clientX;
    const startW = widthOf(column);
    const move = (pointer) => {
      widths = { ...widths, [column.key]: Math.max(72, Math.round(startW + pointer.clientX - startX)) };
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function toggleSort(column) {
    if (!sortable) return;
    if (sortKey !== column.key) {
      sortKey = column.key;
      sortDir = "asc";
      return;
    }
    if (sortDir === "asc") sortDir = "desc";
    else sortKey = "";
  }

  async function removeRow(entry) {
    if (!entry || entry.isNew) {
      newDraft = {};
      return;
    }
    if (onDeleteRow) {
      try {
        const result = await onDeleteRow(entry.record);
        if (result && result.ok === false) cellError = result.message || "This row was not deleted";
      } catch (error) {
        cellError = error?.message || "This row was not deleted";
      }
      return;
    }
    rows = rows.filter((row) => identity(row, -1) !== entry.key);
  }

  function onGridKeydown(event) {
    const field = event.target?.closest?.("input, textarea, select, [data-sheet-editor]");
    if (field) return;
    if (!view.length || !columns.length) return;
    const key = event.key;
    if (event.altKey && event.ctrlKey && key === "ArrowDown") {
      event.preventDefault();
      const entry = view[focusIndex];
      if (entry && !entry.isNew && onOpenCard) onOpenCard(entry.record, columns[focusCol]);
    } else if (event.altKey && key === "ArrowUp") {
      event.preventDefault();
      const entry = view[focusIndex];
      cellError = problemOn(entry, columns[focusCol]) || "No message for this field";
    } else if (event.altKey && key === "F3") {
      event.preventDefault();
      const entry = view[focusIndex];
      const column = columns[focusCol];
      if (entry && column && !entry.isNew) setFilter(column.key, `=${displayCell(column, readValue(entry, column))}`);
    } else if (key === "F3") {
      event.preventDefault();
      const column = columns[focusCol];
      if (column && showFilter) filterOpen = column.key;
    } else if (event.altKey && !event.ctrlKey && key.toLowerCase() === "n") {
      event.preventDefault();
      const fresh = view.find((item) => item.isNew);
      if (!fresh) return;
      const col = Math.max(0, columns.findIndex((column) => column.editable !== false && column.quickEntry !== false));
      moveTo(fresh.key, col).then(() => beginEdit(view.find((item) => item.key === fresh.key) || fresh, col));
    } else if (key === "PageDown") {
      event.preventDefault();
      moveBy(pageStep(viewportHeight, rowHeight), 0, event.shiftKey);
    } else if (key === "PageUp") {
      event.preventDefault();
      moveBy(-pageStep(viewportHeight, rowHeight), 0, event.shiftKey);
    } else if ((key === "ArrowDown" || key === "ArrowUp") && menuOpen && !event.altKey) {
      event.preventDefault();
      if (columns[focusCol]?.editor === "select") stepSelect(key === "ArrowDown" ? 1 : -1);
    } else if (key === "ArrowDown" && !event.altKey) {
      event.preventDefault();
      moveBy(1, 0, event.shiftKey);
    } else if (key === "ArrowUp" && !event.altKey) {
      event.preventDefault();
      moveBy(-1, 0, event.shiftKey);
    } else if (key === "ArrowRight") {
      event.preventDefault();
      moveBy(0, 1, event.shiftKey);
    } else if (key === "ArrowLeft") {
      event.preventDefault();
      moveBy(0, -1, event.shiftKey);
    } else if (key === "Tab") {
      event.preventDefault();
      moveEntry(event.shiftKey ? -1 : 1, "any");
    } else if (key === "Enter") {
      event.preventDefault();
      moveEntry(event.shiftKey ? -1 : 1, "entry");
    } else if (key === "F2") {
      event.preventDefault();
      const entry = view[focusIndex];
      if (columns[focusCol]?.editor === "checkbox") toggleCheck(entry, columns[focusCol]);
      else beginEdit(entry, focusCol);
    } else if (key === "F4" || (event.altKey && key === "ArrowDown")) {
      event.preventDefault();
      beginEdit(view[focusIndex], focusCol);
    } else if (key === "F8") {
      event.preventDefault();
      copyAbove();
    } else if (key === "Escape") {
      event.preventDefault();
      cancelEdit = true;
      editing = null;
      menuOpen = false;
      clearTypeahead();
      cellError = "";
      focusGrid();
    } else if ((key === "Delete" || key === "Backspace") && !(event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      clearRange();
    } else if (key === "Delete" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      const entry = view[focusIndex];
      if (entry && !entry.isNew) removeRow(entry);
    } else if (key === "Home") {
      event.preventDefault();
      moveTo(view[event.ctrlKey ? 0 : focusIndex].key, 0, event.shiftKey);
    } else if (key === "End") {
      event.preventDefault();
      moveTo(view[event.ctrlKey ? view.length - 1 : focusIndex].key, columns.length - 1, event.shiftKey);
    } else if ((event.ctrlKey || event.metaKey) && event.shiftKey && key.toLowerCase() === "c") {
      event.preventDefault();
      writeClipboard(toTsv(currentMatrix(true)));
    } else if ((event.ctrlKey || event.metaKey) && key.toLowerCase() === "v") {
      event.preventDefault();
      navigator.clipboard.readText().then(applyClipboard).catch(() => {
        cellError = "Use Ctrl+V in the grid, or allow clipboard access";
      });
    } else if (key === " " && columns[focusCol]?.editor === "checkbox") {
      event.preventDefault();
      toggleCheck(view[focusIndex], columns[focusCol]);
    } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      beginEdit(view[focusIndex], focusCol, key);
    }
  }

  function onEditorKey(event) {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (menuOpen) {
        menuOpen = false;
        return;
      }
      cancelEdit = true;
      editing = null;
      menuOpen = false;
      clearTypeahead();
      cellError = "";
      focusGrid();
    } else if ((event.key === "ArrowDown" || event.key === "ArrowUp") && menuOpen && columns[editing?.col]?.editor === "select") {
      event.preventDefault();
      event.stopPropagation();
      stepSelect(event.key === "ArrowDown" ? 1 : -1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      if (menuOpen && columns[editing?.col]?.editor === "select" && selectActive) draft = selectActive;
      const live = event.currentTarget?.querySelector?.("input");
      if (menuOpen && columns[editing?.col]?.editor === "date" && live instanceof HTMLInputElement && live.dataset.calendarMoved === "1" && live.dataset.cursorIso) {
        draft = live.dataset.cursorIso;
      }
      moveEntry(event.shiftKey ? -1 : 1, "entry");
    } else if (event.key === "Tab") {
      event.preventDefault();
      event.stopPropagation();
      moveEntry(event.shiftKey ? -1 : 1, "any");
    } else if (event.key === "F8") {
      event.preventDefault();
      event.stopPropagation();
      copyAbove();
    } else if (event.key === "F2") {
      event.preventDefault();
      event.stopPropagation();
      const input = event.currentTarget.querySelector("input, textarea");
      if (!(input instanceof HTMLInputElement || input instanceof HTMLTextAreaElement)) return;
      const all = input.selectionStart === 0 && input.selectionEnd === input.value.length;
      if (all) input.setSelectionRange(input.value.length, input.value.length);
      else input.setSelectionRange(0, input.value.length);
    } else if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && !(menuOpen && columns[editing?.col]?.editor === "date")) {
      const input = event.target;
      if (!(input instanceof HTMLInputElement || input instanceof HTMLTextAreaElement)) return;
      const atStart = input.selectionStart === 0 && input.selectionEnd === 0;
      const atEnd = input.selectionStart === input.value.length && input.selectionEnd === input.value.length;
      if (event.key === "ArrowLeft" && atStart) {
        event.preventDefault();
        event.stopPropagation();
        moveEntry(-1, "any");
      } else if (event.key === "ArrowRight" && atEnd) {
        event.preventDefault();
        event.stopPropagation();
        moveEntry(1, "any");
      }
    } else if (event.key === "F4" || (event.altKey && event.key === "ArrowDown")) {
      event.preventDefault();
      event.stopPropagation();
      menuOpen = !menuOpen;
    }
  }

  function onCopy(event) {
    if (editing) return;
    event.preventDefault();
    event.clipboardData?.setData("text/plain", toTsv(matrix));
  }

  function onPaste(event) {
    if (editing) return;
    const text = event.clipboardData?.getData("text/plain");
    if (!text) return;
    event.preventDefault();
    applyClipboard(text);
  }

  function columnMenu(key) {
    const column = sourceColumns.find((item) => item.key === key);
    if (!column) return [];
    const visibleIndex = columns.findIndex((item) => item.key === key);
    const stops = layout?.quickEntry?.[key] != null ? layout.quickEntry[key] : column.quickEntry !== false;
    const hiddenOnes = sourceColumns.filter((item) => !item.keepVisible && (layout?.hidden || []).includes(item.key));
    const items = [];
    if (sortable) {
      items.push(
        { label: "Sort ascending", action: () => { sortKey = key; sortDir = "asc"; } },
        { label: "Sort descending", action: () => { sortKey = key; sortDir = "desc"; } },
        { type: "separator" }
      );
    }
    if (showFilter) items.push({ label: "Filter", action: () => { filterOpen = key; } });
    items.push(
      { label: stops ? "Enter skips this column" : "Enter stops on this column", action: () => toggleQuick(key) },
      { label: "Freeze through this column", disabled: visibleIndex < 0, action: () => freezeThrough(visibleIndex) },
      { label: "Move left", disabled: visibleIndex <= 0, action: () => moveColumn(key, -1) },
      { label: "Move right", disabled: visibleIndex < 0 || visibleIndex >= columns.length - 1, action: () => moveColumn(key, 1) },
      { label: "Hide column", disabled: Boolean(column.keepVisible), action: () => toggleColumn(key) }
    );
    if (hiddenOnes.length) {
      items.push({ type: "separator" });
      for (const hidden of hiddenOnes) items.push({ label: `Show ${hidden.label}`, action: () => toggleColumn(hidden.key) });
    }
    items.push({ type: "separator" }, { label: "Reset columns", action: resetLayout });
    return items;
  }

  let menuItems = $derived(headerKey ? columnMenu(headerKey) : [
    { label: "Copy cell", shortcut: "Ctrl+C", action: () => writeClipboard(toTsv(matrix)) },
    { label: "Copy field above", shortcut: "F8", action: copyAbove },
    { label: "Copy rows", shortcut: "Ctrl+Shift+C", action: () => writeClipboard(toTsv(currentMatrix(true))) },
    { type: "separator" },
    {
      label: "Paste",
      shortcut: "Ctrl+V",
      action: () => navigator.clipboard.readText().then(applyClipboard).catch(() => {
        cellError = "Use Ctrl+V in the grid, or allow clipboard access";
      })
    },
    { type: "separator" },
    { label: "Delete row", destructive: true, disabled: !activeEntry || activeEntry.isNew, action: () => removeRow(activeEntry) }
  ]);
</script>

{#snippet editor(entry, column, floating = true)}
  <div
    data-sheet-editor
    class={floating
      ? "absolute inset-0 z-20 flex items-stretch overflow-hidden bg-white dark:bg-slate-950 [&>*]:h-full [&>*]:min-w-0 [&>*]:w-full"
      : "relative min-w-0"}
    onmousedown={(event) => event.stopPropagation()}
    onclick={(event) => event.stopPropagation()}
    onkeydowncapture={onEditorKey}
  >
    {#if column.editor === "select"}
      <Select
        class="h-full w-full rounded-none border-0 bg-transparent px-2 shadow-none ring-0"
        bind:open={menuOpen}
        bind:value={draft}
        activeValue={selectActive}
        options={column.options || []}
        onValueChange={(next) => {
          draft = next;
          if (same(next, editing?.original)) return;
          commitActive();
        }}
      />
    {:else if column.editor === "date"}
      <DatePicker
        class={sheetEditorClass}
        bind:open={menuOpen}
        bind:value={draft}
        workDate={workDate}
        density="sm"
        clearable={false}
        onChange={(next) => {
          const value = next || "";
          draft = value;
          if (same(value, editing?.original)) return;
          commitActive();
        }}
      />
    {:else if column.editor === "lookup"}
      <Combobox
        class={sheetEditorClass}
        dropdownClass="w-80 max-w-80 sm:w-80 md:w-80"
        bind:open={menuOpen}
        bind:value={draft}
        bind:searchQuery={typeQuery}
        items={column.options || []}
        valueKey="value"
        labelKey="label"
        density="sm"
        showFactBox={false}
        clearable={false}
        onSelect={(item) => {
          const next = item?.value ?? item?.id ?? "";
          typeQuery = "";
          draft = next;
          if (same(next, editing?.original)) {
            editing = null;
            menuOpen = false;
            clearTypeahead();
            return;
          }
          commitActive();
        }}
      />
    {:else}
      <Input
        class={sheetEditorClass}
        bind:value={draft}
        type={column.editor === "code" ? "code" : "text"}
        size="sm"
        evalMath={column.editor === "number"}
      />
    {/if}
  </div>
{/snippet}

<div
  data-testid="datasheet"
  class={cn("flex h-full min-h-0 w-full min-w-0 max-w-full flex-col overflow-hidden rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950", className)}
>
  {#if workNote}
    <div class="px-2 pt-2">
      <Banner tone="warning" title={workNote} />
    </div>
  {/if}
  {#if sharedNote}
    <div class="px-2 pt-2">
      <Banner tone="warning" title={sharedNote} />
    </div>
  {/if}

  {#if issueCount}
    <div class="flex shrink-0 flex-wrap items-center gap-2 border-b border-slate-200 px-2 py-1 text-xs dark:border-slate-800">
      <span class="font-medium text-rose-700">{issueCount} {issueCount === 1 ? "issue" : "issues"}</span>
      <button type="button" class="rounded border border-slate-200 px-2 py-0.5 hover:bg-slate-50 dark:border-slate-700" onclick={() => (issuesOnly = !issuesOnly)}>
        {issuesOnly ? "Show every line" : "Show lines with issues"}
      </button>
    </div>
  {/if}

  {#if activeFilters.length}
    <div class="flex shrink-0 flex-wrap items-center gap-1 border-b border-slate-200 bg-slate-50 px-2 py-1 dark:border-slate-800 dark:bg-slate-900">
      {#each activeFilters as column (column.key)}
        <button
          type="button"
          class="inline-flex items-center gap-1 rounded-full border border-indigo-200 bg-white px-2 py-0.5 text-[11px] text-indigo-700 dark:border-indigo-900 dark:bg-slate-950 dark:text-indigo-200"
          onclick={() => setFilter(column.key, "")}
        >
          <span class="font-medium">{column.label}</span>
          <span class="max-w-32 truncate text-slate-500">{filters[column.key]}</span>
          <span aria-hidden="true">×</span>
        </button>
      {/each}
      <button type="button" class="px-1 text-[11px] text-slate-500 hover:text-slate-800" onclick={clearFilters}>Clear</button>
    </div>
  {/if}

  {#if cellError}
    <div class="px-2 pt-2">
      <Banner tone="danger" title={cellError} />
    </div>
  {/if}

  {#if narrow}
    <div class="shrink-0 border-b border-slate-200 px-2 py-1.5 dark:border-slate-800">
      <label class="sr-only" for="{label}-search">Search records</label>
      <input
        id="{label}-search"
        data-sheet-filter
        class="h-8 w-full rounded border border-slate-200 bg-white px-2 text-sm outline-none focus:border-indigo-400 dark:border-slate-700 dark:bg-slate-950"
        placeholder="Search records"
        bind:value={query}
      />
    </div>
    <div class="min-h-0 flex-1 space-y-2 overflow-auto p-2" role="list" aria-label={label}>
      {#if view.length === 0}
        <EmptyState title={emptyTitle} description={emptyDescription} />
      {:else}
        {#each view as entry (entry.key)}
          <article class="rounded-lg border border-slate-200 p-3 dark:border-slate-800" role="listitem">
            <header class="mb-2 text-xs font-semibold text-slate-500">
              {entry.isNew ? "New record" : `Record ${entry.key}`}
            </header>
            <div class="space-y-2">
              {#each columns as column, colIndex (column.key)}
                <div class="grid grid-cols-[7rem_1fr] items-center gap-2">
                  <span class="truncate text-xs text-slate-500">{column.label}</span>
                  {#if editing?.key === entry.key && editing?.col === colIndex}
                    {@render editor(entry, column, false)}
                  {:else if column.editor === "checkbox"}
                    <Checkbox
                      checked={Boolean(readValue(entry, column))}
                      disabled={!editable || columnLocked(column, entry.isNew ? newDraft : entry.record)}
                      onCheckedChange={() => toggleCheck(entry, column)}
                    />
                  {:else}
                    <button
                      type="button"
                      class={cn(
                        "truncate rounded px-2 py-1 text-left text-sm",
                        columnLocked(column, entry.isNew ? newDraft : entry.record) ? "bg-slate-100 text-slate-500 dark:bg-slate-900" : "bg-white dark:bg-slate-950"
                      )}
                      onclick={() => {
                        focusKey = entry.key;
                        focusCol = colIndex;
                        beginEdit(entry, colIndex);
                      }}
                    >
                      {displayCell(column, readValue(entry, column)) || "—"}
                    </button>
                  {/if}
                </div>
              {/each}
            </div>
          </article>
        {/each}
      {/if}
    </div>
  {:else}
    <ContextMenu items={menuItems}>
      <div
        bind:this={scrollEl}
        role="grid"
        aria-label={label}
        aria-rowcount={view.length}
        aria-colcount={columns.length}
        tabindex="0"
        class="min-h-0 w-full min-w-0 max-w-full flex-1 overflow-auto outline-none"
        onscroll={() => (scrollTop = scrollEl?.scrollTop || 0)}
        oncontextmenu={(event) => {
          const header = event.target instanceof Element ? event.target.closest("th[data-column-key]") : null;
          headerKey = header?.getAttribute("data-column-key") || "";
        }}
        onkeydown={onGridKeydown}
        oncopy={onCopy}
        onpaste={onPaste}
      >
        <table class="table-fixed border-separate border-spacing-0 text-xs" style={`width:${tableWidth}px;min-width:${tableWidth}px`}>
          <colgroup>
            <col style="width:36px" />
            {#each columns as column (column.key)}
              <col style={`width:${widthOf(column)}px`} />
            {/each}
          </colgroup>
          <thead>
            <tr>
              <th class="sticky top-0 left-0 z-30 h-8 w-9 border-b border-r border-slate-200 bg-slate-50 dark:border-slate-800 dark:bg-slate-900"></th>
              {#each columns as column, colIndex (column.key)}
                {@const left = frozenLeft(colIndex)}
                {@const filtered = String(filters[column.key] || "").trim()}
                <th
                  data-column-key={column.key}
                  class={cn(
                    "sticky top-0 z-20 h-8 overflow-hidden border-b border-r border-slate-200 bg-slate-50 text-left font-semibold text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300",
                    left != null && "z-30",
                    filtered && "text-indigo-700 dark:text-indigo-300"
                  )}
                  style={left != null ? `left:${left}px` : ""}
                >
                  <div class="flex h-8 items-center gap-0.5 px-1.5">
                    <button type="button" class={cn("flex min-w-0 flex-1 items-center gap-1 truncate text-left", !sortable && "cursor-default")} onclick={() => toggleSort(column)}>
                      <span class="truncate">{column.label}</span>
                      {#if column.required}<span class="text-rose-500" title="Required to post">*</span>{/if}
                      {#if sortKey === column.key}
                        <span class="text-[10px] text-indigo-500">{sortDir === "asc" ? "↑" : "↓"}</span>
                      {/if}
                    </button>
                    {#if showFilter}
                      <Popover.Root
                        open={filterOpen === column.key}
                        onOpenChange={(next) => {
                          filterOpen = next ? column.key : "";
                        }}
                      >
                        <Popover.Trigger
                          type="button"
                          class={cn(
                            "inline-flex size-5 shrink-0 items-center justify-center rounded text-slate-400 hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200",
                            filtered && "bg-indigo-50 text-indigo-600 dark:bg-indigo-950 dark:text-indigo-300"
                          )}
                          aria-label={`Filter ${column.label}`}
                        >
                          <ListFilter class="size-3" />
                        </Popover.Trigger>
                        <Popover.Portal>
                          <Popover.Content class="z-50 w-56 rounded-md border border-slate-200 bg-white p-2 shadow-lg dark:border-slate-700 dark:bg-slate-900" sideOffset={6}>
                            <p class="mb-1.5 text-[10px] font-semibold uppercase tracking-wide text-slate-400">Filter {column.label}</p>
                            {#if (column.editor === "select" || column.editor === "lookup") && column.options?.length}
                              <div class="max-h-48 space-y-0.5 overflow-auto">
                                <button type="button" class="block w-full rounded px-2 py-1 text-left text-xs hover:bg-slate-100 dark:hover:bg-slate-800" onclick={() => { setFilter(column.key, ""); filterOpen = ""; }}>All values</button>
                                {#each column.options as option (option.value)}
                                  <button
                                    type="button"
                                    class={cn("block w-full truncate rounded px-2 py-1 text-left text-xs hover:bg-slate-100 dark:hover:bg-slate-800", filters[column.key] === option.label && "bg-indigo-50 text-indigo-700")}
                                    onclick={() => { setFilter(column.key, option.label); filterOpen = ""; }}
                                  >
                                    {option.label}
                                  </button>
                                {/each}
                              </div>
                            {:else}
                              <input
                                data-sheet-filter
                                class="h-8 w-full rounded border border-slate-200 px-2 text-xs outline-none focus:border-indigo-400 dark:border-slate-700 dark:bg-slate-950"
                                placeholder="=EAST, 10..20, EAST|WEST"
                                value={filters[column.key] || ""}
                                oninput={(event) => setFilter(column.key, event.currentTarget.value)}
                              />
                            {/if}
                          </Popover.Content>
                        </Popover.Portal>
                      </Popover.Root>
                    {/if}
                  </div>
                  <span
                    class="absolute top-0 right-0 h-full w-1 cursor-col-resize"
                    role="separator"
                    aria-orientation="vertical"
                    aria-label={`Resize ${column.label}`}
                    onpointerdown={(event) => startResize(event, column)}
                  ></span>
                </th>
              {/each}
            </tr>
          </thead>
          <tbody>
            {#if view.length === 0}
              <tr>
                <td colspan={columns.length + 1}>
                  <EmptyState title={emptyTitle} description={emptyDescription} />
                </td>
              </tr>
            {:else}
              {#if windowed.padTop}
                <tr style={`height:${windowed.padTop}px`}><td colspan={columns.length + 1}></td></tr>
              {/if}
              {#each view.slice(windowed.start, windowed.end) as entry, offsetIndex (entry.key)}
                {@const rowIndex = windowed.start + offsetIndex}
                {@const rowActive = focusKey === entry.key}
                <tr data-sheet-row={rowIndex} style={`height:${rowHeight}px`} class="bg-white dark:bg-slate-950">
                  <td
                    class={cn(
                      "sticky left-0 z-10 border-b border-r border-slate-200 bg-slate-50 text-center text-[10px] text-slate-400 dark:border-slate-800 dark:bg-slate-900",
                      rowActive && "shadow-[inset_1px_0_0_0_#0284c7,inset_0_1px_0_0_#0284c7,inset_0_-1px_0_0_#0284c7] dark:shadow-[inset_1px_0_0_0_#38bdf8,inset_0_1px_0_0_#38bdf8,inset_0_-1px_0_0_#38bdf8]"
                    )}
                    title={entry.isNew ? "New record" : dirty[entry.key] ? "Unsaved in this session" : "Current record"}
                  >
                    {#if entry.isNew}
                      <span class="font-bold text-indigo-500">*</span>
                    {:else if dirty[entry.key]}
                      <Pencil class="mx-auto size-3 text-amber-500" aria-label="Changed" />
                    {:else if rowActive}
                      <span class="text-indigo-600" aria-hidden="true">▶</span>
                    {:else}
                      {offset + rowIndex + 1}
                    {/if}
                  </td>
                  {#each columns as column, colIndex (column.key)}
                    {@const left = frozenLeft(colIndex)}
                    {@const active = rowActive && focusCol === colIndex}
                    {@const selected = inRange(range, rowIndex, colIndex)}
                    {@const isEditing = editing?.key === entry.key && editing?.col === colIndex}
                    {@const locked = columnLocked(column, entry.isNew ? newDraft : entry.record)}
                    {@const problem = problemOn(entry, column)}
                    <td
                      role="gridcell"
                      aria-selected={selected}
                      aria-readonly={locked}
                      data-sheet-key={entry.key}
                      class={cn(
                        "relative h-8 overflow-hidden border-b border-r border-slate-100 bg-white px-0 align-middle dark:border-slate-800 dark:bg-slate-950",
                        locked && "bg-slate-100 text-slate-500 dark:bg-slate-900",
                        selected && !active && !locked && "ring-1 ring-inset ring-sky-300 dark:ring-sky-700",
                        active && "outline outline-2 outline-indigo-500 -outline-offset-2",
                        rowActive && colIndex === columns.length - 1 && "shadow-[inset_-1px_0_0_0_#0284c7,inset_0_1px_0_0_#0284c7,inset_0_-1px_0_0_#0284c7] dark:shadow-[inset_-1px_0_0_0_#38bdf8,inset_0_1px_0_0_#38bdf8,inset_0_-1px_0_0_#38bdf8]",
                        rowActive && colIndex !== columns.length - 1 && "shadow-[inset_0_1px_0_0_#0284c7,inset_0_-1px_0_0_#0284c7] dark:shadow-[inset_0_1px_0_0_#38bdf8,inset_0_-1px_0_0_#38bdf8]",
                        problem && "text-rose-700 italic dark:text-rose-300",
                        left != null && "sticky z-10"
                      )}
                      style={left != null ? `left:${left}px` : ""}
                      onclick={(event) => {
                        if (event.target instanceof Element && event.target.closest("[data-sheet-editor]")) return;
                        if (locked && column.drillable && onDrillDown && entry.record) onDrillDown(entry.record, column);
                        selectCell(entry, colIndex, event.shiftKey);
                      }}
                    >
                      {#if column.editor === "checkbox" && !isEditing}
                        <span onclick={(event) => event.stopPropagation()}>
                          <Checkbox
                            checked={Boolean(readValue(entry, column))}
                            disabled={!editable || locked}
                            onCheckedChange={() => {
                              focusKey = entry.key;
                              focusCol = colIndex;
                              anchorKey = entry.key;
                              anchorCol = colIndex;
                              toggleCheck(entry, column);
                            }}
                          />
                        </span>
                      {:else if isEditing}
                        {@render editor(entry, column)}
                      {:else}
                        <span class={cn("flex h-8 items-center gap-1 px-2", column.editor === "number" && "justify-end font-mono", problem && "text-rose-700 italic")} title={problem || column.hint || ""}>
                          <span class="min-w-0 truncate">{displayCell(column, readValue(entry, column))}</span>
                          {#if !locked && (column.editor === "select" || column.editor === "lookup" || column.editor === "date")}
                            <ChevronDown class="size-3 shrink-0 text-slate-400" />
                          {/if}
                        </span>
                      {/if}
                      {#if active && editable && !entry.isNew}
                        <button
                          type="button"
                          class="absolute bottom-0.5 right-0.5 z-20 size-1.5 cursor-crosshair border border-white bg-indigo-600"
                          aria-label="Fill down"
                          onclick={(event) => event.stopPropagation()}
                          onpointerdown={startFill}
                        ></button>
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
              {#if windowed.padBottom}
                <tr style={`height:${windowed.padBottom}px`}><td colspan={columns.length + 1}></td></tr>
              {/if}
            {/if}
          </tbody>
          {#if showTotals}
          <tfoot class="sticky bottom-0 z-20">
            <tr class="bg-slate-50 font-medium text-slate-600 dark:bg-slate-900 dark:text-slate-300">
              <td class="sticky left-0 border-t border-r border-slate-200 bg-slate-50 px-1 text-center dark:border-slate-800 dark:bg-slate-900">Σ</td>
              {#each columns as column, colIndex (column.key)}
                {@const total = columnTotal(view.filter((entry) => !entry.isNew).map((entry) => entry.record), column)}
                {@const left = frozenLeft(colIndex)}
                <td
                  class={cn("border-t border-r border-slate-200 px-2 text-right font-mono dark:border-slate-800", left != null && "sticky bg-slate-50 dark:bg-slate-900")}
                  style={left != null ? `left:${left}px` : ""}
                >
                  {total == null ? "" : formatNumber(total, column.decimals)}
                </td>
              {/each}
            </tr>
          </tfoot>
          {/if}
        </table>
      </div>
    </ContextMenu>
  {/if}

  {#if showStatusBar}
    <div class="flex shrink-0 items-stretch border-t border-slate-200 dark:border-slate-800">
      <StatusBar
        class="flex-1 border-t-0"
        leading={[
          { id: "pos", label: view.length ? `Record ${offset + Math.min(focusIndex, Math.max(rows.length - 1, 0)) + 1} of ${recordTotal}` : `0 of ${recordTotal}` }
        ]}
        trailing={stats.count > 1 ? [
          {
            id: "stats",
            label: stats.numeric
              ? `Count ${stats.count} · Sum ${formatNumber(stats.sum, 2)} · Avg ${formatNumber(stats.average, 2)}`
              : `Count ${stats.count}`
          }
        ] : []}
      />
      {#if onPreviousPage || onNextPage}
        <div class="flex items-center gap-1 bg-slate-50 px-1 dark:bg-slate-900">
          <Button size="xs" variant="ghost" iconOnly icon={ChevronLeft} aria-label="Previous page" disabled={!onPreviousPage || offset === 0} onclick={() => onPreviousPage?.()} />
          <Button size="xs" variant="ghost" iconOnly icon={ChevronRight} aria-label="Next page" disabled={!onNextPage} onclick={() => onNextPage?.()} />
        </div>
      {/if}
    </div>
  {/if}
</div>
