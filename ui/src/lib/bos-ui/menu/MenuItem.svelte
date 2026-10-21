<script>
  import {
    Item as MenubarItem,
    Separator as MenubarSeparator,
    Shortcut as MenubarShortcut,
    Sub as MenubarSub,
    SubTrigger as MenubarSubTrigger,
    SubContent as MenubarSubContent,
    CheckboxItem as MenubarCheckboxItem,
    RadioGroup as MenubarRadioGroup,
    RadioItem as MenubarRadioItem,
    Group as MenubarGroup,
    GroupHeading as MenubarGroupHeading
  } from "$lib/components/ui/menubar/index.js";
  import MenuItem from "./MenuItem.svelte";
  import { cn } from "$lib/utils.js";
  import { resolveSurface } from "../theme/index.js";

  let {
    item,
    onSelect,
    itemClass = "",
    shortcutClass = "",
    itemSnippet
  } = $props();

  let surface = $derived(resolveSurface("menu"));

  function handleClick() {
    if (item.disabled) return;
    if (item.action) {
      item.action();
    }
    if (item.href) {
      if (item.target === "_blank") {
        window.open(item.href, "_blank");
      } else {
        window.location.href = item.href;
      }
    }
    if (onSelect) {
      onSelect(item);
    }
  }

  function getBadgeClasses(variant) {
    switch (variant) {
      case "accent":
        return "bg-blue-100 text-blue-700 dark:bg-blue-900/60 dark:text-blue-300";
      case "warning":
        return "bg-amber-100 text-amber-700 dark:bg-amber-900/60 dark:text-amber-300";
      case "destructive":
        return "bg-red-100 text-red-700 dark:bg-red-900/60 dark:text-red-300";
      default:
        return "bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-300";
    }
  }
</script>

{#if !item}
  <!-- Empty item safeguard -->
{:else if item.type === "separator"}
  <MenubarSeparator />
{:else if item.type === "group"}
  <MenubarGroup>
    {#if item.heading}
      <MenubarGroupHeading>{item.heading}</MenubarGroupHeading>
    {/if}
    {#each item.items || [] as childItem}
      <MenuItem
        item={childItem}
        {onSelect}
        {itemClass}
        {shortcutClass}
        {itemSnippet}
      />
    {/each}
  </MenubarGroup>
{:else if item.type === "submenu"}
  <MenubarSub disabled={item.disabled}>
    <MenubarSubTrigger
      class={cn(
        "gap-2",
        surface.textClass,
        itemClass
      )}
    >
      {#if item.icon}
        {@const IconComponent = item.icon}
        <IconComponent class="size-4 shrink-0 opacity-70" />
      {/if}
      <span class="truncate">{item.label}</span>
      {#if item.badge}
        <span class={cn("ml-1.5 px-1.5 py-0.2 rounded text-[10px] font-medium leading-normal", getBadgeClasses(item.badgeVariant))}>
          {item.badge}
        </span>
      {/if}
    </MenubarSubTrigger>
    <MenubarSubContent class={cn("min-w-[12rem]", surface.panelClass)}>
      {#each item.items || [] as subChild}
        <MenuItem
          item={subChild}
          {onSelect}
          {itemClass}
          {shortcutClass}
          {itemSnippet}
        />
      {/each}
    </MenubarSubContent>
  </MenubarSub>
{:else if item.type === "checkbox"}
  <MenubarCheckboxItem
    bind:checked={item.checked}
    disabled={item.disabled}
    class={cn("gap-2", surface.textClass, itemClass)}
    onCheckedChange={(checked) => {
      if (item.onCheckedChange) item.onCheckedChange(checked);
      if (onSelect) onSelect({ ...item, checked });
    }}
  >
    {#if item.icon}
      {@const IconComponent = item.icon}
      <IconComponent class="size-4 shrink-0 opacity-70" />
    {/if}
    <span class="truncate">{item.label}</span>
    {#if item.shortcut}
      <MenubarShortcut class={shortcutClass}>{item.shortcut}</MenubarShortcut>
    {/if}
  </MenubarCheckboxItem>
{:else if item.type === "radio-group"}
  <MenubarRadioGroup
    bind:value={item.value}
    onValueChange={(val) => {
      if (item.onValueChange) item.onValueChange(val);
      if (onSelect) onSelect({ ...item, value: val });
    }}
  >
    {#each item.options || [] as opt}
      <MenubarRadioItem
        value={opt.value}
        disabled={opt.disabled}
        class={cn("gap-2", surface.textClass, itemClass)}
      >
        <span class="truncate">{opt.label}</span>
      </MenubarRadioItem>
    {/each}
  </MenubarRadioGroup>
{:else}
  <!-- Standard Action Item -->
  <MenubarItem
    disabled={item.disabled}
    onclick={handleClick}
    class={cn(
      "flex items-center gap-2",
      surface.textClass,
      item.destructive && "text-red-600 focus:text-red-600 focus:bg-red-50 dark:text-red-400 dark:focus:bg-red-950/40",
      itemClass
    )}
  >
    {#if itemSnippet}
      {@render itemSnippet({ item })}
    {:else}
      {#if item.icon}
        {@const IconComponent = item.icon}
        <IconComponent class={cn("size-4 shrink-0", item.destructive ? "text-red-500 opacity-90" : "opacity-70")} />
      {/if}
      <span class="truncate">{item.label}</span>

      {#if item.badge}
        <span class={cn("ml-1 px-1.5 py-0.2 rounded text-[10px] font-medium leading-normal", getBadgeClasses(item.badgeVariant))}>
          {item.badge}
        </span>
      {/if}

      {#if item.shortcut}
        <MenubarShortcut class={shortcutClass}>{item.shortcut}</MenubarShortcut>
      {/if}
    {/if}
  </MenubarItem>
{/if}
