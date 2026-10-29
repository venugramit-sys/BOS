<script>
  /**
   * NewObjectModal.svelte
   * Create dialog built on bos-ui Dialog, Input, and Button.
   */
  import { Table, Cpu, Workflow, Layout, ListFilter, GitBranch, Layers, Check } from "lucide-svelte";
  import { Dialog, Button, Input, Banner } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  let objectName = $state("");
  let selectedType = $state("table");
  let targetObject = $state("");
  let packageName = $state("bos.app");
  let errorMessage = $state("");

  const objectTypes = [
    { id: "table", name: "Table", desc: "Schema, keys, and FlowFields.", icon: Table },
    { id: "codeunit", name: "Codeunit", desc: "Posting routines and events.", icon: Cpu },
    { id: "statemachine", name: "State Machine", desc: "Document lifecycle.", icon: Workflow },
    { id: "page", name: "Page", desc: "Card, list, or document UI.", icon: Layout },
    { id: "pageextension", name: "Page Extension", desc: "Extend page fields, groups & actions.", icon: Layers },
    { id: "tableextension", name: "Table Extension", desc: "Extend table fields & keys.", icon: Layers },
    { id: "query", name: "Query", desc: "Joined projection.", icon: ListFilter },
    { id: "enum", name: "Enum", desc: "Option set.", icon: GitBranch },
    { id: "enumextension", name: "Enum Extension", desc: "Add values to extensible enum.", icon: Layers },
  ];

  let isExtension = $derived(
    selectedType === "pageextension" ||
    selectedType === "tableextension" ||
    selectedType === "enumextension"
  );

  let targetType = $derived(
    selectedType === "pageextension" ? "page" :
    selectedType === "tableextension" ? "table" :
    selectedType === "enumextension" ? "enum" : null
  );

  let availableTargets = $derived(
    targetType
      ? (studioStore.catalog || []).filter((o) => o.object_type === targetType).map((o) => o.name || o.object_name)
      : []
  );

  $effect(() => {
    if (isExtension && availableTargets.length > 0 && !targetObject) {
      targetObject = availableTargets[0];
    }
  });

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
    if (isExtension && !targetObject.trim()) {
      errorMessage = `Please select an existing ${targetType} to extend.`;
      return;
    }
    errorMessage = "";
    studioStore.openObject(
      objectName.trim(),
      "design",
      selectedType,
      packageName.trim() || "bos.app",
      targetObject.trim() || null
    );
    objectName = "";
    targetObject = "";
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
      <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
        {#each objectTypes as type}
          {@const Icon = type.icon}
        <Button
          variant={selectedType === type.id ? "secondary" : "ghost"}
          class="h-auto w-full whitespace-normal justify-start p-2.5 text-left"
          onclick={() => {
            selectedType = type.id;
            targetObject = "";
          }}
        >
          <span class="flex w-full items-center gap-1.5 text-xs font-bold">
            <Icon class="size-3.5 text-indigo-500 shrink-0" />
            <span class="truncate">{type.name}</span>
            {#if selectedType === type.id}<Check class="size-3.5 ml-auto text-indigo-600 shrink-0" />{/if}
          </span>
          <span class="block text-[10px] font-normal text-slate-400 mt-1 line-clamp-1">{type.desc}</span>
        </Button>
        {/each}
      </div>

      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <Input label="Object name" bind:value={objectName} placeholder="Customer" required />
        <Input label="Package" type="code" bind:value={packageName} placeholder="bos.sales" />
      </div>

      {#if isExtension}
        <div class="space-y-1">
          <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
            Target {targetType ? targetType.charAt(0).toUpperCase() + targetType.slice(1) : "Object"} to Extend <span class="text-rose-500">*</span>
          </label>
          {#if availableTargets.length > 0}
            <select
              bind:value={targetObject}
              class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500 font-mono"
            >
              {#each availableTargets as t}
                <option value={t}>{t}</option>
              {/each}
            </select>
          {:else}
            <input
              type="text"
              bind:value={targetObject}
              placeholder={`e.g. Existing${targetType ? targetType.charAt(0).toUpperCase() + targetType.slice(1) : "Object"}`}
              class="w-full text-xs font-mono px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            />
          {/if}
          <p class="text-[10px] text-slate-400">Specifies the base {targetType} that this extension enhances without invasive changes.</p>
        </div>
      {/if}

      {#if errorMessage}
        <Banner tone="danger" title="Validation notice">{errorMessage}</Banner>
      {/if}
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" onclick={() => (studioStore.isNewObjectModalOpen = false)}>Cancel</Button>
    <Button onclick={handleCreate}>Create & Open</Button>
  {/snippet}
</Dialog>
