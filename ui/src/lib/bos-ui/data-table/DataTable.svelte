<script>
  import { createTable } from "@tanstack/svelte-table";
  import {
    ArrowDown,
    ArrowUp,
    ArrowUpDown,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight,
    Columns3,
    Download,
    Filter,
    Pin,
    Search
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import { themeStore } from "../theme/index.js";
  import Badge from "../badge/Badge.svelte";
  import Banner from "../banner/Banner.svelte";
  import Button from "../button/Button.svelte";
  import Checkbox from "../checkbox/Checkbox.svelte";
  import Combobox from "../combobox/Combobox.svelte";
  import ContextMenu from "../context-menu/ContextMenu.svelte";
  import DatePicker from "../date-picker/DatePicker.svelte";
  import Dialog from "../dialog/Dialog.svelte";
  import EmptyState from "../empty-state/EmptyState.svelte";
  import Input from "../input/Input.svelte";
  import Select from "../select/Select.svelte";
  import Spinner from "../spinner/Spinner.svelte";
  import Switch from "../switch/Switch.svelte";
  import Toolbar from "../toolbar/Toolbar.svelte";
  import { bosTableFeatures } from "./features.js";
  import {
    badgeTone,
    buildColumnDefs,
    buildInitialState,
    downloadCsv,
    formatValue,
    isUtilityKind,
    tableToCsv
  } from "./columns.js";
  import { loadTableState, saveTableState } from "./state.js";

  let {
    columns = [],
    rows = $bindable([]),
    rowKey = "id",
    label = "Table",
    selectable = true,
    expandable = false,
    searchable = true,
    filterable = true,
    sortable = true,
    paginated = true,
    resizable = true,
    exportable = true,
    showColumnControls = true,
    showTotals = true,
    pageSize = 20,
    pageSizes = [10, 20, 50, 100],
    manualSorting = false,
    manualFiltering = false,
    manualPagination = false,
    rowCount = undefined,
    stateKey = "",
    loading = false,
    error = "",
    emptyTitle = "No records",
    emptyDescription = "Change the search or filters to see more records.",
    onCellChange = null,
    onRowActivate = null,
    onSelectionChange = null,
    onQueryChange = null,
    contextItems = null,
    getSubRows = null,
    class: className = "",
    api = $bindable(null),
    cell = null,
    expanded = null,
    toolbar = null,
    batch = null,
    empty = null
  } = $props();

  const saved = loadTableState(stateKey);
  const initiallyExpandable = expandable || typeof expanded === "function";
  let canExpand = $derived(expandable || typeof expanded === "function");

  let columnDefs = $derived(
    buildColumnDefs(columns, { selectable, expandable: canExpand })
  );
  let query = $state("");
  let filtersOpen = $state(false);
  let columnsOpen = $state(false);
  let editing = $state(null);
  let draft = $state("");
  let editorRef = $state(null);
  let active = $state({ rowId: null, columnId: null });

  const table = createTable({
    features: bosTableFeatures,
    get data() {
      return rows;
    },
    get columns() {
      return columnDefs;
    },
    initialState: buildInitialState(columns, { selectable, expandable: initiallyExpandable, pageSize }, saved),
    columnResizeMode: "onChange",
    enableSortingRemoval: true,
    enableMultiSort: true,
    autoResetPageIndex: false,
    groupedColumnMode: false,
    filterFromLeafRows: true,
    defaultColumn: { minSize: 96, size: 160, maxSize: 720 },
    globalFilterFn: "includesString",
    get enableSorting() {
      return sortable;
    },
    get enableFilters() {
      return filterable || searchable;
    },
    get enableColumnFilters() {
      return filterable;
    },
    get enableGlobalFilter() {
      return searchable;
    },
    get enableColumnResizing() {
      return resizable;
    },
    get enableRowSelection() {
      return selectable;
    },
    get enableMultiRowSelection() {
      return selectable;
    },
    get manualSorting() {
      return manualSorting;
    },
    get manualFiltering() {
      return manualFiltering;
    },
    get manualPagination() {
      return manualPagination;
    },
    get rowCount() {
      return rowCount;
    },
    getRowId: (original, index, parent) => {
      const raw = original?.[rowKey];
      const own = raw == null || raw === "" ? String(index) : String(raw);
      return parent ? `${parent.id}.${own}` : own;
    },
    getSubRows: (original, index) => {
      if (getSubRows) return getSubRows(original, index);
      return Array.isArray(original?.subRows) ? original.subRows : undefined;
    },
    getRowCanExpand: (row) => {
      if (row.subRows?.length) return true;
      return canExpand;
    }
  });

  let state = $derived(table.store.get());
  let pagination = $derived(state.pagination || { pageIndex: 0, pageSize: 20 });
  let grouping = $derived(state.grouping || []);
  let columnFilters = $derived(state.columnFilters || []);
  let selectedRows = $derived.by(() => {
    void state.rowSelection;
    return table.getSelectedRowModel().rows;
  });
  let leafColumns = $derived.by(() => {
    void state.columnOrder;
    void state.columnVisibility;
    return table.getAllLeafColumns();
  });
  let visibleColumns = $derived.by(() => {
    void state.columnVisibility;
    void state.columnOrder;
    void state.columnPinning;
    void state.columnSizing;
    return table.getVisibleLeafColumns();
  });
  let groupOptions = $derived([
    { value: "none", label: "No grouping" },
    ...leafColumns
      .filter((column) => column.getCanGroup())
      .map((column) => ({
        value: column.id,
        label: String(column.columnDef.header || column.id)
      }))
  ]);
  let pageSizeOptions = $derived.by(() => {
    const sizes = new Set(pageSizes);
    if (Number.isFinite(pagination.pageSize) && pagination.pageSize < 100000) sizes.add(pagination.pageSize);
    return [...sizes]
      .sort((left, right) => left - right)
      .map((size) => ({ value: String(size), label: `${size} / page` }));
  });
  let hasTotals = $derived(
    showTotals && visibleColumns.some((column) => column.columnDef.meta?.aggregate)
  );
  let density = $derived(themeStore.density || "md");

  const rowHeights = { sm: "min-h-7", md: "min-h-9", lg: "min-h-11" };

  $effect(() => {
    api = table;
  });

  $effect(() => {
    if (!paginated) table.setPageSize(100000);
  });

  $effect(() => {
    if (!editing) return;
    queueMicrotask(() => editorRef?.focus?.());
  });

  let lastQuery = "";
  $effect(() => {
    if (!onQueryChange) return;
    const next = {
      sorting: state.sorting || [],
      columnFilters: state.columnFilters || [],
      globalFilter: state.globalFilter || "",
      pagination: state.pagination
    };
    const key = JSON.stringify(next);
    if (key === lastQuery) return;
    lastQuery = key;
    onQueryChange(next);
  });

  let lastSelection = "";
  $effect(() => {
    if (!onSelectionChange) return;
    const key = selectedRows.map((row) => row.id).join("|");
    if (key === lastSelection) return;
    lastSelection = key;
    onSelectionChange(selectedRows.map((row) => row.original));
  });

  $effect(() => {
    if (!stateKey) return;
    saveTableState(stateKey, state);
  });

  let view = $derived.by(() => {
    void state;
    return {
      headers: table.getHeaderGroups(),
      top: table.getTopRows(),
      center: table.getCenterRows(),
      bottom: table.getBottomRows(),
      rowCount: table.getRowCount(),
      pageCount: table.getPageCount(),
      totalSize: table.getTotalSize(),
      canPrevious: table.getCanPreviousPage(),
      canNext: table.getCanNextPage(),
      canLast: table.getCanLastPage(),
      allPageSelected: table.getIsAllPageRowsSelected(),
      somePageSelected: table.getIsSomePageRowsSelected()
    };
  });

  let rowStart = $derived(view.rowCount === 0 ? 0 : pagination.pageIndex * pagination.pageSize + 1);
  let rowEnd = $derived(Math.min(view.rowCount, (pagination.pageIndex + 1) * pagination.pageSize));

  function listedRows() {
    return [...view.top, ...view.center, ...view.bottom];
  }

  function replaceOriginal(list, target, next) {
    let changed = false;
    const mapped = list.map((item) => {
      if (item === target) {
        changed = true;
        return next;
      }
      if (Array.isArray(item?.subRows)) {
        const subRows = replaceOriginal(item.subRows, target, next);
        if (subRows !== item.subRows) {
          changed = true;
          return { ...item, subRows };
        }
      }
      return item;
    });
    return changed ? mapped : list;
  }

  function writeCell(row, column, value) {
    const meta = column.columnDef.meta || {};
    const key = meta.accessorKey || column.id;
    const previous = row.original?.[key];
    if (Object.is(previous, value)) return;
    const nextOriginal = { ...row.original, [key]: value };
    rows = replaceOriginal(rows, row.original, nextOriginal);
    onCellChange?.(nextOriginal, key, value, previous);
  }

  function beginEdit(row, column, seed) {
    const meta = column.columnDef.meta || {};
    if (!meta.editable || isUtilityKind(meta.kind)) return;
    if (meta.kind === "checkbox" || meta.kind === "switch" || meta.editor === "checkbox" || meta.editor === "switch") return;
    const current = row.getValue(column.id);
    draft = seed != null ? seed : current == null ? "" : String(current);
    editing = { rowId: row.id, columnId: column.id };
    active = { rowId: row.id, columnId: column.id };
  }

  function commit() {
    if (!editing) return;
    const row = table.getRow(editing.rowId);
    const column = table.getColumn(editing.columnId);
    const meta = column?.columnDef.meta || {};
    const next = editing;
    editing = null;
    if (!row || !column) return;
    let value = draft;
    if (meta.kind === "number" || meta.editor === "number") {
      if (draft === "") value = null;
      else {
        const parsed = Number(draft);
        if (Number.isNaN(parsed)) return;
        value = parsed;
      }
    }
    writeCell(row, column, value);
    void next;
  }

  function cancelEdit() {
    editing = null;
  }

  function toggleBoolean(row, column) {
    const current = Boolean(row.getValue(column.id));
    writeCell(row, column, !current);
  }

  function copyAbove(row, column) {
    const visible = listedRows();
    const index = visible.findIndex((item) => item.id === row.id);
    if (index <= 0) return;
    writeCell(row, column, visible[index - 1].getValue(column.id));
  }

  function stickyStyle(column, size) {
    const width = size ?? column.getSize();
    const pinned = column.getIsPinned();
    const parts = [`width:${width}px`, `min-width:${width}px`, `max-width:${width}px`];
    if (pinned === "start") parts.push("position:sticky", `left:${column.getStart("start")}px`);
    if (pinned === "end") parts.push("position:sticky", `right:${column.getAfter("end")}px`);
    return parts.join(";");
  }

  function edgeClass(column) {
    const pinned = column.getIsPinned();
    if (pinned === "start") {
      const start = table.getStartVisibleLeafColumns();
      if (start.at(-1)?.id === column.id) return "shadow-[4px_0_8px_-6px_rgba(15,23,42,0.45)]";
    }
    if (pinned === "end") {
      const end = table.getEndVisibleLeafColumns();
      if (end[0]?.id === column.id) return "shadow-[-4px_0_8px_-6px_rgba(15,23,42,0.45)]";
    }
    return "";
  }

  function headerItems(header) {
    const column = header.column;
    const items = [];
    if (column.getCanSort()) {
      items.push(
        { label: "Sort ascending", action: () => column.toggleSorting(false) },
        { label: "Sort descending", action: () => column.toggleSorting(true) },
        { label: "Clear sort", action: () => column.clearSorting() },
        { type: "separator" }
      );
    }
    if (column.getCanPin()) {
      items.push(
        { label: "Pin to start", action: () => column.pin("start") },
        { label: "Pin to end", action: () => column.pin("end") },
        { label: "Unpin column", disabled: !column.getIsPinned(), action: () => column.pin(false) }
      );
    }
    if (column.getCanGroup()) {
      items.push({
        label: column.getIsGrouped() ? "Stop grouping" : "Group by this column",
        action: () => column.toggleGrouping()
      });
    }
    if (column.getCanHide()) {
      items.push({ label: "Hide column", action: () => column.toggleVisibility(false) });
    }
    if (column.getCanResize()) {
      items.push({ label: "Reset width", action: () => column.resetSize() });
    }
    if (column.getIsFiltered?.()) {
      items.push({ label: "Clear filter", action: () => column.setFilterValue(undefined) });
    }
    return items;
  }

  function rowItems(row) {
    const items = [];
    if (onRowActivate) items.push({ label: "Open", action: () => onRowActivate(row.original) });
    if (row.getCanPin()) {
      items.push(
        { label: "Pin to top", action: () => row.pin("top") },
        { label: "Pin to bottom", action: () => row.pin("bottom") },
        { label: "Unpin row", disabled: !row.getIsPinned(), action: () => row.pin(false) }
      );
    }
    const extra = contextItems?.(row.original) || [];
    if (items.length && extra.length) items.push({ type: "separator" });
    return items.concat(extra);
  }

  function moveActive(dRow, dCol) {
    const visible = listedRows();
    const cols = visibleColumns.filter((column) => !isUtilityKind(column.columnDef.meta?.kind));
    if (!visible.length || !cols.length) return;
    let rowIndex = Math.max(0, visible.findIndex((row) => row.id === active.rowId));
    let colIndex = Math.max(0, cols.findIndex((column) => column.id === active.columnId));
    if (active.rowId == null) rowIndex = 0;
    if (active.columnId == null) colIndex = 0;
    rowIndex = Math.min(visible.length - 1, Math.max(0, rowIndex + dRow));
    colIndex = Math.min(cols.length - 1, Math.max(0, colIndex + dCol));
    if (dRow > 0 && rowIndex === visible.length - 1 && table.getCanNextPage()) {
      table.nextPage();
    }
    if (dRow < 0 && rowIndex === 0 && active.rowId === visible[0]?.id && table.getCanPreviousPage()) {
      table.previousPage();
    }
    active = { rowId: visible[rowIndex].id, columnId: cols[colIndex].id };
  }

  function onGridKeydown(event) {
    const tag = event.target?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    const visible = listedRows();
    const row = visible.find((item) => item.id === active.rowId) || visible[0];
    const column = table.getColumn(active.columnId) || visibleColumns.find((item) => !isUtilityKind(item.columnDef.meta?.kind));
    if (!row || !column) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      moveActive(1, 0);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      moveActive(-1, 0);
    } else if (event.key === "ArrowRight" || (event.key === "Tab" && !event.shiftKey)) {
      event.preventDefault();
      moveActive(0, 1);
    } else if (event.key === "ArrowLeft" || (event.key === "Tab" && event.shiftKey)) {
      event.preventDefault();
      moveActive(0, -1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (column.columnDef.meta?.editable) beginEdit(row, column);
      else onRowActivate?.(row.original);
    } else if (event.key === "F2") {
      event.preventDefault();
      beginEdit(row, column);
    } else if (event.key === "F8") {
      event.preventDefault();
      copyAbove(row, column);
    } else if (event.key === " " && (column.columnDef.meta?.kind === "checkbox" || column.columnDef.meta?.editor === "checkbox")) {
      event.preventDefault();
      toggleBoolean(row, column);
    } else if (event.key === "Escape") {
      cancelEdit();
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      beginEdit(row, column, event.key);
    }
  }

  function filterChoices(column) {
    const meta = column.columnDef.meta || {};
    if (meta.options?.length) {
      return [{ value: "*", label: "Any" }, ...meta.options.map((option) => ({ value: String(option.value), label: option.label ?? String(option.value) }))];
    }
    const unique = column.getFacetedUniqueValues?.();
    if (!unique) return [{ value: "*", label: "Any" }];
    return [
      { value: "*", label: "Any" },
      ...[...unique.keys()]
        .filter((value) => value != null && value !== "")
        .slice(0, 40)
        .map((value) => ({ value: String(value), label: String(value) }))
    ];
  }

  function moveColumn(id, direction) {
    const current = state.columnOrder?.length ? [...state.columnOrder] : leafColumns.map((column) => column.id);
    const index = current.indexOf(id);
    const next = index + direction;
    if (index < 0 || next < 0 || next >= current.length) return;
    const neighbor = leafColumns.find((column) => column.id === current[next]);
    if (neighbor && isUtilityKind(neighbor.columnDef.meta?.kind)) return;
    const [item] = current.splice(index, 1);
    current.splice(next, 0, item);
    table.setColumnOrder(current);
  }

  function resetLayout() {
    table.resetColumnVisibility();
    table.resetColumnOrder();
    table.resetColumnSizing();
    table.resetColumnPinning();
    if (stateKey) localStorage.removeItem(`bos.table.${stateKey}`);
  }

  function exportView() {
    const safe = String(label || "table").trim().toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "table";
    downloadCsv(`${safe}.csv`, tableToCsv(table));
  }

  function alignClass(meta) {
    if (meta?.align === "right" || meta?.kind === "number") return "text-right";
    if (meta?.align === "center") return "text-center";
    return "text-left";
  }

  function footerValue(column) {
    const aggregate = column.columnDef.meta?.aggregate;
    if (!aggregate) return "";
    const values = [];
    const walk = (list) => {
      for (const row of list) {
        if (row.getIsGrouped?.() && row.subRows?.length) walk(row.subRows);
        else values.push(row.getValue(column.id));
      }
    };
    walk(table.getFilteredRowModel().rows);
    if (aggregate === "count") return formatValue(values.length, { kind: "number", decimals: 0 });
    if (aggregate === "uniqueCount") return formatValue(new Set(values.map((value) => String(value ?? ""))).size, { kind: "number", decimals: 0 });
    const nums = values.map(Number).filter((value) => !Number.isNaN(value));
    if (!nums.length) return "";
    if (aggregate === "sum") return formatValue(nums.reduce((sum, value) => sum + value, 0), column.columnDef.meta);
    if (aggregate === "mean") return formatValue(nums.reduce((sum, value) => sum + value, 0) / nums.length, column.columnDef.meta);
    if (aggregate === "min") return formatValue(Math.min(...nums), column.columnDef.meta);
    if (aggregate === "max") return formatValue(Math.max(...nums), column.columnDef.meta);
    return "";
  }
</script>

{#snippet dataCell(row, gridCell)}
  {@const column = gridCell.column}
  {@const meta = column.columnDef.meta || {}}
  {@const value = gridCell.getValue()}
  {@const isEditing = editing?.rowId === row.id && editing?.columnId === column.id}
  {#if gridCell.getIsPlaceholder?.()}
    <span></span>
  {:else if gridCell.getIsGrouped?.()}
    <button
      type="button"
      class="inline-flex items-center gap-1 font-medium text-slate-800 dark:text-slate-100"
      style={`padding-left:${row.depth * 12}px`}
      onclick={() => row.toggleExpanded()}
    >
      {#if row.getIsExpanded()}
        <ChevronDown class="size-3.5" />
      {:else}
        <ChevronRight class="size-3.5" />
      {/if}
      <span class="truncate">{formatValue(row.groupingValue ?? value, meta) || "(Blank)"}</span>
      <Badge tone="neutral">{row.subRows.length}</Badge>
    </button>
  {:else if gridCell.getIsAggregated?.() && !isEditing}
    <span class={cn("block truncate px-1 font-medium", alignClass(meta))}>{formatValue(value, meta)}</span>
  {:else if meta.kind === "custom" && cell}
    {@render cell({ row: row.original, column: meta, value, tableRow: row })}
  {:else if typeof meta.cell === "function"}
    {meta.cell({ row: row.original, column: meta, value })}
  {:else if meta.kind === "checkbox" || meta.editor === "checkbox"}
    <span data-stop-row>
      <Checkbox
        checked={Boolean(value)}
        disabled={meta.editable === false}
        onCheckedChange={(next) => writeCell(row, column, next)}
      />
    </span>
  {:else if meta.kind === "switch" || meta.editor === "switch"}
    <span data-stop-row>
      <Switch checked={Boolean(value)} disabled={meta.editable === false} onCheckedChange={(next) => writeCell(row, column, next)} />
    </span>
  {:else if isEditing && (meta.editor === "select" || meta.filter === "select")}
    <span data-stop-row>
      <Select
        bind:value={draft}
        options={meta.options || []}
        onValueChange={(next) => {
          draft = next;
          commit();
        }}
      />
    </span>
  {:else if isEditing && (meta.editor === "date" || meta.kind === "date")}
    <span data-stop-row>
      <DatePicker bind:value={draft} density="sm" onChange={(next) => { draft = next; commit(); }} />
    </span>
  {:else if isEditing && (meta.editor === "lookup" || meta.kind === "lookup")}
    <span data-stop-row>
      <Combobox
        bind:value={draft}
        items={meta.options || []}
        valueKey="value"
        labelKey="label"
        density="sm"
        onSelect={(item) => {
          draft = item?.value ?? "";
          commit();
        }}
      />
    </span>
  {:else if isEditing}
    <div
      data-stop-row
      role="group"
      onkeydown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          cancelEdit();
        } else if (event.key === "F8") {
          event.preventDefault();
          const previous = listedRows()[listedRows().findIndex((item) => item.id === row.id) - 1];
          if (previous) draft = previous.getValue(column.id) == null ? "" : String(previous.getValue(column.id));
        }
      }}
    >
      <Input
        bind:ref={editorRef}
        bind:value={draft}
        type="text"
        inputmode="decimal"
        size="sm"
        onEnter={commit}
      />
    </div>
  {:else if meta.kind === "badge"}
    <Badge tone={badgeTone(meta, value, row.original)} dot>{formatValue(value, meta) || "—"}</Badge>
  {:else}
    <span class={cn("block truncate px-1", alignClass(meta), meta.kind === "number" && "font-mono")}>
      {formatValue(value, meta)}
    </span>
  {/if}
{/snippet}

{#snippet recordRow(row, striped)}
  {@const pinned = row.getIsPinned()}
  <ContextMenu items={rowItems(row)}>
    <tr
      class={cn(
        rowHeights[density] || rowHeights.md,
        "border-b border-slate-100 dark:border-slate-800",
        striped && "bg-slate-50/70 dark:bg-slate-900/40",
        pinned && "bg-amber-50/80 dark:bg-amber-950/30",
        row.getIsSelected() && "bg-indigo-50 dark:bg-indigo-950/40",
        active.rowId === row.id && "bg-indigo-50/70 dark:bg-indigo-950/30"
      )}
      ondblclick={() => onRowActivate?.(row.original)}
    >
      {#each row.getVisibleCells() as gridCell (gridCell.id)}
        {@const meta = gridCell.column.columnDef.meta || {}}
        {@const focused = active.rowId === row.id && active.columnId === gridCell.column.id}
        <td
          role="gridcell"
          class={cn(
            "px-2 py-1 align-middle bg-white dark:bg-slate-950",
            striped && "bg-slate-50/70 dark:bg-slate-900/40",
            row.getIsSelected() && "bg-indigo-50 dark:bg-indigo-950/40",
            focused && "outline outline-2 outline-indigo-500 -outline-offset-2",
            edgeClass(gridCell.column)
          )}
          style={stickyStyle(gridCell.column)}
          onclick={(event) => {
            if (event.target?.closest?.("[data-stop-row], button, a, input")) return;
            active = { rowId: row.id, columnId: gridCell.column.id };
            if (meta.editable) beginEdit(row, gridCell.column);
          }}
          onkeydown={(event) => {
            if (event.key !== "Enter" && event.key !== "F2") return;
            active = { rowId: row.id, columnId: gridCell.column.id };
            if (meta.editable) beginEdit(row, gridCell.column);
          }}
        >
          {#if meta.kind === "selection"}
            <span data-stop-row>
              <Checkbox checked={row.getIsSelected()} onCheckedChange={(next) => row.toggleSelected(next)} />
            </span>
          {:else if meta.kind === "expansion"}
            {#if row.getCanExpand()}
              <button type="button" class="inline-flex text-slate-500" aria-label={row.getIsExpanded() ? "Collapse row" : "Expand row"} onclick={() => row.toggleExpanded()}>
                {#if row.getIsExpanded()}
                  <ChevronDown class="size-3.5" />
                {:else}
                  <ChevronRight class="size-3.5" />
                {/if}
              </button>
            {/if}
          {:else}
            {@render dataCell(row, gridCell)}
          {/if}
        </td>
      {/each}
    </tr>
  </ContextMenu>
  {#if expanded && row.getIsExpanded() && !row.getIsGrouped()}
    <tr>
      <td colspan={visibleColumns.length} class="border-b border-slate-100 bg-slate-50 px-3 py-2 dark:border-slate-800 dark:bg-slate-900/50">
        {@render expanded({ row: row.original, tableRow: row })}
      </td>
    </tr>
  {/if}
{/snippet}

<div class={cn("flex h-full min-h-96 min-w-0 flex-col overflow-hidden rounded-lg border border-slate-200 bg-white dark:border-slate-800 dark:bg-slate-950", className)}>
  {#if error}
    <div class="p-2">
      <Banner tone="danger" title="This list could not be loaded">{error}</Banner>
    </div>
  {/if}

  <Toolbar label={label} class="h-auto min-h-10 flex-wrap gap-1 py-1">
    {#if searchable}
      <div class="w-56 max-w-full">
        <Input
          bind:value={query}
          size="sm"
          placeholder={`Search ${label}`}
          prefixIcon={Search}
          clearable
          onDebounce={(next) => table.setGlobalFilter(next || undefined)}
          onClear={() => table.setGlobalFilter(undefined)}
        />
      </div>
    {/if}
    {#if filterable}
      <Button size="xs" variant={filtersOpen || columnFilters.length ? "secondary" : "ghost"} icon={Filter} onclick={() => (filtersOpen = !filtersOpen)}>
        Filters
        {#if columnFilters.length}
          <Badge tone="info">{columnFilters.length}</Badge>
        {/if}
      </Button>
    {/if}
    {#if groupOptions.length > 1}
      <div class="w-40">
        <Select
          value={grouping[0] || "none"}
          options={groupOptions}
          placeholder="Group by"
          onValueChange={(next) => table.setGrouping(!next || next === "none" ? [] : [next])}
        />
      </div>
    {/if}
    {#if grouping.length || canExpand}
      <Button size="xs" variant="ghost" onclick={() => table.toggleAllRowsExpanded(true)}>Expand</Button>
      <Button size="xs" variant="ghost" onclick={() => table.toggleAllRowsExpanded(false)}>Collapse</Button>
    {/if}
    <span class="hidden h-4 w-px bg-slate-200 sm:block dark:bg-slate-700"></span>
    {#if showColumnControls}
      <Button size="xs" variant="ghost" icon={Columns3} onclick={() => (columnsOpen = true)}>Columns</Button>
    {/if}
    {#if exportable}
      <Button size="xs" variant="ghost" icon={Download} onclick={exportView}>Export</Button>
    {/if}
    {#if toolbar}
      {@render toolbar()}
    {/if}
  </Toolbar>

  {#if selectable && selectedRows.length}
    <div class="flex flex-wrap items-center gap-2 border-b border-indigo-100 bg-indigo-50 px-2 py-1.5 dark:border-indigo-900 dark:bg-indigo-950/40">
      <Badge tone="info" dot>{selectedRows.length} selected</Badge>
      {#if batch}
        {@render batch({ rows: selectedRows.map((row) => row.original) })}
      {/if}
      <Button size="xs" variant="ghost" onclick={() => table.toggleAllRowsSelected(false)}>Clear</Button>
    </div>
  {/if}

  <div
    class="relative min-h-0 flex-1 overflow-auto outline-none"
    role="grid"
    tabindex="0"
    aria-label={label}
    aria-rowcount={view.rowCount}
    aria-colcount={visibleColumns.length}
    aria-busy={loading || undefined}
    onkeydown={onGridKeydown}
  >
    <table class="border-collapse text-xs" style={`width:${Math.max(view.totalSize, 640)}px;min-width:100%`}>
      <thead class="sticky top-0 z-20 bg-slate-50 dark:bg-slate-900">
        {#each view.headers as headerGroup (headerGroup.id)}
          <tr>
            {#each headerGroup.headers as header (header.id)}
              {#if header.rowSpan !== 0}
                {@const column = header.column}
                {@const sorted = column.getIsSorted()}
                {@const showLabel = !header.isPlaceholder || header.rowSpan > 1}
                <th
                  colspan={header.colSpan}
                  rowspan={header.rowSpan}
                  aria-sort={showLabel ? (sorted === "asc" ? "ascending" : sorted === "desc" ? "descending" : "none") : undefined}
                  class={cn(
                    "relative border-b border-slate-200 bg-slate-50 px-2 text-left align-middle font-semibold text-slate-500 dark:border-slate-800 dark:bg-slate-900",
                    header.rowSpan > 1 ? "h-16" : "h-8",
                    alignClass(column.columnDef.meta),
                    edgeClass(column)
                  )}
                  style={stickyStyle(column, header.getSize())}
                >
                  {#if showLabel}
                  {#if column.columnDef.meta?.kind === "selection"}
                    <Checkbox
                      checked={view.allPageSelected}
                      indeterminate={!view.allPageSelected && view.somePageSelected}
                      onCheckedChange={(next) => table.toggleAllPageRowsSelected(next)}
                    />
                  {:else if column.columnDef.meta?.kind === "expansion"}
                    <span class="sr-only">Expand</span>
                  {:else}
                    <ContextMenu items={headerItems(header)}>
                      <button
                        type="button"
                        class={cn("inline-flex max-w-full items-center gap-1", alignClass(column.columnDef.meta))}
                        onclick={(event) => column.getCanSort() && column.getToggleSortingHandler()?.(event)}
                      >
                        <span class="truncate">{column.columnDef.header}</span>
                        {#if column.getCanSort()}
                          {#if sorted === "asc"}
                            <ArrowUp class="size-3 shrink-0" />
                          {:else if sorted === "desc"}
                            <ArrowDown class="size-3 shrink-0" />
                          {:else}
                            <ArrowUpDown class="size-3 shrink-0 opacity-40" />
                          {/if}
                          {#if column.getSortIndex() > 0}
                            <span class="text-[10px]">{column.getSortIndex() + 1}</span>
                          {/if}
                        {/if}
                        {#if column.getIsPinned()}
                          <Pin class="size-3 shrink-0 text-indigo-500" />
                        {/if}
                      </button>
                    </ContextMenu>
                  {/if}
                  {/if}
                  {#if showLabel && resizable && column.getCanResize() && !column.columns?.length}
                    <div
                      role="separator"
                      aria-orientation="vertical"
                      aria-label={`Resize ${column.columnDef.header || column.id}`}
                      class={cn("absolute top-0 right-0 z-10 h-full w-1 cursor-col-resize touch-none", column.getIsResizing() && "bg-indigo-500")}
                      onpointerdown={(event) => {
                        event.preventDefault();
                        event.stopPropagation();
                        header.getResizeHandler()(event);
                      }}
                    ></div>
                  {/if}
                </th>
              {/if}
            {/each}
          </tr>
        {/each}
        {#if filterable && filtersOpen}
          <tr>
            {#each visibleColumns as column (column.id)}
              <th class={cn("border-b border-slate-200 bg-white px-1 py-1 dark:border-slate-800 dark:bg-slate-950", edgeClass(column))} style={stickyStyle(column)}>
                {#if column.getCanFilter()}
                  {#if column.columnDef.meta?.filter === "select"}
                    <Select
                      value={column.getFilterValue() == null || column.getFilterValue() === "" ? "*" : String(column.getFilterValue())}
                      options={filterChoices(column)}
                      onValueChange={(next) => column.setFilterValue(!next || next === "*" ? undefined : next)}
                    />
                  {:else}
                    <Input
                      value={column.getFilterValue() ?? ""}
                      size="sm"
                      placeholder={column.columnDef.meta?.kind === "number" ? ">100 or 10..20" : "Contains"}
                      onDebounce={(next) => column.setFilterValue(next || undefined)}
                    />
                  {/if}
                {/if}
              </th>
            {/each}
          </tr>
        {/if}
      </thead>
      <tbody>
        {#if loading && view.rowCount === 0}
          <tr>
            <td colspan={Math.max(visibleColumns.length, 1)} class="py-16 text-center text-slate-500">
              <Spinner label="Loading records" />
            </td>
          </tr>
        {:else if view.rowCount === 0}
          <tr>
            <td colspan={Math.max(visibleColumns.length, 1)}>
              {#if empty}
                {@render empty()}
              {:else}
                <EmptyState title={emptyTitle} description={emptyDescription} />
              {/if}
            </td>
          </tr>
        {:else}
          {#each view.top as row (row.id)}
            {@render recordRow(row, false)}
          {/each}
          {#each view.center as row, index (row.id)}
            {@render recordRow(row, index % 2 === 1)}
          {/each}
          {#each view.bottom as row (row.id)}
            {@render recordRow(row, false)}
          {/each}
        {/if}
      </tbody>
      {#if hasTotals}
        <tfoot class="sticky bottom-0 z-20 bg-slate-50 dark:bg-slate-900">
          <tr>
            {#each visibleColumns as column, index (column.id)}
              <td
                class={cn("h-8 border-t border-slate-200 px-2 font-semibold text-slate-700 dark:border-slate-800 dark:text-slate-200", alignClass(column.columnDef.meta), edgeClass(column))}
                style={stickyStyle(column)}
              >
                {#if index === 0}
                  Total
                {:else}
                  {footerValue(column)}
                {/if}
              </td>
            {/each}
          </tr>
        </tfoot>
      {/if}
    </table>
    {#if loading && view.rowCount > 0}
      <div class="pointer-events-none absolute top-2 right-2 rounded-md bg-white/90 p-1 text-indigo-600 shadow-sm dark:bg-slate-900/90">
        <Spinner label="Refreshing" />
      </div>
    {/if}
  </div>

  <footer class="flex flex-wrap items-center justify-between gap-2 border-t border-slate-200 px-2 py-1.5 text-[11px] text-slate-500 dark:border-slate-800">
    <div class="flex items-center gap-2">
      <span>{rowStart}–{rowEnd} of {view.rowCount}</span>
      <span class="hidden text-slate-400 md:inline">Arrows move · Enter edits · F8 copies the cell above</span>
    </div>
    {#if paginated}
      <div class="flex items-center gap-1">
        <div class="w-28">
          <Select
            value={String(pagination.pageSize)}
            options={pageSizeOptions}
            onValueChange={(next) => table.setPageSize(Number(next))}
          />
        </div>
        <Button size="xs" variant="ghost" icon={ChevronsLeft} iconOnly aria-label="First page" disabled={!view.canPrevious} onclick={() => table.setPageIndex(0)} />
        <Button size="xs" variant="ghost" icon={ChevronLeft} iconOnly aria-label="Previous page" disabled={!view.canPrevious} onclick={() => table.previousPage()} />
        <span class="px-1">Page {pagination.pageIndex + 1} of {Math.max(view.pageCount, 1)}</span>
        <Button size="xs" variant="ghost" icon={ChevronRight} iconOnly aria-label="Next page" disabled={!view.canNext} onclick={() => table.nextPage()} />
        <Button size="xs" variant="ghost" icon={ChevronsRight} iconOnly aria-label="Last page" disabled={!view.canLast} onclick={() => table.setPageIndex(view.pageCount - 1)} />
      </div>
    {/if}
  </footer>
</div>

<Dialog bind:open={columnsOpen} title="Columns" description="Choose which columns stay visible, where they sit, and their order." size="md">
  <div class="space-y-2">
    {#each leafColumns as column, index (column.id)}
      {#if !isUtilityKind(column.columnDef.meta?.kind)}
        <div class="flex items-center gap-2">
          <Checkbox
            checked={column.getIsVisible()}
            disabled={!column.getCanHide()}
            onCheckedChange={(next) => column.toggleVisibility(next)}
          />
          <span class="min-w-0 flex-1 truncate text-xs text-slate-700 dark:text-slate-200">{column.columnDef.header || column.id}</span>
          <div class="w-28">
            <Select
              value={column.getIsPinned() || "none"}
              options={[
                { value: "none", label: "Unpinned" },
                { value: "start", label: "Pin start" },
                { value: "end", label: "Pin end" }
              ]}
              onValueChange={(next) => column.pin(next === "none" ? false : next)}
            />
          </div>
          <Button size="xs" variant="ghost" icon={ChevronLeft} iconOnly aria-label="Move column left" disabled={index === 0} onclick={() => moveColumn(column.id, -1)} />
          <Button size="xs" variant="ghost" icon={ChevronRight} iconOnly aria-label="Move column right" disabled={index === leafColumns.length - 1} onclick={() => moveColumn(column.id, 1)} />
        </div>
      {/if}
    {/each}
  </div>
  {#snippet footer()}
    <Button size="xs" variant="ghost" onclick={resetLayout}>Reset layout</Button>
    <Button size="xs" onclick={() => (columnsOpen = false)}>Done</Button>
  {/snippet}
</Dialog>
