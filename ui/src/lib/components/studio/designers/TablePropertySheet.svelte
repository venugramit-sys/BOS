<script>
  /**
   * TablePropertySheet.svelte
   * Right-edge sheet for the selected table (table.bos) or the selected field.
   * A Group selector shows every group, or one property group or event group.
   * Logic members are the table.bl.bos companion.
   */
  import { X, Ellipsis, Sliders, Calculator } from "lucide-svelte";
  import { Button, Input, Select, Combobox, Checkbox, Badge, EmptyState } from "$lib/bos-ui/index.js";
  import FlowFieldWizardModal from "./FlowFieldWizardModal.svelte";
  import { FIELD_TYPES, TABLE_EVENTS, applyBaseType, applyLength, applyPrecision, typeParts } from "./tableModel.js";

  let {
    model = null,
    selectedField = null,
    onClose = null,
    onJumpToCode = null,
    onOpenEvent = null,
    onAddMethod = null,
    onSelectTable = null,
    onMoveLogic = null,
  } = $props();

  let isFlowFieldWizardOpen = $state(false);
  const tableTypes = ["Normal", "Document", "Ledger", "Temporal", "Temporary", "Virtual"].map((value) => ({ value, label: value }));
  const deleteOptions = [
    { value: "Restrict", label: "Restrict" },
    { value: "Cascade", label: "Cascade" },
    { value: "SetNull", label: "Set null" },
  ];
  const classifications = ["", "CustomerContent", "EUII", "SystemMetadata"].map((value) => ({
    value,
    label: value || "Customer content",
  }));

  let groupChoice = $state("all");

  let groups = $derived.by(() => {
    if (!model) return [];
    if (!selectedField) {
      const items = [
        { value: "all", label: "All" },
        { value: "properties", label: "Properties" },
        { value: "events", label: "Events" },
        { value: "methods", label: "Methods" },
      ];
      for (const [index, extension] of (model.extensions ?? []).entries()) {
        items.push({
          value: `ext:${index}`,
          label: extension.name || `Extension ${index + 1}`,
        });
      }
      return items;
    }
    return [
      { value: "all", label: "All" },
      { value: "general", label: "General" },
      { value: "relation", label: "Relation" },
      { value: "flowfield", label: "FlowField" },
      { value: "events", label: "Events" },
    ];
  });

  let group = $derived(groups.some((item) => item.value === groupChoice) ? groupChoice : "all");

  function show(id) {
    return group === "all" || group === id;
  }

  $effect(() => {
    const choices = groups.map((item) => item.value);
    if (choices.length > 0 && !choices.includes(groupChoice)) {
      groupChoice = "all";
    }
  });
</script>

<div class="flex h-full w-full flex-col overflow-hidden bg-white select-none dark:bg-slate-950">
  {#snippet eventRow(label, active, home, onclick)}
    <div class="grid grid-cols-[7.25rem_minmax(0,1fr)_1.75rem] items-center gap-x-2 border-b border-slate-100 px-3 py-1 dark:border-slate-900">
      <span class="truncate text-[11px] text-slate-500">{label}</span>
      <button
        type="button"
        class="truncate text-left font-mono text-[11px] {active ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-300'}"
        title={home === "logic" ? "Saved in the logic companion" : home === "table" ? "Saved in the table" : ""}
        onclick={onclick}
      >
        {active ? "[Event Procedure]" : ""}
      </button>
      <Button variant="ghost" size="xs" iconOnly icon={Ellipsis} aria-label="Build {label}" title="Open the code editor for this event" onclick={onclick} />
    </div>
  {/snippet}
  <div class="flex h-10 shrink-0 items-center justify-between border-b border-slate-200 bg-slate-50/80 px-3 dark:border-slate-800 dark:bg-slate-900/60">
    <div class="flex min-w-0 items-center gap-1.5">
      <Sliders class="size-3.5 shrink-0 text-indigo-500" />
      <span class="truncate text-xs font-bold tracking-wider uppercase">
        {#if selectedField}Field properties{:else}Table properties{/if}
      </span>
    </div>
    <div class="flex shrink-0 items-center gap-1">
      {#if selectedField}
        <Badge tone="info" class="max-w-28 truncate">{selectedField.name || "(unnamed)"}</Badge>
        <Button variant="ghost" size="xs" onclick={() => onSelectTable?.()}>Table</Button>
      {:else if model}
        <Badge>{model.tableType || "Normal"}</Badge>
      {/if}
      {#if onClose}
        <Button variant="ghost" size="xs" icon={X} onclick={onClose}>
          <span class="hidden sm:inline">Hide</span>
          <span class="sm:hidden font-semibold">Done</span>
        </Button>
      {/if}
    </div>
  </div>

  {#if groups.length}
    <div class="grid shrink-0 grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-200 px-3 py-1.5 dark:border-slate-800">
      <span class="text-[11px] text-slate-500">Group</span>
      <Select bind:value={groupChoice} options={groups} />
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto text-xs">
    {#if !model}
      <EmptyState title="No table" description="Open a table to edit its fields and logic." />
    {:else if !selectedField}
      {#if show("properties")}
      {#if group === "all"}
        <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Properties</div>
      {/if}
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Name</span>
        <span class="font-mono text-[11px]">{model.name}</span>
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Caption</span>
        <Input size="sm" bind:value={model.caption} placeholder={model.name} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Description</span>
        <Input size="sm" bind:value={model.description} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Table type</span>
        <Select bind:value={model.tableType} options={tableTypes} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Per company</span>
        <Checkbox bind:checked={model.dataPerCompany} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Lookup page</span>
        <Input size="sm" type="code" bind:value={model.lookupPage} placeholder="Page.CustomerList" />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Drill down</span>
        <Input size="sm" type="code" bind:value={model.drillDownPage} placeholder="Page.CustomerCard" />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Classification</span>
        <Select bind:value={model.dataClassification} options={classifications} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Obsolete</span>
        <Select
          bind:value={model.obsoleteState}
          options={[
            { value: "", label: "None" },
            { value: "Pending", label: "Pending" },
            { value: "Removed", label: "Removed" },
          ]}
        />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Reason</span>
        <Input size="sm" bind:value={model.obsoleteReason} />
      </div>
      {/if}

      {#if show("events")}
      {#if group === "all"}
        <div class="px-3 pt-4 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Events</div>
      {/if}
      {#each TABLE_EVENTS as event (event.name)}
        {@const trigger = model.triggers.find((item) => item.name === event.name)}
        {@render eventRow(event.label, Boolean(trigger?.body?.trim()), trigger?.home, () => onOpenEvent?.(event.name))}
      {/each}
      {#if model.triggers.some((trigger) => trigger.home === "table") || model.fields.some((field) => field.validate?.home === "table")}
        <div class="px-3 py-2">
          <Button size="xs" variant="ghost" onclick={() => onMoveLogic?.()}>Move inline events to the logic companion</Button>
        </div>
      {/if}
      {/if}

      {#if show("methods")}
      {#if group === "all"}
        <div class="px-3 pt-4 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Methods</div>
      {/if}
      {#each model.methods as method (method.id)}
        {@render eventRow(method.name || "method", Boolean(method.body?.trim()), method.home, () => onOpenEvent?.(`method:${method.name}`))}
      {/each}
      <div class="px-3 py-2">
        <Button size="xs" variant="secondary" onclick={() => onAddMethod?.()}>Add method</Button>
      </div>
      {/if}

      {#each model.extensions as extension, index}
        {#if show(`ext:${index}`)}
        {#if group === "all"}
          <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">{extension.name}.ext.bl.bos</div>
        {/if}
        {#each extension.triggers as trigger (trigger.id)}
          {@render eventRow(trigger.name, Boolean(trigger.body?.trim()), "extension", () => onOpenEvent?.(`method:${trigger.name}`))}
        {/each}
        {/if}
      {/each}
    {:else}
      {@const parts = typeParts(selectedField.dataType)}
      {#if show("general")}
      {#if group === "all"}
        <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">General</div>
      {/if}
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Field name</span>
        <Input size="sm" type="code" bind:value={selectedField.name} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Data type</span>
        <Combobox
          density="sm"
          clearable={false}
          showFactBox={false}
          showRecordCount={false}
          sortable={false}
          placeholder="Type"
          searchPlaceholder="Search types..."
          dropdownClass="w-52 max-w-52 sm:w-52 md:w-52"
          items={parts.base && !FIELD_TYPES.some((type) => type.id === parts.base) ? [{ id: parts.base, name: parts.base }, ...FIELD_TYPES] : FIELD_TYPES}
          value={parts.base}
          onSelect={(item) => applyBaseType(selectedField, item.id)}
        />
      </div>
      {#if parts.base === "String"}
        <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
          <span class="text-[11px] text-slate-500">Length</span>
          <Input size="sm" type="code" value={parts.length} placeholder="20" oninput={(event) => applyLength(selectedField, event.target.value)} />
        </div>
      {:else if parts.base === "Decimal"}
        <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
          <span class="text-[11px] text-slate-500">Precision</span>
          <Input size="sm" type="code" value={parts.precision} placeholder="18" oninput={(event) => applyPrecision(selectedField, event.target.value, parts.scale)} />
        </div>
        <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
          <span class="text-[11px] text-slate-500">Scale</span>
          <Input size="sm" type="code" value={parts.scale} placeholder="2" oninput={(event) => applyPrecision(selectedField, parts.precision, event.target.value)} />
        </div>
      {/if}
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Caption</span>
        <Input size="sm" bind:value={selectedField.caption} placeholder={selectedField.name} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Default value</span>
        <Input size="sm" type="code" bind:value={selectedField.defaultVal} placeholder='0, true, "Draft", auto()' />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Required</span>
        <Checkbox bind:checked={selectedField.notNull} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Unique</span>
        <Checkbox bind:checked={selectedField.unique} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Primary key</span>
        <Checkbox bind:checked={selectedField.isPrimaryKey} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Editable</span>
        <Checkbox bind:checked={selectedField.editable} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Visible</span>
        <Checkbox bind:checked={selectedField.visible} />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Searchable</span>
        <Checkbox bind:checked={selectedField.searchable} />
      </div>
      {/if}

      {#if show("relation")}
      {#if group === "all"}
        <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Relation</div>
      {/if}
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Foreign table</span>
        <Input size="sm" type="code" bind:value={selectedField.foreignTable} placeholder="Customer" />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">Foreign column</span>
        <Input size="sm" type="code" bind:value={selectedField.foreignColumn} placeholder="id" />
      </div>
      <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 px-3 py-1.5 dark:border-slate-900">
        <span class="text-[11px] text-slate-500">On delete</span>
        <Select bind:value={selectedField.onDeleteAction} options={deleteOptions} />
      </div>
      {/if}

      {#if show("flowfield")}
      {#if group === "all"}
        <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">FlowField</div>
      {/if}
      <div class="space-y-2 border-b border-slate-100 px-3 py-2 dark:border-slate-900">
        <Input size="sm" type="code" bind:value={selectedField.computed} placeholder={"sum(SalesLine.amount where { documentId == this.id })"} />
        <Button class="w-full" size="sm" icon={Calculator} variant="secondary" onclick={() => (isFlowFieldWizardOpen = true)}>
          Configure wizard
        </Button>
      </div>
      {/if}

      {#if show("events")}
      {#if group === "all"}
        <div class="px-3 pt-4 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Events</div>
      {/if}
      {@render eventRow("On Validate", Boolean(selectedField.validate?.body?.trim()), selectedField.validate?.home, () => onOpenEvent?.(`validate ${selectedField.name}`))}
      {/if}
    {/if}
  </div>
</div>

<FlowFieldWizardModal bind:isOpen={isFlowFieldWizardOpen} bind:field={selectedField} />
