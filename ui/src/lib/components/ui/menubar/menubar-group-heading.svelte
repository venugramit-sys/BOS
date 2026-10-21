<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    inset,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.GroupHeading
  bind:ref
  data-slot="menubar-group-heading"
  data-inset={inset}
  class={cn(
    "text-slate-400 dark:text-slate-500 px-2 py-1 text-[10px] font-semibold uppercase tracking-wider",
    inset && "pl-8",
    className
  )}
  {...restProps}
/>
