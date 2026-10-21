<script>
  /**
   * Toolbox: table fields (NAV Field Explorer: unplaced first; dragging one in auto-fills caption,
   * control and validation from the table) and predefined controls. Drag onto the canvas, or press
   * Enter / click "Add" to append to the selected container.
   */
  import { Search, Plus } from "lucide-svelte";
  import { CONTAINER_KINDS } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  let query = $state("");

  const CONTROLS = [
    { kind: "group", label: "Group (FastTab)", source: 'group(NewGroup) { caption = "New group"; }' },
    { kind: "grid", label: "Grid (columns)", source: "grid(NewGrid) { cols = 2; }" },
    { kind: "card", label: "Card", source: 'card(NewCard) { caption = "Card"; }' },
    { kind: "flex", label: "Row (flex)", source: "flex(NewRow) { }" },
    { kind: "tabs", label: "Tabs", source: 'tabs(NewTabs) {\n    tab(First) { caption = "First"; }\n}' },
    { kind: "split", label: "Split panes", source: 'split(NewSplit) {\n    group(Left) { caption = "Left"; }\n    group(Right) { caption = "Right"; }\n}' },
    { kind: "part", label: "Part (sub page)", source: "part(NewPart; PageName) { }" },
    { kind: "cuegroup", label: "Cue group", source: 'cuegroup(NewCues) { caption = "Activities"; }' },
    { kind: "label", label: "Label", source: 'label(NewLabel) { caption = "Text"; }' },
    { kind: "image", label: "Image", source: 'image(NewImage) { src = "/logo.png"; }' },
    { kind: "spacer", label: "Spacer", source: "spacer(NewSpacer);" },
    { kind: "divider", label: "Divider", source: "divider(NewDivider);" },
    { kind: "headline", label: "Headline", source: 'headline(NewHeadline) { caption = "Welcome"; }' }
  ];

  const pageType = $derived(model.compiled?.page_type ?? "Card");
  const allowed = $derived(new Set(model.schema?.page_types?.find((t) => t.name === pageType)?.allows ?? CONTROLS.map((c) => c.kind)));
  const placed = $derived(new Set(model.compiledControls.filter((c) => c.field).map((c) => (c.field.source ?? c.name).toLowerCase())));
  const fields = $derived(
    model.fields
      .filter((f) => !query || f.name.toLowerCase().includes(query.toLowerCase()) || (f.caption ?? "").toLowerCase().includes(query.toLowerCase()))
      .map((f) => ({ ...f, placed: placed.has(f.name.toLowerCase()) }))
      .sort((a, b) => Number(a.placed) - Number(b.placed))
  );
  const controls = $derived(CONTROLS.filter((c) => allowed.has(c.kind)));

  function fieldItem(f) {
    return { kind: "field", field: f.name, label: f.caption || f.name, source: `field(${f.name});` };
  }

  function start(e, item) {
    if (e.button !== 0) return;
    e.preventDefault();
    model.dragItem = item;
  }

  /** Keyboard / click: add into the selected container, or after the selected control. */
  function add(item) {
    const sel = model.selectedCompiled;
    if (sel && CONTAINER_KINDS.has(sel.kind)) return model.insertAt(item, { path: sel.path }, "inside");
    if (sel) return model.insertAt(item, { path: sel.path }, "after");
    return model.insertAt(item, null, "inside");
  }
</script>

<div class="flex flex-col gap-3 text-xs">
  <label class="flex items-center gap-1.5 rounded-md border border-slate-200 px-2 py-1.5 dark:border-slate-700">
    <Search class="size-3.5 text-slate-400" />
    <span class="sr-only">Filter fields</span>
    <input class="min-w-0 flex-1 bg-transparent outline-none" placeholder="Filter fields…" bind:value={query} />
  </label>

  <section>
    <h4 class="mb-1 text-[10px] font-bold uppercase tracking-wide text-slate-400">Table fields</h4>
    {#if fields.length === 0}
      <p class="px-1 text-slate-400">{model.fields.length ? "No field matches." : "The page has no source table."}</p>
    {/if}
    <ul class="flex flex-col">
      {#each fields as f (f.name)}
        <li>
          <div
            class={"group flex touch-none select-none items-center gap-2 rounded px-2 py-1 " + (f.placed ? "text-slate-400" : "cursor-grab hover:bg-slate-100 dark:hover:bg-slate-800")}
            onpointerdown={(e) => !f.placed && start(e, fieldItem(f))}
            role="button"
            tabindex={f.placed ? -1 : 0}
            aria-disabled={f.placed}
            aria-label={`Add field ${f.name}`}
            onkeydown={(e) => {
              if (!f.placed && (e.key === "Enter" || e.key === " ")) {
                e.preventDefault();
                add(fieldItem(f));
              }
            }}
          >
            <span class="truncate">{f.caption || f.name}</span>
            <span class="ml-auto shrink-0 font-mono text-[10px] text-slate-400">{f.flow ? "FlowField" : f.type}</span>
            {#if !f.placed}
              <button type="button" class="hidden rounded p-0.5 hover:bg-white group-hover:inline-flex dark:hover:bg-slate-900" aria-label={`Add ${f.name}`} onpointerdown={(e) => e.stopPropagation()} onclick={() => add(fieldItem(f))}><Plus class="size-3" /></button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  </section>

  <section class={model.mode === "customize" ? "opacity-50" : ""}>
    <h4 class="mb-1 text-[10px] font-bold uppercase tracking-wide text-slate-400">Controls {model.mode === "customize" ? "(Develop mode)" : ""}</h4>
    <ul class="grid grid-cols-2 gap-1">
      {#each controls as c (c.kind)}
        <li>
          <div
            class="flex touch-none select-none items-center rounded border border-slate-200 px-2 py-1.5 hover:border-indigo-400 dark:border-slate-700"
            class:cursor-grab={model.mode === "develop"}
            role="button"
            tabindex={model.mode === "develop" ? 0 : -1}
            aria-label={`Add ${c.label}`}
            onpointerdown={(e) => model.mode === "develop" && start(e, c)}
            onkeydown={(e) => {
              if (model.mode === "develop" && (e.key === "Enter" || e.key === " ")) {
                e.preventDefault();
                add(c);
              }
            }}
          >
            {c.label}
          </div>
        </li>
      {/each}
    </ul>
  </section>
</div>
