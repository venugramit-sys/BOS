<script>
  import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import Kbd from "../kbd/Kbd.svelte";

  let {
    items = [],
    children
  } = $props();
</script>

<ContextMenuPrimitive.Root>
  <ContextMenuPrimitive.Trigger class="contents">
    {@render children?.()}
  </ContextMenuPrimitive.Trigger>
  <ContextMenuPrimitive.Portal>
    <ContextMenuPrimitive.Content
      class="z-50 min-w-44 rounded-md border border-slate-200 bg-white p-1 shadow-md dark:border-slate-700 dark:bg-slate-900"
    >
      {#each items as item, index (`${item.type || "item"}-${item.label || index}`)}
        {#if item.type === "separator"}
          <ContextMenuPrimitive.Separator class="my-1 h-px bg-slate-200 dark:bg-slate-800" />
        {:else}
          <ContextMenuPrimitive.Item
            disabled={item.disabled}
            onSelect={() => item.action?.()}
            class={cn(
              "flex cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-xs outline-none",
              "data-highlighted:bg-indigo-50 data-highlighted:text-indigo-700 dark:data-highlighted:bg-indigo-950/50 dark:data-highlighted:text-indigo-200",
              "data-disabled:pointer-events-none data-disabled:opacity-40",
              item.destructive ? "text-rose-600 dark:text-rose-400" : "text-slate-700 dark:text-slate-200"
            )}
          >
            {#if item.icon}
              {@const Icon = item.icon}
              <Icon class="size-3.5 shrink-0" />
            {/if}
            <span class="flex-1">{item.label}</span>
            {#if item.shortcut}
              <Kbd keys={item.shortcut} />
            {/if}
          </ContextMenuPrimitive.Item>
        {/if}
      {/each}
    </ContextMenuPrimitive.Content>
  </ContextMenuPrimitive.Portal>
</ContextMenuPrimitive.Root>
