<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.Root
  bind:ref
  data-slot="menubar"
  class={cn(
    "h-9 gap-1 rounded-md border border-slate-200 bg-white p-1 shadow-xs flex items-center dark:border-slate-800 dark:bg-slate-900",
    className
  )}
  {...restProps}
/>
