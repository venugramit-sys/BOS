<script>
  /**
   * NewObjectModal.svelte
   * Create dialog built on bos-ui Dialog, Input, and Button.
   */
  import { Table, Cpu, Workflow, Layout, ListFilter, GitBranch, Check } from "lucide-svelte";
  import { Dialog, Button, Input, Banner } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  let objectName = $state("");
  let selectedType = $state("table");
  let packageName = $state("bos.app");
  let errorMessage = $state("");

  const objectTypes = [
    { id: "table", name: "Table", desc: "Schema, keys, and FlowFields.", icon: Table },
    { id: "codeunit", name: "Codeunit", desc: "Posting routines and events.", icon: Cpu },
    { id: "statemachine", name: "State Machine", desc: "Document lifecycle.", icon: Workflow },
    { id: "page", name: "Page", desc: "Card, list, or document.", icon: Layout },
    { id: "query", name: "Query", desc: "Joined projection.", icon: ListFilter },
    { id: "enum", name: "Enum", desc: "Option set.", icon: GitBranch },
  ];

  function validatePascalCase(name) {
    if (!name.trim()) return "Object name is required.";
    if (!/^[A-Z][a-zA-Z0-9]*$/.test(name.trim())) {
      return "Object name must be singular PascalCase (e.g. Customer, SalesHeader).";
    }
    if (/^[A-Z]{2,3}[A-Z]/.test(name.trim())) {
      return "Avoid module acronym prefixes such as SecUser. The package name is the namespace.";
    }
    return "";
  }

  function handleCreate() {
    const error = validatePascalCase(objectName);
    if (error) {
      errorMessage = error;
      return;
    }
    errorMessage = "";
    studioStore.openObject(
      objectName.trim(),
      "design",
      selectedType,
      packageName.trim() || "bos.app"
    );
    objectName = "";
    studioStore.isNewObjectModalOpen = false;
  }
</script>

<Dialog
  bind:open={studioStore.isNewObjectModalOpen}
  title="Create meta-object"
  description="Use a singular PascalCase name. The package supplies the namespace."
  size="md"
>
  {#snippet children()}
    <div class="space-y-4">
      <div class="grid grid-cols-2 gap-2">
        {#each objectTypes as type}
          {@const Icon = type.icon}
        <Button
          variant={selectedType === type.id ? "secondary" : "ghost"}
          class="h-auto w-full whitespace-normal justify-start p-3 text-left"
          onclick={() => (selectedType = type.id)}
        >
          <span class="flex w-full items-center gap-2 text-xs font-bold">
            <Icon class="size-4 text-indigo-500" />
            {type.name}
            {#if selectedType === type.id}<Check class="size-3.5 ml-auto text-indigo-600" />{/if}
          </span>
          <span class="block text-[11px] font-normal text-slate-400 mt-1">{type.desc}</span>
        </Button>
        {/each}
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <Input label="Object name" bind:value={objectName} placeholder="Customer" required />
        <Input label="Package" type="code" bind:value={packageName} placeholder="bos.sales" />
      </div>
      {#if errorMessage}
        <Banner tone="danger" title="Name is not valid">{errorMessage}</Banner>
      {/if}
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (studioStore.isNewObjectModalOpen = false)}>Cancel</Button>
    <Button onclick={handleCreate}>Create & Open</Button>
  {/snippet}
</Dialog>
