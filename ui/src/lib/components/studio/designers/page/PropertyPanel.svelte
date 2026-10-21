<script>
  /**
   * Property sheet of the selected page, control or action, generated from the server's property
   * schema. Values filled automatically from the table (NAV-style) are shown as "auto" with their
   * source; setting a value overrides it and "reset" returns to the automatic value.
   * Customize mode lists only what super users may change.
   */
  import { RotateCcw } from "lucide-svelte";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";
  import StyleEditor from "./StyleEditor.svelte";
  import { toBosValue, fromBosValue, lastSegment } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  let tab = $state("properties");
  let fieldError = $state("");

  const path = $derived(model.selected);
  const isPage = $derived(path === "page");
  const isAction = $derived(Boolean(path?.startsWith("action:")));
  const compiled = $derived(isPage ? null : isAction ? model.selectedAction : model.selectedCompiled);
  const node = $derived(model.mode === "develop" ? (isPage ? model.object : model.selectedNode) : null);
  const kind = $derived(isPage ? "page" : isAction ? "action" : (compiled?.kind ?? node?.kind ?? ""));
  const customize = $derived(model.mode === "customize");
  const authored = $derived(model.mode === "develop" && path ? model.isAuthored(path) : true);

  const customChange = $derived.by(() => {
    if (!customize || !path) return null;
    if (isPage) return model.changes.page ?? {};
    if (isAction) return model.changes.actions.find((a) => a.name === path.slice(7)) ?? {};
    return model.changes.controls.find((c) => c.name === lastSegment(path).name) ?? {};
  });

  const ACTION_PROPS = [
    { name: "caption", type: "text", customize: true },
    { name: "visible", type: "bool", customize: true },
    { name: "tooltip", type: "text" },
    { name: "icon", type: "icon" },
    { name: "confirm", type: "text" },
    { name: "primary", type: "bool" },
    { name: "openPage", type: "page" },
    { name: "runCodeunit", type: "text" },
    { name: "enableWhen", type: "expr" },
    { name: "showWhen", type: "expr" }
  ];

  const rows = $derived.by(() => {
    const schema = model.schema;
    if (!schema || !path) return [];
    const list = isPage ? schema.page_properties : isAction ? ACTION_PROPS : schema.control_properties;
    return list.filter((r) => (!r.kinds || r.kinds.includes(kind)) && (!customize || r.customize));
  });

  function enumOptions(type) {
    const spec = type.slice("enum:".length);
    if (spec.includes(",")) return spec.split(",");
    if (spec === "page_types") return (model.schema?.page_types ?? []).map((t) => t.name);
    if (spec === "field_controls") return model.schema?.field_controls ?? [];
    return model.schema?.[spec] ?? [];
  }

  const pageNames = $derived(Object.keys(pageManifest.manifest?.pages ?? {}).sort());

  /** Authored (develop) or customized (customize) value of a property. */
  function ownValue(name, type) {
    if (customize) return customChange?.[name] ?? null;
    const prop = node?.props?.find((p) => p.name === name);
    return prop ? fromBosValue(type, prop.value) : null;
  }

  /** Effective compiled value, for the "auto" hint. */
  function effective(name) {
    if (isPage) return model.compiled?.[name === "caption" ? "caption" : name] ?? null;
    if (!compiled) return null;
    const f = compiled.field;
    switch (name) {
      case "caption":
        return compiled.caption ?? null;
      case "tooltip":
        return compiled.tooltip ?? null;
      case "visible":
        return compiled.visible ?? null;
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
  const STYLE_TYPES = { background: "color", textColor: "color", borderColor: "color", borderWidth: "int", icon: "text" };
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
  <p class="p-3 text-xs text-slate-400">Select a control on the canvas or in the outline.</p>
{:else}
  <div class="flex flex-col gap-3 text-xs">
    <div>
      <div class="font-mono text-[10px] text-slate-400">{kind}</div>
      <div class="truncate text-sm font-semibold">{isPage ? (model.compiled?.caption ?? model.name) : (compiled?.caption ?? lastSegment(path).name)}</div>
      {#if compiled?.field}
        <div class="mt-0.5 text-[11px] text-slate-500">
          {compiled.field.binding === "variable" ? "Page variable" : compiled.field.binding === "flowField" ? "FlowField" : "Field"}
          <span class="font-mono">{compiled.field.source}</span> · {compiled.field.data_type}
        </div>
      {/if}
      {#if !authored}
        <p class="mt-1 rounded bg-amber-50 px-2 py-1 text-[11px] text-amber-800 dark:bg-amber-950 dark:text-amber-300">Defined by a page extension. Edit it there, or change it in Customize mode.</p>
      {/if}
    </div>

    <div class="flex gap-1 border-b border-slate-200 dark:border-slate-800" role="tablist">
      {#each [["properties", "Properties"], ["style", "Style"], ...(isPage || isAction ? [] : [["layout", "Layout"]])] as [id, label] (id)}
        <button type="button" role="tab" aria-selected={tab === id} class={"-mb-px border-b-2 px-2 py-1 font-semibold " + (tab === id ? "border-indigo-600 text-indigo-600" : "border-transparent text-slate-500")} onclick={() => (tab = id)}>{label}</button>
      {/each}
    </div>

    {#if fieldError}<p class="text-[11px] text-rose-600" role="alert">{fieldError}</p>{/if}

    <fieldset disabled={!authored || model.busy} class="contents">
      {#if tab === "properties"}
        <div class="flex flex-col gap-2">
          {#each rows as row (row.name)}
            {@const own = ownValue(row.name, row.type)}
            {@const eff = effective(row.name)}
            {@const auto = autoSource(row.name)}
            <label class="flex flex-col gap-0.5">
              <span class="flex items-center gap-1 font-medium text-slate-600 dark:text-slate-300">
                {row.name}
                {#if own === null && auto}<span class="rounded bg-emerald-50 px-1 text-[9px] font-semibold text-emerald-700 dark:bg-emerald-950 dark:text-emerald-300" title={`Filled automatically from ${auto}`}>auto · {auto}</span>{/if}
                {#if own !== null}
                  <button type="button" class="ml-auto text-slate-400 hover:text-slate-700" aria-label={`Reset ${row.name}`} onclick={(e) => { e.preventDefault(); set(row, null); }}><RotateCcw class="size-3" /></button>
                {/if}
              </span>
              {#if row.type === "bool"}
                <select class="rounded border border-slate-200 bg-transparent px-1.5 py-1 dark:border-slate-700" value={own === null ? "" : String(own)} onchange={(e) => set(row, e.currentTarget.value === "" ? null : e.currentTarget.value === "true")}>
                  <option value="">{eff === null ? "—" : `auto (${eff ? "Yes" : "No"})`}</option>
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              {:else if row.type.startsWith("enum:")}
                <select class="rounded border border-slate-200 bg-transparent px-1.5 py-1 dark:border-slate-700" value={own ?? ""} onchange={(e) => set(row, e.currentTarget.value || null)}>
                  <option value="">{eff === null ? "—" : `auto (${eff})`}</option>
                  {#each enumOptions(row.type) as opt (opt)}<option value={opt}>{opt}</option>{/each}
                </select>
              {:else if row.type === "page"}
                <select class="rounded border border-slate-200 bg-transparent px-1.5 py-1 dark:border-slate-700" value={own ?? ""} onchange={(e) => set(row, e.currentTarget.value || null)}>
                  <option value="">{eff === null ? "—" : `auto (${eff})`}</option>
                  {#each pageNames as n (n)}<option value={n}>{n}</option>{/each}
                </select>
              {:else if row.type === "icon"}
                <input class="rounded border border-slate-200 bg-transparent px-1.5 py-1 dark:border-slate-700" value={own ?? ""} placeholder={eff ?? ""} onchange={(e) => set(row, e.currentTarget.value.trim() || null)} />
              {:else}
                <input
                  class={"rounded border border-slate-200 bg-transparent px-1.5 py-1 dark:border-slate-700 " + (row.type === "expr" ? "font-mono" : "")}
                  type={row.type === "int" ? "number" : "text"}
                  value={own ?? ""}
                  placeholder={eff === null ? "" : String(eff)}
                  onchange={(e) => set(row, e.currentTarget.value.trim() || null)}
                />
              {/if}
            </label>
          {/each}
          {#if !customize && compiled?.triggers?.length}
            <div class="text-[11px] text-slate-500">Triggers: <span class="font-mono">{compiled.triggers.join(", ")}</span> (edit in code)</div>
          {/if}
        </div>
      {:else if tab === "style"}
        <StyleEditor style={styleValues} inherited={isPage ? {} : (compiled?.style ?? {})} schema={model.schema} onSet={setStyle} />
      {:else if tab === "layout"}
        <div class="flex flex-col gap-2">
          <p class="text-[11px] text-slate-500">Columns out of 12 per screen size (empty = automatic).</p>
          {#each model.schema?.breakpoints ?? [] as bp (bp)}
            <label class="flex items-center justify-between gap-2">
              <span class="font-medium text-slate-600 dark:text-slate-300">{bp}</span>
              <select class="rounded border border-slate-200 bg-transparent px-1 py-0.5 dark:border-slate-700" value={spans?.[bp] ?? ""} onchange={(e) => setResponsive(bp, e.currentTarget.value)}>
                <option value="">auto</option>
                {#each Array.from({ length: 12 }, (_, i) => i + 1) as n (n)}<option value={n}>{n}</option>{/each}
              </select>
            </label>
          {/each}
          <label class="flex items-center justify-between gap-2">
            <span class="font-medium text-slate-600 dark:text-slate-300">Hide below</span>
            <select class="rounded border border-slate-200 bg-transparent px-1 py-0.5 dark:border-slate-700" value={responsiveValues.hideBelow ?? ""} onchange={(e) => setResponsive("hideBelow", e.currentTarget.value)}>
              <option value="">never</option>
              {#each (model.schema?.breakpoints ?? []).filter((b) => b !== "xs") as bp (bp)}<option value={bp}>{bp}</option>{/each}
            </select>
          </label>
        </div>
      {/if}
    </fieldset>
  </div>
{/if}
