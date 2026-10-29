<script>
  /**
   * ExpressionRuleBuilder.svelte
   * Declarative Visual Rule & Expression Builder for `showWhen` and `enableWhen` conditions.
   * Empowers developers to build multi-criteria visibility and editability logic without manual syntax bugs.
   */
  import { Plus, Trash2, Sparkles, Check, Code2 } from "lucide-svelte";
  import { Dialog, Button } from "$lib/bos-ui/index.js";

  let {
    open = $bindable(false),
    title = "Conditional Rule Builder",
    ruleType = "showWhen", // "showWhen" | "enableWhen"
    sourceFields = [],
    initialExpression = "",
    onApply = null
  } = $props();

  let clauses = $state([
    { field: "", operator: "==", value: "", conjunction: "&&" }
  ]);
  let manualMode = $state(false);
  let rawExpression = $state("");

  const OPERATORS = [
    { id: "==", label: "Equals (==)" },
    { id: "!=", label: "Does not equal (!=)" },
    { id: ">", label: "Greater than (>)" },
    { id: "<", label: "Less than (<)" },
    { id: ">=", label: "Greater or equal (>=)" },
    { id: "<=", label: "Less or equal (<=)" },
    { id: "is_true", label: "Is True" },
    { id: "is_false", label: "Is False" },
    { id: "not_empty", label: "Has Value (Not Empty)" },
    { id: "is_empty", label: "Is Empty / Nil" }
  ];

  $effect(() => {
    if (open) {
      rawExpression = initialExpression || "";
      if (initialExpression && !initialExpression.includes("&&") && !initialExpression.includes("||")) {
        // Simple single clause parse attempt
        const m = initialExpression.match(/(?:rec\.)?([a-zA-Z0-9_]+)\s*(==|!=|>|<|>=|<=)\s*(.+)/);
        if (m) {
          clauses = [{
            field: m[1],
            operator: m[2],
            value: m[3].replace(/^["']|["']$/g, ""),
            conjunction: "&&"
          }];
          manualMode = false;
          return;
        }
      }
      if (!initialExpression) {
        const defaultField = sourceFields[0]?.name ?? "status";
        clauses = [{ field: defaultField, operator: "==", value: "", conjunction: "&&" }];
        manualMode = false;
      } else {
        manualMode = true;
      }
    }
  });

  const generatedExpression = $derived.by(() => {
    if (manualMode) return rawExpression;
    const parts = clauses.filter((c) => c.field.trim()).map((c, i) => {
      const fieldRef = `rec.${c.field.trim()}`;
      let clauseExpr = "";
      if (c.operator === "is_true") clauseExpr = `${fieldRef} === true`;
      else if (c.operator === "is_false") clauseExpr = `${fieldRef} === false`;
      else if (c.operator === "not_empty") clauseExpr = `Boolean(${fieldRef})`;
      else if (c.operator === "is_empty") clauseExpr = `!${fieldRef}`;
      else {
        // String vs Number detection
        const rawVal = c.value.trim();
        const valFormatted = !isNaN(Number(rawVal)) && rawVal !== "" ? rawVal : (rawVal === "true" || rawVal === "false") ? rawVal : `"${rawVal.replace(/"/g, '\\"')}"`;
        clauseExpr = `${fieldRef} ${c.operator} ${valFormatted}`;
      }
      return i > 0 ? `${clauses[i - 1].conjunction} ${clauseExpr}` : clauseExpr;
    });
    return parts.join(" ");
  });

  function addClause() {
    const defaultField = sourceFields[0]?.name ?? "status";
    clauses = [...clauses, { field: defaultField, operator: "==", value: "", conjunction: "&&" }];
  }

  function removeClause(index) {
    if (clauses.length <= 1) return;
    clauses = clauses.filter((_, i) => i !== index);
  }

  function handleCommit() {
    const expr = generatedExpression.trim();
    if (onApply) onApply(expr);
    open = false;
  }
</script>

<Dialog
  bind:open
  title={`${title} (${ruleType})`}
  description="Build declarative multi-criteria evaluation rules for field and action visibility or editability."
  size="md"
>
  {#snippet children()}
    <div class="space-y-4 text-xs">
      <div class="flex items-center justify-between border-b border-slate-200 pb-2 dark:border-slate-800">
        <span class="font-bold text-[11px] text-slate-700 dark:text-slate-300 uppercase tracking-wide">
          Rule Criteria Mode
        </span>
        <div class="inline-flex rounded-md bg-slate-100 p-0.5 dark:bg-slate-800 text-[11px]">
          <button
            type="button"
            class={"rounded px-2.5 py-0.5 font-semibold transition-colors " +
              (!manualMode ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-900 dark:text-indigo-400" : "text-slate-500")}
            onclick={() => (manualMode = false)}
          >
            Visual Builder
          </button>
          <button
            type="button"
            class={"rounded px-2.5 py-0.5 font-semibold transition-colors " +
              (manualMode ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-900 dark:text-indigo-400" : "text-slate-500")}
            onclick={() => { manualMode = true; rawExpression = generatedExpression; }}
          >
            Raw Expression
          </button>
        </div>
      </div>

      {#if !manualMode}
        <div class="space-y-2">
          {#each clauses as c, i}
            <div class="space-y-1 rounded-xl border border-slate-200/80 bg-slate-50/70 p-2.5 dark:border-slate-800 dark:bg-slate-900/50">
              {#if i > 0}
                <div class="flex items-center gap-2 pb-1">
                  <span class="font-bold text-[10px] uppercase text-slate-400">Combine with:</span>
                  <select
                    class="rounded border border-slate-300 bg-white px-2 py-0.5 text-xs font-semibold text-indigo-600 dark:border-slate-700 dark:bg-slate-800 dark:text-indigo-400"
                    bind:value={clauses[i - 1].conjunction}
                  >
                    <option value="&&">AND (&&)</option>
                    <option value="||">OR (||)</option>
                  </select>
                </div>
              {/if}

              <div class="flex items-center gap-1.5">
                <!-- Field Picker -->
                <select
                  class="min-w-0 flex-1 rounded border border-slate-300 bg-white px-2 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800 font-mono"
                  bind:value={c.field}
                >
                  <option value="">(Select Field)</option>
                  {#each sourceFields as f}
                    <option value={f.name}>{f.name} ({f.type || "string"})</option>
                  {/each}
                </select>

                <!-- Operator -->
                <select
                  class="rounded border border-slate-300 bg-white px-2 py-1.5 text-xs font-semibold text-slate-700 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 shrink-0"
                  bind:value={c.operator}
                >
                  {#each OPERATORS as op}
                    <option value={op.id}>{op.label}</option>
                  {/each}
                </select>

                <!-- Value Input (if applicable) -->
                {#if !["is_true", "is_false", "not_empty", "is_empty"].includes(c.operator)}
                  <input
                    type="text"
                    placeholder='Value (e.g. "Open", 100)'
                    class="min-w-0 flex-1 rounded border border-slate-300 bg-white px-2 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800 font-mono"
                    bind:value={c.value}
                  />
                {/if}

                {#if clauses.length > 1}
                  <button
                    type="button"
                    class="p-1.5 rounded text-slate-400 hover:text-rose-600 hover:bg-rose-50 dark:hover:bg-rose-950 shrink-0"
                    title="Remove clause"
                    onclick={() => removeClause(i)}
                  >
                    <Trash2 class="size-3.5" />
                  </button>
                {/if}
              </div>
            </div>
          {/each}

          <Button size="xs" variant="outline" class="w-full gap-1 border-dashed" onclick={addClause}>
            <Plus class="size-3" /> Add Condition Clause
          </Button>
        </div>
      {:else}
        <div>
          <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
            Raw JavaScript / Client Expression
          </label>
          <textarea
            rows="3"
            class="w-full rounded border border-slate-300 bg-slate-50 p-2 font-mono text-xs dark:border-slate-700 dark:bg-slate-900 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            bind:value={rawExpression}
            placeholder='rec.status === "Open" && rec.amount > 0'
          ></textarea>
        </div>
      {/if}

      <!-- Generated Expression Live Preview -->
      <div class="rounded-xl border border-indigo-200/80 bg-indigo-50/50 p-2.5 dark:border-indigo-900/50 dark:bg-indigo-950/40 space-y-1">
        <span class="font-bold text-[10px] uppercase tracking-wider text-indigo-700 dark:text-indigo-300 flex items-center gap-1">
          <Code2 class="size-3" /> Compiled Expression Preview
        </span>
        <code class="block font-mono text-[11px] text-slate-800 dark:text-slate-200 break-all bg-white dark:bg-slate-900 px-2 py-1 rounded border border-indigo-100 dark:border-indigo-900">
          {generatedExpression || "(always true)"}
        </code>
      </div>

      <!-- Action Footer -->
      <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-200 dark:border-slate-800">
        <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
        <Button variant="primary" onclick={handleCommit}>
          <Check class="size-3.5 mr-1" /> Apply Rule
        </Button>
      </div>
    </div>
  {/snippet}
</Dialog>
