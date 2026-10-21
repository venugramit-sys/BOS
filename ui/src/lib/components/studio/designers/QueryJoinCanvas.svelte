<script>
  /**
   * QueryJoinCanvas.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Upper Visual Join Canvas
   * Movable entity cards with field selection & SVG relationship join connectors.
   */
  import { Plus, X, Database } from "lucide-svelte";
  import { Button, EmptyState } from "$lib/bos-ui/index.js";
  import { StudioApi } from "../stores/studioApi.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  let {
    selectedTables = $bindable([]),
    joins = $bindable([]),
    onFieldSelect = null,
  } = $props();

  let isAddTableDropdownOpen = $state(false);

  // Available tables from catalog
  let availableTables = $derived(
    studioStore.catalog.filter((o) => o.object_type === "table")
  );

  async function handleAddTable(tableName) {
    if (selectedTables.some((t) => t.name === tableName)) return;

    let fields = ["id"];
    try {
      const schema = await StudioApi.getEntitySchema(tableName);
      if (schema && schema.columns && schema.columns.length > 0) {
        fields = schema.columns.map((c) => c.name).filter((n) => !n.startsWith("_"));
      }
    } catch {
      // use minimal id column
    }

    const newTable = {
      name: tableName,
      x: 30 + selectedTables.length * 200,
      y: 30 + (selectedTables.length % 2) * 40,
      fields,
    };

    selectedTables = [...selectedTables, newTable];

    // If there's already another table, auto-suggest an ID link
    if (selectedTables.length >= 2 && joins.length === 0) {
      joins = [
        {
          fromTable: selectedTables[0].name,
          fromField: "id",
          toTable: tableName,
          toField: `${selectedTables[0].name.toLowerCase()}Id`,
          type: "INNER JOIN",
        },
      ];
    }

    isAddTableDropdownOpen = false;
  }

  function handleRemoveTable(name) {
    selectedTables = selectedTables.filter((t) => t.name !== name);
    joins = joins.filter((j) => j.fromTable !== name && j.toTable !== name);
  }

  function calculateJoinPath(join) {
    const fromIdx = selectedTables.findIndex((t) => t.name === join.fromTable);
    const toIdx = selectedTables.findIndex((t) => t.name === join.toTable);
    if (fromIdx === -1 || toIdx === -1) return null;

    const fromTbl = selectedTables[fromIdx];
    const toTbl = selectedTables[toIdx];
    const fromX = (fromTbl.x ?? 16 + fromIdx * 200) + 176;
    const toX = toTbl.x ?? 16 + toIdx * 200;

    const fromFieldIdx = Math.max(0, (fromTbl.fields || []).indexOf(join.fromField));
    const toFieldIdx = Math.max(0, (toTbl.fields || []).indexOf(join.toField));

    const fromY = (fromTbl.y ?? 44) + 32 + fromFieldIdx * 24 + 12;
    const toY = (toTbl.y ?? 44) + 32 + toFieldIdx * 24 + 12;
    const ctrlOffset = Math.max(40, Math.abs(toX - fromX) * 0.45);

    return {
      d: `M ${fromX} ${fromY} C ${fromX + ctrlOffset} ${fromY}, ${toX - ctrlOffset} ${toY}, ${toX} ${toY}`,
      fromX,
      fromY,
      toX,
      toY,
      labelX: (fromX + toX) / 2,
      labelY: (fromY + toY) / 2 - 6,
      type: join.type || "INNER JOIN",
    };
  }
</script>

<div class="h-64 border-b border-slate-200 dark:border-slate-800 bg-slate-100/70 dark:bg-slate-950/70 relative overflow-hidden flex flex-col shrink-0 select-none">
  <!-- Top Bar: Add Table Dropdown -->
  <div class="h-8 px-3 border-b border-slate-200/80 dark:border-slate-800/80 bg-white/70 dark:bg-slate-900/60 backdrop-blur-xs flex items-center justify-between z-20">
    <div class="flex items-center gap-1.5 text-xs font-semibold text-slate-700 dark:text-slate-300">
      <Database class="size-3.5 text-indigo-500" />
      <span>Entity Joins Canvas</span>
    </div>

    <div class="relative">
      <Button size="xs" variant="secondary" icon={Plus} onclick={() => (isAddTableDropdownOpen = !isAddTableDropdownOpen)}>
        Add table
      </Button>

      {#if isAddTableDropdownOpen}
        <div class="absolute right-0 top-7 w-48 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 shadow-xl py-1 z-30">
          {#each availableTables as t}
            <Button class="w-full justify-start" variant="ghost" size="xs" onclick={() => handleAddTable(t.name)}>
              {t.name}
            </Button>
          {:else}
            <div class="px-3 py-1.5 text-xs text-slate-400 italic">No tables found</div>
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <!-- SVG Connectors Area -->
  <svg class="absolute inset-0 w-full h-full pointer-events-none z-0">
    {#each joins as join}
      {@const pathInfo = calculateJoinPath(join)}
      {#if pathInfo}
        <path
          d={pathInfo.d}
          fill="none"
          stroke="#6366F1"
          stroke-width="2"
          stroke-dasharray="4"
        />
        <circle cx={pathInfo.fromX} cy={pathInfo.fromY} r="3.5" fill="#6366F1" />
        <circle cx={pathInfo.toX} cy={pathInfo.toY} r="3.5" fill="#10B981" />
        <text
          x={pathInfo.labelX}
          y={pathInfo.labelY}
          text-anchor="middle"
          class="fill-indigo-500 text-[9px] font-mono"
        >
          {pathInfo.type}
        </text>
      {/if}
    {/each}
  </svg>

  <!-- Canvas Surface with Movable Table Cards -->
  <div class="flex-1 p-4 flex gap-6 overflow-x-auto relative z-10">
    {#each selectedTables as table}
      <div
        class="w-44 bg-white dark:bg-slate-900 rounded-lg border border-slate-300 dark:border-slate-700 shadow-sm flex flex-col shrink-0 overflow-hidden"
      >
        <!-- Table Card Header -->
        <div class="px-2.5 py-1.5 bg-slate-100 dark:bg-slate-800/80 border-b border-slate-200 dark:border-slate-700 flex items-center justify-between">
          <span class="text-xs font-bold text-slate-800 dark:text-slate-200 truncate">
            {table.name}
          </span>
          <Button variant="ghost" size="xs" iconOnly icon={X} aria-label={`Remove ${table.name}`} onclick={() => handleRemoveTable(table.name)} />
        </div>

        <!-- Fields List -->
        <div class="p-1.5 flex-1 overflow-y-auto space-y-0.5 text-[11px] font-mono">
          {#each table.fields as field}
            <Button
              class="w-full justify-start"
              variant="ghost"
              size="xs"
              onclick={() => onFieldSelect?.(table.name, field)}
            >
              {field}
            </Button>
          {/each}
        </div>
      </div>
    {:else}
      <EmptyState title="No tables on the canvas" description="Add a table to start the join." />
    {/each}
  </div>
</div>
