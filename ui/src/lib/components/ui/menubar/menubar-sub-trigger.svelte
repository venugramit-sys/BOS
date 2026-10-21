<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { ChevronRight } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    inset,
    children,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.SubTrigger
  bind:ref
  data-slot="menubar-sub-trigger"
  data-inset={inset}
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 focus:bg-slate-100 focus:text-slate-900 data-state-open:bg-slate-100 data-state-open:text-slate-900 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:focus:bg-slate-800 dark:focus:text-slate-100 dark:data-state-open:bg-slate-800 dark:data-state-open:text-slate-100 gap-2 rounded-sm px-2 py-1.5 text-xs flex cursor-pointer items-center outline-hidden select-none transition-colors",
    inset && "pl-8",
    className
  )}
  {...restProps}
>
  {@render children?.()}
  <ChevronRight class="ml-auto size-3.5 opacity-60" />
</MenubarPrimitive.SubTrigger>
