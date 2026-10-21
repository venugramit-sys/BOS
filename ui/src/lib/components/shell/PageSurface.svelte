<script>
  /**
   * PageSurface.svelte (Svelte 5 Runes)
   * Runtime Page Renderer supporting all 8 BOS Page Layout Types:
   * List, Record, Document, Board, Workspace, Split, Timeline, Wizard.
   * Features:
   * - Live CRUD Creation / Editing Modal & Inline Form (POST / PATCH / DELETE /api/entities/{table})
   * - Search & Filter Bar
   * - Command Bar Actions (Codeunit execution, State transitions, Page navigation)
   * - FactBox `side` Panel
   * - Interactive Layout Switcher
   */
  import {
    Plus,
    Search,
    RefreshCw,
    Edit3,
    Trash2,
    Play,
    PanelRight,
    Check,
    X,
    Layers,
    ChevronRight,
    Clock,
    BarChart3,
    AlertCircle,
  } from "lucide-svelte";
  import { Button, Input, Badge, Banner, Dialog, Select, EmptyState, Spinner } from "$lib/bos-ui/index.js";
  import { shell } from "./shellStore.svelte.js";
  import { notificationStore } from "$lib/services/notificationStore.svelte.js";

  const LAYOUT_OPTIONS = [
    { value: "List", label: "List" },
    { value: "Record", label: "Record" },
    { value: "Document", label: "Document" },
    { value: "Board", label: "Board" },
    { value: "Workspace", label: "Workspace" },
    { value: "Split", label: "Split" },
    { value: "Timeline", label: "Timeline" },
    { value: "Wizard", label: "Wizard" },
    { value: "Dialog", label: "Dialog / Portal" },
  ];

  let { pageName } = $props();

  let caption = $state("");
  let sourceTable = $state("");
  let sourceKind = $state("table");
  let activeLayout = $state("List");
  let columns = $state([]);
  let schemaColumns = $state([]);
  let sections = $state([]);
  let childParts = $state([]);
  let sideParts = $state([]);
  let tiles = $state([]);
  let steps = $state([]);
  let commands = $state([]);

  let rows = $state([]);
  let childRows = $state([]);
  let selectedRow = $state(null);
  let searchFilter = $state("");
  let isLoading = $state(false);
  let message = $state("Loading page...");
  let feedbackBanner = $state(null);

  // FactBox side panel state
  let isFactBoxOpen = $state(false);

  // CRUD Modal / Inline Form state
  let isCrudModalOpen = $state(false);
  let crudMode = $state("create"); // 'create' | 'edit'
  let formDraft = $state({});
  let isSaving = $state(false);

  // Wizard step index
  let wizardStepIndex = $state(0);

  function normalizeLayout(raw = "") {
    const val = String(raw || "").trim().toLowerCase();
    if (val === "list") return "List";
    if (val === "card" || val === "record") return "Record";
    if (val === "document") return "Document";
    if (val === "board" || val === "kanban") return "Board";
    if (val === "workspace" || val === "home" || val === "rolecenter") return "Workspace";
    if (val === "split") return "Split";
    if (val === "timeline") return "Timeline";
    if (val === "wizard" || val === "steps") return "Wizard";
    if (val === "dialog" || val === "portal" || val === "login" || val === "modal") return "Dialog";
    return "List";
  }

  function extractControls(controls, ctx) {
    for (const control of controls || []) {
      const kind = String(control.kind || "").toLowerCase();
      if (kind === "field") {
        const colName = control.source_expr || control.name;
        if (colName && !ctx.fields.includes(colName)) {
          ctx.fields.push(colName);
        }
      } else if (kind === "group" || kind === "section") {
        ctx.sections.push({
          name: control.name || "General",
          caption: control.caption || control.name || "General",
        });
      } else if (kind === "part") {
        const target = control.source_expr || control.page || control.source?.name || "";
        const isSide = /fact|side/i.test(control.name || "") || /fact/i.test(target);
        if (isSide) {
          ctx.sideParts.push({ name: control.name || "Details", target });
        } else {
          ctx.childParts.push({ name: control.name || "Lines", target });
        }
      } else if (kind === "tile") {
        ctx.tiles.push({
          name: control.name || "Metric",
          caption: control.caption || control.name || "Metric",
          source: control.source_expr || "",
        });
      } else if (kind === "step") {
        ctx.steps.push({
          name: control.name || `Step ${ctx.steps.length + 1}`,
          caption: control.caption || control.name || `Step ${ctx.steps.length + 1}`,
        });
      }
      if (Array.isArray(control.children) && control.children.length > 0) {
        extractControls(control.children, ctx);
      }
    }
  }

  let filteredRows = $derived.by(() => {
    const q = searchFilter.trim().toLowerCase();
    if (!q) return rows;
    return rows.filter((r) =>
      Object.entries(r).some(
        ([k, v]) => !k.startsWith("_") && v != null && String(v).toLowerCase().includes(q)
      )
    );
  });

  let editableColumns = $derived.by(() => {
    if (schemaColumns.length > 0) {
      return schemaColumns.filter((c) => !c.name.startsWith("_") && !c.is_flow_field);
    }
    return columns.map((name) => ({ name, data_type: "text", is_primary_key: name === "id" }));
  });

  async function loadRecords(table) {
    if (!table) return;
    isLoading = true;
    try {
      const dataRes = await fetch(`/api/entities/${encodeURIComponent(table)}?page=1&limit=50`);
      if (!dataRes.ok) {
        message = "The source table could not be read.";
        return;
      }
      const payload = await dataRes.json();
      const list = Array.isArray(payload) ? payload : payload.data || payload.records || [];
      rows = list;
      if (!columns.length && list[0]) {
        columns = Object.keys(list[0]).filter((key) => !key.startsWith("_"));
      }
      if (list.length > 0 && !selectedRow) {
        selectedRow = list[0];
      }
      message = list.length ? "" : "No records found. Click New Record to create one.";
    } catch (err) {
      message = err instanceof Error ? err.message : "Failed to load records.";
    } finally {
      isLoading = false;
    }
  }

  async function loadChildPartRecords(partTable) {
    if (!partTable) return;
    try {
      const res = await fetch(`/api/entities/${encodeURIComponent(partTable)}?page=1&limit=25`);
      if (res.ok) {
        const data = await res.json();
        childRows = Array.isArray(data) ? data : data.records || data.data || [];
      }
    } catch {
      childRows = [];
    }
  }

  $effect(() => {
    const name = pageName;
    let cancelled = false;
    caption = "";
    sourceTable = "";
    columns = [];
    schemaColumns = [];
    sections = [];
    childParts = [];
    sideParts = [];
    tiles = [];
    steps = [];
    commands = [];
    rows = [];
    childRows = [];
    selectedRow = null;
    message = "Loading page...";

    (async () => {
      // Static-First AOT Loading: Check static compiled package endpoint first
      let definition = null;
      let meta = null;

      try {
        const staticRes = await fetch(`/packages/bos.app/pages/${encodeURIComponent(name)}.json`);
        if (staticRes.ok) {
          definition = await staticRes.json();
          meta = { caption: definition.caption, sub_type: definition.page_type, definition };
        }
      } catch (_) {}

      // Fallback to /api/meta/objects/:name if static artifact not yet generated
      if (!definition) {
        const metaRes = await fetch(`/api/meta/objects/${encodeURIComponent(name)}`);
        if (!metaRes.ok) {
          if (!cancelled) message = "This page is not in the catalog.";
          return;
        }
        meta = await metaRes.json();
        definition = meta.definition || {};
      }
      const ctx = {
        fields: [],
        sections: [],
        childParts: [],
        sideParts: [],
        tiles: [],
        steps: [],
      };
      extractControls(definition.controls || definition.main || [], ctx);
      if (Array.isArray(definition.side)) {
        for (const s of definition.side) {
          ctx.sideParts.push({ name: s.name || "FactBox", target: s.page || s.source?.name || "" });
        }
      }
      if (Array.isArray(definition.tiles)) {
        for (const t of definition.tiles) {
          ctx.tiles.push({ name: t.name, caption: t.caption || t.name, source: t.source?.name || "" });
        }
      }
      if (Array.isArray(definition.steps)) {
        for (const st of definition.steps) {
          ctx.steps.push({ name: st.name, caption: st.caption || st.name });
        }
      }

      const rawActions = definition.actions || definition.commands || [];
      const parsedCommands = rawActions.map((a) => ({
        name: a.name || "Action",
        caption: a.caption || a.name || "Action",
        run: a.run || a.codeunit || null,
        transition: a.transition || null,
        open: a.open || null,
      }));

      const table =
        definition.source_table ||
        definition.source?.name ||
        definition.dataset ||
        "";
      const rawLayout =
        definition.page_type ||
        definition.layout ||
        (ctx.childParts.length > 0 ? "Document" : "List");

      if (!cancelled) {
        caption = definition.caption || meta.caption || name;
        sourceTable = table;
        sourceKind = definition.source?.kind || "table";
        activeLayout = normalizeLayout(rawLayout);
        columns = ctx.fields.length ? ctx.fields : [];
        sections = ctx.sections;
        childParts = ctx.childParts;
        sideParts = ctx.sideParts;
        isFactBoxOpen = ctx.sideParts.length > 0;
        tiles = ctx.tiles;
        steps = ctx.steps;
        commands = parsedCommands;
      }

      if (!table) {
        if (!cancelled) message = activeLayout === "Workspace" ? "" : "This page has no source table.";
        return;
      }

      // Load table schema for accurate form field types & columns
      try {
        const schemaRes = await fetch(`/api/entities/${encodeURIComponent(table)}/schema`);
        if (schemaRes.ok) {
          const schemaData = await schemaRes.json();
          if (!cancelled && Array.isArray(schemaData.columns)) {
            schemaColumns = schemaData.columns;
            if (columns.length === 0) {
              columns = schemaData.columns.map((c) => c.name).filter((k) => !k.startsWith("_"));
            }
          }
        }
      } catch {
        // schema optional
      }

      if (!cancelled) {
        await loadRecords(table);
        if (ctx.childParts[0]?.target) {
          await loadChildPartRecords(ctx.childParts[0].target);
        }
      }
    })().catch((err) => {
      if (!cancelled) message = err instanceof Error ? err.message : "The page could not be loaded.";
    });

    return () => {
      cancelled = true;
    };
  });

  function openCreateModal() {
    crudMode = "create";
    const draft = {};
    for (const col of editableColumns) {
      if (col.name === "id") continue;
      draft[col.name] = "";
    }
    formDraft = draft;
    notificationStore.clearAllFieldErrors();
    isCrudModalOpen = true;
  }

  function openEditModal(row) {
    if (!row) return;
    selectedRow = row;
    crudMode = "edit";
    const draft = {};
    for (const col of editableColumns) {
      draft[col.name] = row[col.name] ?? "";
    }
    formDraft = draft;
    notificationStore.clearAllFieldErrors();
    isCrudModalOpen = true;
  }

  async function handleSaveRecord() {
    if (!sourceTable) return;
    isSaving = true;
    feedbackBanner = null;
    notificationStore.clearAllFieldErrors();
    try {
      const payload = { ...formDraft };
      let res;
      let data;
      if (crudMode === "create") {
        res = await fetch(`/api/entities/${encodeURIComponent(sourceTable)}`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        data = await res.json().catch(() => ({}));
      } else {
        const recId = selectedRow?._id || selectedRow?.id || selectedRow?.code;
        if (!recId) throw new Error("Selected record has no primary key id");
        if (selectedRow?._system_version !== undefined) {
          payload._system_version = selectedRow._system_version;
        }
        res = await fetch(
          `/api/entities/${encodeURIComponent(sourceTable)}/${encodeURIComponent(recId)}`,
          {
            method: "PATCH",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(payload),
          }
        );
        data = await res.json().catch(() => ({}));
      }

      notificationStore.handleApiResponse(res, data);

      if (!res.ok) {
        throw new Error(data.message || data.error || "Failed to save record");
      }

      feedbackBanner = { tone: "success", message: crudMode === "create" ? `Created record in ${sourceTable}.` : `Updated record.` };
      isCrudModalOpen = false;
      notificationStore.clearAllFieldErrors();
      await loadRecords(sourceTable);
    } catch (err) {
      feedbackBanner = { tone: "danger", message: err.message || "Failed to save record" };
    } finally {
      isSaving = false;
    }
  }

  async function handleDeleteRecord(row) {
    const target = row || selectedRow;
    if (!sourceTable || !target) return;
    const recId = target._id || target.id || target.code;
    if (!recId) return;
    feedbackBanner = null;
    try {
      const res = await fetch(
        `/api/entities/${encodeURIComponent(sourceTable)}/${encodeURIComponent(recId)}`,
        { method: "DELETE" }
      );
      if (!res.ok) {
        const data = await res.json().catch(() => ({}));
        throw new Error(data.message || data.error || "Delete failed");
      }
      if (selectedRow === target) selectedRow = null;
      feedbackBanner = { tone: "success", message: `Deleted record ${recId}.` };
      await loadRecords(sourceTable);
    } catch (err) {
      feedbackBanner = { tone: "danger", message: err.message || "Delete failed" };
    }
  }

  async function handleRunCommand(cmd) {
    feedbackBanner = null;
    try {
      if (cmd.open) {
        shell.openTab({ id: cmd.open, title: cmd.open, type: "page" });
        return;
      }
      if (cmd.transition && sourceTable && selectedRow) {
        const recId = selectedRow._id || selectedRow.id;
        const res = await fetch(
          `/api/entities/${encodeURIComponent(sourceTable)}/${encodeURIComponent(recId)}/transition`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              transition: cmd.transition,
              status_column: "status",
              comments: `Triggered via ${cmd.caption}`,
            }),
          }
        );
        const data = await res.json().catch(() => ({}));
        if (!res.ok) throw new Error(data.message || data.error || "Transition failed");
        feedbackBanner = { tone: "success", message: `Transition '${cmd.transition}' applied.` };
        await loadRecords(sourceTable);
        return;
      }
      if (cmd.run) {
        const parts = String(cmd.run.codeunit ? `${cmd.run.codeunit}.${cmd.run.method}` : cmd.run).split(".");
        const cuName = parts[0];
        const method = parts[1] || "execute";
        const res = await fetch(`/api/codeunits/${encodeURIComponent(cuName)}/run`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ method, params: selectedRow || {} }),
        });
        const data = await res.json().catch(() => ({}));
        if (!res.ok) throw new Error(data.message || data.error || "Codeunit execution failed");
        feedbackBanner = { tone: "success", message: `Executed ${cuName}.${method}` };
        await loadRecords(sourceTable);
        return;
      }
      feedbackBanner = { tone: "info", message: `Action '${cmd.caption}' triggered.` };
    } catch (err) {
      feedbackBanner = { tone: "danger", message: err.message || "Command failed" };
    }
  }
</script>

<section class="flex-1 flex flex-col space-y-4">
  <!-- Top Page Header, Command Bar & Layout Switcher -->
  <div class="flex flex-wrap items-center justify-between gap-3 pb-3 border-b border-slate-200 dark:border-slate-800">
    <div>
      <div class="flex items-center gap-2">
        <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">{caption || pageName}</h2>
        <Badge tone="info">{activeLayout}</Badge>
        {#if sourceTable}
          <span class="text-xs text-slate-500 font-mono">source: {sourceTable}</span>
        {/if}
      </div>
    </div>

    <!-- Command Bar & Layout Switcher -->
    <div class="flex flex-wrap items-center gap-2">
      <div class="w-36">
        <Select bind:value={activeLayout} options={LAYOUT_OPTIONS} />
      </div>

      {#if sourceKind === "table" && sourceTable}
        <Button size="xs" icon={Plus} onclick={openCreateModal}>New</Button>
        {#if selectedRow}
          <Button size="xs" variant="secondary" icon={Edit3} onclick={() => openEditModal(selectedRow)}>
            Edit
          </Button>
          <Button size="xs" variant="destructive" icon={Trash2} onclick={() => handleDeleteRecord(selectedRow)}>
            Delete
          </Button>
        {/if}
      {/if}

      {#each commands as cmd (cmd.name)}
        <Button size="xs" variant="secondary" icon={Play} onclick={() => handleRunCommand(cmd)}>
          {cmd.caption}
        </Button>
      {/each}

      {#if sourceTable}
        <Button size="xs" variant="ghost" icon={RefreshCw} onclick={() => loadRecords(sourceTable)} title="Refresh" />
      {/if}

      <Button
        size="xs"
        variant={isFactBoxOpen ? "secondary" : "ghost"}
        icon={PanelRight}
        onclick={() => (isFactBoxOpen = !isFactBoxOpen)}
      >
        FactBox
      </Button>
    </div>
  </div>

  <!-- Search & Filter Bar -->
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div class="relative flex-1 max-w-sm">
      <Input
        bind:value={searchFilter}
        placeholder={`Search ${caption || sourceTable || "records"}...`}
      />
    </div>
    <div class="text-xs text-slate-400 font-mono">
      {filteredRows.length} {filteredRows.length === 1 ? "record" : "records"}
    </div>
  </div>

  {#if feedbackBanner}
    <Banner
      tone={feedbackBanner.tone}
      title={feedbackBanner.tone === "danger" ? "Error" : "Success"}
      dismissible
      onDismiss={() => (feedbackBanner = null)}
    >
      {feedbackBanner.message}
    </Banner>
  {/if}

  {#if isLoading}
    <div class="py-12 flex flex-col items-center justify-center gap-2 text-xs text-slate-400">
      <Spinner />
      <span>{message || "Loading records..."}</span>
    </div>
  {:else}
    <!-- Main Layout Area + Optional FactBox Side Panel -->
    <div class="grid grid-cols-1 {isFactBoxOpen ? 'lg:grid-cols-4' : ''} gap-6 items-start">
      <div class="{isFactBoxOpen ? 'lg:col-span-3' : ''} space-y-4">
        <!-- 1. WORKSPACE LAYOUT (Tiles + Embedded Summary) -->
        {#if activeLayout === "Workspace"}
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
            {#each (tiles.length > 0 ? tiles : [{ name: "Total", caption: `${caption || sourceTable} Count` }]) as tile}
              <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-1">
                <div class="flex items-center justify-between text-xs text-slate-500">
                  <span>{tile.caption}</span>
                  <BarChart3 class="size-4 text-indigo-500" />
                </div>
                <div class="text-2xl font-extrabold text-indigo-600 dark:text-indigo-400">
                  {filteredRows.length}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <!-- 2. LIST LAYOUT -->
        {#if activeLayout === "List" || activeLayout === "Workspace"}
          {#if filteredRows.length === 0}
            <EmptyState title="No records" description={message || "No matching records found."}>
              {#if sourceKind === "table" && sourceTable}
                <Button size="xs" icon={Plus} onclick={openCreateModal}>Create First Record</Button>
              {/if}
            </EmptyState>
          {:else}
            <div class="overflow-auto rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
              <table class="w-full text-xs">
                <thead class="bg-slate-50 dark:bg-slate-900 text-left border-b border-slate-200 dark:border-slate-800">
                  <tr>
                    {#each columns as column}
                      <th class="px-3 py-2.5 font-semibold text-slate-600 dark:text-slate-300">{column}</th>
                    {/each}
                    <th class="px-3 py-2.5 text-right w-24">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
                  {#each filteredRows as row}
                    {@const isSelected = selectedRow === row}
                    <tr
                      onclick={() => (selectedRow = row)}
                      class="cursor-pointer transition-colors {isSelected ? 'bg-indigo-50/50 dark:bg-indigo-950/30' : 'hover:bg-slate-50 dark:hover:bg-slate-800/40'}"
                    >
                      {#each columns as column}
                        <td class="px-3 py-2 text-slate-800 dark:text-slate-200">{row[column] ?? ""}</td>
                      {/each}
                      <td class="px-3 py-2 text-right space-x-1">
                        <button
                          type="button"
                          onclick={(e) => {
                            e.stopPropagation();
                            openEditModal(row);
                          }}
                          class="text-indigo-600 hover:underline font-medium"
                        >
                          Edit
                        </button>
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}

        <!-- 3. RECORD OR DOCUMENT LAYOUT (Header Form + Optional Child Lines Part) -->
        {:else if activeLayout === "Record" || activeLayout === "Document"}
          {@const current = selectedRow || filteredRows[0] || {}}
          <div class="p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-4">
            <div class="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-3">
              <div class="flex items-center gap-2">
                <Layers class="size-4 text-indigo-500" />
                <span class="text-xs font-bold uppercase tracking-wider text-slate-500">
                  {sections[0]?.caption || "General"}
                </span>
              </div>
              {#if filteredRows.length > 1}
                <div class="flex items-center gap-1 text-xs">
                  <span class="text-slate-400">Record:</span>
                  <select
                    class="rounded border border-slate-200 dark:border-slate-800 bg-transparent px-2 py-0.5 text-xs"
                    onchange={(e) => {
                      const idx = Number(e.currentTarget.value);
                      if (!Number.isNaN(idx) && filteredRows[idx]) selectedRow = filteredRows[idx];
                    }}
                  >
                    {#each filteredRows as r, idx}
                      <option value={idx} selected={selectedRow === r}>
                        {r.name || r.code || r.orderNo || r.id || `#${idx + 1}`}
                      </option>
                    {/each}
                  </select>
                </div>
              {/if}
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {#each columns as col}
                <div class="space-y-1">
                  <label class="block text-xs font-semibold text-slate-600 dark:text-slate-400">{col}</label>
                  <Input
                    value={current[col] ?? ""}
                    oninput={(e) => {
                      if (selectedRow) selectedRow[col] = e.currentTarget.value;
                    }}
                  />
                </div>
              {/each}
            </div>

            {#if sourceKind === "table" && sourceTable && selectedRow}
              <div class="flex justify-end pt-2">
                <Button
                  size="xs"
                  icon={Check}
                  onclick={() => {
                    crudMode = "edit";
                    formDraft = { ...selectedRow };
                    handleSaveRecord();
                  }}
                >
                  Save Record
                </Button>
              </div>
            {/if}
          </div>

          {#if activeLayout === "Document" || childParts.length > 0}
            <div class="p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold uppercase tracking-wider text-slate-500">
                  {childParts[0]?.name || "Document Lines"} ({childParts[0]?.target || "Lines"})
                </span>
                <Badge>part</Badge>
              </div>
              {#if childRows.length > 0}
                {@const lineCols = Object.keys(childRows[0]).filter((k) => !k.startsWith("_")).slice(0, 6)}
                <div class="overflow-auto rounded-lg border border-slate-200 dark:border-slate-800">
                  <table class="w-full text-xs">
                    <thead class="bg-slate-50 dark:bg-slate-800/70 text-left">
                      <tr>
                        {#each lineCols as lc}
                          <th class="px-3 py-2 font-semibold text-slate-500">{lc}</th>
                        {/each}
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
                      {#each childRows as lr}
                        <tr>
                          {#each lineCols as lc}
                            <td class="px-3 py-1.5">{lr[lc] ?? ""}</td>
                          {/each}
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {:else}
                <div class="p-4 text-center text-xs text-slate-400 border border-dashed border-slate-200 dark:border-slate-800 rounded-lg">
                  No child line items linked to this document header.
                </div>
              {/if}
            </div>
          {/if}

        <!-- 4. BOARD LAYOUT (Kanban grouped by status) -->
        {:else if activeLayout === "Board"}
          {@const statuses = Array.from(new Set(["Draft", "Open", "Released", ...filteredRows.map((r) => r.status || "Open")]))}
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            {#each statuses as st}
              {@const laneRows = filteredRows.filter((r) => (r.status || "Open") === st)}
              <div class="p-3 rounded-xl bg-slate-100/70 dark:bg-slate-900/60 border border-slate-200 dark:border-slate-800 space-y-2.5">
                <div class="flex items-center justify-between text-xs font-bold text-slate-700 dark:text-slate-300">
                  <span>{st}</span>
                  <Badge>{laneRows.length}</Badge>
                </div>
                <div class="space-y-2">
                  {#each laneRows as card}
                    <div
                      onclick={() => (selectedRow = card)}
                      role="presentation"
                      class="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-2xs space-y-1 cursor-pointer hover:border-indigo-500"
                    >
                      <div class="text-xs font-bold text-slate-800 dark:text-slate-100">
                        {card.name || card.code || card.orderNo || card.id || "Record"}
                      </div>
                      {#each columns.slice(0, 3) as c}
                        <div class="flex justify-between text-[11px] text-slate-500">
                          <span>{c}:</span>
                          <span class="font-mono text-slate-700 dark:text-slate-300">{card[c] ?? "—"}</span>
                        </div>
                      {/each}
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>

        <!-- 5. SPLIT LAYOUT (Master List Left + Detail Card Right) -->
        {:else if activeLayout === "Split"}
          <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div class="md:col-span-1 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 divide-y divide-slate-100 dark:divide-slate-800 max-h-[480px] overflow-y-auto">
              {#each filteredRows as r, idx}
                <button
                  type="button"
                  onclick={() => (selectedRow = r)}
                  class="w-full p-3 text-left flex items-center justify-between text-xs hover:bg-slate-50 dark:hover:bg-slate-800/50 cursor-pointer {selectedRow === r ? 'bg-indigo-50/60 dark:bg-indigo-950/30 font-semibold' : ''}"
                >
                  <span class="truncate">{r.name || r.code || r.id || `Record #${idx + 1}`}</span>
                  <ChevronRight class="size-3.5 text-slate-400 shrink-0" />
                </button>
              {/each}
            </div>
            <div class="md:col-span-2 p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-4">
              {#if selectedRow}
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  {#each columns as col}
                    <div class="space-y-1">
                      <span class="text-xs font-semibold text-slate-500">{col}</span>
                      <div class="p-2 rounded border border-slate-200 dark:border-slate-800 text-xs bg-slate-50 dark:bg-slate-950">
                        {selectedRow[col] ?? "—"}
                      </div>
                    </div>
                  {/each}
                </div>
              {:else}
                <EmptyState title="Select a record" description="Choose a row on the left to inspect its details." />
              {/if}
            </div>
          </div>

        <!-- 6. TIMELINE LAYOUT -->
        {:else if activeLayout === "Timeline"}
          <div class="p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
            <div class="space-y-4 pl-4 border-l-2 border-indigo-500/40">
              {#each filteredRows as r, idx}
                <div class="relative pl-4 space-y-1">
                  <span class="absolute -left-[21px] top-1 size-3 rounded-full bg-indigo-600 ring-4 ring-white dark:ring-slate-900"></span>
                  <div class="flex items-center gap-2 text-xs font-bold text-slate-800 dark:text-slate-200">
                    <Clock class="size-3.5 text-indigo-500" />
                    <span>{r.name || r.code || r.id || `Entry #${idx + 1}`}</span>
                  </div>
                  <div class="text-[11px] text-slate-500 font-mono">
                    {columns.slice(0, 4).map((c) => `${c}: ${r[c] ?? "—"}`).join(" · ")}
                  </div>
                </div>
              {/each}
            </div>
          </div>

        <!-- 7. WIZARD LAYOUT -->
        {:else if activeLayout === "Wizard"}
          {@const wizardSteps = steps.length > 0 ? steps : [{ name: "Step1", caption: "1. Enter Details" }, { name: "Step2", caption: "2. Review & Confirm" }]}
          <div class="p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-5">
            <div class="flex items-center gap-2 border-b border-slate-200 dark:border-slate-800 pb-3">
              {#each wizardSteps as st, idx}
                <button
                  type="button"
                  onclick={() => (wizardStepIndex = idx)}
                  class="px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer {wizardStepIndex === idx ? 'bg-indigo-600 text-white' : 'bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400'}"
                >
                  {st.caption}
                </button>
              {/each}
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              {#each editableColumns as col}
                {#if col.name !== "id"}
                  <div class="space-y-1">
                    <label class="block text-xs font-semibold text-slate-600 dark:text-slate-400">{col.name}</label>
                    <Input bind:value={formDraft[col.name]} placeholder={col.name} />
                  </div>
                {/if}
              {/each}
            </div>

            <div class="flex items-center justify-between pt-2">
              <Button
                size="xs"
                variant="secondary"
                disabled={wizardStepIndex === 0}
                onclick={() => (wizardStepIndex = Math.max(0, wizardStepIndex - 1))}
              >
                Previous
              </Button>
              {#if wizardStepIndex < wizardSteps.length - 1}
                <Button size="xs" onclick={() => (wizardStepIndex = wizardStepIndex + 1)}>
                  Next Step
                </Button>
              {:else}
                <Button
                  size="xs"
                  icon={Check}
                  loading={isSaving}
                  onclick={() => {
                    crudMode = "create";
                    handleSaveRecord();
                  }}
                >
                  Finish & Save
                </Button>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- FactBox Side Panel -->
      {#if isFactBoxOpen}
        <aside class="lg:col-span-1 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 space-y-4">
          <div class="flex items-center justify-between border-b border-slate-100 dark:border-slate-800 pb-2">
            <span class="text-xs font-bold uppercase tracking-wider text-slate-500">FactBox Side Panel</span>
            <button
              type="button"
              onclick={() => (isFactBoxOpen = false)}
              class="text-slate-400 hover:text-slate-600 cursor-pointer"
            >
              <X class="size-3.5" />
            </button>
          </div>

          {#if sideParts.length > 0}
            {#each sideParts as sp}
              <div class="p-3 rounded-lg bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 space-y-1 text-xs">
                <div class="font-bold text-slate-800 dark:text-slate-200">{sp.name}</div>
                <div class="text-[11px] text-slate-500 font-mono">source: {sp.target || sourceTable}</div>
              </div>
            {/each}
          {/if}

          {#if selectedRow}
            <div class="space-y-2 text-xs">
              <div class="font-semibold text-slate-700 dark:text-slate-300">Selected Record Summary</div>
              <div class="p-3 rounded-lg bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 space-y-1.5">
                {#each columns.slice(0, 5) as col}
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-slate-500 truncate">{col}:</span>
                    <span class="font-mono font-medium text-slate-800 dark:text-slate-200 truncate">
                      {selectedRow[col] ?? "—"}
                    </span>
                  </div>
                {/each}
              </div>
            </div>
          {:else}
            <p class="text-xs text-slate-400">Select a record to view related FactBox insights.</p>
          {/if}
        </aside>
      {/if}
    </div>
  {/if}
</section>

<!-- CRUD Create / Edit Modal Dialog -->
<Dialog
  bind:open={isCrudModalOpen}
  title={crudMode === "create" ? `New ${caption || sourceTable}` : `Edit ${caption || sourceTable}`}
  description={`Save changes directly to ${sourceTable} via BOS Entity API.`}
  size="md"
>
  {#snippet children()}
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
      {#each editableColumns as col}
        {#if !(crudMode === "create" && col.name === "id")}
          <div>
            <Input
              bind:value={formDraft[col.name]}
              label={col.name}
              required={!col.is_nullable}
              disabled={crudMode === "edit" && col.is_primary_key}
              placeholder={col.data_type || col.name}
              status={notificationStore.fieldErrors[col.name] ? "error" : "default"}
              errorMessage={notificationStore.fieldErrors[col.name] || ""}
              oninput={() => notificationStore.clearFieldError(col.name)}
            />
          </div>
        {/if}
      {/each}
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (isCrudModalOpen = false)}>Cancel</Button>
    <Button icon={Check} loading={isSaving} onclick={handleSaveRecord}>
      {crudMode === "create" ? "Create Record" : "Save Changes"}
    </Button>
  {/snippet}
</Dialog>
