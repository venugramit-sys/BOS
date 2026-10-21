<script>
  import { Dialog as DialogPrimitive } from "bits-ui";
  import { X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    title = "",
    description = "",
    size = "md",
    showClose = true,
    onClose = null,
    class: className = "",
    children,
    footer
  } = $props();

  const widths = {
    sm: "max-w-sm",
    md: "max-w-lg",
    lg: "max-w-2xl",
    xl: "max-w-4xl"
  };
</script>

<DialogPrimitive.Root
  bind:open
  onOpenChange={(next) => {
    if (!next) onClose?.();
  }}
>
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay class="fixed inset-0 z-50 bg-slate-950/60" />
    <DialogPrimitive.Content
      class={cn(
        "fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] -translate-x-1/2 -translate-y-1/2",
        "rounded-xl border border-slate-200 bg-white shadow-xl dark:border-slate-800 dark:bg-slate-900",
        "flex flex-col max-h-[min(85vh,48rem)]",
        widths[size] || widths.md,
        className
      )}
    >
      <div class="flex items-start justify-between gap-3 px-5 py-4 border-b border-slate-200 dark:border-slate-800">
        <div class="min-w-0">
          <DialogPrimitive.Title class="text-sm font-semibold text-slate-900 dark:text-slate-100">
            {title}
          </DialogPrimitive.Title>
          {#if description}
            <DialogPrimitive.Description class="mt-1 text-xs text-slate-500 dark:text-slate-400">
              {description}
            </DialogPrimitive.Description>
          {/if}
        </div>
        {#if showClose}
          <DialogPrimitive.Close
            class="p-1 rounded-md text-slate-400 hover:text-slate-700 hover:bg-slate-100 dark:hover:text-slate-200 dark:hover:bg-slate-800 cursor-pointer"
            aria-label="Close"
          >
            <X class="size-4" />
          </DialogPrimitive.Close>
        {/if}
      </div>
      <div class="px-5 py-4 overflow-y-auto">
        {@render children?.()}
      </div>
      {#if footer}
        <div class="px-5 py-3 border-t border-slate-200 bg-slate-50 dark:border-slate-800 dark:bg-slate-950/60 flex items-center justify-end gap-2">
          {@render footer()}
        </div>
      {/if}
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>
