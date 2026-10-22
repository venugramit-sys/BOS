<script>
  /**
   * WYSIWYG page designer (Svelte 5 runes).
   * - Develop: drag fields and controls, edit every property; changes are span edits of the `.bos`
   *   source (comments and formatting kept), applied with "Apply".
   * - Customize: super-user layer for everyone on this database (caption, visibility, importance,
   *   order, style, icons, added table fields), published with "Publish".
   * The canvas is the real compiled page (design build) with real records.
   */
  import { onMount, untrack } from "svelte";
  import { Code2, Monitor, Redo2, Smartphone, Tablet, Undo2, Maximize2, Rocket, RotateCcw } from "lucide-svelte";
  import { Button, Banner, Spinner } from "$lib/bos-ui/index.js";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";
  import DesignerCanvas from "./page/DesignerCanvas.svelte";
  import Outline from "./page/Outline.svelte";
  import Toolbox from "./page/Toolbox.svelte";
  import PropertyPanel from "./page/PropertyPanel.svelte";
  import { PageDesignerModel } from "./page/designer-model.svelte.js";

  let {
    pageName = "",
    sourceCode = "",
    packageName = "bos.app",
    onSourceChange = null,
    onJumpToCode = null,
    mode = $bindable("design"),
    isDirty = $bindable(false)
  } = $props();

  const model = untrack(() => new PageDesignerModel({ name: pageName, packageName, onSourceChange: (src) => onSourceChange?.(src) }));
  let leftTab = $state("toolbox");
  let root = $state(null);

  // The Studio resolves the object's package after the tab opens; Apply must store the object there.
  $effect(() => {
    model.packageName = packageName || "bos.app";
  });

  // Unsaved designer work keeps the Studio tab marked (close / switch warnings).
  $effect(() => {
    isDirty = model.dirty;
  });

  onMount(() => {
    pageManifest.start();
    model.init(untrack(() => sourceCode));
  });

  // The Studio fetches the object source asynchronously; pick it up once it arrives.
  let mounted = false;
  $effect(() => {
    const src = sourceCode;
    if (!mounted) {
      mounted = true;
      return;
    }
    untrack(() => model.sourceLoaded(src));
  });

  const VIEWPORTS = [
    { id: "phone", label: "Phone (375px)", icon: Smartphone },
    { id: "tablet", label: "Tablet (768px)", icon: Tablet },
    { id: "desktop", label: "Desktop (1280px)", icon: Monitor },
    { id: "full", label: "Full width", icon: Maximize2 }
  ];

  function onKey(e) {
    const mod = e.ctrlKey || e.metaKey;
    if (!mod) return;
    // Only while this designer is the visible tab, and never inside text fields (their own undo / typing).
    if (!root || root.offsetParent === null) return;
    const t = e.target;
    if (t instanceof HTMLElement && (t.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(t.tagName))) return;
    if (t instanceof Node && t !== document.body && !root.contains(t)) return;
    const k = e.key.toLowerCase();
    if (k === "z" && !e.shiftKey) {
      e.preventDefault();
      model.undo();
    } else if (k === "y" || (k === "z" && e.shiftKey)) {
      e.preventDefault();
      model.redo();
    } else if (k === "s") {
      e.preventDefault();
      model.save();
    }
  }

  function jumpToCode(line = null) {
    const node = model.selectedNode;
    onJumpToCode?.(model.source, line ?? node?.line ?? model.object?.line ?? 0);
    mode = "code";
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full min-h-0 flex-col bg-white text-slate-800 dark:bg-slate-900 dark:text-slate-100" bind:this={root}>
  <!-- toolbar -->
  <div class="flex flex-wrap items-center gap-2 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
    <div class="mr-2 min-w-0">
      <div class="truncate text-sm font-semibold">{model.compiled?.caption ?? pageName}</div>
      <div class="font-mono text-[10px] text-slate-400">{model.objectKind} {pageName}{model.objectKind === "pageextension" ? ` → ${model.targetPage}` : ""}</div>
    </div>

    <div class="inline-flex rounded-md bg-slate-100 p-0.5 dark:bg-slate-800" role="radiogroup" aria-label="Designer mode">
      {#each [["develop", "Develop"], ["customize", "Customize"]] as [id, label] (id)}
        <button
          type="button"
          role="radio"
          aria-checked={model.mode === id}
          disabled={(id === "develop" && !model.object) || (id === "customize" && !model.customizable)}
          class={"rounded px-2.5 py-1 text-xs font-semibold disabled:opacity-40 " + (model.mode === id ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300")}
          onclick={() => model.setMode(id)}
        >{label}</button>
      {/each}
    </div>

    <div class="inline-flex rounded-md border border-slate-200 dark:border-slate-700" role="radiogroup" aria-label="Preview width">
      {#each VIEWPORTS as v (v.id)}
        <button type="button" role="radio" aria-checked={model.viewport === v.id} title={v.label} aria-label={v.label} class={"p-1.5 " + (model.viewport === v.id ? "bg-indigo-50 text-indigo-600 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-500")} onclick={() => (model.viewport = v.id)}>
          <v.icon class="size-3.5" />
        </button>
      {/each}
    </div>

    <div class="flex items-center gap-1">
      <button type="button" class="rounded p-1.5 text-slate-500 hover:bg-slate-100 disabled:opacity-30 dark:hover:bg-slate-800" disabled={!model.canUndo || model.busy} title="Undo (Ctrl+Z)" aria-label="Undo" onclick={() => model.undo()}><Undo2 class="size-4" /></button>
      <button type="button" class="rounded p-1.5 text-slate-500 hover:bg-slate-100 disabled:opacity-30 dark:hover:bg-slate-800" disabled={!model.canRedo || model.busy} title="Redo (Ctrl+Y)" aria-label="Redo" onclick={() => model.redo()}><Redo2 class="size-4" /></button>
    </div>
    {#if model.busy}<Spinner size="sm" label="Updating" />{/if}

    <div class="ml-auto flex items-center gap-2">
      {#if model.mode === "customize"}
        <span class="text-[11px] text-slate-500">{model.changeCount} change{model.changeCount === 1 ? "" : "s"}</span>
        <Button size="xs" variant="ghost" icon={RotateCcw} disabled={model.changeCount === 0} onclick={() => model.resetCustomization()}>Reset all</Button>
      {:else}
        <Button size="xs" variant="ghost" icon={Code2} onclick={() => jumpToCode()}>Code</Button>
      {/if}
      <Button size="xs" icon={Rocket} loading={model.saving} disabled={!model.dirty || model.errors.length > 0} onclick={() => model.save()}>
        {model.mode === "customize" ? "Publish" : "Apply"}
      </Button>
    </div>
  </div>

  {#if model.apiPage}
    <div class="px-3 pt-2">
      <Banner tone="info" title="API page">
        <p class="text-xs">API pages publish data to integrations and have no user interface, so there is nothing to lay out visually. Their fields and actions are edited in code.</p>
        <div class="mt-2"><Button size="xs" variant="secondary" icon={Code2} onclick={() => jumpToCode()}>Open in code</Button></div>
      </Banner>
    </div>
  {/if}
  {#if model.parseError}
    <div class="px-3 pt-2">
      <Banner tone="warning" title="The page source has a syntax error">
        <p class="font-mono text-xs">{model.parseError.message}</p>
        <p class="mt-1 text-xs">
          {model.showingPublished ? "The canvas shows the published page (read-only) until the source parses again." : "Visual editing is paused until the source parses."}
        </p>
        <div class="mt-2"><Button size="xs" variant="secondary" icon={Code2} onclick={() => jumpToCode(model.parseError.line || null)}>Fix in code{model.parseError.line ? ` (line ${model.parseError.line})` : ""}</Button></div>
      </Banner>
    </div>
  {/if}
  {#if model.error}
    <div class="px-3 pt-2"><Banner tone="danger" title="Designer" dismissible onDismiss={() => (model.error = "")}><p class="text-xs">{model.error}</p></Banner></div>
  {/if}
  {#if model.notice}
    <div class="px-3 pt-2"><Banner tone="info" dismissible onDismiss={() => (model.notice = "")}><p class="text-xs">{model.notice}</p></Banner></div>
  {/if}

  <!-- body -->
  <div class="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[15rem_minmax(0,1fr)] xl:grid-cols-[15rem_minmax(0,1fr)_18rem]">
    <aside class="flex max-h-72 min-h-0 flex-col border-b border-slate-200 md:max-h-none md:border-b-0 md:border-r dark:border-slate-800">
      <div class="flex gap-1 border-b border-slate-200 px-2 pt-2 dark:border-slate-800" role="tablist">
        {#each [["toolbox", "Toolbox"], ["outline", "Outline"], ["properties", "Properties"]] as [id, label] (id)}
          <button type="button" role="tab" aria-selected={leftTab === id} class={"-mb-px border-b-2 px-2 py-1 text-xs font-semibold " + (id === "properties" ? "xl:hidden " : "") + (leftTab === id ? "border-indigo-600 text-indigo-600" : "border-transparent text-slate-500")} onclick={() => (leftTab = id)}>{label}</button>
        {/each}
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto p-2">
        {#if leftTab === "toolbox"}<Toolbox {model} />{:else if leftTab === "outline"}<Outline {model} />{:else}
          <div class="xl:hidden"><PropertyPanel {model} /></div>
          <p class="hidden p-2 text-xs text-slate-400 xl:block">Properties are shown on the right.</p>
        {/if}
      </div>
    </aside>

    <main class="min-h-[28rem] md:min-h-0">
      <DesignerCanvas {model} />
    </main>

    <aside class="hidden min-h-0 overflow-y-auto border-l border-slate-200 p-3 xl:block dark:border-slate-800">
      <PropertyPanel {model} />
    </aside>
  </div>

  <!-- diagnostics -->
  {#if model.diagnostics.length}
    <div class="max-h-32 overflow-y-auto border-t border-slate-200 px-3 py-2 text-xs dark:border-slate-800">
      {#each model.diagnostics as d, i (i)}
        <button type="button" class="flex w-full items-start gap-2 py-0.5 text-left hover:underline" onclick={() => d.path && (model.selected = d.path)}>
          <span class={"font-mono font-semibold " + (d.severity === "error" ? "text-rose-600" : "text-amber-600")}>{d.code}</span>
          <span>{d.message}</span>
          {#if d.path}<span class="ml-auto shrink-0 font-mono text-[10px] text-slate-400">{d.path}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
