<script>
  import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    inset = false,
    ...restProps
  } = $props();
</script>

<DropdownMenuPrimitive.Item
  bind:ref
  data-slot="dropdown-menu-item"
  data-inset={inset}
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 focus:bg-slate-100 focus:text-slate-900 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:focus:bg-slate-800 dark:focus:text-slate-100 gap-2 rounded-lg px-2.5 py-1.5 text-xs data-disabled:opacity-50 data-disabled:pointer-events-none relative flex cursor-pointer items-center outline-hidden select-none transition-colors",
    inset && "pl-8",
    className
  )}
  {...restProps}
/>
