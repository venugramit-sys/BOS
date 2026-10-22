<script module>
  // Lookup results are shared by every editor of the same relation for a short time, so a list with
  // 50 rows asks the server once per search instead of once per row.
  const LOOKUP_TTL_MS = 30_000;
  const lookupCache = new Map();

  export function cachedLookup(key, load) {
    const hit = lookupCache.get(key);
    if (hit && Date.now() - hit.at < LOOKUP_TTL_MS) return hit.promise;
    const promise = load();
    lookupCache.set(key, { at: Date.now(), promise });
    promise.catch(() => lookupCache.delete(key));
    if (lookupCache.size > 200) lookupCache.delete(lookupCache.keys().next().value);
    return promise;
  }
</script>

<script>
  /**
   * Value editor / viewer of one field control, dispatched on the resolved `field.control`.
   * Used by `Field` (card layouts) and by `Repeater` cells (`row` set, `compact`).
   */
  import { Input } from "$lib/bos-ui/input/index.js";
  import { Select } from "$lib/bos-ui/select/index.js";
  import { Switch } from "$lib/bos-ui/switch/index.js";
  import { Checkbox } from "$lib/bos-ui/checkbox/index.js";
  import { Textarea } from "$lib/bos-ui/textarea/index.js";
  import { DatePicker } from "$lib/bos-ui/date-picker/index.js";
  import { Combobox } from "$lib/bos-ui/combobox/index.js";
  import { themeStore } from "$lib/bos-ui/theme/index.js";
  import { listRecords } from "./api.js";
  import { formatValue, toLocalDateTimeInput, fromLocalDateTimeInput } from "./format.js";
  import { resolveEmphasis } from "./style.js";

  let { p, c, row = null, compact = false, readonly = false, id = "", label = "" } = $props();

  const f = $derived(c.field);
  const value = $derived(p.valueOf(f, row));
  const editable = $derived(!readonly && p.editable(c, row));
  const error = $derived(row ? "" : (p.fieldErrors[f.key] ?? ""));
  const emphasis = $derived(resolveEmphasis(c.style, p.styleResult(c, row)));
  const text = $derived(formatValue(f, value));
  const inputClass = "bos-native-input";

  // Value at focus time: a blur only validates when the user changed something.
  let focusValue;
  function onFocusIn() {
    focusValue = p.valueOf(f, row);
  }
  function onFocusOut() {
    const now = p.valueOf(f, row);
    if (String(now ?? "") !== String(focusValue ?? "")) p.validate(c, row);
  }

  const write = (v) => p.setValue(c, v, row);
  const commit = (v) => p.commit(c, v, row);

  const selectOptions = $derived([
    ...(f.required ? [] : [{ value: "", label: " " }]),
    ...(f.options ?? []).map((o) => ({ value: o.value, label: o.caption }))
  ]);

  // ----- lookup (TableRelation) -----
  const lk = $derived(f.lookup);
  const lookupLabelKey = $derived(lk?.display_fields?.find((k) => k !== lk.sql_field) ?? lk?.sql_field);
  const lookupColumns = $derived((lk?.display_fields ?? []).map((k, i) => ({ key: k, label: k.replace(/_/g, " "), primary: i === 0, mono: i === 0 })));

  function fetchLookup(query) {
    const q = String(query ?? "").trim();
    return cachedLookup(`${lk.sql_table}|${lk.sql_field}|${lookupLabelKey}|${q.toLowerCase()}`, () => searchLookup(q));
  }

  async function searchLookup(q) {
    if (!q) return (await listRecords(lk.sql_table, { limit: 25, sort: `${lk.sql_field}:asc` }))?.records ?? [];
    // NAV lookups match the code or the description: query both and merge.
    const keys = [...new Set([lk.sql_field, lookupLabelKey])];
    const results = await Promise.all(keys.map((k) => listRecords(lk.sql_table, { limit: 25, filter: { [k]: q } }).catch(() => null)));
    const seen = new Set();
    const out = [];
    for (const r of results) {
      for (const rec of r?.records ?? []) {
        if (seen.has(rec[lk.sql_field])) continue;
        seen.add(rec[lk.sql_field]);
        out.push(rec);
      }
    }
    return out;
  }

  // ----- json -----
  let jsonText = $state("");
  let jsonError = $state("");
  $effect(() => {
    if (f.control !== "json") return;
    const v = value;
    jsonText = v === null || v === undefined ? "" : typeof v === "string" ? v : JSON.stringify(v, null, 2);
  });
  function commitJson() {
    if (!jsonText.trim()) {
      jsonError = "";
      commit(null);
      return;
    }
    try {
      const parsed = JSON.parse(jsonText);
      jsonError = "";
      commit(parsed);
    } catch (e) {
      jsonError = `Invalid JSON: ${e.message}`;
    }
  }

  // ----- image -----
  const MAX_IMAGE_BYTES = 2 * 1024 * 1024;
  let imageError = $state("");
  function safeImage(src) {
    const s = String(src ?? "");
    return /^(https?:\/\/|\/|data:image\/(png|jpe?g|gif|webp|svg\+xml);base64,)/i.test(s) ? s : "";
  }
  function pickImage(event) {
    const file = event.currentTarget.files?.[0];
    if (!file) return;
    if (!file.type.startsWith("image/")) {
      imageError = "Choose an image file";
      return;
    }
    if (file.size > MAX_IMAGE_BYTES) {
      imageError = "Images are limited to 2 MB";
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      imageError = "";
      commit(String(reader.result));
    };
    reader.readAsDataURL(file);
  }

  function linkHref() {
    const s = String(value ?? "");
    if (!s) return "";
    if (f.control === "email") return `mailto:${s}`;
    if (f.control === "phone") return `tel:${s.replace(/[^\d+]/g, "")}`;
    if (f.control === "url") return /^https?:\/\//i.test(s) ? s : "";
    return "";
  }

  function drillDown() {
    if (f.drill_down?.page) p.ctx.openPage?.(f.drill_down.page, { mode: "view" });
  }
</script>

{#if !editable}
  {#if f.control === "image" && safeImage(value)}
    <img src={safeImage(value)} alt={label || c.caption || c.name} class={compact ? "h-8 w-8 rounded object-cover" : "max-h-40 rounded-md border border-[var(--bos-color-border)] object-contain"} />
  {:else if f.control === "checkbox" || f.control === "switch"}
    <Checkbox checked={value === true || value === "true"} disabled />
  {:else if f.drill_down?.page && !p.design}
    <button type="button" class={"bos-value bos-link " + emphasis} onclick={drillDown} {id}>{text || "0"}</button>
  {:else if linkHref()}
    <a class={"bos-value bos-link " + emphasis} href={linkHref()} target={f.control === "url" ? "_blank" : undefined} rel="noopener noreferrer" {id}>{text}</a>
  {:else}
    <span class={["bos-value", compact ? "bos-value-compact" : "bos-value-box", f.control === "number" ? "bos-num" : "", emphasis].join(" ")} {id}>{text || (compact ? "" : " ")}</span>
  {/if}
{:else}
  <div class={"min-w-0 " + emphasis} onfocusin={onFocusIn} onfocusout={onFocusOut}>
    {#if f.control === "textArea"}
      <Textarea {id} bind:value={() => value ?? "", (v) => write(v)} rows={compact ? 1 : 3} maxlength={f.max_length} required={f.required} error={error} placeholder={f.placeholder ?? ""} />
    {:else if f.control === "number"}
      <Input {id} type="number" bind:value={() => value ?? "", (v) => write(v === "" ? null : v)} decimals={f.decimals?.[1] ?? 2} evalMath errorMessage={error} required={f.required} placeholder={f.placeholder ?? ""} size={compact ? "sm" : null} />
    {:else if f.control === "checkbox"}
      <Checkbox {id} checked={value === true || value === "true"} onCheckedChange={(v) => commit(Boolean(v))} />
    {:else if f.control === "switch"}
      <Switch {id} checked={value === true || value === "true"} onCheckedChange={(v) => commit(Boolean(v))} />
    {:else if f.control === "date"}
      <DatePicker value={value ?? ""} workDate={themeStore.workDate} errorMessage={error} required={f.required} density={compact ? "sm" : null} onChange={(v) => commit(v || null)} />
    {:else if f.control === "dateTime"}
      <input {id} type="datetime-local" class={inputClass} value={toLocalDateTimeInput(value)} onchange={(e) => commit(fromLocalDateTimeInput(e.currentTarget.value))} required={f.required} />
    {:else if f.control === "time"}
      <input {id} type="time" class={inputClass} value={value ?? ""} onchange={(e) => commit(e.currentTarget.value || null)} required={f.required} />
    {:else if f.control === "select"}
      <Select {id} value={value ?? ""} options={selectOptions} error={error} required={f.required} searchable={selectOptions.length > 8} onValueChange={(v) => commit(v === "" ? null : v)} />
    {:else if f.control === "lookup" && lk}
      <Combobox value={value ?? null} valueKey={lk.sql_field} labelKey={lookupLabelKey} columns={lookupColumns} fetchItems={fetchLookup} errorMessage={error} required={f.required} density={compact ? "sm" : null} showFactBox={!compact} onSelect={(item) => commit(item?.[lk.sql_field] ?? null)} onClear={() => commit(null)} />
    {:else if f.control === "color"}
      <div class="flex items-center gap-2">
        <input type="color" class="h-8 w-10 cursor-pointer rounded border border-[var(--bos-color-border)] bg-transparent" value={/^#[0-9a-f]{6}$/i.test(value ?? "") ? value : "#000000"} oninput={(e) => write(e.currentTarget.value)} onchange={(e) => commit(e.currentTarget.value)} aria-label={label || c.caption} />
        <input {id} class={inputClass + " font-mono"} value={value ?? ""} maxlength="7" oninput={(e) => write(e.currentTarget.value)} />
      </div>
    {:else if f.control === "json"}
      <div onfocusout={commitJson}>
        <Textarea {id} bind:value={jsonText} mono rows={compact ? 1 : 6} error={jsonError || error} />
      </div>
    {:else if f.control === "image"}
      <div class="flex flex-wrap items-center gap-3">
        {#if safeImage(value)}
          <img src={safeImage(value)} alt={label || c.caption || c.name} class="max-h-32 rounded-md border border-[var(--bos-color-border)] object-contain" />
        {/if}
        <input {id} type="file" accept="image/*" class="text-xs" onchange={pickImage} />
        {#if value}
          <button type="button" class="text-xs text-[var(--bos-color-unfavorable)] hover:underline" onclick={() => commit(null)}>Remove</button>
        {/if}
      </div>
      {#if imageError}<p class="mt-1 text-xs text-[var(--bos-color-unfavorable)]">{imageError}</p>{/if}
    {:else}
      <Input
        {id}
        type={f.control === "email" ? "email" : f.control === "phone" ? "tel" : f.control === "url" ? "url" : f.control === "masked" ? "password" : "text"}
        bind:value={() => value ?? "", (v) => write(v)}
        maxlength={f.max_length}
        errorMessage={error}
        required={f.required}
        placeholder={f.placeholder ?? ""}
        size={compact ? "sm" : null}
      />
    {/if}
  </div>
{/if}
