<script>
  /**
   * BoxModelEditor.svelte
   * Visual Spacing Box Model Widget inspired by Builder.io & Elementor.
   * Renders concentric boxes for Margin (outer) and Padding (inner)
   * with directional 4-way inputs (Top, Right, Bottom, Left), link/unlink toggles,
   * and dedicated quick presets for both Margin and Padding.
   */
  import { Link, Unlink, RotateCcw } from "lucide-svelte";

  /**
   * @type {{
   *   margin?: string,
   *   padding?: string,
   *   density?: string,
   *   onSetMargin: (val: string | null) => void,
   *   onSetPadding: (val: string | null) => void
   * }}
   */
  let { margin = "", padding = "", density = "", onSetMargin, onSetPadding } = $props();

  let linkMargin = $state(true);
  let linkPadding = $state(true);

  // Parse 1 to 4 space-separated values into { top, right, bottom, left }
  function parseSides(val) {
    if (val === null || val === undefined || val === "") {
      return { top: "", right: "", bottom: "", left: "" };
    }
    const parts = String(val).trim().split(/\s+/);
    if (parts.length === 1) {
      return { top: parts[0], right: parts[0], bottom: parts[0], left: parts[0] };
    }
    if (parts.length === 2) {
      return { top: parts[0], right: parts[1], bottom: parts[0], left: parts[1] };
    }
    if (parts.length === 3) {
      return { top: parts[0], right: parts[1], bottom: parts[2], left: parts[1] };
    }
    return { top: parts[0] || "0", right: parts[1] || "0", bottom: parts[2] || "0", left: parts[3] || "0" };
  }

  function formatSides(sides) {
    const { top, right, bottom, left } = sides;
    const clean = (v) => {
      const s = String(v ?? "").trim();
      if (!s) return "0";
      return s.match(/^\d+$/) ? `${s}px` : s;
    };
    const cT = clean(top);
    const cR = clean(right);
    const cB = clean(bottom);
    const cL = clean(left);

    if (cT === cR && cR === cB && cB === cL) {
      return cT;
    }
    if (cT === cB && cR === cL) {
      return `${cT} ${cR}`;
    }
    return `${cT} ${cR} ${cB} ${cL}`;
  }

  const effectivePadding = $derived(
    padding || (density === "compact" ? "8px" : density === "comfortable" ? "14px" : density === "spacious" ? "20px" : "")
  );

  const mSides = $derived(parseSides(margin));
  const pSides = $derived(parseSides(effectivePadding));

  function updateMarginSide(side, rawValue) {
    const current = { ...mSides };
    if (linkMargin) {
      current.top = rawValue;
      current.right = rawValue;
      current.bottom = rawValue;
      current.left = rawValue;
    } else {
      current[side] = rawValue;
    }
    onSetMargin(formatSides(current));
  }

  function updatePaddingSide(side, rawValue) {
    const current = { ...pSides };
    if (linkPadding) {
      current.top = rawValue;
      current.right = rawValue;
      current.bottom = rawValue;
      current.left = rawValue;
    } else {
      current[side] = rawValue;
    }
    onSetPadding(formatSides(current));
  }
</script>

<div class="flex flex-col gap-2.5 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50 text-xs">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <span class="font-semibold text-slate-700 dark:text-slate-200">Box Spacing</span>
    {#if margin || padding}
      <button
        type="button"
        class="flex items-center gap-1 text-[11px] font-medium text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
        title="Reset all box spacing"
        onclick={() => {
          onSetMargin(null);
          onSetPadding(null);
        }}
      >
        <RotateCcw class="size-3" />
        <span>Reset</span>
      </button>
    {/if}
  </div>

  <!-- Concentric Box Diagram -->
  <div class="relative rounded-lg border border-dashed border-amber-400/80 bg-amber-50/50 px-12 py-7 dark:border-amber-600/60 dark:bg-amber-950/20 text-center select-none">
    <!-- Margin Header & Link Toggle -->
    <div class="absolute top-1 left-2 flex items-center gap-1.5">
      <span class="text-[9px] font-bold uppercase tracking-wider text-amber-700 dark:text-amber-400">
        Margin
      </span>
      <button
        type="button"
        class={"p-0.5 rounded transition-colors " + (linkMargin ? "text-amber-700 dark:text-amber-300" : "text-slate-400 hover:text-slate-600")}
        title={linkMargin ? "Unlink Margin sides" : "Link Margin sides uniformly"}
        onclick={() => (linkMargin = !linkMargin)}
      >
        {#if linkMargin}<Link class="size-3" />{:else}<Unlink class="size-3" />{/if}
      </button>
    </div>

    <!-- Margin Top -->
    <div class="absolute top-1 left-1/2 -translate-x-1/2">
      <input
        class="h-5 w-11 rounded border border-amber-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-amber-500 focus:outline-none dark:border-amber-700 dark:bg-slate-900 dark:text-slate-100"
        value={mSides.top}
        placeholder="0"
        onchange={(e) => updateMarginSide("top", e.currentTarget.value)}
        onkeydown={(e) => { if (e.key === "Enter") updateMarginSide("top", e.currentTarget.value); }}
        title="Margin Top"
        aria-label="Margin Top"
      />
    </div>

    <!-- Margin Right -->
    <div class="absolute top-1/2 right-1 -translate-y-1/2">
      <input
        class="h-5 w-10 rounded border border-amber-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-amber-500 focus:outline-none dark:border-amber-700 dark:bg-slate-900 dark:text-slate-100"
        value={mSides.right}
        placeholder="0"
        onchange={(e) => updateMarginSide("right", e.currentTarget.value)}
        onkeydown={(e) => { if (e.key === "Enter") updateMarginSide("right", e.currentTarget.value); }}
        title="Margin Right"
        aria-label="Margin Right"
      />
    </div>

    <!-- Margin Bottom -->
    <div class="absolute bottom-1 left-1/2 -translate-x-1/2">
      <input
        class="h-5 w-11 rounded border border-amber-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-amber-500 focus:outline-none dark:border-amber-700 dark:bg-slate-900 dark:text-slate-100"
        value={mSides.bottom}
        placeholder="0"
        onchange={(e) => updateMarginSide("bottom", e.currentTarget.value)}
        onkeydown={(e) => { if (e.key === "Enter") updateMarginSide("bottom", e.currentTarget.value); }}
        title="Margin Bottom"
        aria-label="Margin Bottom"
      />
    </div>

    <!-- Margin Left -->
    <div class="absolute top-1/2 left-1 -translate-y-1/2">
      <input
        class="h-5 w-10 rounded border border-amber-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-amber-500 focus:outline-none dark:border-amber-700 dark:bg-slate-900 dark:text-slate-100"
        value={mSides.left}
        placeholder="0"
        onchange={(e) => updateMarginSide("left", e.currentTarget.value)}
        onkeydown={(e) => { if (e.key === "Enter") updateMarginSide("left", e.currentTarget.value); }}
        title="Margin Left"
        aria-label="Margin Left"
      />
    </div>

    <!-- Inner Padding Box -->
    <div class="relative rounded-md border border-dashed border-emerald-400/80 bg-emerald-50/60 px-10 py-6 dark:border-emerald-600/60 dark:bg-emerald-950/30">
      <!-- Padding Header & Link Toggle -->
      <div class="absolute top-1 left-2 flex items-center gap-1.5">
        <span class="text-[9px] font-bold uppercase tracking-wider text-emerald-700 dark:text-emerald-400">
          Padding
        </span>
        <button
          type="button"
          class={"p-0.5 rounded transition-colors " + (linkPadding ? "text-emerald-700 dark:text-emerald-300" : "text-slate-400 hover:text-slate-600")}
          title={linkPadding ? "Unlink Padding sides" : "Link Padding sides uniformly"}
          onclick={() => (linkPadding = !linkPadding)}
        >
          {#if linkPadding}<Link class="size-3" />{:else}<Unlink class="size-3" />{/if}
        </button>
      </div>

      <!-- Padding Top -->
      <div class="absolute top-1 left-1/2 -translate-x-1/2">
        <input
          class="h-5 w-11 rounded border border-emerald-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-emerald-500 focus:outline-none dark:border-emerald-700 dark:bg-slate-900 dark:text-slate-100"
          value={pSides.top}
          placeholder="0"
          onchange={(e) => updatePaddingSide("top", e.currentTarget.value)}
          onkeydown={(e) => { if (e.key === "Enter") updatePaddingSide("top", e.currentTarget.value); }}
          title="Padding Top"
          aria-label="Padding Top"
        />
      </div>

      <!-- Padding Right -->
      <div class="absolute top-1/2 right-1 -translate-y-1/2">
        <input
          class="h-5 w-9 rounded border border-emerald-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-emerald-500 focus:outline-none dark:border-emerald-700 dark:bg-slate-900 dark:text-slate-100"
          value={pSides.right}
          placeholder="0"
          onchange={(e) => updatePaddingSide("right", e.currentTarget.value)}
          onkeydown={(e) => { if (e.key === "Enter") updatePaddingSide("right", e.currentTarget.value); }}
          title="Padding Right"
          aria-label="Padding Right"
        />
      </div>

      <!-- Padding Bottom -->
      <div class="absolute bottom-1 left-1/2 -translate-x-1/2">
        <input
          class="h-5 w-11 rounded border border-emerald-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-emerald-500 focus:outline-none dark:border-emerald-700 dark:bg-slate-900 dark:text-slate-100"
          value={pSides.bottom}
          placeholder="0"
          onchange={(e) => updatePaddingSide("bottom", e.currentTarget.value)}
          onkeydown={(e) => { if (e.key === "Enter") updatePaddingSide("bottom", e.currentTarget.value); }}
          title="Padding Bottom"
          aria-label="Padding Bottom"
        />
      </div>

      <!-- Padding Left -->
      <div class="absolute top-1/2 left-1 -translate-y-1/2">
        <input
          class="h-5 w-9 rounded border border-emerald-300 bg-white px-1 text-center font-mono text-[10px] text-slate-800 shadow-xs focus:ring-1 focus:ring-emerald-500 focus:outline-none dark:border-emerald-700 dark:bg-slate-900 dark:text-slate-100"
          value={pSides.left}
          placeholder="0"
          onchange={(e) => updatePaddingSide("left", e.currentTarget.value)}
          onkeydown={(e) => { if (e.key === "Enter") updatePaddingSide("left", e.currentTarget.value); }}
          title="Padding Left"
          aria-label="Padding Left"
        />
      </div>

      <!-- Center Element Indicator -->
      <div class="rounded border border-slate-200/80 bg-white/90 py-1.5 px-2 font-mono text-[10px] font-semibold text-slate-600 shadow-xs dark:border-slate-700 dark:bg-slate-900/90 dark:text-slate-300">
        Element
      </div>
    </div>
  </div>

  <!-- Dedicated Quick Presets for Margin & Padding -->
  <div class="flex flex-col gap-1.5 pt-1 border-t border-slate-200 dark:border-slate-800">
    <!-- Margin Presets Row -->
    <div class="flex items-center justify-between text-[11px]">
      <div class="flex items-center gap-1 font-medium text-amber-700 dark:text-amber-400">
        <span class="size-1.5 rounded-full bg-amber-500"></span>
        <span>Margin:</span>
      </div>
      <div class="flex items-center gap-1">
        {#each [["0", "0"], ["4px", "4"], ["8px", "8"], ["16px", "16"], ["24px", "24"]] as [val, lbl] (val)}
          <button
            type="button"
            class={"rounded border px-1.5 py-0.5 text-[10px] font-mono transition-colors " +
              (margin === val || (val === "0" && margin === "0px")
                ? "border-amber-400 bg-amber-100 text-amber-900 font-bold dark:border-amber-600 dark:bg-amber-900/40 dark:text-amber-200"
                : "border-slate-200 bg-white hover:bg-amber-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300")}
            onclick={() => onSetMargin(val)}
          >
            {lbl}
          </button>
        {/each}
        {#if margin}
          <button
            type="button"
            class="p-0.5 text-slate-400 hover:text-amber-600 dark:hover:text-amber-400"
            title="Clear Margin"
            onclick={() => onSetMargin(null)}
          >
            <RotateCcw class="size-2.5" />
          </button>
        {/if}
      </div>
    </div>

    <!-- Padding Presets Row -->
    <div class="flex items-center justify-between text-[11px]">
      <div class="flex items-center gap-1 font-medium text-emerald-700 dark:text-emerald-400">
        <span class="size-1.5 rounded-full bg-emerald-500"></span>
        <span>Padding:</span>
      </div>
      <div class="flex items-center gap-1">
        {#each [["0", "0"], ["4px", "4"], ["8px", "8"], ["16px", "16"], ["24px", "24"]] as [val, lbl] (val)}
          <button
            type="button"
            class={"rounded border px-1.5 py-0.5 text-[10px] font-mono transition-colors " +
              (effectivePadding === val || (val === "0" && effectivePadding === "0px")
                ? "border-emerald-400 bg-emerald-100 text-emerald-900 font-bold dark:border-emerald-600 dark:bg-emerald-900/40 dark:text-emerald-200"
                : "border-slate-200 bg-white hover:bg-emerald-50 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300")}
            onclick={() => onSetPadding(val)}
          >
            {lbl}
          </button>
        {/each}
        {#if padding}
          <button
            type="button"
            class="p-0.5 text-slate-400 hover:text-emerald-600 dark:hover:text-emerald-400"
            title="Clear Padding"
            onclick={() => onSetPadding(null)}
          >
            <RotateCcw class="size-2.5" />
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>
