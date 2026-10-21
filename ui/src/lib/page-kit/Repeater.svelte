<script>
  import { untrack } from "svelte";
  /**
   * `repeater(...)`: the records of a list-like page.
   * - List / ListPart / Worksheet: a table from `sm` up, cards on phones; sort, search, paging,
   *   selection and inline editing (Worksheet, ListPart, lists without a card page, new rows).
   * - Board: lanes by the first option (enum) column; drag a card or pick its lane to move it.
   * - Timeline: rows ordered by the first date column, grouped by day.
   */
  import { ArrowDown, ArrowUp, ArrowUpDown, ChevronLeft, ChevronRight, Search, Trash2 } from "lucide-svelte";
  import { Checkbox } from "$lib/bos-ui/checkbox/index.js";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { EmptyState } from "$lib/bos-ui/empty-state/index.js";
  import FieldInput from "./FieldInput.svelte";
  import { formatValue, formatDate } from "./format.js";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, columns = [] } = $props();

  const type = $derived(p.meta.page_type);
  const layout = $derived(type === "Board" ? "board" : type === "Timeline" ? "timeline" : "table");
  const selectable = $derived((type === "List" || type === "Worksheet") && !p.embedded);
  const dateCol = $derived(columns.find((x) => x.field && (x.field.control === "date" || x.field.control === "dateTime")));
  const laneCol = $derived(columns.find((x) => x.field?.control === "select" && x.field.options?.length) ?? null);
  // Timelines read newest first; set before the controller's first load runs.
  untrack(() => {
    if (layout === "timeline" && dateCol && !p.sort) p.sort = `${dateCol.field.key}:desc`;
  });

  const shown = $derived(isShown(p, c, visible));
  const cols = $derived(columns.filter((x) => x.field && isShown(p, x, undefined)));
  const titleCol = $derived(cols[0] ?? null);
  const inlineEdit = (row) => p.inlineEdit || row._new;
  const sortKey = $derived(p.sort.split(":")[0]);
  const sortDir = $derived(p.sort.split(":")[1] ?? "asc");
  const first = $derived(p.total === 0 ? 0 : (p.pageNo - 1) * p.limit + 1);
  const last = $derived(Math.min(p.pageNo * p.limit, p.total));
  const allSelected = $derived(p.rows.length > 0 && p.rows.every((r) => p.selected.includes(r._id)));

  let searchText = $state(untrack(() => p.search));
  let searchTimer;
  function onSearch(value) {
    searchText = value;
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => p.applySearch(value.trim()), 300);
  }

  function sortable(col) {
    return col.field.binding === "column";
  }

  function toggle(row) {
    p.selected = p.selected.includes(row._id) ? p.selected.filter((id) => id !== row._id) : [...p.selected, row._id];
  }

  function toggleAll() {
    p.selected = allSelected ? [] : p.rows.map((r) => r._id);
  }

  function onRowKey(event, row, index) {
    if (event.target !== event.currentTarget) return;
    if (event.key === "Enter") {
      event.preventDefault();
      p.openRow(row);
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const next = p.rows[index + (event.key === "ArrowDown" ? 1 : -1)];
      if (next) {
        p.currentId = next._id;
        event.currentTarget.parentElement?.children[index + (event.key === "ArrowDown" ? 1 : -1)]?.focus();
      }
    } else if (event.key === " " && selectable) {
      event.preventDefault();
      toggle(row);
    }
  }

  function cellText(col, row) {
    return formatValue(col.field, p.valueOf(col.field, row));
  }

  // ----- board -----
  const lanes = $derived(laneCol ? laneCol.field.options : []);
  let dragId = $state(null);
  let dropLane = $state(null);

  function rowsOfLane(value) {
    return p.rows.filter((r) => String(p.valueOf(laneCol.field, r) ?? "") === String(value));
  }

  async function moveCard(row, value) {
    if (!row || String(p.valueOf(laneCol.field, row) ?? "") === String(value)) return;
    if (!p.editable(laneCol, row)) return;
    await p.commit(laneCol, value, row);
    await p.saveRows();
  }

  // ----- timeline -----
  const days = $derived.by(() => {
    if (!dateCol) return [{ day: "", rows: p.rows }];
    const groups = [];
    for (const row of p.rows) {
      const v = String(p.valueOf(dateCol.field, row) ?? "");
      const day = v.slice(0, 10);
      const g = groups.at(-1);
      if (g && g.day === day) g.rows.push(row);
      else groups.push({ day, rows: [row] });
    }
    return groups;
  });
</script>

{#if shown}
  <section class={boxClass(p, c, "bos-repeater")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if !p.isList}
      <div class="bos-notice" role="status">A repeater lists records on list pages only; use a part to show related lines here.</div>
    {:else}
      {#if p.meta.show_filter && !p.design}
        <div class="bos-repeater-toolbar">
          <label class="bos-search">
            <Search class="size-4 shrink-0 text-[var(--bos-color-subordinate)]" />
            <span class="sr-only">Search</span>
            <input type="search" placeholder="Search…" value={searchText} oninput={(e) => onSearch(e.currentTarget.value)} />
          </label>
          {#if p.searchFields.length > 1}
            <label class="flex items-center gap-1 text-xs text-[var(--bos-color-subordinate)]">
              in
              <select class="bos-native-input bos-native-sm w-auto" bind:value={p.searchField} onchange={() => p.search && p.applySearch(p.search)} aria-label="Search column">
                {#each p.searchFields as f (f.key)}<option value={f.key}>{f.caption}</option>{/each}
              </select>
            </label>
          {/if}
          {#if p.loading}<Spinner size="sm" />{/if}
          <span class="ml-auto text-xs text-[var(--bos-color-subordinate)]">{p.total} record{p.total === 1 ? "" : "s"}</span>
        </div>
      {/if}

      {#if p.rows.length === 0}
        {#if p.loading}
          <div class="flex justify-center py-10"><Spinner /></div>
        {:else}
          <EmptyState title="Nothing to show" description={p.search ? "No record matches the search." : "There are no records yet."} />
        {/if}
      {:else if layout === "board" && laneCol}
        <div class="bos-board" role="list">
          {#each lanes as lane (lane.value)}
            <div
              class={"bos-lane " + (dropLane === lane.value ? "is-drop" : "")}
              role="listitem"
              aria-label={lane.caption}
              ondragover={(e) => {
                if (dragId) {
                  e.preventDefault();
                  dropLane = lane.value;
                }
              }}
              ondragleave={() => (dropLane = null)}
              ondrop={(e) => {
                e.preventDefault();
                const row = p.rows.find((r) => r._id === dragId);
                dragId = null;
                dropLane = null;
                moveCard(row, lane.value);
              }}
            >
              <h4 class="bos-lane-title">{lane.caption}<span>{rowsOfLane(lane.value).length}</span></h4>
              {#each rowsOfLane(lane.value) as row (row._id)}
                <article
                  class={"bos-board-card " + (p.currentId === row._id ? "is-current" : "")}
                  draggable={p.editable(laneCol, row)}
                  ondragstart={(e) => {
                    dragId = row._id;
                    e.dataTransfer.effectAllowed = "move";
                  }}
                  ondragend={() => (dragId = null)}
                >
                  <button type="button" class="bos-board-card-title" onclick={() => p.openRow(row)}>{titleCol ? cellText(titleCol, row) : row._id}</button>
                  {#each cols.slice(1).filter((x) => x !== laneCol) as col (col.i)}
                    <div class="bos-kv"><span>{col.caption ?? col.name}</span><span class={col.field.control === "number" ? "bos-num" : ""}>{cellText(col, row)}</span></div>
                  {/each}
                  {#if p.editable(laneCol, row)}
                    <label class="bos-kv">
                      <span>{laneCol.caption ?? laneCol.name}</span>
                      <select class="bos-native-input bos-native-sm" value={p.valueOf(laneCol.field, row) ?? ""} onchange={(e) => moveCard(row, e.currentTarget.value)}>
                        {#each lanes as l (l.value)}<option value={l.value}>{l.caption}</option>{/each}
                      </select>
                    </label>
                  {/if}
                </article>
              {/each}
            </div>
          {/each}
        </div>
      {:else if layout === "timeline"}
        <ol class="bos-timeline">
          {#each days as g (g.day)}
            <li>
              {#if g.day}<h4 class="bos-timeline-day">{formatDate(g.day)}</h4>{/if}
              <ul>
                {#each g.rows as row (row._id)}
                  <li class="bos-timeline-item">
                    <span class="bos-timeline-dot" aria-hidden="true"></span>
                    <button type="button" class="min-w-0 flex-1 text-left" onclick={() => p.openRow(row)}>
                      {#each cols.filter((x) => x !== dateCol) as col, i (col.i)}
                        {#if i === 0}
                          <span class="block font-medium">{cellText(col, row)}</span>
                        {:else}
                          <span class="mr-3 text-xs text-[var(--bos-color-subordinate)]">{col.caption ?? col.name}: {cellText(col, row)}</span>
                        {/if}
                      {/each}
                      {#if dateCol && dateCol.field.control === "dateTime"}
                        <span class="text-xs text-[var(--bos-color-subordinate)]">{cellText(dateCol, row)}</span>
                      {/if}
                    </button>
                  </li>
                {/each}
              </ul>
            </li>
          {/each}
        </ol>
      {:else}
        <!-- table (sm and up) -->
        <div class="bos-table-wrap">
          <table class="bos-table">
            <thead>
              <tr>
                {#if selectable}
                  <th class="w-8"><Checkbox checked={allSelected} onCheckedChange={toggleAll} aria-label="Select all" /></th>
                {/if}
                {#each cols as col (col.i)}
                  <th class={col.field.control === "number" ? "text-right" : ""} {...nodeAttrs(p, col)} aria-sort={sortKey === col.field.key ? (sortDir === "desc" ? "descending" : "ascending") : undefined}>
                    {#if sortable(col) && !p.design}
                      <button type="button" class="bos-th-sort" onclick={() => p.setSort(col.field.key)}>
                        {col.caption ?? col.name}
                        {#if sortKey === col.field.key}
                          {#if sortDir === "desc"}<ArrowDown class="size-3" />{:else}<ArrowUp class="size-3" />{/if}
                        {:else}
                          <ArrowUpDown class="size-3 opacity-40" />
                        {/if}
                      </button>
                    {:else}
                      {col.caption ?? col.name}
                    {/if}
                  </th>
                {/each}
                {#if p.meta.delete_allowed && p.inlineEdit && !p.design}<th class="w-8"><span class="sr-only">Actions</span></th>{/if}
              </tr>
            </thead>
            <tbody>
              {#each p.rows as row, index (row._id)}
                <tr
                  class={p.currentId === row._id ? "is-current" : ""}
                  tabindex="0"
                  aria-selected={p.currentId === row._id}
                  onclick={() => (p.currentId = row._id)}
                  ondblclick={() => p.openRow(row)}
                  onkeydown={(e) => onRowKey(e, row, index)}
                >
                  {#if selectable}
                    <td><Checkbox checked={p.selected.includes(row._id)} onCheckedChange={() => toggle(row)} aria-label="Select row" /></td>
                  {/if}
                  {#each cols as col, ci (col.i)}
                    <td class={col.field.control === "number" ? "text-right bos-num" : ""}>
                      {#if inlineEdit(row) && p.editable(col, row)}
                        <FieldInput {p} c={col} {row} compact label={col.caption ?? col.name} />
                      {:else if ci === 0 && p.meta.card_page && !row._new && !p.design}
                        <button type="button" class="bos-link font-medium" onclick={() => p.openRow(row)}>{cellText(col, row) || "—"}</button>
                      {:else}
                        <FieldInput {p} c={col} {row} compact readonly label={col.caption ?? col.name} />
                      {/if}
                    </td>
                  {/each}
                  {#if p.meta.delete_allowed && p.inlineEdit && !p.design}
                    <td>
                      <button type="button" class="bos-icon-btn" aria-label="Delete row" onclick={(e) => { e.stopPropagation(); row._new ? (p.rows = p.rows.filter((r) => r._id !== row._id)) : p.deleteCurrent(row); }}>
                        <Trash2 class="size-3.5" />
                      </button>
                    </td>
                  {/if}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
        <!-- cards (phones) -->
        <ul class="bos-row-cards">
          {#each p.rows as row (row._id)}
            <li class={p.currentId === row._id ? "is-current" : ""}>
              <div class="flex items-start gap-2">
                {#if selectable}<Checkbox checked={p.selected.includes(row._id)} onCheckedChange={() => toggle(row)} aria-label="Select row" />{/if}
                <button type="button" class="min-w-0 flex-1 text-left font-medium bos-link" onclick={() => p.openRow(row)}>{titleCol ? cellText(titleCol, row) || "—" : row._id}</button>
              </div>
              {#each cols.slice(1) as col (col.i)}
                <div class="bos-kv">
                  <span>{col.caption ?? col.name}</span>
                  {#if inlineEdit(row) && p.editable(col, row)}
                    <FieldInput {p} c={col} {row} compact label={col.caption ?? col.name} />
                  {:else}
                    <span class={col.field.control === "number" ? "bos-num" : ""}>{cellText(col, row)}</span>
                  {/if}
                </div>
              {/each}
            </li>
          {/each}
        </ul>
      {/if}

      {#if p.total > p.limit}
        <nav class="bos-pager" aria-label="Pages">
          <span>{first}–{last} of {p.total}</span>
          <button type="button" class="bos-icon-btn" disabled={p.pageNo <= 1} onclick={() => p.setPage(p.pageNo - 1)} aria-label="Previous page"><ChevronLeft class="size-4" /></button>
          <span>{p.pageNo} / {p.pageCount}</span>
          <button type="button" class="bos-icon-btn" disabled={p.pageNo >= p.pageCount} onclick={() => p.setPage(p.pageNo + 1)} aria-label="Next page"><ChevronRight class="size-4" /></button>
        </nav>
      {/if}
    {/if}
  </section>
{/if}
