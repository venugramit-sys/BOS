<script>
  import { Tabs as TabsPrimitive } from "bits-ui";
  import { X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    value = $bindable(""),
    items = [],
    variant = "segmented",
    onValueChange = null,
    onClose = null,
    class: className = ""
  } = $props();

  let listClass = $derived(
    variant === "document"
      ? "flex items-end gap-1 overflow-x-auto"
      : variant === "underline"
        ? "flex items-center gap-1 border-b border-slate-200 dark:border-slate-800"
        : "inline-flex items-center gap-0.5 rounded-md bg-slate-100 p-0.5 dark:bg-slate-800"
  );
</script>

<TabsPrimitive.Root
  {value}
  onValueChange={(next) => {
    if (!next) return;
    value = next;
    onValueChange?.(next);
  }}
  class={cn("min-w-0", className)}
>
  <TabsPrimitive.List class={listClass}>
    {#each items as item (item.id)}
      <div
        class="relative inline-flex items-center shrink-0"
        onauxclick={(event) => {
          if (event.button === 1 && item.closable) {
            event.preventDefault();
            onClose?.(item.id);
          }
        }}
      >
        <TabsPrimitive.Trigger
          value={item.id}
          disabled={item.disabled}
          class={cn(
            "inline-flex items-center gap-1.5 text-xs font-semibold whitespace-nowrap cursor-pointer transition-colors",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/40",
            "disabled:opacity-40 disabled:pointer-events-none",
            variant === "segmented" &&
              "rounded px-2.5 py-1 text-slate-600 data-[state=active]:bg-white data-[state=active]:text-indigo-600 data-[state=active]:shadow-xs dark:text-slate-300 dark:data-[state=active]:bg-slate-950 dark:data-[state=active]:text-indigo-300",
            variant === "underline" &&
              "px-2.5 py-1.5 border-b-2 border-transparent text-slate-500 data-[state=active]:border-indigo-600 data-[state=active]:text-indigo-600 dark:text-slate-400",
            variant === "document" &&
              "h-8 rounded-t-lg border-t-2 border-transparent text-slate-600 data-[state=active]:bg-white data-[state=active]:border-indigo-600 data-[state=active]:text-indigo-600 dark:text-slate-400 dark:data-[state=active]:bg-slate-950 dark:data-[state=active]:text-indigo-300",
            variant === "document" && (item.closable ? "pl-3 pr-7" : "px-3"),
            variant !== "document" && item.closable && "pr-6"
          )}
        >
          {#if item.icon}
            {@const Icon = item.icon}
            <Icon class="size-3.5 shrink-0" />
          {/if}
          <span>{item.label}</span>
          {#if item.dirty}
            <span class="text-amber-500" aria-label="Unsaved changes">*</span>
          {/if}
        </TabsPrimitive.Trigger>
        {#if item.closable}
          <button
            type="button"
            class="absolute right-1.5 top-1/2 z-10 inline-flex size-4 -translate-y-1/2 items-center justify-center rounded text-slate-400 hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200 cursor-pointer"
            aria-label={`Close ${item.label}`}
            onclick={(event) => {
              event.stopPropagation();
              onClose?.(item.id);
            }}
          >
            <X class="size-3" />
          </button>
        {/if}
      </div>
    {/each}
  </TabsPrimitive.List>
</TabsPrimitive.Root>
