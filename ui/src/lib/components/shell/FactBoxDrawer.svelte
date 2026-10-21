<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    PanelRightClose,
    PanelRight,
    Info
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { factboxSnippet } = $props();
</script>

{#if shell.isFactBoxOpen}
  <aside
    class="w-72 hidden lg:flex flex-col border-l border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shrink-0 select-none transition-all duration-200"
  >
    <div class="h-9 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between px-3">
      <span class="text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
        Context FactBox
      </span>
      <button
        onclick={() => shell.toggleFactBox()}
        class="p-1 rounded text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
        title="Collapse FactBox"
        aria-label="Collapse FactBox"
      >
        <PanelRightClose class="size-4" />
      </button>
    </div>

    <div class="flex-1 overflow-y-auto p-4 text-xs">
      {#if factboxSnippet}
        {@render factboxSnippet()}
      {:else}
        <div class="h-full flex flex-col items-center justify-center text-center p-4 text-slate-400 space-y-2">
          <Info class="size-6 opacity-40" />
          <p class="text-xs">No active record selected for contextual insights.</p>
        </div>
      {/if}
    </div>
  </aside>
{/if}
