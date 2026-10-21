<script>
  import { DropdownMenu as DropdownMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import DropdownMenuPortal from "./dropdown-menu-portal.svelte";

  let {
    ref = $bindable(null),
    class: className,
    sideOffset = 4,
    align = "end",
    alignOffset = 0,
    portalProps,
    ...restProps
  } = $props();
</script>

<DropdownMenuPortal {...portalProps}>
  <DropdownMenuPrimitive.Content
    bind:ref
    data-slot="dropdown-menu-content"
    {sideOffset}
    {align}
    {alignOffset}
    class={cn(
      "bg-white text-slate-800 border border-slate-200/90 shadow-lg dark:bg-slate-900 dark:text-slate-100 dark:border-slate-800 min-w-[14rem] rounded-xl p-1.5 duration-100 z-50 overflow-hidden focus:outline-hidden animate-in fade-in-50 zoom-in-95",
      className
    )}
    {...restProps}
  />
</DropdownMenuPortal>
