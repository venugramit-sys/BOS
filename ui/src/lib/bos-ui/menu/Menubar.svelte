<script>
  import {
    Root as MenubarRoot,
    Menu as MenubarMenu,
    Trigger as MenubarTrigger,
    Content as MenubarContent
  } from "$lib/components/ui/menubar/index.js";
  import MenuItem from "./MenuItem.svelte";
  import { cn } from "$lib/utils.js";
  import { resolveSurface } from "../theme/index.js";

  let {
    menus = [],
    class: className = "",
    menubarClass = "",
    triggerClass = "",
    contentClass = "",
    itemClass = "",
    shortcutClass = "",
    density = null,
    variant = null,
    rounded = null,
    onSelect,
    triggerSnippet,
    itemSnippet,
    ...restProps
  } = $props();

  let surface = $derived(resolveSurface("menu", { density, variant, rounded }));
</script>

<MenubarRoot
  class={cn(
    "gap-1 transition-colors",
    surface.menuClass,
    menubarClass,
    className
  )}
  {...restProps}
>
  {#each menus as menu (menu.id || menu.label)}
    <MenubarMenu>
      <MenubarTrigger
        disabled={menu.disabled}
        class={cn(
          "gap-1.5 px-2.5 font-medium text-slate-700 dark:text-slate-200 transition-colors",
          surface.textClass,
          triggerClass,
          menu.triggerClass
        )}
      >
        {#if triggerSnippet}
          {@render triggerSnippet({ menu })}
        {:else}
          {#if menu.icon}
            {@const IconComponent = menu.icon}
            <IconComponent class="size-3.5 shrink-0 opacity-70" />
          {/if}
          <span>{menu.label}</span>
        {/if}
      </MenubarTrigger>

      <MenubarContent
        align={menu.align || "start"}
        class={cn(
          "min-w-[13rem]",
          surface.panelClass,
          contentClass,
          menu.contentClass
        )}
      >
        {#each menu.items || [] as item}
          <MenuItem
            {item}
            {onSelect}
            {itemClass}
            {shortcutClass}
            {itemSnippet}
          />
        {/each}
      </MenubarContent>
    </MenubarMenu>
  {/each}
</MenubarRoot>
