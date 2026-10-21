<script>
  import { shell } from "./shellStore.svelte.js";
  import { X, LayoutDashboard, FileText } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
</script>

{#if shell.openTabs.length > 0}
  <div class="h-9 bg-slate-100/90 dark:bg-slate-950/70 border-b border-slate-200 dark:border-slate-800 flex items-center px-2 gap-1 overflow-x-auto no-scrollbar shrink-0 select-none">
    {#each shell.openTabs as tab (tab.id)}
      <div
        role="button"
        tabindex="0"
        onclick={() => (shell.activeTabId = tab.id)}
        onkeydown={(e) => e.key === "Enter" && (shell.activeTabId = tab.id)}
        class={cn(
          "group h-7 px-2.5 rounded-t-md text-xs font-medium flex items-center gap-1.5 cursor-pointer border-t-2 border-transparent transition-all max-w-[200px] shrink-0",
          shell.activeTabId === tab.id
            ? "bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 border-indigo-600 dark:border-indigo-500 shadow-xs"
            : "text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-900/40"
        )}
      >
        <!-- Tab Icon / Type -->
        {#if tab.type === "dashboard"}
          <LayoutDashboard class="size-3.5 opacity-70 shrink-0" />
        {:else}
          <FileText class="size-3.5 opacity-70 shrink-0" />
        {/if}

        <span class="truncate">{tab.title}</span>

        <!-- Unsaved Changes Dirty State Indicator -->
        {#if tab.isDirty}
          <span class="size-1.5 rounded-full bg-amber-500 shrink-0" title="Unsaved changes"></span>
        {/if}

        <!-- Close Button -->
        {#if tab.closable !== false}
          <button
            onclick={(e) => {
              e.stopPropagation();
              shell.closeTab(tab.id);
            }}
            class="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 opacity-60 group-hover:opacity-100 transition-opacity ml-0.5 cursor-pointer"
            title="Close tab"
            aria-label="Close Tab"
          >
            <X class="size-3" />
          </button>
        {/if}
      </div>
    {/each}
  </div>
{/if}
