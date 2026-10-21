<script>
  import { cn } from "$lib/utils.js";
  import { shortcutParts } from "../hotkey.js";

  let {
    keys = "",
    class: className = ""
  } = $props();

  let parts = $derived(shortcutParts(keys));
</script>

{#if parts.length > 0}
  <kbd class={cn("inline-flex items-center gap-0.5 font-mono text-[10px] text-slate-500 dark:text-slate-400", className)}>
    {#each parts as part, index (part + index)}
      {#if index > 0}<span class="opacity-60">+</span>{/if}
      <span class="px-1 py-0.5 rounded border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900 leading-none">
        {part}
      </span>
    {/each}
  </kbd>
{/if}
