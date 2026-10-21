<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.Trigger
  bind:ref
  data-slot="menubar-trigger"
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 aria-expanded:bg-slate-100 aria-expanded:text-slate-900 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:aria-expanded:bg-slate-800 dark:aria-expanded:text-slate-100 rounded-sm px-2.5 py-1 text-xs font-medium flex items-center outline-hidden cursor-pointer select-none transition-colors",
    className
  )}
  {...restProps}
/>
