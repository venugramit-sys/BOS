<script>
  /**
   * PageLinkMapper.svelte
   * Enterprise 2-Column Key-Value Mapper for Page Navigation Links (`runPageLink`).
   * Binds target page filter fields to source record fields or constants without manual syntax.
   */
  import { Plus, Trash2, ArrowRight, Link, Check } from "lucide-svelte";
  import { Button } from "$lib/bos-ui/index.js";

  let {
    value = "",
    sourceFields = [],
    targetPage = "",
    onChange = null
  } = $props();

  let rows = $state([]);

  // Parse initial value into rows
  $effect(() => {
    rows = parseLinkString(value);
  });

  function parseLinkString(str) {
    if (!str || typeof str !== "string") return [];
    const items = str.split(",").map((s) => s.trim()).filter(Boolean);
    const parsed = [];
    for (const item of items) {
      // Matches: "TargetField" = FIELD(SourceField) or TargetField = CONST(Val) or TargetField = rec.field
      const mField = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*FIELD\(([^)]+)\)/i);
      const mConst = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*CONST\(([^)]+)\)/i);
      const mFilter = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*FILTER\(([^)]+)\)/i);
      const mDirect = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*(?:rec\.)?([^;\s]+)/i);

      if (mField) {
        parsed.push({ targetField: mField[1], type: "FIELD", sourceValue: mField[2] });
      } else if (mConst) {
        parsed.push({ targetField: mConst[1], type: "CONST", sourceValue: mConst[2] });
      } else if (mFilter) {
        parsed.push({ targetField: mFilter[1], type: "FILTER", sourceValue: mFilter[2] });
      } else if (mDirect) {
        parsed.push({ targetField: mDirect[1], type: "FIELD", sourceValue: mDirect[2] });
      } else {
        parsed.push({ targetField: item, type: "FIELD", sourceValue: "" });
      }
    }
    return parsed;
  }

  function serializeRows(rowsList) {
    const valid = rowsList.filter((r) => r.targetField.trim() && r.sourceValue.trim());
    if (valid.length === 0) return "";
    return valid
      .map((r) => {
        const tf = r.targetField.trim();
        const sv = r.sourceValue.trim();
        if (r.type === "CONST") return `"${tf}" = CONST(${sv})`;
        if (r.type === "FILTER") return `"${tf}" = FILTER(${sv})`;
        return `"${tf}" = FIELD(${sv})`;
      })
      .join(", ");
  }

  function emitChange() {
    const serialized = serializeRows(rows);
    if (onChange) onChange(serialized);
  }

  function addRow() {
    const defaultSource = sourceFields[0]?.name ?? "no";
    rows = [...rows, { targetField: "", type: "FIELD", sourceValue: defaultSource }];
    emitChange();
  }

  function removeRow(index) {
    rows = rows.filter((_, i) => i !== index);
    emitChange();
  }

  function updateRow(index, patch) {
    rows = rows.map((r, i) => (i === index ? { ...r, ...patch } : r));
    emitChange();
  }
</script>

<div class="space-y-2 rounded-xl border border-slate-200/80 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/40 text-xs">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-1.5 font-bold text-slate-700 dark:text-slate-300 text-[11px] uppercase tracking-wider">
      <Link class="size-3 text-indigo-500" />
      <span>Navigation Page Links (runPageLink)</span>
    </div>
    <Button size="xs" variant="ghost" class="h-5 px-1.5 text-[10px] gap-1 text-indigo-600 dark:text-indigo-400" onclick={addRow}>
      <Plus class="size-2.5" /> Add Link
    </Button>
  </div>

  {#if rows.length === 0}
    <p class="text-[10px] text-slate-400 italic py-1">
      No field links configured. The target page {targetPage ? `(${targetPage})` : ""} will open without record filters.
    </p>
  {:else}
    <div class="space-y-1.5 pt-1">
      {#each rows as r, i}
        <div class="flex items-center gap-1.5 bg-white dark:bg-slate-800 p-1.5 rounded-lg border border-slate-200 dark:border-slate-700 shadow-2xs">
          <!-- Target Field Name -->
          <input
            type="text"
            placeholder="Target Field"
            class="min-w-0 flex-1 rounded border border-slate-200 bg-slate-50 px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-900 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
            value={r.targetField}
            oninput={(e) => updateRow(i, { targetField: e.target.value })}
          />

          <!-- Link Type Dropdown -->
          <select
            class="rounded border border-slate-200 bg-white px-1.5 py-1 text-[11px] font-semibold text-slate-600 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 shrink-0"
            value={r.type}
            onchange={(e) => updateRow(i, { type: e.target.value })}
          >
            <option value="FIELD">FIELD</option>
            <option value="CONST">CONST</option>
            <option value="FILTER">FILTER</option>
          </select>

          <ArrowRight class="size-3 text-slate-400 shrink-0" />

          <!-- Source Value or Field -->
          {#if r.type === "FIELD" && sourceFields.length > 0}
            <select
              class="min-w-0 flex-1 rounded border border-slate-200 bg-slate-50 px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-900 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
              value={r.sourceValue}
              onchange={(e) => updateRow(i, { sourceValue: e.target.value })}
            >
              <option value="">(Select Source Field)</option>
              {#each sourceFields as sf}
                <option value={sf.name}>{sf.name} ({sf.type || "field"})</option>
              {/each}
            </select>
          {:else}
            <input
              type="text"
              placeholder={r.type === "CONST" ? '"Invoice", 10' : 'Source Field'}
              class="min-w-0 flex-1 rounded border border-slate-200 bg-slate-50 px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-900 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
              value={r.sourceValue}
              oninput={(e) => updateRow(i, { sourceValue: e.target.value })}
            />
          {/if}

          <!-- Delete Row -->
          <button
            type="button"
            class="p-1 rounded text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-950 shrink-0"
            title="Remove criterion"
            onclick={() => removeRow(i)}
          >
            <Trash2 class="size-3" />
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>
