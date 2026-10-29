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
  import { Code2, Monitor, Redo2, Smartphone, Tablet, Undo2, Maximize2, Rocket, RotateCcw, Columns2, Sparkles, UserCheck, PanelLeft, PanelRight, PanelLeftClose, PanelRightClose, Sliders, X } from "lucide-svelte";
  import MonacoCodeStudio from "../code-editor/MonacoCodeStudio.svelte";
  import { Button, Banner, Spinner } from "$lib/bos-ui/index.js";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";
  import DesignerCanvas from "./page/DesignerCanvas.svelte";
  import LayersPanel from "./page/LayersPanel.svelte";
  import Outline from "./page/Outline.svelte";
  import Toolbox from "./page/Toolbox.svelte";
  import PropertyPanel from "./page/PropertyPanel.svelte";
  import BreadcrumbBar from "./page/BreadcrumbBar.svelte";
  import ActionBuilderModal from "./page/ActionBuilderModal.svelte";
  import { PageDesignerModel } from "./page/designer-model.svelte.js";

  let {
    pageName = "",
    sourceCode = "",
    packageName = "bos.app",
    onSourceChange = null,
    onJumpToCode = null,
    mode = $bindable("design"),
    isDirty = $bindable(false),
    theme = "dark"
  } = $props();

  const model = untrack(() => new PageDesignerModel({ name: pageName, packageName, onSourceChange: (src) => onSourceChange?.(src) }));
  let leftTab = $state("layers");
  let showLeftPanel = $state(true);
  let showRightPanel = $state(true);
  let root = $state(null);
  let viewMode = $state("design"); // "design" | "split"
  let isActionModalOpen = $state(false);
  let modalDefaultArea = $state("Processing");
  let modalDefaultGroup = $state("");
  let simulatedPersona = $state("admin");

  // Ensure property sheet is visible when an element is actively selected
  $effect(() => {
    if (model.selected) {
      showRightPanel = true;
    }
  });

  function openActionModal(area = "Processing", group = "") {
    modalDefaultArea = area || "Processing";
    modalDefaultGroup = group || "";
    isActionModalOpen = true;
  }

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
    const handleOpenActionModal = (e) => {
      openActionModal(e.detail?.area, e.detail?.group);
    };
    window.addEventListener("bos:open-action-builder", handleOpenActionModal);
    return () => {
      window.removeEventListener("bos:open-action-builder", handleOpenActionModal);
    };
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
      {#each [["develop", "Design"], ["preview", "Interact"]] as [id, label] (id)}
        <button
          type="button"
          role="radio"
          aria-checked={model.mode === id}
          disabled={id === "develop" && !model.object}
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

    <div class="inline-flex rounded-md bg-slate-100 p-0.5 dark:bg-slate-800" role="radiogroup" aria-label="Layout view">
      <button
        type="button"
        class={"rounded px-2.5 py-1 text-xs font-semibold " + (viewMode === "design" ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300")}
        onclick={() => (viewMode = "design")}
        title="Visual Designer Only"
      >
        Canvas
      </button>
      <button
        type="button"
        class={"inline-flex items-center gap-1 rounded px-2.5 py-1 text-xs font-semibold " + (viewMode === "split" ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300")}
        onclick={() => (viewMode = "split")}
        title="Side-by-Side Dual View (Monaco + Canvas)"
      >
        <Columns2 class="size-3" /> Split
      </button>
    </div>

    <div class="ml-auto flex items-center gap-1.5">
      <Button
        size="xs"
        variant={showLeftPanel ? "secondary" : "ghost"}
        icon={PanelLeft}
        title="Toggle Layers & Elements Panel"
        onclick={() => (showLeftPanel = !showLeftPanel)}
      >
        <span class="hidden md:inline">Layers</span>
      </Button>
      <Button
        size="xs"
        variant={showRightPanel ? "secondary" : "ghost"}
        icon={Sliders}
        title="Toggle Properties Panel"
        onclick={() => (showRightPanel = !showRightPanel)}
      >
        <span class="hidden md:inline">Properties</span>
      </Button>
      <Button size="xs" variant="ghost" icon={Code2} onclick={() => jumpToCode()}>Full Code</Button>
      <Button size="xs" icon={Rocket} loading={model.saving} disabled={!model.dirty || model.errors.length > 0} onclick={() => model.save()}>
        Apply
      </Button>
    </div>
  </div>

  <!-- Hierarchical DOM / Control Ancestry Bar -->
  <BreadcrumbBar {model} />

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
  {#if viewMode === "split"}
    <div class="grid min-h-0 flex-1 grid-cols-1 lg:grid-cols-2 divide-y lg:divide-y-0 lg:divide-x divide-slate-200 dark:divide-slate-800">
      <div class="min-h-[20rem] flex flex-col min-h-0 overflow-hidden">
        <MonacoCodeStudio
          objectName={pageName}
          objectType={model.objectKind || "page"}
          packageName={packageName}
          bind:content={model.source}
          focusLine={model.selectedNode?.line || 0}
          {theme}
          onChange={(newSrc) => {
            if (newSrc !== undefined) model.sourceLoaded(newSrc);
          }}
          onApplySuccess={() => {
            model.dirty = false;
          }}
        />
      </div>
      <div class="min-h-[20rem] flex flex-col min-h-0 overflow-hidden bg-slate-100/50 dark:bg-slate-950">
        <main class="min-h-0 flex-1 overflow-auto">
          <DesignerCanvas {model} />
        </main>
      </div>
    </div>
  {:else if model.mode === "preview"}
    <div class="flex min-h-0 flex-1 flex-col overflow-hidden bg-slate-100/70 dark:bg-slate-950">
      <div class="flex items-center justify-between border-b border-slate-200 bg-white px-4 py-2 text-xs dark:border-slate-800 dark:bg-slate-900">
        <span class="inline-flex items-center gap-2 font-medium text-emerald-600 dark:text-emerald-400">
          <span class="relative flex size-2">
            <span class="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-75"></span>
            <span class="relative inline-flex size-2 rounded-full bg-emerald-500"></span>
          </span>
          Interactive Multi-Device Preview &middot; Live Draft
        </span>
        <div class="flex items-center gap-2">
          <span class="text-[11px] font-semibold text-slate-500 flex items-center gap-1">
            <UserCheck class="size-3 text-indigo-500" />
            <span>Simulate Persona:</span>
          </span>
          <select
            class="rounded border border-slate-300 bg-white px-2 py-0.5 text-xs font-semibold text-slate-700 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            bind:value={simulatedPersona}
          >
            <option value="admin">System Administrator (Full Access)</option>
            <option value="sales">Sales Representative (Orders Only)</option>
            <option value="accountant">Financial Accountant (Posting Active)</option>
            <option value="auditor">Auditor (Read-Only Enforced)</option>
          </select>
        </div>
      </div>
      <main class="min-h-0 flex-1">
        <DesignerCanvas {model} {simulatedPersona} />
      </main>
    </div>
  {:else}
    <div class="relative flex min-h-0 flex-1 overflow-hidden">
      <!-- Left Sidebar: Layers & Elements (Collapsible & Responsive) -->
      {#if showLeftPanel}
        <aside class="flex w-60 sm:w-64 md:w-64 lg:w-68 shrink-0 flex-col border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 z-10 transition-all duration-150">
          <div class="flex items-center justify-between border-b border-slate-200 px-2 pt-1.5 dark:border-slate-800" role="tablist">
            <div class="flex gap-1">
              {#each [["layers", "Layers"], ["toolbox", "Elements"]] as [id, label] (id)}
                <button
                  type="button"
                  role="tab"
                  aria-selected={leftTab === id}
                  class={"-mb-px border-b-2 px-2.5 py-1 text-xs font-semibold transition-colors " +
                    (leftTab === id
                      ? "border-indigo-600 text-indigo-600 dark:text-indigo-400"
                      : "border-transparent text-slate-500 hover:text-slate-800 dark:hover:text-slate-200")}
                  onclick={() => (leftTab = id)}
                >
                  {label}
                </button>
              {/each}
            </div>
            <button
              type="button"
              class="p-1 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 rounded"
              title="Collapse left panel"
              onclick={() => (showLeftPanel = false)}
            >
              <PanelLeftClose class="size-3.5" />
            </button>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
            {#if leftTab === "layers"}
              <LayersPanel {model} {openActionModal} />
            {:else}
              <Toolbox {model} {openActionModal} />
            {/if}
          </div>
        </aside>
      {/if}

      <!-- Center Main: Designer Canvas -->
      <main class="min-h-0 flex-1 flex flex-col overflow-hidden bg-slate-100/50 dark:bg-slate-950">
        <DesignerCanvas {model} />
      </main>

      <!-- Right Panel: Properties Inspector (Docked on lg+, Overlay Drawer on <lg) -->
      {#if showRightPanel}
        <!-- Desktop Docked Aside (visible on lg+) -->
        <aside class="hidden lg:flex w-72 xl:w-80 shrink-0 flex-col border-l border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden z-10 transition-all duration-150">
          <div class="flex items-center justify-between border-b border-slate-200 px-3 py-1.5 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-900/60">
            <span class="text-xs font-bold uppercase tracking-wider text-slate-600 dark:text-slate-300">Properties</span>
            <button
              type="button"
              class="p-1 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 rounded"
              title="Close properties panel"
              onclick={() => (showRightPanel = false)}
            >
              <PanelRightClose class="size-3.5" />
            </button>
          </div>
          <div class="min-h-0 flex-1 overflow-y-auto p-2.5">
            <PropertyPanel {model} />
          </div>
        </aside>

        <!-- Tablet / Mobile Slide-over Drawer (visible on <lg) -->
        <div class="lg:hidden fixed inset-0 z-50 flex justify-end bg-slate-900/40 backdrop-blur-xs" role="dialog" aria-modal="true">
          <button
            type="button"
            class="absolute inset-0 size-full cursor-default bg-transparent"
            aria-label="Close properties drawer"
            onclick={() => (showRightPanel = false)}
          ></button>
          <div class="relative w-full max-w-sm sm:max-w-md h-full bg-white dark:bg-slate-900 shadow-2xl flex flex-col z-10 border-l border-slate-200 dark:border-slate-800 animate-in slide-in-from-right duration-150">
            <div class="flex items-center justify-between border-b border-slate-200 px-3.5 py-2.5 dark:border-slate-800 bg-slate-50 dark:bg-slate-900">
              <span class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-200">Properties Inspector</span>
              <button
                type="button"
                class="p-1 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 rounded-lg hover:bg-slate-200/60 dark:hover:bg-slate-800 transition-colors"
                onclick={() => (showRightPanel = false)}
              >
                <X class="size-4" />
              </button>
            </div>
            <div class="min-h-0 flex-1 overflow-y-auto p-3">
              <PropertyPanel {model} />
            </div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  <!-- diagnostics -->
  {#if model.diagnostics.length}
    <div class="max-h-36 overflow-y-auto border-t border-slate-200 px-3 py-2 text-xs dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/40">
      <div class="flex items-center justify-between pb-1 mb-1 border-b border-slate-200 dark:border-slate-800">
        <span class="font-bold text-[10px] uppercase tracking-wider text-slate-500">
          Problems & Diagnostics ({model.diagnostics.length})
        </span>
      </div>
      {#each model.diagnostics as d, i (i)}
        {@const missingCu = (d.code === "BOS3004" || d.message?.includes("Codeunit '")) ? d.message?.match(/Codeunit '([^']+)'/)?.[1] : null}
        <div class="flex items-center justify-between gap-2 py-1 hover:bg-slate-100/60 dark:hover:bg-slate-800/60 rounded px-1.5 transition-colors">
          <button type="button" class="flex min-w-0 flex-1 items-start gap-2 text-left hover:underline" onclick={() => d.path && (model.selected = d.path)}>
            <span class={"font-mono font-semibold shrink-0 " + (d.severity === "error" ? "text-rose-600" : "text-amber-600")}>{d.code}</span>
            <span class="truncate text-slate-700 dark:text-slate-300">{d.message}</span>
            {#if d.path}<span class="ml-auto shrink-0 font-mono text-[10px] text-slate-400">{d.path}</span>{/if}
          </button>
          {#if missingCu}
            <Button
              size="xs"
              variant="outline"
              class="h-5 px-2 text-[10px] gap-1 shrink-0 text-indigo-600 border-indigo-300 dark:border-indigo-800 hover:bg-indigo-50 dark:hover:bg-indigo-950 font-semibold"
              onclick={() => model.scaffoldCodeunit(missingCu)}
            >
              <Sparkles class="size-2.5" /> Scaffold {missingCu}
            </Button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <ActionBuilderModal
    bind:open={isActionModalOpen}
    {model}
    defaultArea={modalDefaultArea}
    defaultGroup={modalDefaultGroup}
  />
</div>
