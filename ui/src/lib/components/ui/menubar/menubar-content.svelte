<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import MenubarPortal from "./menubar-portal.svelte";

  let {
    ref = $bindable(null),
    class: className,
    align = "start",
    alignOffset = -4,
    sideOffset = 4,
    portalProps,
    ...restProps
  } = $props();
</script>

<MenubarPortal {...portalProps}>
  <MenubarPrimitive.Content
    bind:ref
    data-slot="menubar-content"
    {align}
    {alignOffset}
    {sideOffset}
    class={cn(
      "bg-white text-slate-800 border border-slate-200 shadow-md dark:bg-slate-900 dark:text-slate-100 dark:border-slate-800 min-w-[12rem] rounded-md p-1 duration-100 z-50 origin-(--bits-menubar-content-transform-origin) overflow-hidden focus:outline-hidden",
      className
    )}
    {...restProps}
  />
</MenubarPortal>
