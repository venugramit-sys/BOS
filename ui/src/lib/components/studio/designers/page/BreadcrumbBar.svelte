<script>
  /**
   * BreadcrumbBar.svelte
   * Builder.io-inspired hierarchical path & selection bar (Svelte 5 runes).
   * Displays the DOM / control ancestry of the active selection (e.g. Area > Group > Field).
   * Allows 1-click ancestor selection, sibling jumping, and node kind inspection.
   */
  import { ChevronRight, Layers, ArrowUp, Hash } from "lucide-svelte";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  const selectedPath = $derived(model.selected);

  /**
   * Deconstructs a node path (e.g. "area:Content/group:General/field:creditLimit")
   * into structured ancestry segments: [{ path, kind, name, isLast }].
   */
  const segments = $derived.by(() => {
    if (!selectedPath) return [];
    if (selectedPath === "page") {
      return [{ path: "page", kind: "page", name: model.name || "Page", isLast: true }];
    }
    if (selectedPath === "header") {
      return [
        { path: "page", kind: "page", name: model.name || "Page", isLast: false },
        { path: "header", kind: "header", name: "Header", isLast: true }
      ];
    }
    if (selectedPath === "header:title") {
      return [
        { path: "page", kind: "page", name: model.name || "Page", isLast: false },
        { path: "header", kind: "header", name: "Header", isLast: false },
        { path: "header:title", kind: "headline", name: model.compiled?.caption || "Title", isLast: true }
      ];
    }
    if (selectedPath === "header:instruction") {
      return [
        { path: "page", kind: "page", name: model.name || "Page", isLast: false },
        { path: "header", kind: "header", name: "Header", isLast: false },
        { path: "header:instruction", kind: "label", name: "Instruction", isLast: true }
      ];
    }
    if (selectedPath.startsWith("action:")) {
      const actionName = selectedPath.slice(7);
      return [
        { path: "page", kind: "page", name: model.name || "Page", isLast: false },
        { path: "actions", kind: "actions", name: "Actions", isLast: false },
        { path: selectedPath, kind: "action", name: actionName, isLast: true }
      ];
    }

    const parts = selectedPath.split("/");
    const result = [{ path: "page", kind: "page", name: model.name || "Page", isLast: false }];
    let currentPath = "";

    for (let i = 0; i < parts.length; i++) {
      currentPath = currentPath ? `${currentPath}/${parts[i]}` : parts[i];
      const colonIdx = parts[i].indexOf(":");
      const kind = colonIdx !== -1 ? parts[i].slice(0, colonIdx) : "node";
      const name = colonIdx !== -1 ? parts[i].slice(colonIdx + 1) : parts[i];
      result.push({
        path: currentPath,
        kind,
        name,
        isLast: i === parts.length - 1
      });
    }

    return result;
  });

  const parentPath = $derived.by(() => {
    if (!selectedPath || selectedPath === "page") return null;
    if (selectedPath === "header:title" || selectedPath === "header:instruction") return "header";
    if (selectedPath === "header") return "page";
    if (selectedPath.startsWith("action:")) return "page";
    const slashIdx = selectedPath.lastIndexOf("/");
    return slashIdx !== -1 ? selectedPath.slice(0, slashIdx) : "page";
  });

  function selectAncestor(path) {
    model.selected = path;
  }
</script>

{#if segments.length > 0}
  <div class="flex items-center gap-1 overflow-x-auto border-b border-slate-200 bg-slate-50/80 px-3 py-1.5 text-xs text-slate-600 dark:border-slate-800 dark:bg-slate-900/60 dark:text-slate-300">
    <div class="flex items-center gap-1 text-slate-400">
      <Layers class="size-3.5" />
      <span class="text-[10px] font-medium uppercase tracking-wider">Hierarchy:</span>
    </div>

    <div class="flex items-center gap-1 min-w-0">
      {#each segments as seg, idx (seg.path)}
        {#if idx > 0}
          <ChevronRight class="size-3 shrink-0 text-slate-300 dark:text-slate-600" />
        {/if}
        <button
          type="button"
          class={"group inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-xs transition-colors " +
            (seg.isLast
              ? "bg-indigo-100 font-semibold text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300 ring-1 ring-indigo-400/30"
              : "hover:bg-slate-200/80 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100")}
          onclick={() => selectAncestor(seg.path)}
          title={`Select ${seg.kind}: ${seg.name}`}
        >
          <span class="font-mono text-[9px] uppercase tracking-wider text-slate-400 group-hover:text-slate-600 dark:text-slate-500 dark:group-hover:text-slate-300">
            {seg.kind}
          </span>
          <span class="truncate max-w-[120px]">{seg.name}</span>
        </button>
      {/each}
    </div>

    {#if parentPath}
      <button
        type="button"
        class="ml-auto flex items-center gap-1 rounded border border-slate-200 bg-white px-2 py-0.5 text-[11px] font-medium text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700"
        title="Select Parent Container (Alt+Up)"
        onclick={() => selectAncestor(parentPath)}
      >
        <ArrowUp class="size-3 text-indigo-500" />
        <span>Parent</span>
      </button>
    {/if}
  </div>
{/if}
