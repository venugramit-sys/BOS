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

<DropdownMenuPrimitive.GroupHeading
  bind:ref
  data-slot="dropdown-menu-group-heading"
  data-inset={inset}
  class={cn("px-2.5 py-1 text-[10px] font-semibold tracking-wider uppercase text-slate-400 dark:text-slate-500", inset && "pl-8", className)}
  {...restProps}
/>
