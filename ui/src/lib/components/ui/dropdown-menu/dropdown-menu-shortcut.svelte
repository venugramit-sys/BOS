<script>
  import { cn } from "$lib/utils.js";

  let { class: className, children, ...restProps } = $props();
</script>

<span
  class={cn("ml-auto text-[10px] font-mono tracking-widest text-slate-400 dark:text-slate-500", className)}
  {...restProps}
>{@render children?.()}</span>
