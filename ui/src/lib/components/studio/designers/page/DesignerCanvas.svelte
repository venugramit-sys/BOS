<script>
  /**
   * WYSIWYG canvas: the real compiled page (design build, real records) with an overlay for hover,
   * selection and drop indicators. Controls carry `data-bos-node` paths; dragging uses pointer events
   * (mouse, pen and touch), with auto-scroll near the edges and Alt+Arrow keys as the keyboard path.
   */
  import { untrack, onMount } from "svelte";
  import { ArrowUp, ArrowDown, Trash2, ArrowUpCircle, Copy, Edit3 } from "lucide-svelte";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { cn } from "$lib/utils.js";
  import { CONTAINER_KINDS, isWithin, parentPath as parentOf } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel, simulatedPersona?: string }} */
  let { model, simulatedPersona = "admin" } = $props();

  onMount(() => {
    const handleReorder = (e) => {
      const { source, target } = e.detail ?? {};
      if (source && target) model.reorderAction(source, target);
    };
    window.addEventListener("bos:reorder-action", handleReorder);
    return () => window.removeEventListener("bos:reorder-action", handleReorder);
  });

  const WIDTHS = { phone: 375, tablet: 768, desktop: 1280, full: null };
  const DRAG_THRESHOLD = 5;
  const EDGE = 48;

  let scroller = $state(null);
  let surface = $state(null);
  let tick = $state(0);
  let hoverPath = $state(null);
  let drag = $state(null); // { path?, item?, x, y, target, placement }
  let pending = null;
  let inlineEdit = $state(null); // { path, text, left, top, width, height }

  function focusInput(node) {
    node.focus();
    node.select();
  }

  function startInlineEdit(path) {
    if (!path || path === "page" || isPreview) return;
    const el = nodeEl(path);
    if (!el || !scroller) return;

    const textTarget = el.querySelector(".bos-field-label, .bos-caption, .bos-action, .bos-headline, h1, h2, h3, label") || el;
    const currentText = model.selectedNode?.caption || model.selectedCompiled?.caption || textTarget?.textContent?.trim() || "";

    const r = (textTarget || el).getBoundingClientRect();
    const o = scroller.getBoundingClientRect();
    inlineEdit = {
      path,
      text: currentText,
      left: r.left - o.left + scroller.scrollLeft,
      top: r.top - o.top + scroller.scrollTop,
      width: Math.max(r.width, 140),
      height: Math.max(r.height, 28)
    };
  }

  async function commitInlineEdit(newText) {
    if (!inlineEdit) return;
    const { path, text } = inlineEdit;
    inlineEdit = null;
    const trimmed = String(newText ?? "").trim();
    if (trimmed !== text) {
      await model.setProperty(path, "caption", JSON.stringify(trimmed));
    }
  }

  function onDoubleClick(e) {
    if (isPreview) return;
    const hit = nodeAt(e.clientX, e.clientY);
    if (hit) {
      model.selected = hit.path;
      startInlineEdit(hit.path);
    }
  }

  const isPreview = $derived(model.mode === "preview");
  const ctx = $derived({
    page: model.targetPage || model.name,
    design: !isPreview,
    mode: isPreview ? "edit" : "view",
    openPage: (_target, _opts) => {
      // Interactive preview handler for sub-page transition
    },
    navigate: (_path) => {
      // Interactive preview handler for route navigation
    }
  });

  function nodeEl(path) {
    if (!surface || !path) return null;
    if (path === "page") return surface.querySelector('.bos-page[data-bos-node="page"]') || surface.querySelector('.bos-page') || surface.querySelector('[data-bos-node="page"]');
    if (path === "header") return surface.querySelector('.bos-page-header');
    if (path === "header:title") return surface.querySelector('.bos-page-title');
    if (path === "header:instruction") return surface.querySelector('.bos-hint');
    return surface.querySelector(`[data-bos-node="${CSS.escape(path)}"]`);
  }

  function rectOf(path) {
    tick;
    const el = nodeEl(path);
    if (!el || !scroller) return null;
    const r = el.getBoundingClientRect();
    const o = scroller.getBoundingClientRect();
    return { left: r.left - o.left + scroller.scrollLeft, top: r.top - o.top + scroller.scrollTop, width: r.width, height: r.height };
  }

  const hoverRect = $derived(drag ? null : rectOf(hoverPath));
  const selectedRect = $derived(rectOf(model.selected));
  const dropRect = $derived.by(() => {
    if (!drag?.target) return null;
    const r = rectOf(drag.target.path);
    if (!r) return null;
    if (drag.placement === "inside") return { ...r, inside: true };
    const horizontal = drag.axis === "x";
    if (horizontal) {
      const left = drag.placement === "before" ? r.left - 2 : r.left + r.width - 1;
      return { left, top: r.top, width: 3, height: r.height };
    }
    const top = drag.placement === "before" ? r.top - 2 : r.top + r.height - 1;
    return { left: r.left, top, width: r.width, height: 3 };
  });

  // Re-measure when the page renders, loads data or resizes.
  $effect(() => {
    if (!surface) return;
    const bump = () => (tick += 1);
    const ro = new ResizeObserver(bump);
    ro.observe(surface);
    const mo = new MutationObserver(bump);
    mo.observe(surface, { childList: true, subtree: true, attributes: true, attributeFilter: ["class", "style"] });
    return () => {
      ro.disconnect();
      mo.disconnect();
    };
  });

  function nodeAt(x, y, exclude = null) {
    let el = document.elementFromPoint(x, y);
    while (el && surface?.contains(el)) {
      const path = el.getAttribute?.("data-bos-node");
      if (path && path !== "page" && !path.startsWith("action:") && !(exclude && isWithin(path, exclude))) {
        return { path, kind: el.getAttribute("data-bos-kind") ?? "", el };
      }
      el = el.parentElement;
    }
    return null;
  }

  function placementFor(hit, x, y) {
    const r = hit.el.getBoundingClientRect();
    const parentRect = hit.el.parentElement?.getBoundingClientRect();
    const axis = parentRect && r.width < parentRect.width * 0.75 ? "x" : "y";
    const rel = axis === "x" ? (x - r.left) / r.width : (y - r.top) / r.height;
    if (CONTAINER_KINDS.has(hit.kind) && rel > 0.25 && rel < 0.75) return { placement: "inside", axis };
    return { placement: rel < 0.5 ? "before" : "after", axis };
  }

  function autoScroll(y) {
    if (!scroller) return;
    const r = scroller.getBoundingClientRect();
    if (y < r.top + EDGE) scroller.scrollTop -= 14;
    else if (y > r.bottom - EDGE) scroller.scrollTop += 14;
  }

  function updateDrag(x, y) {
    if (!drag) return;
    autoScroll(y);
    const hit = nodeAt(x, y, drag.path ?? null);
    if (!hit) {
      drag = { ...drag, x, y, target: null };
      return;
    }
    const { placement, axis } = placementFor(hit, x, y);
    drag = { ...drag, x, y, target: { path: hit.path, kind: hit.kind }, placement, axis };
  }

  async function finishDrag() {
    const d = drag;
    drag = null;
    if (!d) return;
    if (d.item && !d.target) {
      // Empty canvas fallback: drop item onto root page layout
      await model.insertAt(d.item, null, "inside");
      return;
    }
    if (!d.target) return;
    if (d.item) await model.insertAt(d.item, { path: d.target.path }, d.placement);
    else if (d.path && d.target.path !== d.path) await model.moveTo(d.path, { path: d.target.path }, d.placement);
  }

  // ----- pointer handling on the canvas (capture: the page underneath never reacts) -----
  function onPointerDown(e) {
    if (e.button !== 0) return;
    // Check if clicked the canvas + Action button trigger
    const actionTrigger = e.target.closest?.("[data-bos-action-trigger], .bos-action-designer-add");
    if (actionTrigger) {
      e.preventDefault();
      e.stopPropagation();
      window.dispatchEvent(new CustomEvent("bos:open-action-builder"));
      return;
    }
    const action = e.target.closest?.("[data-bos-node^='action:']");
    if (action) {
      e.preventDefault();
      scroller?.focus({ preventScroll: true });
      model.selected = action.getAttribute("data-bos-node");
      return;
    }
    const hit = nodeAt(e.clientX, e.clientY);
    e.preventDefault();
    scroller?.focus({ preventScroll: true });
    if (!hit) {
      // Check if clicked in header regions or page background
      const titleEl = e.target.closest?.(".bos-page-title, [data-bos-node='header:title']");
      if (titleEl) {
        model.selected = "header:title";
        return;
      }
      const hintEl = e.target.closest?.(".bos-hint, [data-bos-node='header:instruction']");
      if (hintEl) {
        model.selected = "header:instruction";
        return;
      }
      const headerEl = e.target.closest?.(".bos-page-header, [data-bos-node='header']");
      if (headerEl) {
        model.selected = "header";
        return;
      }
      model.selected = "page";
      return;
    }
    model.selected = hit.path;
    pending = { path: hit.path, x: e.clientX, y: e.clientY, id: e.pointerId };
  }

  function onPointerMove(e) {
    if (pending && !drag) {
      if (Math.hypot(e.clientX - pending.x, e.clientY - pending.y) > DRAG_THRESHOLD) {
        drag = { path: pending.path, x: e.clientX, y: e.clientY, target: null, placement: "after", axis: "y" };
        scroller?.setPointerCapture?.(pending.id);
      }
      return;
    }
    if (drag) {
      updateDrag(e.clientX, e.clientY);
      return;
    }
    hoverPath = nodeAt(e.clientX, e.clientY)?.path ?? null;
  }

  function onPointerUp() {
    pending = null;
    if (drag && !drag.item) finishDrag();
  }

  function swallow(e) {
    e.preventDefault();
    e.stopPropagation();
  }

  // ----- toolbox drags (start in the toolbox, follow the pointer anywhere) -----
  $effect(() => {
    const item = model.dragItem;
    if (!item) return;
    untrack(() => (drag = { item, x: 0, y: 0, target: null, placement: "after", axis: "y" }));
    const move = (e) => updateDrag(e.clientX, e.clientY);
    const up = async () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      await finishDrag();
      model.dragItem = null;
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  });

  function onKeyDown(e) {
    const path = model.selected;
    if (!path || path === "page") return;
    if (e.altKey && (e.key === "ArrowUp" || e.key === "ArrowLeft")) {
      e.preventDefault();
      model.nudge(path, -1);
    } else if (e.altKey && (e.key === "ArrowDown" || e.key === "ArrowRight")) {
      e.preventDefault();
      model.nudge(path, 1);
    } else if (e.key === "Delete") {
      e.preventDefault();
      model.remove(path);
    } else if (e.key === "Escape") {
      model.selected = null;
    }
  }

  const width = $derived(WIDTHS[model.viewport] ?? null);
</script>

<!-- role=application: the canvas owns its keyboard and pointer handling -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="relative h-full overflow-auto bg-slate-100 outline-none dark:bg-slate-950"
  bind:this={scroller}
  tabindex="0"
  role="application"
  aria-label="Page design canvas"
  onkeydown={isPreview ? undefined : onKeyDown}
  onpointermove={isPreview ? undefined : onPointerMove}
  onpointerup={isPreview ? undefined : onPointerUp}
  onpointercancel={isPreview ? undefined : () => { pending = null; drag = null; }}
  onpointerleave={isPreview ? undefined : () => (hoverPath = null)}
>
  <div class="mx-auto my-4 min-h-[calc(100%-2rem)] px-2 transition-[max-width]" style:max-width={width ? `${width}px` : "100%"}>
    <div
      class={cn(
        "relative transition-all duration-200",
        isPreview && model.viewport === "phone"
          ? "rounded-[2.5rem] border-[10px] border-slate-800 bg-white p-6 shadow-2xl ring-1 ring-slate-900/10 dark:border-slate-700 dark:bg-slate-900"
          : isPreview && model.viewport === "tablet"
          ? "rounded-[2rem] border-[12px] border-slate-800 bg-white p-6 shadow-2xl ring-1 ring-slate-900/10 dark:border-slate-700 dark:bg-slate-900"
          : "rounded-xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900 sm:p-6"
      )}
      bind:this={surface}
      onpointerdowncapture={isPreview ? undefined : onPointerDown}
      onclickcapture={isPreview ? undefined : swallow}
      ondblclickcapture={isPreview ? undefined : onDoubleClick}
      onsubmitcapture={isPreview ? undefined : swallow}
      onkeydowncapture={isPreview ? undefined : (e) => {
        if (e.key === "Enter" || e.key === " ") swallow(e);
      }}
      role="presentation"
    >
      {#if model.Component}
        {#key model.previewVersion}
          {@const Page = model.Component}
          <Page {ctx} />
        {/key}
      {:else if model.busy}
        <div class="flex justify-center py-20"><Spinner label="Compiling page" /></div>
      {:else}
        <p class="py-20 text-center text-xs text-slate-400">{model.apiPage ? "API pages have no visual layout." : "The page preview appears here once the page compiles."}</p>
      {/if}
    </div>
  </div>

  {#if !isPreview}
    <!-- overlay -->
    <div class="pointer-events-none absolute inset-0" aria-hidden="true">
      {#if hoverRect && hoverPath !== model.selected}
        <div class="absolute rounded border border-dashed border-indigo-400" style:left="{hoverRect.left}px" style:top="{hoverRect.top}px" style:width="{hoverRect.width}px" style:height="{hoverRect.height}px"></div>
      {/if}
      {#if selectedRect}
        <div class="absolute rounded border-2 border-indigo-600 shadow-xs" style:left="{selectedRect.left - 1}px" style:top="{selectedRect.top - 1}px" style:width="{selectedRect.width + 2}px" style:height="{selectedRect.height + 2}px">
          <!-- Element Tag Pill on Top-Left -->
          <span class="absolute -top-6 left-0 rounded-t bg-indigo-600 px-2 py-0.5 font-mono text-[10px] font-semibold text-white whitespace-nowrap shadow-xs">
            {#if model.selected === "header:title"}
              h1.bos-page-title
            {:else if model.selected === "header:instruction"}
              p.bos-hint
            {:else if model.selected === "header"}
              header.bos-page-header
            {:else if model.selected === "page"}
              page.{model.compiled?.page_type ?? "form"}
            {:else}
              {model.selected?.split("/").at(-1)}
            {/if}
          </span>

          <!-- Floating Action Toolbar on Top-Right -->
          <div class="pointer-events-auto absolute -top-8 right-0 flex items-center gap-0.5 rounded-lg border border-indigo-200 dark:border-indigo-800 bg-white dark:bg-slate-900 p-0.5 shadow-md z-20">
            {#if model.selected && model.selected !== "page"}
              {@const pPath = parentOf(model.selected)}
              {#if pPath && pPath !== "layout"}
                <button
                  type="button"
                  class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 hover:text-indigo-600 transition-colors cursor-pointer"
                  title="Select Parent Container"
                  onclick={(e) => {
                    e.stopPropagation();
                    model.selected = pPath;
                  }}
                  aria-label="Select Parent"
                >
                  <ArrowUpCircle class="size-3.5" />
                </button>
                <span class="w-px h-3 bg-slate-200 dark:bg-slate-700 my-auto"></span>
              {/if}
            {/if}
            <button
              type="button"
              class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 hover:text-indigo-600 transition-colors cursor-pointer"
              title="Move Up (Alt+Up)"
              onclick={(e) => {
                e.stopPropagation();
                model.nudge(model.selected, -1);
              }}
              aria-label="Move Up"
            >
              <ArrowUp class="size-3.5" />
            </button>
            <button
              type="button"
              class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 hover:text-indigo-600 transition-colors cursor-pointer"
              title="Move Down (Alt+Down)"
              onclick={(e) => {
                e.stopPropagation();
                model.nudge(model.selected, 1);
              }}
              aria-label="Move Down"
            >
              <ArrowDown class="size-3.5" />
            </button>
            <span class="w-px h-3 bg-slate-200 dark:bg-slate-700 my-auto"></span>
            <button
              type="button"
              class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 hover:text-indigo-600 transition-colors cursor-pointer"
              title="Edit text in place (or double-click element)"
              onclick={(e) => {
                e.stopPropagation();
                startInlineEdit(model.selected);
              }}
              aria-label="Edit text"
            >
              <Edit3 class="size-3.5" />
            </button>
            <button
              type="button"
              class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 hover:text-indigo-600 transition-colors cursor-pointer"
              title="Duplicate element"
              onclick={(e) => {
                e.stopPropagation();
                model.duplicate(model.selected);
              }}
              aria-label="Duplicate element"
            >
              <Copy class="size-3.5" />
            </button>
            <span class="w-px h-3 bg-slate-200 dark:bg-slate-700 my-auto"></span>
            <button
              type="button"
              class="p-1 rounded hover:bg-rose-50 dark:hover:bg-rose-950 text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 transition-colors cursor-pointer"
              title="Remove from layout (Delete)"
              onclick={(e) => {
                e.stopPropagation();
                model.remove(model.selected);
              }}
              aria-label="Remove element"
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>
        </div>
      {/if}

      <!-- Inline Text Edit Overlay -->
      {#if inlineEdit}
        <div
          class="pointer-events-auto absolute z-30 flex items-center rounded-md border-2 border-indigo-600 bg-white p-0.5 shadow-xl ring-2 ring-indigo-400/40 dark:bg-slate-900"
          style:left="{inlineEdit.left}px"
          style:top="{inlineEdit.top}px"
          style:min-width="{inlineEdit.width}px"
          style:min-height="{inlineEdit.height}px"
        >
          <input
            class="size-full rounded px-2 py-1 text-sm font-semibold bg-transparent text-slate-900 dark:text-slate-100 outline-none"
            value={inlineEdit.text}
            use:focusInput
            onkeydown={(e) => {
              if (e.key === "Enter") {
                commitInlineEdit(e.currentTarget.value);
              } else if (e.key === "Escape") {
                inlineEdit = null;
              }
            }}
            onblur={(e) => commitInlineEdit(e.currentTarget.value)}
          />
        </div>
      {/if}
      {#if dropRect}
        {#if dropRect.inside}
          <div class="absolute rounded bg-indigo-500/10 ring-2 ring-indigo-500" style:left="{dropRect.left}px" style:top="{dropRect.top}px" style:width="{dropRect.width}px" style:height="{dropRect.height}px"></div>
        {:else}
          <div class="absolute rounded bg-indigo-600" style:left="{dropRect.left}px" style:top="{dropRect.top}px" style:width="{dropRect.width}px" style:height="{dropRect.height}px"></div>
        {/if}
      {/if}
    </div>
    {#if drag}
      <div class="pointer-events-none fixed z-50 rounded-md bg-indigo-600 px-2 py-1 text-xs font-medium text-white shadow-lg" style:left="{drag.x + 12}px" style:top="{drag.y + 12}px">
        {drag.item ? drag.item.label : drag.path?.split("/").at(-1)}
      </div>
    {/if}
  {/if}
</div>
