<script>
  /**
   * StyleEditor.svelte
   * Builder.io & Elementor-grade Visual Style Inspector (Svelte 5 runes).
   * - Active Overrides Chip Bar with 1-click removal
   * - Concentric Spacing Box Model (Margin & Padding 4-way inputs + Presets)
   * - Dimensions & Sizing (Width, Max Width, Min Width, Height, Min/Max Height, Overflow)
   * - Full Typography Suite (Size, Weight, Line Height, Char Spacing, Alignment, Formats)
   * - Colors & Themes (Palette Swatches + Live Hex & Alpha Picker)
   * - Border & Corners (Solid, Dashed, Dotted, Width, Radius, Shadow, Gap)
   * - Layout & Flexbox (Display, Direction, Justify, Align, Wrap)
   * - Visual Effects & Icon (Opacity slider, Lucide Icon Picker)
   */
  import { X, Sliders, Palette, Type, Box, Move, Sparkles, Maximize2, LayoutGrid, RotateCcw } from "lucide-svelte";
  import IconPicker from "./IconPicker.svelte";
  import BoxModelEditor from "./BoxModelEditor.svelte";
  import TypographyEditor from "./TypographyEditor.svelte";
  import Icon from "$lib/page-kit/Icon.svelte";

  /**
   * @type {{
   *   style: Record<string, any>,
   *   schema: any,
   *   inherited?: Record<string, any>,
   *   onSet: (name: string, value: any) => void
   * }}
   */
  let {
    style = {},
    schema,
    inherited = {},
    activeIcon = null,
    onSet,
    onSetIcon = null
  } = $props();

  const effectiveIcon = $derived(value("icon") || activeIcon || inherited?.icon || null);

  function handleIconChange(newIcon) {
    if (onSetIcon) {
      onSetIcon(newIcon);
    } else {
      onSet("icon", newIcon);
    }
  }

  const HEX = /^#[0-9a-f]{6}$/i;
  const COLOR_PROPS = [
    { name: "background", label: "Background" },
    { name: "textColor", label: "Text Color" },
    { name: "borderColor", label: "Border Color" }
  ];

  const tokens = $derived(schema?.color_tokens ?? []);
  let hexDraft = $state({});
  let activeSection = $state("all"); // 'all' | 'spacing' | 'sizing' | 'typography' | 'colors' | 'borders' | 'layout' | 'effects'

  function swatch(token) {
    return `var(--bos-color-${token})`;
  }

  function commitHex(name) {
    const v = String(hexDraft[name] ?? "").trim();
    if (!v) return;
    if (HEX.test(v)) onSet(name, v.toLowerCase());
    hexDraft = { ...hexDraft, [name]: undefined };
  }

  const value = (name) => style?.[name] ?? null;

  // Active overrides list (ignoring empty or default values)
  const activeOverrides = $derived.by(() => {
    return Object.entries(style || {})
      .filter(([_, v]) => v !== null && v !== undefined && v !== "")
      .map(([k, v]) => ({ key: k, value: String(v) }));
  });
</script>

<div class="flex flex-col gap-3 text-xs">
  <!-- Active Overrides Bar (Elementor/Builder.io Style) -->
  {#if activeOverrides.length > 0}
    <div class="flex flex-col gap-1.5 rounded-lg border border-indigo-200/80 bg-indigo-50/50 p-2 dark:border-indigo-900/50 dark:bg-indigo-950/20">
      <div class="flex items-center justify-between text-[11px]">
        <span class="font-bold text-indigo-700 dark:text-indigo-300">
          Applied Styles ({activeOverrides.length})
        </span>
        <button
          type="button"
          class="flex items-center gap-1 text-[10px] text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 font-medium"
          onclick={() => {
            for (const { key } of activeOverrides) onSet(key, null);
          }}
          title="Clear all overrides on this element"
        >
          <RotateCcw class="size-3" />
          <span>Clear All</span>
        </button>
      </div>
      <div class="flex flex-wrap gap-1">
        {#each activeOverrides as { key, value } (key)}
          <span class="inline-flex items-center gap-1 rounded bg-white px-1.5 py-0.5 text-[10px] font-mono shadow-2xs border border-indigo-100 dark:border-indigo-800 dark:bg-slate-900 text-slate-700 dark:text-slate-300">
            <span class="font-semibold text-indigo-600 dark:text-indigo-400">{key}:</span>
            <span class="max-w-[7rem] truncate">{value}</span>
            <button
              type="button"
              class="text-slate-400 hover:text-rose-600 dark:hover:text-rose-400"
              title={`Remove ${key}`}
              onclick={() => onSet(key, null)}
            >
              <X class="size-2.5" />
            </button>
          </span>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Categorized Inspector Tabs -->
  <div class="flex items-center gap-1 border-b border-slate-200 pb-1.5 dark:border-slate-800 overflow-x-auto text-[11px]" role="tablist">
    {#each [
      ["all", "All"],
      ["spacing", "Spacing"],
      ["sizing", "Sizing"],
      ["typography", "Type"],
      ["colors", "Color"],
      ["borders", "Borders"],
      ["layout", "Layout"],
      ["effects", "Effects & Icon"]
    ] as [secId, label] (secId)}
      <button
        type="button"
        role="tab"
        aria-selected={activeSection === secId}
        class={"rounded px-2 py-1 font-semibold whitespace-nowrap transition-colors " +
          (activeSection === secId
            ? "bg-indigo-600 text-white shadow-2xs font-bold"
            : "text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-200")}
        onclick={() => (activeSection = secId)}
      >
        {label}
      </button>
    {/each}
  </div>

  <!-- 1. SPACING BOX MODEL (MARGIN & PADDING) -->
  {#if activeSection === "all" || activeSection === "spacing"}
    <BoxModelEditor
      margin={value("margin")}
      padding={value("padding")}
      density={value("density")}
      onSetMargin={(val) => onSet("margin", val)}
      onSetPadding={(val) => onSet("padding", val)}
    />
  {/if}

  <!-- 2. SIZING & DIMENSIONS -->
  {#if activeSection === "all" || activeSection === "sizing"}
    <div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
      <div class="flex items-center justify-between">
        <span class="font-semibold text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
          <Maximize2 class="size-3.5 text-indigo-500" />
          <span>Sizing & Dimensions</span>
        </span>
      </div>

      <!-- Width -->
      <div class="flex flex-col gap-1.5 border-b border-slate-200/60 pb-2 dark:border-slate-800/60">
        <div class="flex items-center justify-between">
          <span class="font-medium text-slate-600 dark:text-slate-300">Width</span>
          <div class="flex items-center gap-1">
            {#each [["auto", "Auto"], ["100%", "Full"], ["320px", "320"], ["480px", "480"]] as [presetVal, lbl] (presetVal)}
              <button
                type="button"
                class="rounded border border-slate-200 bg-white px-1.5 py-0.5 text-[10px] hover:bg-slate-100 dark:border-slate-700 dark:bg-slate-800 dark:hover:bg-slate-700"
                onclick={() => onSet("width", presetVal)}
              >
                {lbl}
              </button>
            {/each}
          </div>
        </div>
        <div class="flex items-center gap-2">
          <input
            class="flex-1 rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800 text-slate-800 dark:text-slate-100"
            placeholder="e.g. 100%, 400px, auto"
            value={value("width") ?? ""}
            onchange={(e) => onSet("width", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
          {#if value("width")}
            <button type="button" class="text-slate-400 hover:text-rose-500" onclick={() => onSet("width", null)}><X class="size-3" /></button>
          {/if}
        </div>
      </div>

      <!-- Max Width & Min Width -->
      <div class="grid grid-cols-2 gap-2 border-b border-slate-200/60 pb-2 dark:border-slate-800/60">
        <div class="flex flex-col gap-1">
          <span class="font-medium text-slate-600 dark:text-slate-300 text-[11px]">Max Width</span>
          <input
            class="rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
            placeholder="e.g. 40rem"
            value={value("maxWidth") ?? ""}
            onchange={(e) => onSet("maxWidth", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="font-medium text-slate-600 dark:text-slate-300 text-[11px]">Min Width</span>
          <input
            class="rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
            placeholder="e.g. 240px"
            value={value("minWidth") ?? ""}
            onchange={(e) => onSet("minWidth", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
        </div>
      </div>

      <!-- Height, Min Height, Max Height -->
      <div class="grid grid-cols-3 gap-2 border-b border-slate-200/60 pb-2 dark:border-slate-800/60">
        <div class="flex flex-col gap-1">
          <span class="font-medium text-slate-600 dark:text-slate-300 text-[11px]">Height</span>
          <input
            class="rounded border border-slate-200 bg-white px-1.5 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
            placeholder="auto"
            value={value("height") ?? ""}
            onchange={(e) => onSet("height", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="font-medium text-slate-600 dark:text-slate-300 text-[11px]">Min Height</span>
          <input
            class="rounded border border-slate-200 bg-white px-1.5 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
            placeholder="0"
            value={value("minHeight") ?? ""}
            onchange={(e) => onSet("minHeight", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
        </div>
        <div class="flex flex-col gap-1">
          <span class="font-medium text-slate-600 dark:text-slate-300 text-[11px]">Max Height</span>
          <input
            class="rounded border border-slate-200 bg-white px-1.5 py-1 font-mono text-xs dark:border-slate-700 dark:bg-slate-800"
            placeholder="none"
            value={value("maxHeight") ?? ""}
            onchange={(e) => onSet("maxHeight", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
          />
        </div>
      </div>

      <!-- Overflow -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Overflow</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("overflow") ?? ""}
          onchange={(e) => onSet("overflow", e.currentTarget.value || null)}
        >
          <option value="">Default (Visible)</option>
          <option value="hidden">Hidden (Clip)</option>
          <option value="auto">Auto (Scroll if needed)</option>
          <option value="scroll">Always Scroll</option>
        </select>
      </div>
    </div>
  {/if}

  <!-- 3. TYPOGRAPHY INSPECTOR -->
  {#if activeSection === "all" || activeSection === "typography"}
    <TypographyEditor {style} {schema} {onSet} />
  {/if}

  <!-- 4. COLORS & PALETTE -->
  {#if activeSection === "all" || activeSection === "colors"}
    <div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
      <span class="font-semibold text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
        <Palette class="size-3.5 text-indigo-500" />
        <span>Colors & Theme</span>
      </span>
      {#each COLOR_PROPS as cp (cp.name)}
        {@const current = value(cp.name)}
        <div class="flex flex-col gap-1.5 border-b border-slate-200/60 pb-2 last:border-0 last:pb-0 dark:border-slate-800/60">
          <div class="flex items-center justify-between">
            <span class="font-medium text-slate-600 dark:text-slate-300">{cp.label}</span>
            {#if current}
              <button
                type="button"
                class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-slate-700 dark:hover:text-slate-200"
                onclick={() => onSet(cp.name, null)}
              >
                <X class="size-3" />reset
              </button>
            {:else if inherited?.[cp.name]}
              <span class="text-[10px] text-slate-400">inherits {inherited[cp.name]}</span>
            {/if}
          </div>

          <!-- Color Tokens -->
          <div class="flex flex-wrap gap-1" role="radiogroup" aria-label={cp.label}>
            {#each tokens as t (t)}
              <button
                type="button"
                role="radio"
                aria-checked={current === t}
                title={t}
                class={"size-4.5 rounded border transition-transform " +
                  (current === t
                    ? "ring-2 ring-indigo-500 ring-offset-1 scale-110 border-indigo-600"
                    : "border-slate-300 hover:scale-105 dark:border-slate-600")}
                style:background={swatch(t)}
                onclick={() => onSet(cp.name, t)}
              >
                <span class="sr-only">{t}</span>
              </button>
            {/each}
          </div>

          <!-- Hex Input -->
          <div class="flex items-center gap-1.5 mt-0.5">
            <input
              type="color"
              class="h-6 w-7 cursor-pointer rounded border border-slate-300 bg-transparent p-0 dark:border-slate-600"
              value={current && HEX.test(current) ? current : "#000000"}
              onchange={(e) => onSet(cp.name, e.currentTarget.value.toLowerCase())}
              aria-label={`${cp.label} colour`}
            />
            <input
              class="w-24 rounded border border-slate-200 bg-white px-1.5 py-0.5 font-mono text-xs dark:border-slate-700 dark:bg-slate-800 text-slate-800 dark:text-slate-100"
              placeholder="#rrggbb"
              maxlength="7"
              value={hexDraft[cp.name] ?? (current && HEX.test(current) ? current : "")}
              oninput={(e) => (hexDraft = { ...hexDraft, [cp.name]: e.currentTarget.value })}
              onblur={() => commitHex(cp.name)}
              onkeydown={(e) => e.key === "Enter" && commitHex(cp.name)}
              aria-label={`${cp.label} hex value`}
            />
            {#if hexDraft[cp.name] && !HEX.test(hexDraft[cp.name])}
              <span class="text-[10px] text-rose-600">invalid hex</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}

  <!-- 5. BORDERS, CORNERS & SHADOWS -->
  {#if activeSection === "all" || activeSection === "borders"}
    <div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
      <span class="font-semibold text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
        <Box class="size-3.5 text-indigo-500" />
        <span>Border & Corners</span>
      </span>

      <!-- Border Style -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Border Style</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("borderStyle") ?? "solid"}
          onchange={(e) => onSet("borderStyle", e.currentTarget.value === "solid" ? null : e.currentTarget.value)}
        >
          <option value="solid">Solid</option>
          <option value="dashed">Dashed</option>
          <option value="dotted">Dotted</option>
          <option value="none">None</option>
        </select>
      </div>

      <!-- Border Width -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Border Width</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("borderWidth") ?? ""}
          onchange={(e) => onSet("borderWidth", e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}
        >
          <option value="">0px (None)</option>
          {#each [1, 2, 3, 4, 6, 8] as w (w)}
            <option value={w}>{w}px</option>
          {/each}
        </select>
      </div>

      <!-- Corners (Radius) -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Corner Radius</span>
        <select
          class="max-w-[9rem] rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("radius") ?? ""}
          onchange={(e) => onSet("radius", e.currentTarget.value || null)}
        >
          <option value="">None (0px)</option>
          {#each schema?.radius ?? ["sm", "md", "lg", "full"] as opt (opt)}
            <option value={opt}>{opt}</option>
          {/each}
        </select>
      </div>

      <!-- Shadow / Elevation -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Elevation (Shadow)</span>
        <select
          class="max-w-[9rem] rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("shadow") ?? ""}
          onchange={(e) => onSet("shadow", e.currentTarget.value || null)}
        >
          <option value="">None</option>
          {#each schema?.shadow ?? ["sm", "md", "lg"] as opt (opt)}
            <option value={opt}>{opt}</option>
          {/each}
        </select>
      </div>

      <!-- Layout Gap -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Element Gap</span>
        <input
          class="w-24 rounded border border-slate-200 bg-white px-2 py-0.5 font-mono text-xs dark:border-slate-700 dark:bg-slate-800 text-slate-800 dark:text-slate-100"
          placeholder="e.g. 8px or 16px"
          value={value("gap") ?? ""}
          onchange={(e) => onSet("gap", e.currentTarget.value ? e.currentTarget.value.trim() : null)}
        />
      </div>
    </div>
  {/if}

  <!-- 6. LAYOUT & FLEXBOX -->
  {#if activeSection === "all" || activeSection === "layout"}
    <div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
      <span class="font-semibold text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
        <LayoutGrid class="size-3.5 text-indigo-500" />
        <span>Layout & Flexbox</span>
      </span>

      <!-- Display -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Display</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800"
          value={value("display") ?? ""}
          onchange={(e) => onSet("display", e.currentTarget.value || null)}
        >
          <option value="">Default (Block / Flow)</option>
          <option value="flex">Flexbox</option>
          <option value="grid">CSS Grid</option>
          <option value="inline-flex">Inline Flex</option>
          <option value="none">Hidden (None)</option>
        </select>
      </div>

      <!-- Flex Direction -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Direction</span>
        <div class="flex items-center gap-1">
          {#each [["row", "Row →"], ["column", "Col ↓"]] as [dirVal, lbl] (dirVal)}
            <button
              type="button"
              class={"rounded border px-2 py-0.5 text-xs font-medium transition-colors " +
                (value("flexDirection") === dirVal
                  ? "bg-indigo-600 text-white border-indigo-600"
                  : "border-slate-200 bg-white dark:border-slate-700 dark:bg-slate-800 text-slate-600 dark:text-slate-300")}
              onclick={() => onSet("flexDirection", value("flexDirection") === dirVal ? null : dirVal)}
            >
              {lbl}
            </button>
          {/each}
        </div>
      </div>

      <!-- Justify Content -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Justify</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800 text-xs"
          value={value("justifyContent") ?? ""}
          onchange={(e) => onSet("justifyContent", e.currentTarget.value || null)}
        >
          <option value="">Default</option>
          <option value="start">Start</option>
          <option value="center">Center</option>
          <option value="end">End</option>
          <option value="between">Space Between</option>
          <option value="around">Space Around</option>
          <option value="evenly">Space Evenly</option>
        </select>
      </div>

      <!-- Align Items -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Align Items</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800 text-xs"
          value={value("alignItems") ?? ""}
          onchange={(e) => onSet("alignItems", e.currentTarget.value || null)}
        >
          <option value="">Default</option>
          <option value="start">Start</option>
          <option value="center">Center</option>
          <option value="end">End</option>
          <option value="stretch">Stretch</option>
          <option value="baseline">Baseline</option>
        </select>
      </div>

      <!-- Flex Wrap -->
      <div class="flex items-center justify-between gap-2">
        <span class="font-medium text-slate-600 dark:text-slate-300">Wrap</span>
        <select
          class="rounded border border-slate-200 bg-white px-2 py-0.5 dark:border-slate-700 dark:bg-slate-800 text-xs"
          value={value("flexWrap") ?? ""}
          onchange={(e) => onSet("flexWrap", e.currentTarget.value || null)}
        >
          <option value="">No Wrap</option>
          <option value="wrap">Wrap Lines</option>
        </select>
      </div>
    </div>
  {/if}

  <!-- 7. VISUAL EFFECTS & ICON -->
  {#if activeSection === "all" || activeSection === "effects"}
    <div class="flex flex-col gap-3 rounded-lg border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
      <span class="font-semibold text-slate-700 dark:text-slate-200 flex items-center gap-1.5">
        <Sparkles class="size-3.5 text-indigo-500" />
        <span>Effects & Icon Styling</span>
      </span>

      <!-- Icon Selector & Preview -->
      <div class="flex flex-col gap-2 rounded-lg border border-slate-200/80 bg-white p-2.5 shadow-2xs dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <div class="flex flex-col">
            <span class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-1.5">
              <span>Icon</span>
              {#if effectiveIcon}
                <span class="rounded bg-indigo-50 px-1.5 py-0.2 text-[10px] font-mono text-indigo-600 dark:bg-indigo-950 dark:text-indigo-400">
                  active
                </span>
              {/if}
            </span>
            <span class="text-[10px] text-slate-400">
              {effectiveIcon ? "Synced with element properties" : "Assign an icon and customize its appearance"}
            </span>
          </div>
          {#if effectiveIcon}
            <button
              type="button"
              class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-rose-600 dark:hover:text-rose-400"
              onclick={() => {
                handleIconChange(null);
                onSet("icon", null);
                onSet("iconSize", null);
                onSet("iconColor", null);
                onSet("iconBackground", null);
                onSet("iconRadius", null);
              }}
            >
              <X class="size-3" />Clear Icon
            </button>
          {/if}
        </div>

        <IconPicker
          value={effectiveIcon}
          placeholder="Select an icon..."
          onChange={handleIconChange}
          onSelect={handleIconChange}
        />

        {#if effectiveIcon}
          <!-- Live Preview Box -->
          <div class="flex items-center gap-3 rounded-lg border border-slate-100 bg-slate-50/70 p-2.5 dark:border-slate-800/80 dark:bg-slate-950/60 mt-1">
            <div class="flex size-12 shrink-0 items-center justify-center rounded-lg border border-slate-200 bg-white shadow-2xs dark:border-slate-800 dark:bg-slate-900 overflow-hidden">
              <Icon
                name={effectiveIcon}
                size={value("iconSize") ?? "md"}
                color={value("iconColor")}
                background={value("iconBackground")}
                radius={value("iconRadius")}
              />
            </div>
            <div class="flex flex-col min-w-0">
              <span class="font-mono text-[11px] font-bold text-slate-800 dark:text-slate-200 truncate">{effectiveIcon}</span>
              <span class="text-[10px] text-slate-500 dark:text-slate-400">
                Size: {value("iconSize") ?? "md (default)"} · Color: {value("iconColor") ?? "inherit"}
              </span>
            </div>
          </div>

          <!-- Icon Size Presets & Custom -->
          <div class="flex flex-col gap-1.5 pt-1.5 border-t border-slate-100 dark:border-slate-800">
            <div class="flex items-center justify-between text-[11px]">
              <span class="font-medium text-slate-600 dark:text-slate-300">Icon Size</span>
              {#if value("iconSize")}
                <button type="button" class="text-slate-400 hover:text-rose-500" onclick={() => onSet("iconSize", null)}><X class="size-3" /></button>
              {/if}
            </div>
            <div class="grid grid-cols-6 gap-1">
              {#each [
                ["xs", "XS", "14px"],
                ["sm", "SM", "16px"],
                ["md", "MD", "20px"],
                ["lg", "LG", "24px"],
                ["xl", "XL", "32px"],
                ["2xl", "2XL", "40px"]
              ] as [sVal, sLabel, sPx] (sVal)}
                <button
                  type="button"
                  class={"rounded py-1 text-[10px] font-semibold transition-colors border " +
                    (value("iconSize") === sVal || (sVal === "md" && !value("iconSize"))
                      ? "border-indigo-600 bg-indigo-600 text-white font-bold"
                      : "border-slate-200 bg-white text-slate-600 hover:border-indigo-300 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400")}
                  onclick={() => onSet("iconSize", sVal)}
                  title={`${sLabel} (${sPx})`}
                >
                  {sLabel}
                </button>
              {/each}
            </div>
            <input
              type="text"
              class="w-full rounded border border-slate-200 bg-white px-2 py-1 text-xs text-slate-700 outline-none focus:border-indigo-500 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200"
              placeholder="Custom size (e.g. 18px, 1.5rem)"
              value={value("iconSize") ?? ""}
              onchange={(e) => onSet("iconSize", e.currentTarget.value.trim() || null)}
            />
          </div>

          <!-- Icon Color -->
          <div class="flex flex-col gap-1.5 pt-1.5 border-t border-slate-100 dark:border-slate-800">
            <div class="flex items-center justify-between text-[11px]">
              <span class="font-medium text-slate-600 dark:text-slate-300">Icon Color</span>
              {#if value("iconColor")}
                <button type="button" class="text-slate-400 hover:text-rose-500" onclick={() => onSet("iconColor", null)}><X class="size-3" /></button>
              {/if}
            </div>
            <div class="flex flex-wrap gap-1">
              {#each [
                "primary", "secondary", "accent", "info", "favorable", "unfavorable", "attention", "neutral", "subordinate", "strong"
              ] as token (token)}
                <button
                  type="button"
                  class={"size-4.5 rounded-full border transition-transform hover:scale-110 " +
                    (value("iconColor") === token ? "ring-2 ring-indigo-500 ring-offset-1 border-white" : "border-slate-300/80 dark:border-slate-700")}
                  style={`background: ${swatch(token)};`}
                  title={token}
                  onclick={() => onSet("iconColor", token)}
                />
              {/each}
            </div>
            <div class="flex items-center gap-1.5">
              <input
                type="color"
                class="size-6 cursor-pointer rounded border-0 bg-transparent p-0"
                value={HEX.test(String(value("iconColor") ?? "")) ? value("iconColor") : "#4f46e5"}
                oninput={(e) => onSet("iconColor", e.currentTarget.value.toLowerCase())}
              />
              <input
                type="text"
                class="flex-1 rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs text-slate-700 outline-none focus:border-indigo-500 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200"
                placeholder="#rrggbb or token"
                value={value("iconColor") ?? ""}
                onchange={(e) => onSet("iconColor", e.currentTarget.value.trim() || null)}
              />
            </div>
          </div>

          <!-- Icon Background & Shape -->
          <div class="flex flex-col gap-1.5 pt-1.5 border-t border-slate-100 dark:border-slate-800">
            <div class="flex items-center justify-between text-[11px]">
              <span class="font-medium text-slate-600 dark:text-slate-300">Icon Background & Shape</span>
              {#if value("iconBackground") || value("iconRadius")}
                <button
                  type="button"
                  class="text-slate-400 hover:text-rose-500 text-[10px]"
                  onclick={() => {
                    onSet("iconBackground", null);
                    onSet("iconRadius", null);
                  }}
                >
                  <X class="size-3" />
                </button>
              {/if}
            </div>

            <!-- Shape / Corner Radius -->
            <div class="grid grid-cols-4 gap-1">
              {#each [
                ["none", "Square", "0px"],
                ["sm", "Soft", "4px"],
                ["md", "Rounded", "8px"],
                ["full", "Circle", "9999px"]
              ] as [rVal, rLabel, rDesc] (rVal)}
                <button
                  type="button"
                  class={"rounded py-1 text-[10px] font-semibold transition-colors border " +
                    (value("iconRadius") === rVal || (rVal === "none" && !value("iconRadius"))
                      ? "border-indigo-600 bg-indigo-600 text-white font-bold"
                      : "border-slate-200 bg-white text-slate-600 hover:border-indigo-300 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-400")}
                  onclick={() => onSet("iconRadius", rVal === "none" ? null : rVal)}
                  title={`${rLabel} (${rDesc})`}
                >
                  {rLabel}
                </button>
              {/each}
            </div>

            <!-- Background Color -->
            <div class="flex items-center gap-1.5 mt-0.5">
              <input
                type="color"
                class="size-6 cursor-pointer rounded border-0 bg-transparent p-0"
                value={HEX.test(String(value("iconBackground") ?? "")) ? value("iconBackground") : "#e0e7ff"}
                oninput={(e) => onSet("iconBackground", e.currentTarget.value.toLowerCase())}
              />
              <input
                type="text"
                class="flex-1 rounded border border-slate-200 bg-white px-2 py-1 font-mono text-xs text-slate-700 outline-none focus:border-indigo-500 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200"
                placeholder="Background: #rrggbb or token"
                value={value("iconBackground") ?? ""}
                onchange={(e) => onSet("iconBackground", e.currentTarget.value.trim() || null)}
              />
            </div>
          </div>
        {/if}
      </div>

      <!-- Opacity Slider -->
      <div class="flex flex-col gap-1.5 border-t border-slate-200/60 pt-2 dark:border-slate-800/60">
        <div class="flex items-center justify-between">
          <span class="font-medium text-slate-600 dark:text-slate-300">Element Opacity</span>
          <span class="font-mono text-[11px] text-slate-500">
            {value("opacity") ? `${Math.round(Number(value("opacity")) * 100)}%` : "100%"}
          </span>
        </div>
        <div class="flex items-center gap-2">
          <input
            type="range"
            min="0"
            max="1"
            step="0.05"
            class="flex-1 accent-indigo-600"
            value={value("opacity") ?? "1"}
            oninput={(e) => onSet("opacity", e.currentTarget.value === "1" ? null : e.currentTarget.value)}
          />
          {#if value("opacity")}
            <button type="button" class="text-slate-400 hover:text-rose-500" onclick={() => onSet("opacity", null)}><X class="size-3" /></button>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>
