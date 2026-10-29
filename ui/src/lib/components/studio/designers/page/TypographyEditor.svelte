<script>
  /**
   * TypographyEditor.svelte
   * Builder.io & Elementor-inspired Typography Inspector (Svelte 5 runes).
   * Controls font size, weight, line height, letter spacing, text color,
   * alignment (left/center/right/justify), text decoration (B, I, U, S), and transform.
   */
  import {
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    RotateCcw
  } from "lucide-svelte";

  /**
   * @type {{
   *   style: Record<string, any>,
   *   schema: any,
   *   onSet: (name: string, value: any) => void
   * }}
   */
  let { style = {}, schema, onSet } = $props();

  const HEX_RE = /^#[0-9a-f]{6}$/i;

  const currentSize = $derived(style?.fontSize ?? "");
  const currentWeight = $derived(style?.fontWeight ?? "");
  const currentLineHeight = $derived(style?.lineHeight ?? "");
  const currentLetterSpacing = $derived(style?.letterSpacing ?? "");
  const currentAlign = $derived(style?.textAlign ?? "left");
  const currentTransform = $derived(style?.textTransform ?? "none");
  const currentDecoration = $derived(style?.textDecoration ?? "none");
  const currentColor = $derived(style?.textColor ?? "");

  const isBold = $derived(currentWeight === "bold" || currentWeight === "700" || currentWeight === "800");
  const isItalic = $derived(style?.emphasis === "Attention" || style?.emphasis === "Unfavorable");
  const isUnderline = $derived(currentDecoration === "underline");
  const isStrike = $derived(currentDecoration === "line-through");

  const SIZE_PRESETS = [
    { value: "", label: "Default" },
    { value: "12px", label: "12px (XS)" },
    { value: "14px", label: "14px (SM)" },
    { value: "16px", label: "16px (Base)" },
    { value: "18px", label: "18px (LG)" },
    { value: "24px", label: "24px (XL)" },
    { value: "32px", label: "32px (2XL)" },
    { value: "48px", label: "48px (3XL)" },
    { value: "60px", label: "60px (Hero)" }
  ];

  const WEIGHT_OPTIONS = [
    { value: "", label: "Inherit" },
    { value: "400", label: "Regular (400)" },
    { value: "500", label: "Medium (500)" },
    { value: "600", label: "Semibold (600)" },
    { value: "700", label: "Bold (700)" },
    { value: "800", label: "Extrabold (800)" }
  ];

  function toggleBold() {
    onSet("fontWeight", isBold ? null : "700");
  }

  function toggleUnderline() {
    onSet("textDecoration", isUnderline ? null : "underline");
  }

  function toggleStrike() {
    onSet("textDecoration", isStrike ? null : "line-through");
  }

  function toggleItalic() {
    onSet("emphasis", isItalic ? null : "Attention");
  }
</script>

<div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50 text-xs">
  <div class="flex items-center justify-between">
    <span class="font-semibold text-slate-700 dark:text-slate-200">Typography</span>
    {#if currentSize || currentWeight || currentLineHeight || currentLetterSpacing || currentAlign !== "left" || currentTransform !== "none"}
      <button
        type="button"
        class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
        title="Reset typography to defaults"
        onclick={() => {
          onSet("fontSize", null);
          onSet("fontWeight", null);
          onSet("lineHeight", null);
          onSet("letterSpacing", null);
          onSet("textAlign", null);
          onSet("textTransform", null);
          onSet("textDecoration", null);
        }}
      >
        <RotateCcw class="size-3" />
        <span>reset</span>
      </button>
    {/if}
  </div>

  <!-- Formatting & Alignment Toolbar -->
  <div class="flex flex-wrap items-center justify-between gap-1.5 border-b border-slate-200 pb-2 dark:border-slate-800">
    <!-- B / I / U / S -->
    <div class="inline-flex rounded-md border border-slate-200 bg-white p-0.5 dark:border-slate-700 dark:bg-slate-800">
      <button
        type="button"
        class={"p-1 rounded transition-colors " + (isBold ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
        title="Bold"
        onclick={toggleBold}
      >
        <Bold class="size-3.5" />
      </button>
      <button
        type="button"
        class={"p-1 rounded transition-colors " + (isItalic ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
        title="Italic"
        onclick={toggleItalic}
      >
        <Italic class="size-3.5" />
      </button>
      <button
        type="button"
        class={"p-1 rounded transition-colors " + (isUnderline ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
        title="Underline"
        onclick={toggleUnderline}
      >
        <Underline class="size-3.5" />
      </button>
      <button
        type="button"
        class={"p-1 rounded transition-colors " + (isStrike ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
        title="Strikethrough"
        onclick={toggleStrike}
      >
        <Strikethrough class="size-3.5" />
      </button>
    </div>

    <!-- Text Alignment -->
    <div class="inline-flex rounded-md border border-slate-200 bg-white p-0.5 dark:border-slate-700 dark:bg-slate-800">
      {#each [["left", AlignLeft, "Align Left"], ["center", AlignCenter, "Align Center"], ["right", AlignRight, "Align Right"], ["justify", AlignJustify, "Justify"]] as [alignKey, Icon, label] (alignKey)}
        <button
          type="button"
          class={"p-1 rounded transition-colors " + (currentAlign === alignKey ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
          title={label}
          onclick={() => onSet("textAlign", currentAlign === alignKey ? null : alignKey)}
        >
          <Icon class="size-3.5" />
        </button>
      {/each}
    </div>
  </div>

  <!-- Font Size & Weight Grid -->
  <div class="grid grid-cols-2 gap-2">
    <div class="flex flex-col gap-1">
      <span class="text-[11px] font-medium text-slate-600 dark:text-slate-300">Font Size</span>
      <div class="flex items-center gap-1">
        <select
          class="w-full rounded border border-slate-200 bg-white px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
          value={currentSize}
          onchange={(e) => onSet("fontSize", e.currentTarget.value || null)}
        >
          {#each SIZE_PRESETS as p (p.value)}
            <option value={p.value}>{p.label}</option>
          {/each}
        </select>
      </div>
    </div>

    <div class="flex flex-col gap-1">
      <span class="text-[11px] font-medium text-slate-600 dark:text-slate-300">Font Weight</span>
      <select
        class="w-full rounded border border-slate-200 bg-white px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
        value={currentWeight}
        onchange={(e) => onSet("fontWeight", e.currentTarget.value || null)}
      >
        {#each WEIGHT_OPTIONS as w (w.value)}
          <option value={w.value}>{w.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Line Height & Letter Spacing -->
  <div class="grid grid-cols-2 gap-2">
    <div class="flex flex-col gap-1">
      <span class="text-[11px] font-medium text-slate-600 dark:text-slate-300">Line Height</span>
      <input
        class="rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
        placeholder="e.g. 1.5 or 24px"
        value={currentLineHeight}
        onchange={(e) => onSet("lineHeight", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
      />
    </div>

    <div class="flex flex-col gap-1">
      <span class="text-[11px] font-medium text-slate-600 dark:text-slate-300">Char Spacing</span>
      <input
        class="rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800 dark:text-slate-200"
        placeholder="e.g. -0.5px or 1px"
        value={currentLetterSpacing}
        onchange={(e) => onSet("letterSpacing", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
      />
    </div>
  </div>

  <!-- Text Transform -->
  <div class="flex items-center justify-between gap-2">
    <span class="text-[11px] font-medium text-slate-600 dark:text-slate-300">Text Transform</span>
    <div class="inline-flex rounded-md border border-slate-200 bg-white p-0.5 dark:border-slate-700 dark:bg-slate-800">
      {#each [["none", "None"], ["uppercase", "UP"], ["lowercase", "low"], ["capitalize", "Cap"]] as [tKey, label] (tKey)}
        <button
          type="button"
          class={"px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors " +
            (currentTransform === tKey ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-950 dark:text-indigo-300" : "text-slate-600 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700")}
          onclick={() => onSet("textTransform", tKey === "none" ? null : tKey)}
        >
          {label}
        </button>
      {/each}
    </div>
  </div>
</div>
