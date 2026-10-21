<script>
  /** Control tree of the compiled page (structure + actions) with keyboard-friendly reordering. */
  import { ArrowDown, ArrowUp, EyeOff, Zap } from "lucide-svelte";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  const rows = $derived.by(() => {
    const out = [];
    const walk = (nodes, depth) => {
      for (const n of nodes ?? []) {
        out.push({ node: n, depth });
        walk(n.children, depth + 1);
      }
    };
    walk(model.compiled?.controls, 0);
    return out;
  });
  const actions = $derived(model.compiled?.actions ?? []);

  function label(n) {
    if (n.field) return n.caption ?? n.name;
    return n.caption ? `${n.caption}` : n.name;
  }
</script>

<div class="flex flex-col gap-3 text-xs">
  <div role="tree" aria-label="Page structure" class="flex flex-col">
    <button
      type="button"
      role="treeitem"
      aria-selected={model.selected === "page"}
      class={"flex items-center gap-1 rounded px-2 py-1 text-left font-semibold " + (model.selected === "page" ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "hover:bg-slate-100 dark:hover:bg-slate-800")}
      onclick={() => (model.selected = "page")}
    >
      {model.compiled?.caption ?? model.name}
      <span class="ml-auto font-mono text-[10px] text-slate-400">{model.compiled?.page_type ?? ""}</span>
    </button>
    {#each rows as { node, depth } (node.path)}
      {@const selected = model.selected === node.path}
      <div class={"group flex items-center rounded " + (selected ? "bg-indigo-50 dark:bg-indigo-950" : "hover:bg-slate-100 dark:hover:bg-slate-800")} style:padding-left="{depth * 12 + 8}px">
        <button
          type="button"
          role="treeitem"
          aria-selected={selected}
          class={"flex min-w-0 flex-1 items-center gap-1.5 py-1 text-left " + (selected ? "text-indigo-700 dark:text-indigo-300" : "")}
          onclick={() => (model.selected = node.path)}
        >
          <span class="font-mono text-[10px] text-slate-400">{node.kind}</span>
          <span class="truncate">{label(node)}</span>
          {#if !node.visible}<EyeOff class="size-3 shrink-0 text-slate-400" aria-label="hidden" />{/if}
          {#if model.mode === "develop" && !model.isAuthored(node.path)}<span class="shrink-0 rounded bg-amber-100 px-1 text-[9px] text-amber-700 dark:bg-amber-950 dark:text-amber-300" title="Defined by an extension">ext</span>{/if}
        </button>
        {#if selected}
          <button type="button" class="rounded p-1 hover:bg-white dark:hover:bg-slate-900" aria-label="Move up (Alt+Up)" onclick={() => model.nudge(node.path, -1)}><ArrowUp class="size-3" /></button>
          <button type="button" class="rounded p-1 hover:bg-white dark:hover:bg-slate-900" aria-label="Move down (Alt+Down)" onclick={() => model.nudge(node.path, 1)}><ArrowDown class="size-3" /></button>
        {/if}
      </div>
    {/each}
  </div>

  {#if actions.length}
    <div>
      <h4 class="mb-1 px-2 text-[10px] font-bold uppercase tracking-wide text-slate-400">Actions</h4>
      {#each actions as a (a.name)}
        {@const path = `action:${a.name}`}
        <button
          type="button"
          class={"flex w-full items-center gap-1.5 rounded px-2 py-1 text-left " + (model.selected === path ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "hover:bg-slate-100 dark:hover:bg-slate-800")}
          onclick={() => (model.selected = path)}
        >
          <Zap class="size-3 shrink-0 text-slate-400" />
          <span class="truncate">{a.caption}</span>
          <span class="ml-auto font-mono text-[10px] text-slate-400">{a.area}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>
