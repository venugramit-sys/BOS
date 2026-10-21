<script>
  import { Menubar as MenubarPrimitive } from "bits-ui";
  import { Check } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    class: className,
    checked = $bindable(false),
    children: childrenProp,
    closeOnSelect = false,
    ...restProps
  } = $props();
</script>

<MenubarPrimitive.CheckboxItem
  bind:ref
  bind:checked
  {closeOnSelect}
  data-slot="menubar-checkbox-item"
  class={cn(
    "hover:bg-slate-100 hover:text-slate-900 focus:bg-slate-100 focus:text-slate-900 dark:hover:bg-slate-800 dark:hover:text-slate-100 dark:focus:bg-slate-800 dark:focus:text-slate-100 gap-2 rounded-sm py-1.5 pr-2 pl-7 text-xs relative flex cursor-pointer items-center outline-hidden select-none transition-colors",
    className
  )}
  {...restProps}
>
  {#snippet children({ checked })}
    <span class="left-2 size-3.5 absolute flex items-center justify-center pointer-events-none">
      {#if checked}
        <Check class="size-3 stroke-[2.5]" />
      {/if}
    </span>
    {@render childrenProp?.({ checked })}
  {/snippet}
</MenubarPrimitive.CheckboxItem>
