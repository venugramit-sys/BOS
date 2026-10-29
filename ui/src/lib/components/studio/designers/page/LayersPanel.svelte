<script>
  /**
   * LayersPanel.svelte
   * Builder.io / Webflow-inspired visual component & layer tree (Svelte 5 runes).
   * - Interactive DOM and component hierarchy with real semantic tag pills & classes.
   * - Real-time two-way synchronization with canvas selection and hover.
   * - Instant layer search, expand/collapse all, and keyboard navigation.
   * - In-place visibility toggling, layer reordering (nudge), and element deletion.
   */
  import {
    Search,
    ChevronDown,
    ChevronRight,
    Eye,
    EyeOff,
    ArrowUp,
    ArrowDown,
    Trash2,
    Layers,
    SlidersHorizontal,
    FoldVertical,
    UnfoldVertical,
    Zap,
    Box,
    Type,
    LayoutGrid,
    Square
  } from "lucide-svelte";
  import { Input, Button, Badge } from "$lib/bos-ui/index.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  let searchQuery = $state("");
  let collapsedPaths = $state(new Set());
  let draggedPath = $state(null);
  let dropTarget = $state(null); // { path, placement }

  function handleDragStart(path, e) {
    draggedPath = path;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", path);
    }
  }

  function handleDragOver(path, isContainer, e) {
    if (!draggedPath || draggedPath === path) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";

    const rect = e.currentTarget.getBoundingClientRect();
    const relY = (e.clientY - rect.top) / rect.height;

    let placement = "after";
    if (relY < 0.25) {
      placement = "before";
    } else if (isContainer && relY >= 0.25 && relY <= 0.75) {
      placement = "inside";
    } else {
      placement = "after";
    }
    dropTarget = { path, placement };
  }

  function handleDragLeave(e) {
    if (e.relatedTarget && e.currentTarget.contains(e.relatedTarget)) return;
    dropTarget = null;
  }

  async function handleDrop(path, e) {
    e.preventDefault();
    e.stopPropagation();
    const sourcePath = draggedPath;
    const target = dropTarget;
    draggedPath = null;
    dropTarget = null;
    if (!sourcePath || !target || sourcePath === target.path) return;
    await model.moveTo(sourcePath, { path: target.path }, target.placement);
  }

  function handleDragEnd() {
    draggedPath = null;
    dropTarget = null;
  }

  function toggleCollapse(path, e) {
    e?.stopPropagation();
    const next = new Set(collapsedPaths);
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    collapsedPaths = next;
  }

  function expandAll() {
    collapsedPaths = new Set();
  }

  function collapseAll() {
    const next = new Set();
    const walk = (nodes) => {
      for (const n of nodes ?? []) {
        if (n.children?.length) {
          next.add(n.path);
          walk(n.children);
        }
      }
    };
    walk(model.compiled?.controls);
    next.add("header");
    collapsedPaths = next;
  }

  // Format node display info (tag, class, caption)
  function layerTag(n) {
    if (!n) return "div";
    const k = (n.kind ?? "").toLowerCase();
    if (k === "field") return "input.field";
    if (k === "label") return "p.label";
    if (k === "headline") return "h1.headline";
    if (k === "group") return "section.group";
    if (k === "grid") return "div.grid";
    if (k === "flex") return "div.flex";
    if (k === "card") return "div.card";
    if (k === "repeater") return "table.repeater";
    if (k === "part") return "component.part";
    if (k === "cue") return "div.cue";
    if (k === "area") return "main.area";
    return `${k || "div"}`;
  }

  function layerCaption(n) {
    if (n.field) return n.caption ?? n.name;
    return n.caption ? `${n.caption}` : n.name;
  }

  // Flatten tree for rendering while respecting collapsed branches and search filter
  const treeRows = $derived.by(() => {
    const out = [];
    const q = searchQuery.trim().toLowerCase();

    const walk = (nodes, depth) => {
      for (const n of nodes ?? []) {
        const hasChildren = Boolean(n.children?.length);
        const isCollapsed = collapsedPaths.has(n.path);
        const matchesQuery = !q ||
          (n.name && n.name.toLowerCase().includes(q)) ||
          (n.caption && n.caption.toLowerCase().includes(q)) ||
          (n.kind && n.kind.toLowerCase().includes(q));

        if (!q || matchesQuery) {
          out.push({
            node: n,
            depth,
            hasChildren,
            isCollapsed
          });
        }

        if (hasChildren && (!isCollapsed || q)) {
          walk(n.children, depth + 1);
        }
      }
    };

    walk(model.compiled?.controls, 0);
    return out;
  });

  const actions = $derived(model.compiled?.actions ?? []);
</script>

<div class="flex h-full flex-col text-xs bg-white dark:bg-slate-900">
  <!-- Search and Tree Actions Header (Compact & Responsive) -->
  <div class="p-1.5 border-b border-slate-200 dark:border-slate-800 space-y-1.5">
    <div class="relative flex items-center">
      <Search class="absolute left-2.5 size-3.5 text-slate-400" />
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Filter layers..."
        class="w-full h-7 rounded-md border border-slate-200 bg-slate-50 py-1 pl-8 pr-6 text-xs text-slate-800 placeholder-slate-400 outline-none transition focus:border-indigo-500 focus:bg-white dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100 dark:focus:border-indigo-500"
      />
      {#if searchQuery}
        <button
          type="button"
          class="absolute right-2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 text-xs"
          onclick={() => (searchQuery = "")}
        >
          &times;
        </button>
      {/if}
    </div>

    <div class="flex items-center justify-between text-[10px] text-slate-500 px-0.5">
      <div class="flex items-center gap-1 font-semibold uppercase tracking-wider">
        <Layers class="size-3 text-indigo-500" />
        <span>Hierarchy ({treeRows.length + 3})</span>
      </div>
      <div class="flex items-center gap-0.5">
        <button
          type="button"
          title="Expand All"
          class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
          onclick={expandAll}
        >
          <UnfoldVertical class="size-3" />
        </button>
        <button
          type="button"
          title="Collapse All"
          class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
          onclick={collapseAll}
        >
          <FoldVertical class="size-3" />
        </button>
      </div>
    </div>
  </div>

  <!-- Layers Tree View -->
  <div class="flex-1 overflow-y-auto p-1.5 space-y-0.5 select-none" role="tree" aria-label="Page layers">
    <!-- Root Page Layer -->
    <div
      role="treeitem"
      aria-selected={model.selected === "page"}
      class={"group flex items-center gap-1.5 rounded-md px-2 py-1.5 cursor-pointer transition-colors " +
        (model.selected === "page"
          ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
          : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
      onclick={() => (model.selected = "page")}
    >
      <Box class="size-3.5 shrink-0 text-indigo-500" />
      <span class="font-mono text-[10px] px-1 py-0.2 bg-slate-100 dark:bg-slate-800 rounded text-slate-600 dark:text-slate-400">
        page.{model.compiled?.page_type ?? "doc"}
      </span>
      <span class="truncate flex-1 font-medium">{model.compiled?.caption ?? model.name}</span>
    </div>

    <!-- Header Region Layer Group -->
    <div class="mt-1 pt-1 border-t border-slate-100 dark:border-slate-800/80">
      <div
        role="treeitem"
        aria-selected={model.selected === "header"}
        class={"group flex items-center gap-1 rounded-md px-2 py-1 cursor-pointer transition-colors " +
          (model.selected === "header"
            ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
            : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
        onclick={() => (model.selected = "header")}
      >
        <button
          type="button"
          class="p-0.5 text-slate-400 hover:text-slate-600"
          onclick={(e) => toggleCollapse("header", e)}
        >
          {#if collapsedPaths.has("header")}
            <ChevronRight class="size-3" />
          {:else}
            <ChevronDown class="size-3" />
          {/if}
        </button>
        <span class="font-mono text-[10px] px-1 py-0.2 bg-slate-100 dark:bg-slate-800 rounded text-slate-600 dark:text-slate-400">
          header
        </span>
        <span class="truncate flex-1 text-slate-500 dark:text-slate-400">Page Header</span>
      </div>

      {#if !collapsedPaths.has("header")}
        <!-- Header Title Layer -->
        <div
          role="treeitem"
          aria-selected={model.selected === "header:title"}
          class={"group flex items-center gap-1.5 rounded-md pl-6 pr-2 py-1 cursor-pointer transition-colors " +
            (model.selected === "header:title"
              ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
              : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
          onclick={() => (model.selected = "header:title")}
        >
          <Type class="size-3 shrink-0 text-slate-400" />
          <span class="font-mono text-[10px] px-1 py-0.2 bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 rounded">
            h1.title
          </span>
          <span class="truncate flex-1 font-medium">{model.compiled?.caption ?? "Title"}</span>
        </div>

        <!-- Header Instruction/Subtitle Layer -->
        {#if model.compiled?.instructional_text || model.object?.instructional_text}
          <div
            role="treeitem"
            aria-selected={model.selected === "header:instruction"}
            class={"group flex items-center gap-1.5 rounded-md pl-6 pr-2 py-1 cursor-pointer transition-colors " +
              (model.selected === "header:instruction"
                ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
                : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
            onclick={() => (model.selected = "header:instruction")}
          >
            <span class="font-mono text-[10px] px-1 py-0.2 bg-slate-100 dark:bg-slate-800 rounded text-slate-500">
              p.hint
            </span>
            <span class="truncate flex-1 text-slate-500 dark:text-slate-400">
              {model.compiled?.instructional_text ?? model.object?.instructional_text}
            </span>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Main Content Canvas Layers -->
    <div class="mt-2 pt-1 border-t border-slate-100 dark:border-slate-800/80 space-y-0.5">
      {#each treeRows as { node, depth, hasChildren, isCollapsed } (node.path)}
        {@const selected = model.selected === node.path}
        {@const isDropTarget = dropTarget?.path === node.path}
        {@const dropPlacement = isDropTarget ? dropTarget.placement : null}
        <div
          role="treeitem"
          aria-selected={selected}
          draggable={model.mode === "develop"}
          ondragstart={(e) => handleDragStart(node.path, e)}
          ondragover={(e) => handleDragOver(node.path, hasChildren, e)}
          ondragleave={handleDragLeave}
          ondrop={(e) => handleDrop(node.path, e)}
          ondragend={handleDragEnd}
          class={"group relative flex items-center gap-1 rounded-md py-1 pr-2 cursor-pointer transition-colors " +
            (selected
              ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
              : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300") +
            (dropPlacement === "before" ? " border-t-2 border-indigo-600" : "") +
            (dropPlacement === "after" ? " border-b-2 border-indigo-600" : "") +
            (dropPlacement === "inside" ? " ring-1 ring-indigo-500 bg-indigo-50/60 dark:bg-indigo-950/60" : "")}
          style:padding-left="{(depth * 10) + 4}px"
          onclick={() => (model.selected = node.path)}
        >
          <!-- Expand/Collapse Chevron -->
          {#if hasChildren}
            <button
              type="button"
              class="p-0.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
              onclick={(e) => toggleCollapse(node.path, e)}
            >
              {#if isCollapsed}
                <ChevronRight class="size-3" />
              {:else}
                <ChevronDown class="size-3" />
              {/if}
            </button>
          {:else}
            <span class="w-4 shrink-0"></span>
          {/if}

          <!-- Semantic Tag Badge -->
          <span class="font-mono text-[10px] px-1 py-0.2 rounded bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 shrink-0">
            {layerTag(node)}
          </span>

          <!-- Layer Caption / Label -->
          <span class="truncate flex-1 min-w-0">{layerCaption(node)}</span>

          <!-- Status Indicators -->
          {#if !node.visible}
            <EyeOff class="size-3 shrink-0 text-slate-400" aria-label="hidden" />
          {/if}
          {#if model.mode === "develop" && !model.isAuthored(node.path)}
            <span class="shrink-0 rounded bg-amber-100 px-1 text-[9px] text-amber-700 dark:bg-amber-950 dark:text-amber-300" title="Defined by an extension">
              ext
            </span>
          {/if}

          <!-- Quick Action Controls on Hover/Select -->
          <div class={"items-center gap-0.5 " + (selected ? "flex" : "hidden group-hover:flex")}>
            <button
              type="button"
              class="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-500"
              title="Move Up (Alt+Up)"
              onclick={(e) => {
                e.stopPropagation();
                model.nudge(node.path, -1);
              }}
            >
              <ArrowUp class="size-3" />
            </button>
            <button
              type="button"
              class="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-700 text-slate-500"
              title="Move Down (Alt+Down)"
              onclick={(e) => {
                e.stopPropagation();
                model.nudge(node.path, 1);
              }}
            >
              <ArrowDown class="size-3" />
            </button>
            <button
              type="button"
              class="p-0.5 rounded hover:bg-rose-100 dark:hover:bg-rose-950 text-slate-400 hover:text-rose-600"
              title="Delete Layer"
              onclick={(e) => {
                e.stopPropagation();
                model.remove(node.path);
              }}
            >
              <Trash2 class="size-3" />
            </button>
          </div>
        </div>
      {/each}
    </div>

    <!-- Actions Section -->
    {#if actions.length > 0}
      <div class="mt-3 pt-2 border-t border-slate-100 dark:border-slate-800">
        <h4 class="mb-1 px-2 text-[10px] font-bold uppercase tracking-wider text-slate-400">
          Actions ({actions.length})
        </h4>
        {#each actions as a (a.name)}
          {@const path = `action:${a.name}`}
          {@const selected = model.selected === path}
          <div
            role="treeitem"
            aria-selected={selected}
            class={"group flex items-center gap-1.5 rounded-md px-2 py-1 cursor-pointer transition-colors " +
              (selected
                ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 font-semibold border-l-2 border-indigo-600"
                : "hover:bg-slate-100 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
            onclick={() => (model.selected = path)}
          >
            <Zap class="size-3 shrink-0 text-indigo-500" />
            <span class="font-mono text-[10px] px-1 py-0.2 bg-slate-100 dark:bg-slate-800 rounded text-slate-500">
              action
            </span>
            <span class="truncate flex-1 font-medium">{a.caption ?? a.name}</span>
            <span class="font-mono text-[9px] text-slate-400">{a.area ?? "command"}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
