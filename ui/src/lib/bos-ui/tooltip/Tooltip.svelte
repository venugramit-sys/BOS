<script>
  import { Tooltip as TooltipPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import Kbd from "../kbd/Kbd.svelte";

  let {
    content = "",
    shortcut = "",
    side = "top",
    delayDuration = 300,
    children
  } = $props();
</script>

<TooltipPrimitive.Provider {delayDuration}>
  <TooltipPrimitive.Root>
    <TooltipPrimitive.Trigger>
      {#snippet child({ props })}
        {@render children?.(props)}
      {/snippet}
    </TooltipPrimitive.Trigger>
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        {side}
        sideOffset={6}
        class={cn(
          "z-50 max-w-xs rounded-md border border-slate-200 bg-slate-900 px-2 py-1 text-[11px] text-white shadow-md",
          "dark:border-slate-700 dark:bg-slate-800"
        )}
      >
        <span class="inline-flex items-center gap-2">
          <span>{content}</span>
          {#if shortcut}
            <Kbd keys={shortcut} class="text-slate-300" />
          {/if}
        </span>
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  </TooltipPrimitive.Root>
</TooltipPrimitive.Provider>
