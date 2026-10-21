<script>
  /**
   * FlowFieldWizardModal.svelte (Svelte 5 Runes)
   * Microsoft Dynamics NAV / Business Central & Access FlowField Calculation Wizard
   * Declaratively configures virtual subquery aggregations (Sum, Count, Avg, Min, Max, Exist).
   */
  import { Dialog, Button, Input, Select } from "$lib/bos-ui/index.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  let {
    isOpen = $bindable(false),
    field = $bindable(null),
    onSave = null,
  } = $props();

  let method = $state("sum"); // 'sum' | 'count' | 'avg' | 'min' | 'max' | 'exist'
  let targetTable = $state("");
  let targetField = $state("");
  let filterPredicate = $state("");

  $effect(() => {
    if (field && field.flowField) {
      method = field.flowField.method || "sum";
      targetTable = field.flowField.targetTable || "";
      targetField = field.flowField.targetField || "";
      filterPredicate = field.flowField.filter || "";
    }
  });

  // Available tables from catalog for foreign lookup
  let availableTables = $derived(
    studioStore.catalog.filter((o) => o.object_type === "table")
  );

  function handleApplyFlowField() {
    if (!field || !targetTable) return;
    const filter = filterPredicate.trim() || "1 == 1";
    const target = targetField ? `${targetTable}.${targetField}` : targetTable;
    field.flowField = {
      method,
      targetTable,
      targetField,
      filter: filterPredicate,
    };
    field.computed = `${method}(${target} where { ${filter} })`;
    field.dataType = "Decimal(18, 2)";
    if (onSave) onSave(field);
    isOpen = false;
  }
</script>

<Dialog
  bind:open={isOpen}
  title="FlowField calculation"
  description="Virtual subquery aggregation. Nothing is stored on the table."
  size="md"
>
  {#snippet children()}
    <div class="space-y-4">
      <div class="grid grid-cols-3 gap-2">
        {#each ["sum", "count", "avg", "min", "max", "exist"] as m}
          <Button
            variant={method === m ? "primary" : "secondary"}
            size="xs"
            onclick={() => (method = m)}
          >
            {m}
          </Button>
        {/each}
      </div>
      <Select
        label="Target table"
        bind:value={targetTable}
        placeholder="Select a table"
        options={availableTables.map((table) => ({ value: table.name, label: table.name }))}
      />
      <Input label="Target field" type="code" bind:value={targetField} placeholder="lineAmount" />
      <Input label="Filter" type="code" bind:value={filterPredicate} placeholder="customerId == this.id" />
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (isOpen = false)}>Cancel</Button>
    <Button onclick={handleApplyFlowField}>Apply FlowField</Button>
  {/snippet}
</Dialog>

