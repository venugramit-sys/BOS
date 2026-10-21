<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import MenubarPortal from "./menubar-portal.svelte";

  let {
    ref = $bindable(null),
    class: className,
    align = "start",
    alignOffset = -3,
    sideOffset = 2,
    portalProps,
    ...restProps
  } = $props();
</script>

<MenubarPortal {...portalProps}>
  <MenubarPrimitive.SubContent
    bind:ref
    data-slot="menubar-sub-content"
    {align}
    {alignOffset}
    {sideOffset}
    class={cn(
      "bg-white text-slate-800 border border-slate-200 shadow-lg dark:bg-slate-900 dark:text-slate-100 dark:border-slate-800 min-w-[11rem] rounded-md p-1 duration-100 z-50 origin-(--bits-menubar-content-transform-origin) overflow-hidden focus:outline-hidden",
      className
    )}
    {...restProps}
  />
</MenubarPortal>
