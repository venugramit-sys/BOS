<script>
  /**
   * RelationshipsViewer.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Relationships Window (Database Topology)
   * 100% Real PostgreSQL Schema Graph querying live meta-objects & foreign keys.
   */
  import { onMount } from "svelte";
  import { Database, Key, Link, RefreshCw } from "lucide-svelte";
  import { Dialog, Button, Input, Banner, Spinner, EmptyState } from "$lib/bos-ui/index.js";
  import { StudioApi } from "../stores/studioApi.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  let { isOpen = $bindable(false) } = $props();

  let tables = $state([]);
  let relations = $state([]);
  let isLoading = $state(false);
  let errorMsg = $state(null);
  let activeFilter = $state("");

  $effect(() => {
    if (isOpen) {
      loadGraph();
    }
  });

  async function loadGraph() {
    isLoading = true;
    errorMsg = null;
    try {
      const tableSummaries = await StudioApi.listMetaObjects("table");
      if (!Array.isArray(tableSummaries) || tableSummaries.length === 0) {
        tables = [];
        relations = [];
        return;
      }

      const detailsPromises = tableSummaries.map(async (ts, idx) => {
        try {
          const details = await StudioApi.getMetaObject(ts.object_name);
          return { summary: ts, details, idx };
        } catch {
          try {
            const schema = await StudioApi.getEntitySchema(ts.object_name);
            return {
              summary: ts,
              details: {
                fields: (schema.columns || []).map((c) => ({
                  field_name: c.name,
                  field_type: c.data_type,
                  is_primary_key: c.is_primary_key,
                })),
                relations: [],
              },
              idx,
            };
          } catch {
            return null;
          }
        }
      });

      const results = await Promise.all(detailsPromises);

      const loadedTables = [];
      const loadedRelations = [];
      let colIdx = 0;
      let rowIdx = 0;
      const MAX_COLS = 3;

      for (const res of results) {
        if (!res) continue;
        const name = res.summary.object_name;
        const fields = (res.details.fields || []).map((f) => ({
          name: f.field_name,
          pk: !!f.is_primary_key,
          type: f.field_type || "Text",
          fk: null,
        }));

        if (Array.isArray(res.details.relations)) {
          for (const rel of res.details.relations) {
            loadedRelations.push(rel);
            const fld = fields.find(
              (f) => f.name.toLowerCase() === rel.from_column.toLowerCase()
            );
            if (fld) {
              fld.fk = `${rel.to_table}.${rel.to_column}`;
            }
          }
        }

        loadedTables.push({
          name,
          x: 40 + colIdx * 280,
          y: 40 + rowIdx * 260,
          fields,
        });

        colIdx++;
        if (colIdx >= MAX_COLS) {
          colIdx = 0;
          rowIdx++;
        }
      }

      tables = loadedTables;
      relations = loadedRelations;
    } catch (err) {
      errorMsg = err.message || "Failed to load database relationships";
    } finally {
      isLoading = false;
    }
  }

  let filteredTables = $derived(
    activeFilter
      ? tables.filter((t) =>
          t.name.toLowerCase().includes(activeFilter.toLowerCase())
        )
      : tables
  );

  function calculateLinkPath(rel) {
    const fromTbl = tables.find(
      (t) =>
        t.name.toLowerCase() === rel.from_table.toLowerCase() ||
        t.name.toLowerCase() === rel.from_table.replace(/_/g, "").toLowerCase()
    );
    const toTbl = tables.find(
      (t) =>
        t.name.toLowerCase() === rel.to_table.toLowerCase() ||
        t.name.toLowerCase() === rel.to_table.replace(/_/g, "").toLowerCase()
    );

    if (!fromTbl || !toTbl) return null;

    const fromColIdx = fromTbl.fields.findIndex(
      (f) => f.name.toLowerCase() === rel.from_column.toLowerCase()
    );
    const toColIdx = toTbl.fields.findIndex(
      (f) => f.name.toLowerCase() === rel.to_column.toLowerCase()
    );

    const fromY = fromTbl.y + 36 + Math.max(0, fromColIdx) * 26 + 13;
    const toY = toTbl.y + 36 + Math.max(0, toColIdx) * 26 + 13;

    // Node card width is 224px (w-56)
    const isFromLeft = fromTbl.x < toTbl.x;
    const x1 = isFromLeft ? fromTbl.x + 224 : fromTbl.x;
    const x2 = isFromLeft ? toTbl.x : toTbl.x + 224;

    const midX = (x1 + x2) / 2;
    return {
      d: `M ${x1} ${fromY} C ${midX} ${fromY}, ${midX} ${toY}, ${x2} ${toY}`,
      x1,
      y1: fromY,
      x2,
      y2: toY,
      cardinality: rel.cardinality || "1:N",
    };
  }

  let computedLinks = $derived(
    relations.map((r) => ({ rel: r, link: calculateLinkPath(r) })).filter((item) => item.link !== null)
  );
</script>

<Dialog
  bind:open={isOpen}
  title="Relationships"
  description="Live PostgreSQL schema graph"
  size="xl"
  class="max-w-6xl h-[85vh]"
>
  {#snippet children()}
    <div class="flex items-center justify-end gap-2 mb-3">
      <Input class="w-56" type="search" size="sm" bind:value={activeFilter} placeholder="Filter tables..." />
      <Button variant="secondary" size="sm" icon={RefreshCw} loading={isLoading} onclick={loadGraph}>Refresh</Button>
    </div>
    <div class="relative min-h-[28rem] bg-slate-100/60 dark:bg-slate-950 rounded-lg overflow-auto p-6 select-none">
        {#if isLoading && tables.length === 0}
          <div class="h-full flex flex-col items-center justify-center text-xs text-slate-400 space-y-2 py-16">
            <Spinner />
            <p>Querying live tables and foreign keys...</p>
          </div>
        {:else if errorMsg}
          <Banner tone="danger" title="Schema graph failed">{errorMsg}</Banner>
        {:else if tables.length === 0}
          <EmptyState title="No registered tables" description="Create tables with foreign keys, or install a package, to see the topology." />
        {:else}
          <!-- SVG Dynamic Relationship Join Lines -->
          <svg class="absolute inset-0 w-full h-full pointer-events-none z-0 min-w-[900px] min-h-[600px]">
            {#each computedLinks as item (item.rel.relation_id || `${item.rel.from_table}-${item.rel.from_column}`)}
              <path
                d={item.link.d}
                fill="none"
                stroke="#6366F1"
                stroke-width="2"
                stroke-dasharray="0"
                opacity="0.8"
              />
              <circle cx={item.link.x1} cy={item.link.y1} r="3" fill="#6366F1" />
              <circle cx={item.link.x2} cy={item.link.y2} r="3" fill="#6366F1" />
            {/each}
          </svg>

          <!-- Table Nodes Grid -->
          <div class="relative z-10 flex flex-wrap gap-8 items-start">
            {#each filteredTables as table (table.name)}
              <div
                class="w-56 bg-white dark:bg-slate-900 rounded-xl border border-slate-300 dark:border-slate-700 shadow-md overflow-hidden shrink-0"
              >
                <!-- Node Header -->
                <div class="px-3 py-2 bg-slate-100 dark:bg-slate-800 border-b border-slate-200 dark:border-slate-700 flex items-center justify-between">
                  <span class="text-xs font-bold text-slate-800 dark:text-slate-200 truncate">
                    {table.name}
                  </span>
                  <Database class="size-3 text-indigo-500 shrink-0" />
                </div>

                <!-- Node Fields List -->
                <div class="divide-y divide-slate-100 dark:divide-slate-800/60 text-[11px] font-mono max-h-64 overflow-y-auto">
                  {#each table.fields as field}
                    <div class="px-3 py-1.5 flex items-center justify-between hover:bg-slate-50 dark:hover:bg-slate-800/40">
                      <span class="flex items-center gap-1.5 text-slate-700 dark:text-slate-300 min-w-0">
                        {#if field.pk}
                          <Key class="size-3 text-amber-500 shrink-0 fill-current" />
                        {:else if field.fk}
                          <Link class="size-3 text-sky-500 shrink-0" title={`Foreign Key -> ${field.fk}`} />
                        {/if}
                        <span class="truncate {field.pk ? 'font-bold' : ''}">{field.name}</span>
                      </span>
                      <span class="text-[10px] text-slate-400 font-sans shrink-0 ml-2">{field.type}</span>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>
        {/if}
    </div>
  {/snippet}
  {#snippet footer()}
    <span class="mr-auto text-[11px] text-slate-400">{tables.length} tables · {relations.length} relationships</span>
    <Button variant="ghost" onclick={() => (isOpen = false)}>Close</Button>
  {/snippet}
</Dialog>
