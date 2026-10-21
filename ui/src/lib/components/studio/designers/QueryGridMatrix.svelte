<script>
  /**
   * QueryGridMatrix.svelte (Svelte 5 Runes)
   * Microsoft Access Iconic QBE (Query By Example) Grid Matrix
   * Rows: Field, Table, Alias, Total/Aggregate, Sort, Show, Criteria, Or.
   */
  import { Plus, Trash2 } from "lucide-svelte";
  import { Button, Input, Select, Checkbox } from "$lib/bos-ui/index.js";

  let {
    columns = $bindable([]),
    defaultTable = "",
    onColumnChange = null,
  } = $props();

  const aggregateOptions = [
    "Group By",
    "Sum",
    "Count",
    "Avg",
    "Min",
    "Max",
    "Expression",
    "Where",
  ];

  const sortOptions = ["None", "Ascending", "Descending"];

  function handleAddColumn() {
    const fallbackTbl = defaultTable || columns[0]?.table || "";
    const newCol = {
      id: Math.random().toString(36).substring(2, 9),
      field: "id",
      table: fallbackTbl,
      alias: "",
      total: "Group By",
      sort: "None",
      show: true,
      criteria: "",
      orCriteria: "",
    };
    columns = [...columns, newCol];
    if (onColumnChange) onColumnChange(columns);
  }

  function handleDeleteColumn(index) {
    columns = columns.filter((_, i) => i !== index);
    if (onColumnChange) onColumnChange(columns);
  }
</script>

<div class="flex-1 flex flex-col min-h-0 bg-white dark:bg-slate-950 overflow-auto">
  <table class="w-full text-left border-collapse text-xs">
    <!-- Header Labels -->
    <thead class="sticky top-0 bg-slate-100 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 font-semibold select-none z-10">
      <tr>
        <th class="py-1.5 px-3 w-28 bg-slate-200/80 dark:bg-slate-800/80 border-r border-slate-300 dark:border-slate-700">
          QBE Property
        </th>
        {#each columns as col, idx}
          <th class="py-1.5 px-3 min-w-44 border-r border-slate-200 dark:border-slate-800">
            <div class="flex items-center justify-between">
              <span>Col {idx + 1}</span>
              <Button variant="ghost" size="xs" iconOnly icon={Trash2} aria-label="Remove column" onclick={() => handleDeleteColumn(idx)} />
            </div>
          </th>
        {/each}
        <th class="py-1.5 px-3 w-16">
          <Button size="xs" variant="secondary" icon={Plus} onclick={handleAddColumn}>Add</Button>
        </th>
      </tr>
    </thead>
    <tbody class="divide-y divide-slate-100 dark:divide-slate-900 font-mono text-[11px]">
      <!-- Row 1: Field -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Field:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Input size="sm" bind:value={col.field} />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 2: Table -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Table:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Input size="sm" bind:value={col.table} />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 3: Alias / Caption -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Alias:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900 font-sans">
            <Input size="sm" bind:value={col.alias} placeholder="Output caption" />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 4: Total / Aggregate -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30 bg-indigo-50/10 dark:bg-indigo-950/10">
        <td class="py-1 px-3 font-semibold text-indigo-600 dark:text-indigo-400 bg-indigo-50/40 dark:bg-indigo-950/30 border-r border-slate-200 dark:border-slate-800">
          Total (Aggregate):
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Select bind:value={col.total} options={aggregateOptions.map((opt) => ({ value: opt, label: opt }))} />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 5: Sort -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Sort:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Select bind:value={col.sort} options={sortOptions.map((option) => ({ value: option, label: option }))} />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 6: Show -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Show:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900 text-center">
            <Checkbox bind:checked={col.show} />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 7: Criteria -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Criteria:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Input size="sm" bind:value={col.criteria} placeholder="e.g. > 1000" />
          </td>
        {/each}
        <td></td>
      </tr>

      <!-- Row 8: Or -->
      <tr class="hover:bg-slate-50/50 dark:hover:bg-slate-900/30">
        <td class="py-1 px-3 font-semibold text-slate-700 dark:text-slate-300 bg-slate-50 dark:bg-slate-900/50 border-r border-slate-200 dark:border-slate-800">
          Or:
        </td>
        {#each columns as col}
          <td class="py-1 px-2 border-r border-slate-100 dark:border-slate-900">
            <Input size="sm" bind:value={col.orCriteria} placeholder="alternative filter" />
          </td>
        {/each}
        <td></td>
      </tr>
    </tbody>
  </table>
</div>
