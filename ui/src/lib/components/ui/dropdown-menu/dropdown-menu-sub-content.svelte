<script>
  import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    sideOffset = 4,
    ...restProps
  } = $props();
</script>

<DropdownMenuPrimitive.SubContent
  bind:ref
  data-slot="dropdown-menu-sub-content"
  {sideOffset}
  class={cn(
    "bg-white text-slate-800 border border-slate-200/90 shadow-lg dark:bg-slate-900 dark:text-slate-100 dark:border-slate-800 min-w-[12rem] rounded-xl p-1.5 duration-100 z-50 overflow-hidden focus:outline-hidden animate-in fade-in-50 zoom-in-95",
    className
  )}
  {...restProps}
/>
