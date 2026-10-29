<script>
  /**
   * PropertyPanel.svelte
   * Mature Page Designer Property Sheet refactored with bos-ui enterprise components.
   * - Clean categorized property groups (General, Data & Binding, Lookups, Expressions).
   * - Enterprise Select, Input, Switch, Badge, and Button primitives.
   * - Clear auto-filled indicators with instant reset to inherited values.
   */
  import {
    RotateCcw,
    Sliders,
    Layers,
    Palette,
    Search,
    Database,
    Variable,
    Check,
    AlertCircle,
    Info,
    ExternalLink,
    ArrowUp,
    ArrowDown,
    Trash2
  } from "lucide-svelte";
  import { Button, Input, Select, Badge, Tabs } from "$lib/bos-ui/index.js";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";
  import StyleEditor from "./StyleEditor.svelte";
  import IconPicker from "./IconPicker.svelte";
  import PageLinkMapper from "./PageLinkMapper.svelte";
  import ExpressionRuleBuilder from "./ExpressionRuleBuilder.svelte";
  import { studioStore } from "../../stores/studioStore.svelte.js";
  import { Sparkles } from "lucide-svelte";
  import { toBosValue, fromBosValue, lastSegment } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  let tab = $state("properties");
  let fieldError = $state("");
  let isRuleBuilderOpen = $state(false);
  let activeRuleProp = $state("showWhen");
  let activeRuleExpression = $state("");

  function openRuleBuilder(propName, currentExpr) {
    activeRuleProp = propName;
    activeRuleExpression = currentExpr || "";
    isRuleBuilderOpen = true;
  }

  function isCodeunitInCatalog(name) {
    if (!name) return true;
    return (studioStore.catalog || []).some(
      (o) => o.object_type === "codeunit" && (o.name?.toLowerCase() === name.toLowerCase() || o.object_name?.toLowerCase() === name.toLowerCase())
    );
  }
  let propertyFilter = $state("");
  let activeSection = $state("all"); // 'all' | 'data' | 'general' | 'navigation' | 'behavior'

  const path = $derived(model.selected);
  const isPage = $derived(path === "page");
  const isHeaderTitle = $derived(path === "header:title");
  const isHeaderInstruction = $derived(path === "header:instruction");
  const isHeader = $derived(path === "header");
  const isAction = $derived(Boolean(path?.startsWith("action:")));
  const compiled = $derived(isPage ? null : isAction ? model.selectedAction : model.selectedCompiled);
  const node = $derived(model.mode === "develop" ? (isPage || isHeaderTitle || isHeaderInstruction || isHeader ? model.object : model.selectedNode) : null);
  const kind = $derived(isPage ? "page" : isHeaderTitle ? "headline" : isHeaderInstruction ? "label" : isHeader ? "header" : isAction ? "action" : (compiled?.kind ?? node?.kind ?? ""));
  const customize = $derived(model.mode === "customize");
  const authored = $derived(model.mode === "develop" && path ? model.isAuthored(path) : true);

  const customChange = $derived.by(() => {
    if (!customize || !path) return null;
    if (isPage) return model.changes.page ?? {};
    if (isAction) return model.changes.actions.find((a) => a.name === path.slice(7)) ?? {};
    return model.changes.controls.find((c) => c.name === lastSegment(path).name) ?? {};
  });

  const ACTION_PROPS = [
    { name: "caption", type: "text", customize: true, group: "general" },
    { name: "area", type: "enum:Processing,Navigation,Reporting,Creation,Promoted", group: "general" },
    { name: "group", type: "text", group: "general" },
    { name: "icon", type: "icon", customize: true, group: "general" },
    { name: "tooltip", type: "text", customize: true, group: "general" },
    { name: "visible", type: "bool", customize: true, group: "general" },
    { name: "primary", type: "bool", group: "behavior" },
    { name: "promoted", type: "bool", group: "behavior" },
    { name: "promotedCategory", type: "enum:Process,Report,New,Category4,Category5,Category6,Category7,Category8,Category9,Category10", group: "behavior" },
    { name: "promotedOnly", type: "bool", group: "behavior" },
    { name: "confirm", type: "text", group: "behavior" },
    { name: "shortcut", type: "text", group: "behavior" },
    { name: "isDefault", type: "bool", group: "behavior" },
    { name: "inFooterBar", type: "bool", group: "behavior" },
    { name: "runCodeunit", type: "text", group: "navigation" },
    { name: "openPage", type: "page", group: "navigation" },
    { name: "runPageMode", type: "enum:view,edit,create", group: "navigation" },
    { name: "runPageLink", type: "text", group: "navigation" },
    { name: "enableWhen", type: "expr", group: "behavior" },
    { name: "showWhen", type: "expr", group: "behavior" }
  ];

  function assignGroup(propName) {
    const p = propName.toLowerCase();
    if (["control", "extendeddatatype", "editable", "showmandatory", "decimalplaces", "blankzero", "cardpageid", "insertallowed", "modifyallowed", "deleteallowed", "sourcetable"].some(k => p.includes(k))) {
      return "data";
    }
    if (["lookuppageid", "drilldownpageid", "openpage", "runcodeunit", "runpagemode", "runpagelink"].some(k => p.includes(k))) {
      return "navigation";
    }
    if (["showwhen", "enablewhen", "confirm", "primary", "promoted", "promotedcategory", "promotedonly", "shortcut", "isdefault", "infooterbar"].some(k => p.includes(k))) {
      return "behavior";
    }
    return "general";
  }

  const allRows = $derived.by(() => {
    const schema = model.schema;
    if (!schema || !path) return [];
    if (isHeaderTitle) {
      return [
        { name: "caption", type: "text", customize: true, group: "general" },
        { name: "icon", type: "icon", customize: true, group: "general" }
      ];
    }
    if (isHeaderInstruction) {
      return [
        { name: "caption", type: "text", customize: true, group: "general" }
      ];
    }
    if (isHeader) {
      return [
        { name: "caption", type: "text", customize: true, group: "general" }
      ];
    }
    const list = isPage ? schema.page_properties : isAction ? ACTION_PROPS : schema.control_properties;
    return list
      .filter((r) => (!r.kinds || r.kinds.includes(kind)) && (!customize || r.customize))
      .map((r) => ({ ...r, group: r.group || assignGroup(r.name) }));
  });

  const filteredRows = $derived.by(() => {
    return allRows.filter((r) => {
      const matchSection = activeSection === "all" || r.group === activeSection;
      const matchQuery = !propertyFilter || r.name.toLowerCase().includes(propertyFilter.toLowerCase());
      return matchSection && matchQuery;
    });
  });

  function enumOptions(type) {
    const spec = type.slice("enum:".length);
    if (spec.includes(",")) return spec.split(",").map(v => ({ value: v, label: v }));
    if (spec === "page_types") return (model.schema?.page_types ?? []).map((t) => ({ value: t.name, label: t.name }));
    if (spec === "field_controls") return (model.schema?.field_controls ?? []).map(v => ({ value: v, label: v }));
    const raw = model.schema?.[spec] ?? [];
    return raw.map(v => ({ value: v, label: v }));
  }

  const pageOptions = $derived(
    Object.keys(pageManifest.manifest?.pages ?? {})
      .sort()
      .map(n => ({ value: n, label: n }))
  );

  /** Authored (develop) or customized (customize) value of a property. */
  function ownValue(name, type) {
    if (customize) return customChange?.[name] ?? null;
    if (isHeaderTitle) {
      if (name === "caption") {
        const prop = node?.props?.find((p) => p.name === "caption" || p.name.toLowerCase() === "caption");
        return prop ? fromBosValue(type, prop.value) : (model.compiled?.caption ?? null);
      }
      if (name === "icon") {
        const direct = node?.props?.find((p) => p.name === "icon" || p.name.toLowerCase() === "icon");
        if (direct) return fromBosValue(type, direct.value);
        const styleProp = node?.blocks?.find((b) => b.name === "style")?.props?.find((p) => p.name === "icon" || p.name.toLowerCase() === "icon");
        if (styleProp) return fromBosValue(type, styleProp.value);
        return model.compiled?.style?.icon ?? null;
      }
    }
    if (isHeaderInstruction && name === "caption") {
      const prop = node?.props?.find((p) => p.name === "instructional_text" || p.name.toLowerCase() === "instructionaltext");
      return prop ? fromBosValue(type, prop.value) : (model.compiled?.instructional_text ?? null);
    }
    if (name === "icon") {
      const direct = node?.props?.find((p) => p.name === "icon" || p.name.toLowerCase() === "icon");
      if (direct) return fromBosValue(type, direct.value);
      const styleProp = node?.style?.find((p) => p.name === "icon" || p.name.toLowerCase() === "icon");
      if (styleProp) return fromBosValue(type, styleProp.value);
    }
    const prop = node?.props?.find((p) => p.name === name || p.name.toLowerCase() === name.toLowerCase());
    return prop ? fromBosValue(type, prop.value) : null;
  }

  /** Effective compiled value, for the "auto" hint. */
  function effective(name) {
    if (isHeaderTitle) {
      if (name === "caption") return model.compiled?.caption ?? null;
      if (name === "icon") return model.compiled?.style?.icon ?? null;
    }
    if (isHeaderInstruction && name === "caption") {
      return model.compiled?.instructional_text ?? null;
    }
    if (isPage) {
      if (name === "caption") return model.compiled?.caption ?? null;
      if (name === "icon") return model.compiled?.style?.icon ?? null;
      if (name === "instructionalText") return model.compiled?.instructional_text ?? null;
      if (name === "pageType") return model.compiled?.page_type ?? null;
      if (name === "editable") return model.compiled?.editable ?? null;
      if (name === "insertAllowed") return model.compiled?.insert_allowed ?? null;
      if (name === "modifyAllowed") return model.compiled?.modify_allowed ?? null;
      if (name === "deleteAllowed") return model.compiled?.delete_allowed ?? null;
      if (name === "cardPageId") return model.compiled?.card_page_id ?? null;
      return model.compiled?.[name] ?? null;
    }
    if (!compiled) return null;
    const f = compiled.field;
    switch (name) {
      case "caption":
        return compiled.caption ?? null;
      case "icon":
        return compiled.icon ?? compiled.style?.icon ?? null;
      case "area":
        return compiled.area ?? null;
      case "group":
        return compiled.group ?? null;
      case "primary":
        return compiled.primary ?? null;
      case "promoted":
        return compiled.promoted ?? null;
      case "promotedCategory":
        return compiled.promoted_category ?? null;
      case "promotedOnly":
        return compiled.promoted_only ?? null;
      case "confirm":
        return compiled.confirm ?? null;
      case "shortcut":
        return compiled.shortcut ?? null;
      case "runCodeunit":
        return compiled.run_codeunit ?? null;
      case "openPage":
        return compiled.open_page ?? null;
      case "runPageMode":
        return compiled.run_page_mode ?? null;
      case "runPageLink":
        return compiled.run_page_link ? JSON.stringify(compiled.run_page_link) : null;
      case "tooltip":
        return compiled.tooltip ?? null;
      case "visible":
        return compiled.visible ?? null;
      case "enableWhen":
        return compiled.enable_when ?? null;
      case "showWhen":
        return compiled.show_when ?? null;
      case "isDefault":
        return compiled.is_default ?? null;
      case "inFooterBar":
        return compiled.in_footer_bar ?? null;
      case "importance":
        return compiled.responsive?.importance ?? null;
      case "editable":
        return f?.editable ?? null;
      case "showMandatory":
        return f?.required ?? null;
      case "control":
        return f?.control ?? null;
      case "extendedDatatype":
        return f?.extended_type ?? null;
      case "decimalPlaces":
        return f?.decimals ? f.decimals.join(":") : null;
      case "blankZero":
        return f?.blank_zero ?? null;
      case "lookupPageId":
        return f?.lookup?.page ?? null;
      case "drillDownPageId":
        return f?.drill_down?.page ?? null;
      case "placeholder":
        return f?.placeholder ?? null;
      default:
        return compiled.properties?.[name] ?? null;
    }
  }

  function autoSource(name) {
    const f = compiled?.field;
    if (!f?.auto_filled?.includes(name === "showMandatory" ? "required" : name)) return null;
    return f.binding === "variable" ? `page variable ${f.key}` : `${model.compiled?.source_table ?? "table"}.${f.source}`;
  }

  async function set(row, value) {
    fieldError = "";
    try {
      if (customize) {
        await model.setProperty(path, row.name, null, { raw: value === "" ? null : value });
        return;
      }
      const bos = value === null || value === "" ? null : toBosValue(row.type, value);
      await model.setProperty(path, row.name, bos);
    } catch (e) {
      fieldError = `${row.name}: ${e.message}`;
    }
  }

  // ----- style & responsive -----
  const STYLE_TYPES = {
    background: "color",
    textColor: "color",
    borderColor: "color",
    borderWidth: "int",
    icon: "text",
    iconSize: "text",
    iconColor: "color",
    iconBackground: "color",
    iconRadius: "text",
    fontSize: "text",
    fontWeight: "text",
    textAlign: "text",
    letterSpacing: "text",
    lineHeight: "text",
    textTransform: "text",
    textDecoration: "text",
    borderStyle: "text",
    padding: "text",
    margin: "text",
    gap: "text",
    width: "text",
    maxWidth: "text",
    minWidth: "text",
    height: "text",
    minHeight: "text",
    maxHeight: "text",
    opacity: "text",
    overflow: "text",
    display: "text",
    justifyContent: "text",
    alignItems: "text",
    flexDirection: "text",
    flexWrap: "text"
  };
  function blockValues(blockName) {
    if (customize) return blockName === "style" ? (customChange?.style ?? {}) : (customChange?.responsive ?? {});
    const blk = node?.blocks?.find((b) => b.name === blockName);
    const out = {};
    for (const p of blk?.props ?? []) {
      const v = fromBosValue("text", p.value);
      out[p.name] = p.name === "borderWidth" ? Number(v) : v;
    }
    return out;
  }
  const styleValues = $derived(blockValues("style"));
  const responsiveValues = $derived(blockValues("responsive"));
  const spans = $derived(customize ? (responsiveValues.span ?? {}) : responsiveValues);

  async function setStyle(name, value) {
    fieldError = "";
    try {
      if (customize) return await model.setProperty(path, name, null, { block: "style", raw: value });
      const bos = value === null ? null : toBosValue(STYLE_TYPES[name] ?? "enum", value);
      await model.setProperty(path, name, bos, { block: "style" });
    } catch (e) {
      fieldError = `${name}: ${e.message}`;
    }
  }

  async function setResponsive(name, value) {
    const v = value === "" ? null : name === "hideBelow" ? value : Number(value);
    if (customize) return model.setProperty(path, name, null, { block: "responsive", raw: v });
    await model.setProperty(path, name, v === null ? null : String(v), { block: "responsive" });
  }
</script>

{#if !path}
  <div class="p-6 text-center text-xs text-slate-400 space-y-2">
    <Sliders class="size-8 mx-auto text-slate-300 dark:text-slate-600" />
    <p class="font-semibold text-slate-600 dark:text-slate-300">No Control Selected</p>
    <p class="text-[11px] text-slate-400">Select any element on the canvas or outline to configure its properties.</p>
  </div>
{:else}
  <div class="flex flex-col gap-2.5 text-xs">
    <!-- Compact Element Header & Actions (Zero Bloat, High-Density) -->
    <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-900/60 p-2.5 space-y-1.5 shadow-2xs">
      <div class="flex items-center justify-between gap-2">
        <div class="flex items-center gap-1.5 min-w-0">
          <span class="rounded bg-indigo-100 dark:bg-indigo-950/80 px-1.5 py-0.5 font-mono text-[10px] font-bold uppercase tracking-wider text-indigo-700 dark:text-indigo-300 shrink-0">
            {kind}
          </span>
          <span class="font-bold text-xs font-mono truncate text-slate-900 dark:text-slate-100" title={isPage ? (model.compiled?.caption ?? model.name) : (compiled?.caption ?? lastSegment(path).name)}>
            {isPage ? (model.compiled?.caption ?? model.name) : (compiled?.caption ?? lastSegment(path).name)}
          </span>
        </div>

        {#if !isPage}
          <div class="flex items-center gap-0.5 shrink-0">
            <button
              type="button"
              class="p-1 rounded text-slate-400 hover:text-slate-700 hover:bg-slate-200/80 dark:hover:bg-slate-800 dark:hover:text-slate-200 transition-colors"
              title="Move element up (Alt+Up)"
              onclick={() => model.nudge(path, -1)}
            >
              <ArrowUp class="size-3.5" />
            </button>
            <button
              type="button"
              class="p-1 rounded text-slate-400 hover:text-slate-700 hover:bg-slate-200/80 dark:hover:bg-slate-800 dark:hover:text-slate-200 transition-colors"
              title="Move element down (Alt+Down)"
              onclick={() => model.nudge(path, 1)}
            >
              <ArrowDown class="size-3.5" />
            </button>
            <button
              type="button"
              class="p-1 rounded text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-950/60 transition-colors"
              title="Remove element from layout (Delete)"
              onclick={() => model.remove(path)}
            >
              <Trash2 class="size-3.5" />
            </button>
          </div>
        {/if}
      </div>

      {#if compiled?.field}
        <div class="flex items-center gap-1.5 text-[10px] font-mono text-slate-500 dark:text-slate-400 truncate">
          {#if compiled.field.binding === "variable"}
            <span class="inline-flex items-center gap-1 text-purple-600 dark:text-purple-400">
              <Variable class="size-2.5" /> var {compiled.field.key}
            </span>
          {:else if compiled.field.source}
            <span class="inline-flex items-center gap-1 text-blue-600 dark:text-blue-400">
              <Database class="size-2.5" /> {model.compiled?.source_table ?? "table"}.{compiled.field.source}
            </span>
          {/if}
          {#if compiled.field.data_type}
            <span class="text-slate-400">({compiled.field.data_type})</span>
          {/if}
        </div>
      {/if}

      {#if !authored}
        <div class="p-2 rounded-lg bg-amber-50 dark:bg-amber-950/60 text-amber-800 dark:text-amber-200 text-[10px] border border-amber-200 dark:border-amber-800 flex items-start gap-1.5">
          <Info class="size-3.5 shrink-0 mt-0.5 text-amber-600" />
          <div>
            {#if model.objectKind === "pageextension"}
              <span>Defined by <strong>{model.targetPage}</strong>. Switch tab to edit.</span>
            {:else}
              <span>Defined by a page extension. Edit extension directly.</span>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <!-- Inspector Tabs -->
    <div class="flex items-center gap-1 border-b border-slate-200 dark:border-slate-800 pb-1 text-xs font-semibold">
      {#each [
        ["properties", "Properties", Sliders],
        ["style", "Style", Palette],
        ...(isPage || isAction ? [] : [["layout", "Layout", Layers]])
      ] as [id, label, Icon] (id)}
        <button
          type="button"
          class={
            "flex items-center gap-1 px-2.5 py-1 rounded-lg transition-all text-xs " +
            (tab === id
              ? "bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 font-bold border border-indigo-200/70 dark:border-indigo-800/60 shadow-2xs"
              : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800")
          }
          onclick={() => (tab = id)}
        >
          <Icon class="size-3.5" />
          <span>{label}</span>
        </button>
      {/each}
    </div>

    {#if fieldError}
      <div class="p-2 rounded-lg bg-rose-50 dark:bg-rose-950/50 text-rose-700 dark:text-rose-300 text-xs font-medium border border-rose-200 dark:border-rose-800 flex items-center gap-1.5" role="alert">
        <AlertCircle class="size-3.5 shrink-0" />
        <span>{fieldError}</span>
      </div>
    {/if}

    <fieldset disabled={!authored || model.busy} class="contents">
      {#if tab === "properties"}
        <!-- Search & Section Filter for Properties -->
        <div class="space-y-1.5">
          <div class="relative">
            <input
              type="text"
              placeholder="Search properties..."
              bind:value={propertyFilter}
              class="w-full h-7 rounded-lg border border-slate-200 bg-slate-50/70 pl-7 pr-2 text-xs text-slate-800 placeholder:text-slate-400 outline-none focus:border-indigo-500 focus:bg-white dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100 dark:focus:border-indigo-500"
            />
            <Search class="size-3 absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" />
          </div>

          <div class="flex items-center gap-1 overflow-x-auto pb-0.5 no-scrollbar text-[10px]">
            {#each [
              ["all", "All"],
              ["data", "Data"],
              ["general", "General"],
              ["navigation", "Lookups"],
              ["behavior", "Logic & Rules"]
            ] as [secId, secLabel] (secId)}
              <button
                type="button"
                class={
                  "px-2 py-0.5 rounded-md font-medium transition-colors shrink-0 " +
                  (activeSection === secId
                    ? "bg-slate-800 text-white dark:bg-slate-200 dark:text-slate-900 font-semibold"
                    : "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-200")
                }
                onclick={() => (activeSection = secId)}
              >
                {secLabel}
              </button>
            {/each}
          </div>
        </div>

        <!-- Properties Grid List -->
        <div class="divide-y divide-slate-100 dark:divide-slate-800/60 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-1.5 shadow-2xs">
          {#if filteredRows.length === 0}
            <p class="p-3 text-center text-xs text-slate-400 italic">No properties match your filter.</p>
          {:else}
            {#each filteredRows as row (row.name)}
              {@const own = ownValue(row.name, row.type)}
              {@const eff = effective(row.name)}
              {@const auto = autoSource(row.name)}
              <div class="py-1.5 space-y-1 first:pt-0.5 last:pb-0.5">
                <div class="flex items-center justify-between gap-1.5">
                  <div class="flex items-center gap-1.5 min-w-0">
                    <span class="font-semibold text-slate-700 dark:text-slate-200 text-xs font-mono">
                      {row.name}
                    </span>
                    {#if own === null && auto}
                      <Badge tone="success" size="sm" class="text-[9px] py-0 px-1 font-semibold truncate" title={`Auto-filled from ${auto}`}>
                        auto · {auto}
                      </Badge>
                    {/if}
                  </div>

                  {#if own !== null}
                    <button
                      type="button"
                      class="flex items-center gap-0.5 text-[9px] text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
                      title={`Reset ${row.name} to automatic / inherited default`}
                      onclick={(e) => {
                        e.preventDefault();
                        set(row, null);
                      }}
                    >
                      <RotateCcw class="size-2.5" />
                      <span>Reset</span>
                    </button>
                  {/if}
                </div>

                <!-- Control Renderers -->
                {#if row.type === "bool"}
                  <Select
                    value={own === null ? "" : String(own)}
                    options={[
                      { value: "", label: eff === null ? "— (inherited)" : `auto (${eff ? "Yes" : "No"})` },
                      { value: "true", label: "Yes (True)" },
                      { value: "false", label: "No (False)" }
                    ]}
                    onValueChange={(val) => set(row, val === "" ? null : val === "true")}
                  />
                {:else if row.type.startsWith("enum:")}
                  <Select
                    value={own ?? ""}
                    options={[
                      { value: "", label: eff === null ? "— (default)" : `auto (${eff})` },
                      ...enumOptions(row.type)
                    ]}
                    onValueChange={(val) => set(row, val || null)}
                  />
                {:else if row.type === "page"}
                  <Select
                    value={own ?? ""}
                    options={[
                      { value: "", label: eff === null ? "— (none)" : `auto (${eff})` },
                      ...pageOptions
                    ]}
                    onValueChange={(val) => set(row, val || null)}
                  />
                {:else if row.type === "icon" || row.name.toLowerCase() === "icon" || row.name.toLowerCase().endsWith("icon")}
                  <div class="flex items-center gap-1.5">
                    <div class="flex-1 min-w-0">
                      <IconPicker
                        value={own ?? ""}
                        onChange={(val) => set(row, val)}
                        onSelect={(val) => set(row, val)}
                      />
                    </div>
                  </div>
                {:else if row.name === "runPageLink"}
                  <PageLinkMapper
                    value={own ?? ""}
                    sourceFields={model.fields}
                    targetPage={ownValue("openPage", "page") || effective("openPage") || ""}
                    onChange={(val) => set(row, val || null)}
                  />
                {:else if row.type === "expr"}
                  <div class="flex items-center gap-1.5">
                    <Input
                      size="sm"
                      type="text"
                      value={own ?? ""}
                      placeholder={eff === null ? "" : String(eff)}
                      class="font-mono text-xs flex-1"
                      onchange={(e) => set(row, e.currentTarget.value.trim() || null)}
                    />
                    <Button
                      size="xs"
                      variant="outline"
                      class="h-7 px-2 text-[10px] gap-1 shrink-0 text-indigo-600 border-indigo-200 dark:border-indigo-800 hover:bg-indigo-50 dark:hover:bg-indigo-950 font-semibold"
                      title="Open Visual Condition Builder"
                      onclick={() => openRuleBuilder(row.name, own ?? "")}
                    >
                      <Sparkles class="size-2.5" /> Rule
                    </Button>
                  </div>
                {:else if row.name === "runCodeunit"}
                  <div class="space-y-1">
                    <Input
                      size="sm"
                      type="text"
                      value={own ?? ""}
                      placeholder="CodeunitIdentifier"
                      class="font-mono text-xs"
                      onchange={(e) => set(row, e.currentTarget.value.trim() || null)}
                    />
                    {#if own && !isCodeunitInCatalog(own)}
                      <div class="flex items-center justify-between p-1.5 rounded-lg bg-amber-50 dark:bg-amber-950/60 border border-amber-200 dark:border-amber-800 text-[10px] text-amber-800 dark:text-amber-200">
                        <span>Codeunit '{own}' not found</span>
                        <Button
                          size="xs"
                          variant="primary"
                          class="h-5 px-1.5 text-[9px] gap-1 font-semibold"
                          onclick={() => model.scaffoldCodeunit(own)}
                        >
                          <Sparkles class="size-2.5" /> Scaffold
                        </Button>
                      </div>
                    {/if}
                  </div>
                {:else}
                  <Input
                    size="sm"
                    type={row.type === "int" ? "number" : "text"}
                    value={own ?? ""}
                    placeholder={eff === null ? "" : String(eff)}
                    class={row.type === "expr" ? "font-mono text-xs" : ""}
                    onchange={(e) => set(row, e.currentTarget.value.trim() || null)}
                  />
                {/if}
              </div>
            {/each}
          {/if}
        </div>

        {#if !customize && compiled?.triggers?.length}
          <div class="p-2.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900/60 text-xs space-y-1">
            <span class="font-bold text-slate-700 dark:text-slate-300 text-[11px] uppercase tracking-wider block">
              Active Triggers
            </span>
            <div class="flex items-center gap-1.5 flex-wrap">
              {#each compiled.triggers as trg}
                <span class="font-mono text-[10px] px-1.5 py-0.5 rounded bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-indigo-600 dark:text-indigo-300 font-semibold">
                  {trg}
                </span>
              {/each}
            </div>
            <p class="text-[10px] text-slate-400 pt-1">
              Trigger scripts execute on the server inside the QuickJS sandbox. Click "Code" above to edit.
            </p>
          </div>
        {/if}

      {:else if tab === "style"}
        <div class="p-2 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs">
          <StyleEditor
            style={styleValues}
            inherited={isPage ? {} : (compiled?.style ?? {})}
            schema={model.schema}
            activeIcon={ownValue("icon") || styleValues?.icon || compiled?.icon || compiled?.style?.icon || null}
            onSet={setStyle}
            onSetIcon={async (iconName) => {
              if (isHeaderTitle || allRows.some((r) => r.name === "icon")) {
                const iconRow = allRows.find((r) => r.name === "icon") || { name: "icon", type: "icon" };
                await set(iconRow, iconName);
              } else {
                await setStyle("icon", iconName);
              }
            }}
          />
        </div>

      {:else if tab === "layout"}
        <div class="p-3 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-3">
          <div>
            <h4 class="font-bold text-xs text-slate-900 dark:text-slate-100">
              Responsive Grid Spans
            </h4>
            <p class="text-[11px] text-slate-500 mt-0.5">
              Specify 1 to 12 column span per responsive breakpoint (empty = automatic fluid span).
            </p>
          </div>

          <div class="space-y-2">
            {#each model.schema?.breakpoints ?? [] as bp (bp)}
              <div class="flex items-center justify-between gap-3 py-1 border-b border-slate-100 dark:border-slate-800">
                <span class="font-mono font-bold text-slate-700 dark:text-slate-300 uppercase text-xs">
                  {bp}
                </span>
                <div class="w-32">
                  <Select
                    value={String(spans?.[bp] ?? "")}
                    options={[
                      { value: "", label: "auto" },
                      ...Array.from({ length: 12 }, (_, i) => ({ value: String(i + 1), label: `${i + 1} cols` }))
                    ]}
                    onValueChange={(val) => setResponsive(bp, val)}
                  />
                </div>
              </div>
            {/each}

            <div class="flex items-center justify-between gap-3 pt-2">
              <span class="font-semibold text-slate-700 dark:text-slate-300 text-xs">
                Hide below viewport
              </span>
              <div class="w-32">
                <Select
                  value={responsiveValues.hideBelow ?? ""}
                  options={[
                    { value: "", label: "never" },
                    ...((model.schema?.breakpoints ?? []).filter((b) => b !== "xs").map((bp) => ({ value: bp, label: bp.toUpperCase() })))
                  ]}
                  onValueChange={(val) => setResponsive("hideBelow", val)}
                />
              </div>
            </div>
          </div>
        </div>
      {/if}
    </fieldset>
  </div>
{/if}

<style>
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
