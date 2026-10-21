<script>
  import { Select as SelectPrimitive } from "bits-ui";
  import { ChevronDown, Check, Search, X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import { resolveSurface } from "../theme/index.js";
  import Field from "../field/Field.svelte";
  import { tick } from "svelte";

  let {
    value = $bindable(""),
    options = [],
    placeholder = "Select...",
    label = "",
    hint = "",
    error = "",
    required = false,
    disabled = false,
    status = "default",
    class: className = "",
    id = "",
    open = $bindable(false),
    activeValue = "",
    searchable = true,
    searchPlaceholder = "",
    onValueChange = null
  } = $props();

  const uid = $props.id();
  let fieldId = $derived(id || uid);
  let surface = $derived(resolveSurface("field", { status: error ? "error" : status, density: "sm" }));

  let searchInputEl = $state(null);
  let contentEl = $state(null);
  let searchQuery = $state("");

  let items = $derived(
    options.map((option) => ({
      value: String(option.value),
      label: option.label ?? String(option.value),
      disabled: Boolean(option.disabled)
    }))
  );

  let showSearch = $derived(searchable && items.length >= 3);

  let filteredItems = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (item) =>
        item.label.toLowerCase().includes(q) ||
        item.value.toLowerCase().includes(q)
    );
  });

  let selectedLabel = $derived(items.find((item) => item.value === String(value ?? ""))?.label || "");

  let effectiveSearchPlaceholder = $derived(
    searchPlaceholder || (placeholder && placeholder !== "Select..." ? `Search ${placeholder.toLowerCase()}...` : "Search...")
  );

  $effect(() => {
    if (open) {
      searchQuery = "";
      if (showSearch) {
        tick().then(() => {
          setTimeout(() => {
            searchInputEl?.focus();
          }, 30);
        });
      }
    } else {
      searchQuery = "";
    }
  });

  function selectItem(next) {
    value = next ?? "";
    onValueChange?.(value);
    open = false;
  }
</script>

<Field {label} {hint} error={error} {required} htmlFor={fieldId}>
  <SelectPrimitive.Root
    type="single"
    bind:open
    value={value == null || value === "" ? undefined : String(value)}
    items={items}
    {disabled}
    allowDeselect={false}
    onValueChange={(next) => {
      value = next ?? "";
      onValueChange?.(value);
    }}
  >
    <SelectPrimitive.Trigger
      id={fieldId}
      class={cn(
        "w-full px-2 text-xs text-left text-slate-900 dark:text-slate-100 inline-flex items-center justify-between gap-2",
        "focus-visible:outline-none",
        surface.controlClass,
        className
      )}
      aria-invalid={error ? "true" : undefined}
    >
      <span class={cn("truncate", !selectedLabel && "text-slate-400")}>
        {selectedLabel || placeholder}
      </span>
      <ChevronDown class="size-3.5 shrink-0 text-slate-400" />
    </SelectPrimitive.Trigger>
    <SelectPrimitive.Portal>
      <SelectPrimitive.Content
        bind:ref={contentEl}
        class="z-50 max-h-72 min-w-[max(var(--bits-floating-anchor-width),180px)] overflow-hidden rounded-md border border-slate-200 bg-white shadow-lg dark:border-slate-700 dark:bg-slate-900"
        sideOffset={4}
      >
        {#if showSearch}
          <div
            class="p-1.5 border-b border-slate-100 dark:border-slate-800 flex items-center gap-1.5 bg-slate-50/70 dark:bg-slate-900/70"
            onpointerdown={(e) => e.stopPropagation()}
            onclick={(e) => e.stopPropagation()}
          >
            <Search class="size-3.5 text-slate-400 shrink-0 ml-1" />
            <input
              type="text"
              bind:this={searchInputEl}
              bind:value={searchQuery}
              placeholder={effectiveSearchPlaceholder}
              class="w-full bg-transparent text-xs text-slate-900 dark:text-slate-100 placeholder:text-slate-400 focus:outline-none"
              onkeydown={(e) => {
                e.stopPropagation();
                if (e.key === "Escape") {
                  open = false;
                } else if (e.key === "ArrowDown") {
                  e.preventDefault();
                  const firstItem = contentEl?.querySelector('[role="option"]:not([aria-disabled="true"]):not([data-disabled])');
                  if (firstItem instanceof HTMLElement) firstItem.focus();
                } else if (e.key === "Enter") {
                  e.preventDefault();
                  if (filteredItems.length > 0) {
                    selectItem(filteredItems[0].value);
                  }
                }
              }}
              onpointerdown={(e) => e.stopPropagation()}
              onclick={(e) => e.stopPropagation()}
            />
            {#if searchQuery}
              <button
                type="button"
                class="p-0.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 rounded shrink-0 cursor-pointer"
                aria-label="Clear search"
                onpointerdown={(e) => e.stopPropagation()}
                onclick={(e) => {
                  e.stopPropagation();
                  searchQuery = "";
                  searchInputEl?.focus();
                }}
              >
                <X class="size-3" />
              </button>
            {/if}
          </div>
        {/if}
        <SelectPrimitive.Viewport class="p-1 max-h-52 overflow-y-auto">
          {#each filteredItems as item (item.value)}
            <SelectPrimitive.Item
              value={item.value}
              label={item.label}
              disabled={item.disabled}
              class={cn(
                "relative flex cursor-pointer items-center rounded-sm px-2 py-1.5 text-xs text-slate-700 outline-none dark:text-slate-200 data-disabled:opacity-40",
                activeValue
                  ? ""
                  : "data-highlighted:bg-indigo-50 data-highlighted:text-indigo-700 dark:data-highlighted:bg-indigo-950/60 dark:data-highlighted:text-indigo-200",
                activeValue && item.value === String(activeValue) && "bg-indigo-100 font-semibold text-indigo-800 ring-1 ring-inset ring-indigo-400 dark:bg-indigo-950 dark:text-indigo-100"
              )}
            >
              {#snippet children({ selected })}
                <span class="truncate">{item.label}</span>
                {#if selected}
                  <Check class="ml-auto size-3.5 text-indigo-600" />
                {/if}
              {/snippet}
            </SelectPrimitive.Item>
          {:else}
            <div class="px-3 py-3 text-xs text-slate-400 dark:text-slate-500 text-center italic">
              No matching options
            </div>
          {/each}
        </SelectPrimitive.Viewport>
      </SelectPrimitive.Content>
    </SelectPrimitive.Portal>
  </SelectPrimitive.Root>
</Field>
