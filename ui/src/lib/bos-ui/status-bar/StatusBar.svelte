<script>
  import { cn } from "$lib/utils.js";

  let {
    leading = [],
    trailing = [],
    class: className = ""
  } = $props();

  const tones = {
    neutral: "text-slate-500",
    info: "text-sky-600 dark:text-sky-400",
    success: "text-emerald-600 dark:text-emerald-400",
    warning: "text-amber-600 dark:text-amber-400",
    danger: "text-rose-600 dark:text-rose-400"
  };
</script>

<footer
  class={cn(
    "h-7 shrink-0 px-2 border-t border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900",
    "flex items-center justify-between gap-3 text-[11px]",
    className
  )}
>
  <div class="flex items-center gap-3 min-w-0">
    {#each leading as item (item.id)}
      {#if item.onClick}
        <button type="button" class={cn("cursor-pointer hover:underline", tones[item.tone] || tones.neutral)} onclick={() => item.onClick()}>
          {item.label}
        </button>
      {:else}
        <span class={tones[item.tone] || tones.neutral}>{item.label}</span>
      {/if}
    {/each}
  </div>
  <div class="flex items-center gap-3 shrink-0">
    {#each trailing as item (item.id)}
      {#if item.onClick}
        <button type="button" class={cn("cursor-pointer hover:underline", tones[item.tone] || tones.neutral)} onclick={() => item.onClick()}>
          {item.label}
        </button>
      {:else}
        <span class={tones[item.tone] || tones.neutral}>{item.label}</span>
      {/if}
    {/each}
  </div>
</footer>
