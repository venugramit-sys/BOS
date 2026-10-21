<script>
  /**
   * Visual style editor: theme palette swatches + validated hex picker, border, radius, shadow,
   * density, emphasis and icon. Only tokens and #rrggbb values exist, so no raw CSS can be entered.
   */
  import { X } from "lucide-svelte";
  import IconPicker from "./IconPicker.svelte";

  /** @type {{ style: Record<string, any>, schema: any, inherited?: Record<string, any>, onSet: (name: string, value: any) => void }} */
  let { style = {}, schema, inherited = {}, onSet } = $props();

  const HEX = /^#[0-9a-f]{6}$/i;
  const COLOR_PROPS = [
    { name: "background", label: "Background" },
    { name: "textColor", label: "Text" },
    { name: "borderColor", label: "Border" }
  ];
  const tokens = $derived(schema?.color_tokens ?? []);
  let hexDraft = $state({});

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
</script>

<div class="flex flex-col gap-3 text-xs">
  {#each COLOR_PROPS as cp (cp.name)}
    {@const current = value(cp.name)}
    <div class="flex flex-col gap-1">
      <div class="flex items-center justify-between">
        <span class="font-medium text-slate-600 dark:text-slate-300">{cp.label}</span>
        {#if current}
          <button type="button" class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-slate-700" onclick={() => onSet(cp.name, null)}><X class="size-3" />reset</button>
        {:else if inherited?.[cp.name]}
          <span class="text-[10px] text-slate-400">inherits {inherited[cp.name]}</span>
        {/if}
      </div>
      <div class="flex flex-wrap gap-1" role="radiogroup" aria-label={cp.label}>
        {#each tokens as t (t)}
          <button
            type="button"
            role="radio"
            aria-checked={current === t}
            title={t}
            class={"size-5 rounded border " + (current === t ? "ring-2 ring-indigo-500 ring-offset-1" : "border-slate-300 dark:border-slate-600")}
            style:background={swatch(t)}
            onclick={() => onSet(cp.name, t)}
          ><span class="sr-only">{t}</span></button>
        {/each}
      </div>
      <div class="flex items-center gap-1">
        <input
          type="color"
          class="h-6 w-8 cursor-pointer rounded border border-slate-300 bg-transparent dark:border-slate-600"
          value={current && HEX.test(current) ? current : "#000000"}
          onchange={(e) => onSet(cp.name, e.currentTarget.value.toLowerCase())}
          aria-label={`${cp.label} colour`}
        />
        <input
          class="w-24 rounded border border-slate-200 bg-transparent px-1.5 py-0.5 font-mono dark:border-slate-700"
          placeholder="#rrggbb"
          maxlength="7"
          value={hexDraft[cp.name] ?? (current && HEX.test(current) ? current : "")}
          oninput={(e) => (hexDraft = { ...hexDraft, [cp.name]: e.currentTarget.value })}
          onblur={() => commitHex(cp.name)}
          onkeydown={(e) => e.key === "Enter" && commitHex(cp.name)}
          aria-label={`${cp.label} hex value`}
        />
        {#if hexDraft[cp.name] && !HEX.test(hexDraft[cp.name])}<span class="text-[10px] text-rose-600">#rrggbb</span>{/if}
      </div>
    </div>
  {/each}

  <label class="flex items-center justify-between gap-2">
    <span class="font-medium text-slate-600 dark:text-slate-300">Border width</span>
    <select class="rounded border border-slate-200 bg-transparent px-1 py-0.5 dark:border-slate-700" value={value("borderWidth") ?? ""} onchange={(e) => onSet("borderWidth", e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}>
      <option value="">—</option>
      {#each Array.from({ length: (schema?.max_border_width ?? 4) + 1 }, (_, i) => i) as w (w)}<option value={w}>{w}px</option>{/each}
    </select>
  </label>

  {#each [["radius", "Corners"], ["shadow", "Shadow"], ["density", "Padding"], ["emphasis", "Emphasis"]] as [name, label] (name)}
    <label class="flex items-center justify-between gap-2">
      <span class="font-medium text-slate-600 dark:text-slate-300">{label}</span>
      <select class="max-w-[9rem] rounded border border-slate-200 bg-transparent px-1 py-0.5 dark:border-slate-700" value={value(name) ?? ""} onchange={(e) => onSet(name, e.currentTarget.value || null)}>
        <option value="">—</option>
        {#each schema?.[name] ?? [] as opt (opt)}<option value={opt}>{opt}</option>{/each}
      </select>
    </label>
  {/each}

  <div class="flex flex-col gap-1">
    <span class="font-medium text-slate-600 dark:text-slate-300">Icon</span>
    <IconPicker value={value("icon")} onChange={(v) => onSet("icon", v)} />
  </div>
</div>
