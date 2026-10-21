<script>
  import { cn } from "$lib/utils.js";

  let {
    title = "",
    description = "",
    icon = null,
    class: className = "",
    children
  } = $props();
</script>

<div class={cn("flex flex-col items-center justify-center text-center gap-2 px-6 py-10", className)}>
  {#if icon}
    {@const Icon = icon}
    <div class="size-12 rounded-xl border border-indigo-200 bg-indigo-50 text-indigo-600 flex items-center justify-center dark:border-indigo-900 dark:bg-indigo-950/40 dark:text-indigo-300">
      <Icon class="size-6 stroke-[1.5]" />
    </div>
  {/if}
  {#if title}
    <h2 class="text-base font-semibold text-slate-900 dark:text-slate-100">{title}</h2>
  {/if}
  {#if description}
    <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm leading-relaxed">{description}</p>
  {/if}
  {#if children}
    <div class="flex items-center gap-2 pt-1">
      {@render children()}
    </div>
  {/if}
</div>
