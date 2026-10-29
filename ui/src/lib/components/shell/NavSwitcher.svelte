<script>
  import { shell } from "./shellStore.svelte.js";
  import { Building2, ChevronsUpDown, Check } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { collapsed = false } = $props();

  let isOpen = $state(false);

  let hasMultipleCompanies = $derived(
    Array.isArray(shell.companies) && shell.companies.length > 1
  );

  let activeCompanyName = $derived(shell.companyDisplayName);

  function handleSelectCompany(comp) {
    const name = comp.name || comp.code;
    shell.activeCompany = name;
    isOpen = false;
    try {
      const env = JSON.parse(localStorage.getItem("bos_environment") || "{}");
      env.companyName = name;
      localStorage.setItem("bos_environment", JSON.stringify(env));
    } catch {}
  }

  function closeDropdown(e) {
    if (!e.target.closest(".nav-switcher-container")) {
      isOpen = false;
    }
  }
</script>

<svelte:window onclick={closeDropdown} />

<div class="relative nav-switcher-container px-3 py-2 border-b border-slate-200/80 dark:border-slate-800/80 shrink-0 select-none">
  {#if hasMultipleCompanies}
    <!-- Interactive Company Selector (Only when > 1 company exists) -->
    <button
      type="button"
      onclick={() => (isOpen = !isOpen)}
      class={cn(
        "w-full flex items-center gap-2.5 p-2 rounded-lg text-left transition-colors cursor-pointer",
        "hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-900 dark:text-slate-100",
        isOpen && "bg-slate-200/80 dark:bg-slate-800/80"
      )}
      title="Switch company"
      aria-label="Switch company"
    >
      <div class="size-8 rounded-lg bg-indigo-600 dark:bg-indigo-500 text-white flex items-center justify-center font-bold text-xs shrink-0 shadow-xs">
        <Building2 class="size-4 text-white" />
      </div>

      {#if !collapsed}
        <div class="flex-1 min-w-0 text-left">
          <div class="text-xs font-semibold truncate leading-tight">
            {activeCompanyName}
          </div>
          <div class="text-[10px] text-slate-500 dark:text-slate-400 truncate leading-tight mt-0.5">
            Enterprise Workspace
          </div>
        </div>

        <ChevronsUpDown class="size-3.5 text-slate-400 shrink-0 ml-auto" />
      {/if}
    </button>

    <!-- Dropdown Popover Menu (Only for switching between multiple companies) -->
    {#if isOpen}
      <div
        role="menu"
        tabindex="-1"
        class="absolute left-3 top-full mt-1.5 w-60 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-xl p-1.5 z-50 text-xs animate-in fade-in-0 zoom-in-95"
      >
        <div class="px-2 py-1 text-[10px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider">
          Companies
        </div>

        <div class="max-h-48 overflow-y-auto space-y-0.5 my-1">
          {#each shell.companies as comp}
            {@const compName = shell.formatCompanyName(comp.name || comp.code)}
            {@const isSelected = activeCompanyName === compName}
            <button
              type="button"
              onclick={() => handleSelectCompany(comp)}
              class={cn(
                "w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-left transition-colors cursor-pointer",
                isSelected
                  ? "bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-medium"
                  : "text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800"
              )}
            >
              <div class="truncate">
                <div class="font-medium truncate">{compName}</div>
                {#if comp.code && comp.name && comp.code !== comp.name}
                  <div class="text-[10px] text-slate-400">{comp.code}</div>
                {/if}
              </div>
              {#if isSelected}
                <Check class="size-3.5 text-indigo-600 dark:text-indigo-400 shrink-0 ml-2" />
              {/if}
            </button>
          {/each}
        </div>
      </div>
    {/if}
  {:else}
    <!-- Static Header: Company selector does NOT appear if there's only 1 company -->
    <div
      class="w-full flex items-center gap-2.5 p-2 rounded-lg text-left text-slate-900 dark:text-slate-100"
      title={activeCompanyName}
    >
      <div class="size-8 rounded-lg bg-indigo-600 dark:bg-indigo-500 text-white flex items-center justify-center font-bold text-xs shrink-0 shadow-xs">
        <Building2 class="size-4 text-white" />
      </div>

      {#if !collapsed}
        <div class="flex-1 min-w-0 text-left">
          <div class="text-xs font-semibold truncate leading-tight">
            {activeCompanyName}
          </div>
          <div class="text-[10px] text-slate-500 dark:text-slate-400 truncate leading-tight mt-0.5">
            Enterprise Workspace
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
