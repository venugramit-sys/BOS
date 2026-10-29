<script>
  /**
   * EventInspector.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Event Procedure Wiring Inspector
   * Displays lifecycle hooks and triggers with '...' button that jumps directly to Monaco Editor.
   */
  import { Button } from "$lib/bos-ui/index.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  let {
    selectedElement = $bindable(null),
    onJumpToCode = null,
  } = $props();

  let availableEvents = $derived.by(() => {
    if (!selectedElement) return [];
    if (selectedElement.type === "action") {
      return [
        { name: "onAction", desc: "Invoked when user clicks the action button", templateCode: "async function onAction() {\n    // Code here\n}" },
      ];
    }
    if (selectedElement.type === "fasttab") {
      return [
        { name: "onCollapse", desc: "Invoked when fasttab expands or collapses", templateCode: "function onCollapse() {}" },
      ];
    }
    // Form fields
    return [
      { name: "onValidate", desc: "Triggered after value input is validated", templateCode: "async function onValidate(val) {\n    if (!val) error('Field required');\n}" },
      { name: "onLookup", desc: "Custom lookup dialog trigger", templateCode: "async function onLookup() {\n    // Lookup code\n}" },
      { name: "onAssistEdit", desc: "Drill-down or assist edit helper", templateCode: "function onAssistEdit() {}" },
    ];
  });

  function handleJump(ev) {
    if (onJumpToCode) {
      onJumpToCode(ev);
      return;
    }
    const current = studioStore.openTabs.find((tab) => tab.id === studioStore.activeTabId);
    if (current) {
      studioStore.openObject(current.name, "code", current.type);
    }
  }
</script>

<div class="space-y-3 select-none">
  <span class="text-[10px] font-bold text-slate-400 uppercase tracking-wider block">
    Event Procedures
  </span>

  <div class="space-y-2">
    {#each availableEvents as ev}
      <div class="p-2.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 space-y-1.5">
        <div class="flex items-center justify-between">
          <span class="text-xs font-mono font-semibold text-amber-600 dark:text-amber-400">
            {ev.name}
          </span>

          <!-- Access '...' Code Builder Button -->
          <Button size="xs" variant="secondary" tooltip="Edit in Monaco" onclick={() => handleJump(ev)}>
            Edit
          </Button>
        </div>
        <p class="text-[10px] text-slate-400 leading-tight">
          {ev.desc}
        </p>
      </div>
    {:else}
      <p class="text-xs text-slate-400 italic">No events available for this element.</p>
    {/each}
  </div>
</div>
