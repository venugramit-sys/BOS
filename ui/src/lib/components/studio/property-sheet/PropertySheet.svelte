<script>
  /**
   * PropertySheet.svelte (Svelte 5 Runes)
   * Inspector for the selected page control.
   * A Group selector shows every property and event, or one property group or the event group.
   */
  import { Sliders } from "lucide-svelte";
  import { Button, Input, Checkbox, Badge, EmptyState, Select } from "$lib/bos-ui/index.js";
  import EventInspector from "./EventInspector.svelte";

  let {
    selectedElement = $bindable(null),
    onJumpToCode = null,
    onClose = null,
  } = $props();

  const labels = {
    type: "Control type",
    label: "Name",
    caption: "Caption",
    sourceField: "Control source",
    editable: "Editable",
    visible: "Visible",
  };

  const preferred = ["type", "label", "caption", "sourceField", "editable", "visible"];
  const propertyGroupDefs = [
    { id: "identity", label: "Identity", keys: ["type", "label", "caption"] },
    { id: "data", label: "Data", keys: ["sourceField"] },
    { id: "behavior", label: "Behavior", keys: ["editable", "visible"] },
  ];

  let groupChoice = $state("all");

  function textOf(element, key) {
    const value = element?.[key];
    return value == null ? "" : String(value);
  }

  function rowsFor(element) {
    if (!element) return [];
    const keys = Object.keys(element).filter((key) => {
      if (key === "id" || key === "icon") return false;
      const value = element[key];
      return typeof value === "string" || typeof value === "boolean" || typeof value === "number";
    });
    keys.sort((a, b) => {
      const ai = preferred.indexOf(a);
      const bi = preferred.indexOf(b);
      return (ai === -1 ? 99 : ai) - (bi === -1 ? 99 : bi);
    });
    return keys.map((key) => ({
      key,
      label: labels[key] || key,
      kind: typeof element[key] === "boolean" ? "bool" : key === "type" ? "readonly" : key === "sourceField" ? "code" : "text",
    }));
  }

  let propertyGroups = $derived.by(() => {
    const rows = rowsFor(selectedElement);
    const claimed = new Set();
    const grouped = [];
    for (const def of propertyGroupDefs) {
      const members = rows.filter((row) => def.keys.includes(row.key));
      if (!members.length) continue;
      for (const row of members) claimed.add(row.key);
      grouped.push({ id: def.id, label: def.label, rows: members });
    }
    const other = rows.filter((row) => !claimed.has(row.key));
    if (other.length) grouped.push({ id: "other", label: "Other", rows: other });
    return grouped;
  });

  let groups = $derived(
    selectedElement
      ? [
          { value: "all", label: "All" },
          ...propertyGroups.map((item) => ({ value: item.id, label: item.label })),
          { value: "events", label: "Events" },
        ]
      : []
  );

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

<div class="h-full flex flex-col select-none overflow-hidden bg-white dark:bg-slate-950">
  <div class="h-10 px-3 border-b border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-900/60 flex items-center justify-between shrink-0">
    <span class="text-xs font-bold uppercase tracking-wider flex items-center gap-1.5 min-w-0">
      <Sliders class="size-3.5 text-indigo-500 shrink-0" />
      <span class="truncate">Properties</span>
    </span>
    <div class="flex items-center gap-1 shrink-0">
      {#if selectedElement}
        <Badge>{selectedElement.type}</Badge>
      {/if}
      {#if onClose}
        <Button variant="ghost" size="xs" onclick={onClose}>Hide</Button>
      {/if}
    </div>
  </div>

  {#if groups.length}
    <div class="grid shrink-0 grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-200 px-3 py-1.5 dark:border-slate-800">
      <span class="text-[11px] text-slate-500">Group</span>
      <Select bind:value={groupChoice} options={groups} />
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto">
    {#if !selectedElement}
      <EmptyState title="No control selected" description="Select a control on the canvas. Every property of that control is listed here." />
    {:else}
      {#each propertyGroups as propertyGroup (propertyGroup.id)}
        {#if show(propertyGroup.id)}
          {#if group === "all"}
            <div class="px-3 pt-3 pb-1 text-[10px] font-bold tracking-wider text-slate-400 uppercase">{propertyGroup.label}</div>
          {/if}
          {#each propertyGroup.rows as row (row.key)}
            <div class="grid grid-cols-[7.25rem_minmax(0,1fr)] items-center gap-x-2 border-b border-slate-100 dark:border-slate-900 px-3 py-1.5">
              <span class="truncate text-[11px] text-slate-500" title={row.label}>{row.label}</span>
              <div class="min-w-0 text-xs">
                {#if row.kind === "readonly"}
                  <span class="font-mono text-[11px] text-slate-700 dark:text-slate-200">{textOf(selectedElement, row.key)}</span>
                {:else if row.kind === "bool"}
                  <Checkbox
                    bind:checked={() => Boolean(selectedElement[row.key]), (next) => (selectedElement[row.key] = next)}
                  />
                {:else}
                  <Input
                    size="sm"
                    type={row.kind === "code" ? "code" : "text"}
                    bind:value={() => textOf(selectedElement, row.key), (next) => (selectedElement[row.key] = next)}
                  />
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      {/each}

      {#if show("events")}
        <div class="px-3 py-3">
          {#if group === "all"}
            <div class="pb-2 text-[10px] font-bold tracking-wider text-slate-400 uppercase">Events</div>
          {/if}
          <EventInspector bind:selectedElement {onJumpToCode} />
        </div>
      {/if}
    {/if}
  </div>
</div>
