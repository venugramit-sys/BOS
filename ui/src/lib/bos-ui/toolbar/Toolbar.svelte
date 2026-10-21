<script>
  import { cn } from "$lib/utils.js";

  let {
    label = "Toolbar",
    class: className = "",
    children
  } = $props();
</script>

<div
  role="toolbar"
  aria-label={label}
  class={cn(
    "h-10 shrink-0 px-2 border-b border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-900/60",
    "flex items-center gap-1",
    className
  )}
>
  {@render children?.()}
</div>
