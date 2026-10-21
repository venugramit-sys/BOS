<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    value,
    children: childrenProp,
    closeOnSelect = false,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.RadioItem
  bind:ref
  {value}
  {closeOnSelect}
  data-slot="menubar-radio-item"
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 focus:bg-slate-100 focus:text-slate-900 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:focus:bg-slate-800 dark:focus:text-slate-100 gap-2 rounded-sm py-1.5 pr-2 pl-7 text-xs relative flex cursor-pointer items-center outline-hidden select-none transition-colors",
    className
  )}
  {...restProps}
>
  {#snippet children({ checked })}
    <span class="left-2.5 size-2 rounded-full bg-blue-600 dark:bg-blue-400 absolute opacity-0 transition-opacity pointer-events-none" class:opacity-100={checked}>
    </span>
    {@render childrenProp?.({ checked })}
  {/snippet}
</MenubarPrimitive.RadioItem>
