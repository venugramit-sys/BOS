<script>
  import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
  import { ChevronRight } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    inset = false,
    children,
    ...restProps
  } = $props();
</script>

<DropdownMenuPrimitive.SubTrigger
  bind:ref
  data-slot="dropdown-menu-sub-trigger"
  data-inset={inset}
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 data-[state=open]:bg-slate-100 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:data-[state=open]:bg-slate-800 gap-2 rounded-lg px-2.5 py-1.5 text-xs flex cursor-pointer select-none items-center outline-hidden transition-colors",
    inset && "pl-8",
    className
  )}
  {...restProps}
>
  {@render children?.()}
  <ChevronRight class="ml-auto size-3.5 text-slate-400" />
</DropdownMenuPrimitive.SubTrigger>
