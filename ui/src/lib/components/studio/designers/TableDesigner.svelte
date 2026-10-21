<script>
  /**
   * TableDesigner.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Table Designer & Live Datasheet Grid
   * Dual-mode workspace: Design View (Schema Grid + Property Sheet) <--> Datasheet View (Live Records)
   */
  import { onMount } from "svelte";
  import { Play, RefreshCw, Sliders, FileCode } from "lucide-svelte";
  import { Button, Tabs, Toolbar, Banner, Spinner, ConfirmDialog, Input, Datasheet, columnsFromFields } from "$lib/bos-ui/index.js";
  import { toastStore } from "$lib/bos-ui/toast/index.js";
  import TableFieldsGrid from "./TableFieldsGrid.svelte";
  import TablePropertySheet from "./TablePropertySheet.svelte";
  import { StudioApi } from "../stores/studioApi.js";
  import { studioStore } from "../stores/studioStore.svelte.js";
  import { formatBosScriptSource } from "../code-editor/bosPrettier.js";
  import {
    addDocumentStatus,
    addField,
    addSecondaryKey,
    adoptModel,
    bosTypeFromSql,
    createEmptyTable,
    createField,
    documentFromDetails,
    emitDocument,
    ensureEvent,
    ensureMethod,
    ensureValidate,
    eventLine,
    lengthOf,
    moveLogicToCompanion,
    parseTableDocument,
    syncPrimaryKey,
  } from "./tableModel.js";

  let {
    tableName = "",
    mode = $bindable("design"),
    packageName = "bos.app",
    sourceCode = "",
    objectDetails = null,
    detailsLoaded = false,
    onJumpToCode = null,
    onSourceChange = null,
  } = $props();

  let isMobile = $state(false);
  let isPropertyPanelOpen = $state(false);
  let model = $state(createEmptyTable(tableName));
  let fields = $derived(model.fields);
  let selectedField = $state(null);
  let hydrated = $state(false);

  // Datasheet View State (100% Real Live Rows)
  let liveRecords = $state([]);
  let totalRecords = $state(0);
  let isLoadingRecords = $state(false);
  let recordError = $state(null);
  let pageOffset = $state(0);
  let pageSize = $state(50);
  let pendingDeleteId = $state("");
  let sheetColumns = $derived(columnsFromFields(fields));

  // Schema deployment state
  let isDeploying = $state(false);
  let deployMessage = $state("");
  let deploySuccess = $state(false);
  let blockedMigrationMessage = $state("");

  onMount(() => {
    const checkViewport = () => {
      const mobile = window.innerWidth < 1024;
      isMobile = mobile;
      if (!hydrated) {
        // Desktop defaults to open, mobile defaults to closed so fields grid is visible first
        isPropertyPanelOpen = !mobile;
      }
    };
    checkViewport();
    window.addEventListener("resize", checkViewport);
    if (mode === "datasheet") loadLiveRecords();
    return () => window.removeEventListener("resize", checkViewport);
  });

  $effect(() => {
    if (mode === "datasheet" && liveRecords.length === 0 && !isLoadingRecords) {
      loadLiveRecords();
    }
  });

  $effect(() => {
    if (hydrated || !detailsLoaded) return;
    const details = objectDetails;
    const source = sourceCode;
    const fromDetails = details && (details.definition?.columns?.length || details.fields?.length)
      ? documentFromDetails(tableName, details)
      : null;
    const parsed = source && /\btable\s+[A-Za-z_]/.test(source) ? parseTableDocument(source, tableName) : null;

    if (parsed?.fields.length) {
      adoptModel(model, parsed);
    } else if (fromDetails) {
      adoptModel(model, fromDetails);
    } else {
      void loadInitialSchema();
    }
    hydrated = true;
    if (!model.fields.includes(selectedField)) {
      selectedField = model.triggers.length ? null : (model.fields[0] ?? null);
    }
  });

  $effect(() => {
    if (!hydrated || !model.fields.length) return;
    for (const field of model.fields) {
      void field.name;
      void field.caption;
      void field.notNull;
      void field.unique;
      void field.isPrimaryKey;
      void field.defaultVal;
      void field.dataType;
      void field.length;
    }
    syncPrimaryKey(model);
    for (const field of model.fields) {
      const next = lengthOf(field.dataType);
      if ((/^String|^Decimal/i.test(field.dataType || "")) && field.length !== next) field.length = next;
    }
    onSourceChange?.(emitDocument(model).combined);
  });

  async function loadInitialSchema() {
    try {
      const schema = await StudioApi.getEntitySchema(tableName);
      if (!schema?.columns?.length || model.fields.length) return;
      model.fields = schema.columns.map((column) => createField({
        name: column.name,
        dataType: bosTypeFromSql(column.data_type),
        isPrimaryKey: Boolean(column.is_primary_key),
        notNull: column.is_nullable === false || Boolean(column.is_primary_key),
        unique: Boolean(column.is_unique),
        defaultVal: column.default_val || "",
        caption: column.caption && column.caption !== column.name ? column.caption : "",
      }));
      syncPrimaryKey(model);
      selectedField = model.fields[0] ?? null;
    } catch {
      if (!model.fields.length) {
        const starter = addField(model, { name: "code", dataType: "String(20)", notNull: true, unique: true, isPrimaryKey: true });
        syncPrimaryKey(model);
        selectedField = starter;
      }
    }
  }

  let notices = $derived.by(() => {
    const items = [];
    if (hydrated && model.fields.length && !model.primaryKey.length) {
      items.push("This table has no primary key.");
    }
    if (model.tableType === "Document" && !model.fields.some((field) => field.name === "status")) {
      items.push("Document tables keep a status field. New documents start as Draft.");
    }
    if (model.tableType === "Ledger") {
      items.push("Ledger tables are write-once. Corrections are new rows.");
    }
    const names = model.fields.map((field) => field.name);
    if (new Set(names).size !== names.length) items.push("Two fields share the same name.");
    return items;
  });

  async function loadLiveRecords() {
    isLoadingRecords = true;
    recordError = null;
    try {
      const res = await StudioApi.getEntities(tableName, {
        limit: pageSize,
        offset: pageOffset,
      });
      liveRecords = res.records;
      totalRecords = res.total;
    } catch (err) {
      recordError = err.message || "Failed to load live records";
      liveRecords = [];
    } finally {
      isLoadingRecords = false;
    }
  }

  function openCode(line = 0) {
    const script = emitDocument(model).combined;
    onSourceChange?.(script);
    if (onJumpToCode) onJumpToCode(script, line);
    else studioStore.openObject(tableName, "code", "table");
  }

  function openEvent(token) {
    const text = String(token || "");
    let lookup = text;
    if (text.startsWith("validate ")) {
      const fieldName = text.slice("validate ".length);
      const field = model.fields.find((item) => item.name === fieldName);
      if (field) ensureValidate(field);
    } else if (text.startsWith("method:")) {
      lookup = text.slice("method:".length);
    } else if (text) {
      ensureEvent(model, text);
    }
    const script = emitDocument(model).combined;
    onSourceChange?.(script);
    openCode(eventLine(script, lookup));
  }

  async function handleApplySchema(allowDestructive = false) {
    isDeploying = true;
    deployMessage = "";
    deploySuccess = false;
    try {
      const rawScript = emitDocument(model).combined;
      const script = await formatBosScriptSource(rawScript);
      onSourceChange?.(script);
      await StudioApi.applyScript(script, model.packageName || packageName, { allowDestructive });
      deploySuccess = true;
      deployMessage = `Table '${model.name}' and its logic companion were compiled.`;
      toastStore.push({ tone: "success", title: "Schema applied", description: deployMessage });
      await studioStore.loadCatalog();
    } catch (err) {
      deploySuccess = false;
      deployMessage = err.message || "Failed to compile & apply schema";
      if (err.status === 409 && !allowDestructive) blockedMigrationMessage = deployMessage;
    } finally {
      isDeploying = false;
    }
  }

  function confirmDestructiveApply() {
    blockedMigrationMessage = "";
    handleApplySchema(true);
  }

  function recordId(record) {
    return record._id ?? record.id;
  }

  async function commitSheetCell(record, column, value) {
    try {
      /** @type {Record<string, unknown>} */
      const patch = { [column]: value };
      if (record._system_version != null) patch._system_version = record._system_version;
      const result = await StudioApi.updateEntity(tableName, recordId(record), patch);
      const version = result?.data?._system_version ?? result?._system_version;
      if (version != null) record._system_version = version;
    } catch (err) {
      toastStore.push({ tone: "danger", title: "Update failed", description: err.message });
      if (err.status === 409) await loadLiveRecords();
      throw err;
    }
  }

  async function insertSheetRow(payload) {
    try {
      await StudioApi.createEntity(tableName, payload);
      await loadLiveRecords();
      toastStore.push({ tone: "success", title: "Row inserted" });
    } catch (err) {
      toastStore.push({ tone: "danger", title: "Insert failed", description: err.message });
      throw err;
    }
  }

  async function confirmDeleteRow() {
    const id = pendingDeleteId;
    pendingDeleteId = "";
    if (!id) return;
    try {
      await StudioApi.deleteEntity(tableName, id);
      await loadLiveRecords();
      toastStore.push({ tone: "success", title: "Row deleted" });
    } catch (err) {
      toastStore.push({ tone: "danger", title: "Delete failed", description: err.message });
    }
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden min-w-0">
  <Toolbar label="Table designer" class="overflow-x-auto no-scrollbar shrink-0 py-1 px-2 gap-1 sm:gap-1.5 flex items-center justify-between">
    <div class="flex items-center gap-1 sm:gap-1.5 shrink-0">
      <Tabs
        value={mode === "code" ? "design" : mode}
        variant="segmented"
        class="shrink-0 text-xs"
        items={[
          { id: "design", label: "Design" },
          { id: "datasheet", label: "Datasheet" },
        ]}
        onValueChange={(next) => (mode = next)}
      />
      <Button size="xs" variant="ghost" icon={FileCode} class="shrink-0 hidden sm:inline-flex" onclick={() => openCode()}>Code</Button>
      {#if mode === "design"}
        <Button size="xs" variant={selectedField ? "ghost" : "secondary"} class="shrink-0 hidden md:inline-flex" onclick={() => (selectedField = null)}>Table</Button>
        <Button
          size="xs"
          variant={isPropertyPanelOpen ? "secondary" : "ghost"}
          icon={Sliders}
          class="shrink-0"
          onclick={() => (isPropertyPanelOpen = !isPropertyPanelOpen)}
        >
          <span class="hidden sm:inline">Properties</span>
          <span class="sm:hidden text-xs">Props</span>
        </Button>
      {/if}
    </div>

    <div class="flex items-center gap-1 sm:gap-1.5 shrink-0 ml-auto">
      {#if mode === "design"}
        <Button size="xs" icon={Play} loading={isDeploying} shortcut="F5" showShortcut={false} class="shrink-0 text-xs font-semibold" onclick={() => handleApplySchema()}>
          <span class="hidden sm:inline">{isDeploying ? "Migrating..." : "Save & Apply"}</span>
          <span class="sm:hidden">{isDeploying ? "..." : "Apply"}</span>
        </Button>
      {:else}
        <Button size="xs" variant="secondary" icon={RefreshCw} loading={isLoadingRecords} class="shrink-0 text-xs" onclick={loadLiveRecords}>
          <span class="hidden sm:inline">Refresh Rows</span>
          <span class="sm:hidden">Refresh</span>
        </Button>
      {/if}
    </div>
  </Toolbar>
  {#if deployMessage}
    <div class="px-3 pt-2">
      <Banner tone={deploySuccess ? "success" : "danger"} dismissible onDismiss={() => (deployMessage = "")}>{deployMessage}</Banner>
    </div>
  {/if}

  <!-- Workspace Surface -->
  <div class="flex-1 flex flex-row min-h-0 min-w-0 overflow-hidden relative">
    {#if mode === "design"}
      <!-- DESIGN VIEW: Fields grid, property sheet on the right edge -->
      <main class="flex-1 flex flex-col min-w-0 min-h-0 bg-white dark:bg-slate-950">
        {#if notices.length}
          <div class="flex flex-wrap items-center gap-2 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
            {#each notices as notice}
              <span class="text-[11px] text-amber-700 dark:text-amber-300">{notice}</span>
            {/each}
            {#if model.tableType === "Document" && !model.fields.some((field) => field.name === "status")}
              <Button size="xs" variant="secondary" onclick={() => { selectedField = addDocumentStatus(model); }}>Add status</Button>
            {/if}
          </div>
        {/if}
        <TableFieldsGrid
          bind:fields={model.fields}
          bind:selectedField
          onFieldSelect={(field) => {
            if (isMobile && field) isPropertyPanelOpen = true;
          }}
        />
        <div class="flex shrink-0 flex-wrap items-center gap-1.5 sm:gap-2 border-t border-slate-200 bg-slate-50/80 px-2 sm:px-3 py-1.5 sm:py-2 text-xs dark:border-slate-800 dark:bg-slate-900/60">
          <span class="font-semibold text-slate-500">PK:</span>
          {#each model.primaryKey as column}
            <span class="rounded bg-amber-50 px-1.5 py-0.5 font-mono text-[11px] text-amber-700 dark:bg-amber-950/40 dark:text-amber-300">{column}</span>
          {:else}
            <span class="text-slate-400">None</span>
          {/each}
          <span class="ml-1 sm:ml-2 font-semibold text-slate-500">Keys:</span>
          {#each model.keys as key (key.id)}
            <Input
              class="w-28 sm:w-40"
              size="sm"
              type="code"
              value={key.fields.join(", ")}
              placeholder="customerId, orderDate"
              oninput={(event) => {
                key.fields = event.target.value.split(",").map((part) => part.trim()).filter(Boolean);
              }}
            />
          {/each}
          <Button size="xs" variant="secondary" onclick={() => addSecondaryKey(model)}>Add key</Button>
        </div>
      </main>

      {#if isPropertyPanelOpen}
        <!-- Desktop Docked Aside (hidden on mobile, visible on lg:) -->
        <aside class="hidden lg:flex w-80 shrink-0 border-l border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 flex-col overflow-hidden">
          <TablePropertySheet
            {model}
            {selectedField}
            onClose={() => (isPropertyPanelOpen = false)}
            onSelectTable={() => (selectedField = null)}
            onMoveLogic={() => moveLogicToCompanion(model)}
            onOpenEvent={openEvent}
            onAddMethod={() => {
              const method = ensureMethod(model);
              const script = emitDocument(model).combined;
              onSourceChange?.(script);
              openCode(eventLine(script, method.name));
            }}
            onJumpToCode={openCode}
          />
        </aside>

        <!-- Mobile Drawer Overlay (visible on mobile < lg, hidden on lg:) -->
        <div class="lg:hidden fixed inset-0 z-50 flex justify-end bg-slate-900/40 backdrop-blur-sm" role="dialog" aria-modal="true">
          <button
            type="button"
            class="absolute inset-0 w-full h-full cursor-default focus:outline-none bg-transparent"
            aria-label="Close property sheet"
            onclick={() => (isPropertyPanelOpen = false)}
          ></button>
          <div class="relative w-full max-w-sm sm:max-w-md h-full bg-white dark:bg-slate-950 shadow-2xl flex flex-col z-10 border-l border-slate-200 dark:border-slate-800 animate-in slide-in-from-right duration-200">
            <TablePropertySheet
              {model}
              {selectedField}
              onClose={() => (isPropertyPanelOpen = false)}
              onSelectTable={() => (selectedField = null)}
              onMoveLogic={() => moveLogicToCompanion(model)}
              onOpenEvent={openEvent}
              onAddMethod={() => {
                const method = ensureMethod(model);
                const script = emitDocument(model).combined;
                onSourceChange?.(script);
                openCode(eventLine(script, method.name));
              }}
              onJumpToCode={openCode}
            />
          </div>
        </div>
      {/if}
    {:else}
      <div class="flex min-h-0 min-w-0 flex-1 flex-col bg-white dark:bg-slate-950">
        {#if isLoadingRecords && liveRecords.length === 0}
          <div class="py-12 text-center text-xs text-slate-400 space-y-2">
            <Spinner class="mx-auto" />
            <p>Fetching real rows from /api/entities/{tableName}...</p>
          </div>
        {:else if recordError}
          <div class="m-4">
            <Banner tone="danger" title="Rows could not be loaded">{recordError}</Banner>
          </div>
        {:else}
          <Datasheet
            class="min-h-0 flex-1 rounded-none border-0"
            label="{tableName} records"
            bind:rows={liveRecords}
            columns={sheetColumns}
            freezeCount={1}
            totalCount={totalRecords}
            offset={pageOffset}
            onCommitCell={commitSheetCell}
            onInsertRow={insertSheetRow}
            onDeleteRow={(row) => (pendingDeleteId = recordId(row))}
            onPreviousPage={pageOffset === 0 ? null : () => {
              pageOffset = Math.max(0, pageOffset - pageSize);
              loadLiveRecords();
            }}
            onNextPage={pageOffset + liveRecords.length >= totalRecords ? null : () => {
              pageOffset += pageSize;
              loadLiveRecords();
            }}
          />
        {/if}
      </div>
    {/if}
  </div>
</div>

<ConfirmDialog
  bind:open={() => Boolean(pendingDeleteId), (next) => { if (!next) pendingDeleteId = ""; }}
  tone="danger"
  title="Delete this row?"
  description="The row is removed from the live table. This cannot be undone from the designer."
  confirmLabel="Delete row"
  onConfirm={confirmDeleteRow}
  onCancel={() => (pendingDeleteId = "")}
/>

<ConfirmDialog
  bind:open={() => Boolean(blockedMigrationMessage), (next) => { if (!next) blockedMigrationMessage = ""; }}
  tone="danger"
  title="Apply a destructive schema change?"
  description={`${blockedMigrationMessage} Applying it permanently removes or converts existing data.`}
  confirmLabel="Apply anyway"
  onConfirm={confirmDestructiveApply}
  onCancel={() => (blockedMigrationMessage = "")}
/>
