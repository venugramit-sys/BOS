<script>
  import { cn } from "$lib/utils.js";
  import Input from "../input/Input.svelte";
  import Select from "../select/Select.svelte";
  import Checkbox from "../checkbox/Checkbox.svelte";
  import DatePicker from "../date-picker/DatePicker.svelte";
  import Combobox from "../combobox/Combobox.svelte";
  import EmptyState from "../empty-state/EmptyState.svelte";

  let {
    columns = [],
    rows = $bindable([]),
    rowKey = "id",
    label = "Grid",
    onCellChange = null,
    selectedKey = $bindable(null),
    emptyTitle = "No rows",
    emptyDescription = "",
    class: className = ""
  } = $props();

  const rowHeight = 36;
  let scrollEl = $state(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(280);
  let focus = $state({ row: 0, col: 0 });
  let editing = $state(null);
  let draft = $state("");

  let windowed = $derived.by(() => {
    if (rows.length < 40) {
      return { start: 0, end: rows.length, padTop: 0, padBottom: 0 };
    }
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 4);
    const count = Math.ceil(viewportHeight / rowHeight) + 8;
    const end = Math.min(rows.length, start + count);
    return {
      start,
      end,
      padTop: start * rowHeight,
      padBottom: Math.max(0, (rows.length - end) * rowHeight)
    };
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

  function columnAt(col) {
    return columns[col];
  }

  function displayValue(row, column) {
    const raw = row?.[column.key];
    if (raw == null || raw === "") return "";
    if (column.editor === "select" || column.editor === "lookup") {
      const match = (column.options || []).find((option) => String(option.value) === String(raw));
      return match?.label || String(raw);
    }
    return String(raw);
  }

  function beginEdit(rowIndex, colIndex, initial) {
    const column = columnAt(colIndex);
    const row = rows[rowIndex];
    if (!column || !row || column.editable === false || column.editor === "checkbox") return;
    if (editing && editing.row === rowIndex && editing.col === colIndex) return;
    if (editing) commit();
    const current = row[column.key];
    draft = initial != null ? initial : current == null ? "" : String(current);
    editing = { row: rowIndex, col: colIndex, original: current ?? "" };
    focus = { row: rowIndex, col: colIndex };
    selectedKey = row[rowKey] ?? null;
  }

  function commit() {
    if (!editing) return;
    const column = columnAt(editing.col);
    const row = rows[editing.row];
    const original = editing.original;
    editing = null;
    if (!column || !row) return;
    let next = draft;
    if (column.editor === "number") {
      if (next === "") next = "";
      else {
        const parsed = Number(next);
        next = Number.isNaN(parsed) ? original : parsed;
      }
    }
    if (next !== original) {
      row[column.key] = next;
      onCellChange?.(row, column.key, next);
    }
  }

  function cancel() {
    editing = null;
  }

  function reveal(rowIndex) {
    if (!scrollEl) return;
    const top = rowIndex * rowHeight;
    if (top < scrollEl.scrollTop) scrollEl.scrollTop = top;
    if (top + rowHeight > scrollEl.scrollTop + scrollEl.clientHeight) {
      scrollEl.scrollTop = top + rowHeight - scrollEl.clientHeight;
    }
  }

  function move(dRow, dCol) {
    if (!rows.length || !columns.length) return;
    commit();
    const row = Math.min(rows.length - 1, Math.max(0, focus.row + dRow));
    const col = Math.min(columns.length - 1, Math.max(0, focus.col + dCol));
    focus = { row, col };
    selectedKey = rows[row]?.[rowKey] ?? null;
    reveal(row);
  }

  function toggleCheck(rowIndex, colIndex) {
    const column = columnAt(colIndex);
    const row = rows[rowIndex];
    if (!column || !row) return;
    const next = !row[column.key];
    row[column.key] = next;
    onCellChange?.(row, column.key, next);
  }

  function onKeydown(event) {
    const tag = event.target?.tagName;
    const inField = tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
    if (inField) {
      if (event.key === "Escape") {
        event.preventDefault();
        cancel();
      } else if (event.key === "Enter") {
        event.preventDefault();
        const row = focus.row;
        commit();
        focus = { row: Math.min(rows.length - 1, row + 1), col: focus.col };
      }
      return;
    }
    if (!rows.length) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      move(1, 0);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      move(-1, 0);
    } else if (event.key === "ArrowRight" || (event.key === "Tab" && !event.shiftKey)) {
      event.preventDefault();
      move(0, 1);
    } else if (event.key === "ArrowLeft" || (event.key === "Tab" && event.shiftKey)) {
      event.preventDefault();
      move(0, -1);
    } else if (event.key === "F2" || event.key === "Enter") {
      event.preventDefault();
      if (columnAt(focus.col)?.editor === "checkbox") toggleCheck(focus.row, focus.col);
      else beginEdit(focus.row, focus.col);
    } else if (event.key === " " && columnAt(focus.col)?.editor === "checkbox") {
      event.preventDefault();
      toggleCheck(focus.row, focus.col);
    } else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      beginEdit(focus.row, focus.col, event.key);
    }
  }
</script>

<div class={cn("flex h-full min-h-0 flex-col border border-slate-200 dark:border-slate-800 rounded-lg overflow-hidden bg-white dark:bg-slate-950", className)}>
  <div
    bind:this={scrollEl}
    role="grid"
    aria-label={label}
    aria-rowcount={rows.length}
    tabindex="0"
    class="min-h-0 flex-1 overflow-auto outline-none"
    onscroll={() => (scrollTop = scrollEl?.scrollTop || 0)}
    onkeydown={onKeydown}
  >
    <table class="w-full border-collapse text-xs">
      <thead class="sticky top-0 z-10 bg-slate-50 dark:bg-slate-900">
        <tr>
          {#each columns as column (column.key)}
            <th
              class="h-8 px-2 text-left font-semibold text-slate-500 border-b border-slate-200 dark:border-slate-800"
              style={column.width ? `width:${column.width}` : ""}
            >
              {column.label}
            </th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#if rows.length === 0}
          <tr>
            <td colspan={Math.max(columns.length, 1)}>
              <EmptyState title={emptyTitle} description={emptyDescription} />
            </td>
          </tr>
        {:else}
          {#if windowed.padTop}
            <tr style={`height:${windowed.padTop}px`}><td colspan={columns.length}></td></tr>
          {/if}
          {#each rows.slice(windowed.start, windowed.end) as row, offset (row[rowKey] ?? windowed.start + offset)}
            {@const rowIndex = windowed.start + offset}
            <tr style={`height:${rowHeight}px`} role="row" aria-rowindex={rowIndex + 1}>
              {#each columns as column, colIndex (column.key)}
                {@const active = focus.row === rowIndex && focus.col === colIndex}
                {@const isEditing = editing?.row === rowIndex && editing?.col === colIndex}
                <td
                  role="gridcell"
                  class={cn(
                    "px-1 border-b border-slate-100 dark:border-slate-800 align-middle",
                    active && "outline outline-2 outline-indigo-500 -outline-offset-2"
                  )}
                  onclick={() => {
                    focus = { row: rowIndex, col: colIndex };
                    selectedKey = row[rowKey] ?? null;
                    if (column.editor !== "checkbox") beginEdit(rowIndex, colIndex);
                  }}
                >
                  {#if column.editor === "checkbox"}
                    <Checkbox
                      checked={Boolean(row[column.key])}
                      onCheckedChange={(next) => {
                        row[column.key] = next;
                        onCellChange?.(row, column.key, next);
                      }}
                    />
                  {:else if isEditing && column.editor === "select"}
                    <Select
                      bind:value={draft}
                      options={column.options || []}
                      onValueChange={(next) => {
                        draft = next;
                        commit();
                      }}
                    />
                  {:else if isEditing && column.editor === "date"}
                    <DatePicker bind:value={draft} density="sm" onChange={(next) => { draft = next; commit(); }} />
                  {:else if isEditing && column.editor === "lookup"}
                    <Combobox
                      bind:value={draft}
                      items={column.options || []}
                      valueKey="value"
                      labelKey="label"
                      density="sm"
                      onSelect={(item) => {
                        draft = item?.value ?? item?.id ?? "";
                        commit();
                      }}
                    />
                  {:else if isEditing}
                    <Input
                      bind:value={draft}
                      type={column.editor === "number" ? "number" : "text"}
                      size="sm"
                    />
                  {:else}
                    <span class={cn("block truncate px-1", column.editor === "number" && "text-right font-mono")}>
                      {displayValue(row, column)}
                    </span>
                  {/if}
                </td>
              {/each}
            </tr>
          {/each}
          {#if windowed.padBottom}
            <tr style={`height:${windowed.padBottom}px`}><td colspan={columns.length}></td></tr>
          {/if}
        {/if}
      </tbody>
    </table>
  </div>
</div>
