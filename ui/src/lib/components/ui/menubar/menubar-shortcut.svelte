<script>
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  } = $props();
</script>

<span
  bind:this={ref}
  data-slot="menubar-shortcut"
  class={cn(
    "text-slate-400 dark:text-slate-500 text-[10px] font-mono tracking-widest ml-auto pl-3 select-none",
    className
  )}
  {...restProps}
>
  {@render children?.()}
</span>
