<script>
  /**
   * WYSIWYG canvas: the real compiled page (design build, real records) with an overlay for hover,
   * selection and drop indicators. Controls carry `data-bos-node` paths; dragging uses pointer events
   * (mouse, pen and touch), with auto-scroll near the edges and Alt+Arrow keys as the keyboard path.
   */
  import { untrack } from "svelte";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { CONTAINER_KINDS, isWithin } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  const WIDTHS = { phone: 375, tablet: 768, desktop: 1280, full: null };
  const DRAG_THRESHOLD = 5;
  const EDGE = 48;

  let scroller = $state(null);
  let surface = $state(null);
  let tick = $state(0);
  let hoverPath = $state(null);
  let drag = $state(null); // { path?, item?, x, y, target, placement }
  let pending = null;

  const ctx = $derived({ page: model.targetPage || model.name, design: true, mode: "view", openPage: () => {}, navigate: () => {} });

  function nodeEl(path) {
    if (!surface || !path) return null;
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
    if (!d?.target) return;
    if (d.item) await model.insertAt(d.item, { path: d.target.path }, d.placement);
    else if (d.path && d.target.path !== d.path) await model.moveTo(d.path, { path: d.target.path }, d.placement);
  }

  // ----- pointer handling on the canvas (capture: the page underneath never reacts) -----
  function onPointerDown(e) {
    if (e.button !== 0) return;
    const hit = nodeAt(e.clientX, e.clientY);
    const action = e.target.closest?.("[data-bos-node^='action:']");
    e.preventDefault();
    scroller?.focus({ preventScroll: true });
    if (action) {
      model.selected = action.getAttribute("data-bos-node");
      return;
    }
    if (!hit) {
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
  onkeydown={onKeyDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={() => { pending = null; drag = null; }}
  onpointerleave={() => (hoverPath = null)}
>
  <div class="mx-auto my-4 min-h-[calc(100%-2rem)] px-2 transition-[max-width]" style:max-width={width ? `${width}px` : "100%"}>
    <div
      class="relative rounded-xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-800 dark:bg-slate-900 sm:p-6"
      bind:this={surface}
      onpointerdowncapture={onPointerDown}
      onclickcapture={swallow}
      ondblclickcapture={swallow}
      onsubmitcapture={swallow}
      onkeydowncapture={(e) => {
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
        <p class="py-20 text-center text-xs text-slate-400">The page preview appears here once the page compiles.</p>
      {/if}
    </div>
  </div>

  <!-- overlay -->
  <div class="pointer-events-none absolute inset-0" aria-hidden="true">
    {#if hoverRect && hoverPath !== model.selected}
      <div class="absolute rounded border border-dashed border-indigo-400" style:left="{hoverRect.left}px" style:top="{hoverRect.top}px" style:width="{hoverRect.width}px" style:height="{hoverRect.height}px"></div>
    {/if}
    {#if selectedRect}
      <div class="absolute rounded border-2 border-indigo-600" style:left="{selectedRect.left - 1}px" style:top="{selectedRect.top - 1}px" style:width="{selectedRect.width + 2}px" style:height="{selectedRect.height + 2}px">
        <span class="absolute -top-5 left-0 rounded bg-indigo-600 px-1.5 py-0.5 font-mono text-[10px] text-white whitespace-nowrap">{model.selected?.split("/").at(-1)}</span>
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
</div>
