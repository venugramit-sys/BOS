<script>
  import { cn } from "$lib/utils.js";
  import { Popover } from "bits-ui";
  import {
    Search,
    ChevronDown,
    X,
    Plus,
    ExternalLink,
    Check,
    AlertCircle,
    AlertTriangle,
    Info,
    Loader2,
    MoreHorizontal,
    ArrowUp,
    ArrowDown,
    ArrowUpDown,
    Clock,
    Layers
  } from "lucide-svelte";
  import { resolveSurface } from "../theme/index.js";
  import Badge from "../badge/Badge.svelte";

  let {
    ref = $bindable(null),
    value = $bindable(null),
    items = [],
    valueKey = "id",
    labelKey = "name",
    columns = [],
    multiple = false,
    sortable = true,
    groupBy = "",
    placeholder = "Select or search...",
    searchPlaceholder = "Type to filter records...",
    label = "",
    helperText = "",
    errorMessage = "",
    warningMessage = "",
    currency = "USD",
    locale = "en-US",
    factBoxFields = null,
    density = null, // 'sm' | 'md' | 'lg' (defaults to themeStore.density)
    variant = null, // 'outline' | 'filled' | 'underline' | 'ghost' (defaults to themeStore.variant)
    rounded = null, // 'none' | 'sm' | 'md' | 'pill' (defaults to themeStore.rounded)
    status = "default",
    required = false,
    disabled = false,
    readonly = false,
    clearable = true,
    loading = false,
    showFactBox = true,
    showRecordCount = true,
    newRecordLabel = "",
    fullListLabel = "Select from full list (F4)",
    persistRecentKey = "",
    maxDisplayItems = 100,
    fetchItems = null,
    onCreateNew = null,
    onFullList = null,
    onSearch = null,
    onSelect = null,
    onRemove = null,
    onClear = null,
    class: className = "",
    dropdownClass = "",
    open = $bindable(false),
    searchQuery = $bindable(""),
    ...restProps
  } = $props();
  let searchInputRef = $state(null);
  let scrollContainerRef = $state(null);
  let activeIndex = $state(0);
  let hoveredItem = $state(null);
  let sortKey = $state(null);
  let sortDirection = $state("asc"); // 'asc' | 'desc'
  let remoteItems = $state([]);
  let isRemoteLoading = $state(false);
  // Debounce handle only: not reactive (it is read and written inside the fetch effect).
  let fetchTimer = null;
  let recentItemsList = $state([]);

  // Load persisted recent items from localStorage on mount/key change
  $effect(() => {
    if (persistRecentKey && typeof window !== "undefined") {
      try {
        const stored = localStorage.getItem(`bos_recent_${persistRecentKey}`);
        if (stored) recentItemsList = JSON.parse(stored);
      } catch {
        // Safe fallback
      }
    }
  });

  // Effective items pool (remote if fetchItems provided, otherwise local items)
  let activeItemsPool = $derived.by(() => {
    if (fetchItems) return remoteItems;
    return items;
  });

  // Selected items resolution (single or multi-select array)
  let selectedItems = $derived.by(() => {
    if (value === null || value === undefined || value === "") return [];
    if (multiple) {
      if (!Array.isArray(value)) return [];
      return value.map((v) => {
        if (typeof v === "object" && v !== null) return v;
        return activeItemsPool.find((i) => i[valueKey] === v) || { [valueKey]: v, [labelKey]: String(v) };
      });
    } else {
      if (typeof value === "object" && value !== null) return [value];
      const match = activeItemsPool.find((i) => i[valueKey] === value);
      return match ? [match] : [{ [valueKey]: value, [labelKey]: String(value) }];
    }
  });

  let selectedItem = $derived(selectedItems[0] || null);

  // Effective status
  let resolvedStatus = $derived.by(() => {
    if (errorMessage) return "error";
    if (warningMessage) return "warning";
    return status;
  });

  // Display label for single-select mode
  let displayLabel = $derived.by(() => {
    if (multiple || !selectedItem) return "";
    const valCode = selectedItem[valueKey];
    const lbl = selectedItem[labelKey];
    if (valCode && lbl && valCode !== lbl) {
      return `${valCode} · ${lbl}`;
    }
    return lbl || String(valCode || "");
  });

  // Filtered & Sorted items
  let processedItems = $derived.by(() => {
    let list = [...activeItemsPool];
    const q = searchQuery.trim().toLowerCase();

    // 1. Text Search Filter (omni-column across all fields)
    if (q) {
      list = list.filter((item) => {
        if (columns.length > 0) {
          return columns.some((col) => {
            const cellVal = item[col.key];
            return cellVal !== null && cellVal !== undefined && String(cellVal).toLowerCase().includes(q);
          });
        }
        const matchValue = String(item[valueKey] ?? "").toLowerCase().includes(q);
        const matchLabel = String(item[labelKey] ?? "").toLowerCase().includes(q);
        return matchValue || matchLabel;
      });
    }

    // 2. Column Sorting
    if (sortKey) {
      list.sort((a, b) => {
        const valA = a[sortKey];
        const valB = b[sortKey];
        if (valA === valB) return 0;
        if (valA === null || valA === undefined) return 1;
        if (valB === null || valB === undefined) return -1;

        let comp = 0;
        if (typeof valA === "number" && typeof valB === "number") {
          comp = valA - valB;
        } else {
          comp = String(valA).localeCompare(String(valB));
        }
        return sortDirection === "asc" ? comp : -comp;
      });
    }

    return list;
  });

  // Display capped items for high-performance virtual rendering
  let visibleItems = $derived(processedItems.slice(0, maxDisplayItems));

  // Grouped structure if groupBy is specified, mapped directly from visibleItems with global linear index
  let groupedSections = $derived.by(() => {
    if (!groupBy || visibleItems.length === 0) return null;
    const groups = {};
    for (let i = 0; i < visibleItems.length; i++) {
      const item = visibleItems[i];
      const gKey = String(item[groupBy] || "Unassigned");
      if (!groups[gKey]) groups[gKey] = [];
      groups[gKey].push({ item, globalIndex: i });
    }
    return Object.entries(groups).map(([groupName, groupEntries]) => ({
      name: groupName,
      entries: groupEntries
    }));
  });

  let activeItem = $derived(visibleItems[activeIndex] || null);

  let surface = $derived(resolveSurface("lookup", { density, variant, rounded, status: resolvedStatus }));

  function scrollActiveIntoView(index) {
    if (typeof requestAnimationFrame === "undefined" || !scrollContainerRef) return;
    requestAnimationFrame(() => {
      const activeEl = scrollContainerRef?.querySelector(`[data-combobox-index="${index}"]`);
      if (activeEl) {
        activeEl.scrollIntoView({ block: "nearest" });
      }
    });
  }

  function selectItem(item) {
    if (!item) return;

    if (multiple) {
      const currentList = Array.isArray(value) ? [...value] : [];
      const itemVal = item[valueKey];
      const existingIdx = currentList.findIndex((v) => (typeof v === "object" ? v[valueKey] : v) === itemVal);

      if (existingIdx >= 0) {
        currentList.splice(existingIdx, 1);
        onRemove?.(item);
      } else {
        currentList.push(itemVal);
        onSelect?.(item);
      }
      value = currentList;
    } else {
      value = item[valueKey];
      onSelect?.(item);
      open = false;
      searchQuery = "";
    }

    // Save to persistent recent items
    if (persistRecentKey && typeof window !== "undefined") {
      try {
        const filtered = recentItemsList.filter((r) => r[valueKey] !== item[valueKey]);
        const updated = [item, ...filtered].slice(0, 5);
        recentItemsList = updated;
        localStorage.setItem(`bos_recent_${persistRecentKey}`, JSON.stringify(updated));
      } catch {
        // Safe fallback
      }
    }
  }

  function removeSelectedItem(e, item) {
    e?.stopPropagation();
    if (multiple && Array.isArray(value)) {
      const itemVal = item[valueKey];
      value = value.filter((v) => (typeof v === "object" ? v[valueKey] : v) !== itemVal);
      onRemove?.(item);
    }
  }

  function handleClear(e) {
    e?.stopPropagation();
    value = multiple ? [] : null;
    searchQuery = "";
    onClear?.();
  }

  function toggleColumnSort(colKey) {
    if (!sortable) return;
    if (sortKey === colKey) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortKey = colKey;
      sortDirection = "asc";
    }
  }

  function handleTriggerKeyDown(e) {
    if (disabled || readonly) return;

    if (e.key === "F4" || (e.altKey && e.key === "ArrowDown")) {
      e.preventDefault();
      open = !open;
    } else if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
      if (!open) {
        e.preventDefault();
        open = true;
      }
    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (!open) {
        e.preventDefault();
        open = true;
        searchQuery = e.key;
      }
    }
  }

  function handleSearchKeyDown(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const nextIndex = Math.min(activeIndex + 1, visibleItems.length - 1);
      activeIndex = nextIndex;
      scrollActiveIntoView(nextIndex);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const prevIndex = Math.max(activeIndex - 1, 0);
      activeIndex = prevIndex;
      scrollActiveIntoView(prevIndex);
    } else if (e.key === "Enter" || e.key === "Tab") {
      if (visibleItems[activeIndex]) {
        e.preventDefault();
        selectItem(visibleItems[activeIndex]);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      open = false;
    } else if (e.key === "F4") {
      e.preventDefault();
      onFullList?.();
    }
  }

  function formatCellValue(val, format) {
    if (val === null || val === undefined || val === "") return "—";
    if (format === "currency") {
      const num = Number(val);
      if (isNaN(num)) return String(val);
      return num.toLocaleString(locale, { style: "currency", currency: currency || "USD" });
    }
    if (format === "number") {
      const num = Number(val);
      return isNaN(num) ? String(val) : num.toLocaleString(locale);
    }
    return String(val);
  }

  function resolveBadgeTone(val) {
    const s = String(val).toLowerCase();
    if (["active", "approved", "posted", "success", "open", "released"].includes(s)) return "success";
    if (["pending", "in_progress", "warning", "review"].includes(s)) return "warning";
    if (["blocked", "closed", "error", "rejected", "danger"].includes(s)) return "danger";
    if (["draft", "info", "new"].includes(s)) return "info";
    return "neutral";
  }

  function getFactBoxDetails(item) {
    if (!item) return "";
    if (Array.isArray(factBoxFields) && factBoxFields.length > 0) {
      return factBoxFields
        .map((k) => item[k])
        .filter((v) => v !== null && v !== undefined && v !== "")
        .join(" · ");
    }
    // Automatically infer from non-primary columns or fallback properties
    if (columns.length > 2) {
      return columns
        .filter((col) => !col.primary && col.key !== valueKey && col.key !== labelKey)
        .slice(0, 3)
        .map((col) => item[col.key])
        .filter((v) => v !== null && v !== undefined && v !== "")
        .join(" · ");
    }
    return [item.city, item.phone, item.category].filter(Boolean).join(" · ");
  }

  function isItemSelected(item) {
    const itemVal = item[valueKey];
    if (multiple) {
      if (!Array.isArray(value)) return false;
      return value.some((v) => (typeof v === "object" ? v[valueKey] : v) === itemVal);
    }
    return selectedItem && selectedItem[valueKey] === itemVal;
  }

  // Handle async remote searching if fetchItems provided
  $effect(() => {
    if (fetchItems) {
      if (fetchTimer) clearTimeout(fetchTimer);
      const query = searchQuery;
      fetchTimer = setTimeout(async () => {
        isRemoteLoading = true;
        try {
          remoteItems = await fetchItems(query);
        } catch {
          remoteItems = [];
        } finally {
          isRemoteLoading = false;
        }
      }, 250);
    } else {
      onSearch?.(searchQuery);
    }
    activeIndex = 0;
  });

  // Focus search input when popover opens
  $effect(() => {
    if (open) {
      setTimeout(() => {
        searchInputRef?.focus();
      }, 50);
    } else {
      hoveredItem = null;
    }
  });
</script>

<div class={cn("flex flex-col gap-1 w-full text-left font-sans select-none", className)}>
  <!-- 1. Top Form Field Label & Optional Required Mark -->
  {#if label}
    <div class="flex items-center justify-between text-xs">
      <label class="font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
        <span>{label}</span>
        {#if required}
          <span class="text-rose-500 font-bold" title="Required field">*</span>
        {/if}
      </label>
      <span class="text-[10px] text-slate-400 font-mono">F4 for Lookup</span>
    </div>
  {/if}

  <!-- 2. Interactive Trigger Frame using Popover.Root -->
  <Popover.Root bind:open>
    <Popover.Trigger
      bind:ref
      type="button"
      role="combobox"
      aria-expanded={open}
      aria-haspopup="dialog"
      disabled={disabled}
      onkeydown={handleTriggerKeyDown}
      class={cn(
        "group relative flex items-center justify-between w-full transition-all outline-hidden cursor-pointer gap-1.5",
        surface.controlClass,
        disabled && "opacity-50 cursor-not-allowed bg-slate-100 dark:bg-slate-800/40 pointer-events-none",
        readonly && "bg-slate-50 dark:bg-slate-950/60 pointer-events-none cursor-default"
      )}
      {...restProps}
    >
      <!-- Multi-Select Chips or Single-Select Display -->
      <div class="flex-1 min-w-0 flex items-center flex-wrap gap-1 text-left">
        {#if multiple}
          {#if selectedItems.length > 0}
            {#each selectedItems as item}
              <span
                class={cn(
                  "inline-flex items-center gap-1 px-1.5 py-0.5 text-[11px] font-medium bg-slate-100 dark:bg-slate-800 text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-slate-700 max-w-[200px] truncate",
                  surface.chipRadius
                )}
              >
                <span class="truncate">{item[valueKey]} · {item[labelKey]}</span>
                {#if !disabled && !readonly}
                  <button
                    type="button"
                    onclick={(e) => removeSelectedItem(e, item)}
                    class="p-0.5 hover:text-rose-500 text-slate-400 cursor-pointer"
                    aria-label="Remove item"
                  >
                    <X class="size-2.5" />
                  </button>
                {/if}
              </span>
            {/each}
          {:else}
            <span class="text-slate-400 dark:text-slate-500">
              {placeholder}
            </span>
          {/if}
        {:else if displayLabel}
          <span class="text-slate-900 dark:text-slate-100 font-medium truncate">
            {displayLabel}
          </span>
        {:else}
          <span class="text-slate-400 dark:text-slate-500 truncate">
            {placeholder}
          </span>
        {/if}
      </div>

      <!-- Suffix Action Controls -->
      <div class="shrink-0 flex items-center gap-1 self-center">
        {#if loading || isRemoteLoading}
          <Loader2 class={cn("animate-spin text-indigo-500", surface.iconClass)} />
        {/if}

        {#if clearable && (selectedItems.length > 0 || displayLabel) && !disabled && !readonly}
          <button
            type="button"
            onclick={handleClear}
            class="p-0.5 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
            title="Clear selection (Esc)"
            aria-label="Clear selection"
          >
            <X class="size-3.5" />
          </button>
        {/if}

        <!-- ERP Lookup Trigger Assist Indicator -->
        <span class="p-0.5 rounded text-slate-400 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors">
          <MoreHorizontal class="size-3.5" />
        </span>
        <ChevronDown class={cn("text-slate-400 transition-transform duration-200", open && "rotate-180", surface.iconClass)} />
      </div>
    </Popover.Trigger>

    <!-- 3. Multi-Column Dropdown Grid Content (Floating Portal) -->
    <Popover.Portal>
      <Popover.Content
        side="bottom"
        align="start"
        sideOffset={4}
        class={cn(
          "z-50 w-[95vw] sm:w-[500px] md:w-[600px] max-w-[680px] overflow-hidden animate-in fade-in-50 zoom-in-95 duration-150 flex flex-col font-sans",
          surface.panelClass,
          dropdownClass
        )}
      >
        <!-- Search & Filter Bar -->
        <div class="p-2 border-b border-slate-100 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 flex items-center gap-2">
          <div class="relative flex-1">
            <Search class="absolute left-2.5 top-2.5 size-3.5 text-slate-400" />
            <input
              bind:this={searchInputRef}
              type="text"
              role="searchbox"
              aria-label={searchPlaceholder}
              bind:value={searchQuery}
              onkeydown={handleSearchKeyDown}
              placeholder={searchPlaceholder}
              class={cn(
                "w-full h-8 pl-8 pr-3 text-xs border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 outline-hidden focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500/20",
                surface.radiusClass
              )}
            />
            {#if searchQuery}
              <button
                type="button"
                onclick={() => (searchQuery = "")}
                class="absolute right-2 top-2 p-0.5 text-slate-400 hover:text-slate-600 cursor-pointer"
              >
                <X class="size-3" />
              </button>
            {/if}
          </div>
          {#if showRecordCount}
            <span class="text-[10px] text-slate-400 shrink-0 font-mono">
              {processedItems.length} {processedItems.length === 1 ? "record" : "records"}
            </span>
          {/if}
        </div>

        <!-- Optional Recent Records Bar (when search is empty) -->
        {#if !searchQuery && recentItemsList.length > 0}
          <div class="px-3 py-1.5 bg-slate-50 dark:bg-slate-800/40 border-b border-slate-100 dark:border-slate-800 flex items-center gap-1.5 text-[11px] overflow-x-auto">
            <Clock class="size-3 text-slate-400 shrink-0" />
            <span class="text-slate-400 font-medium shrink-0">Recent:</span>
            <div class="flex items-center gap-1">
              {#each recentItemsList as rItem}
                <button
                  type="button"
                  onclick={() => selectItem(rItem)}
                  class="px-1.5 py-0.5 rounded text-[10px] font-mono bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:border-indigo-400 hover:text-indigo-600 dark:hover:text-indigo-400 cursor-pointer transition-colors shrink-0"
                >
                  {rItem[valueKey]}
                </button>
              {/each}
            </div>
          </div>
        {/if}

        <!-- Main Multi-Column Table Viewport -->
        <div
          bind:this={scrollContainerRef}
          class="max-h-[300px] overflow-auto divide-y divide-slate-100 dark:divide-slate-800"
        >
          {#if columns.length > 0}
            <!-- Multi-Column Table Mode -->
            <table class="w-full text-left text-xs border-collapse">
              <thead class="sticky top-0 bg-slate-100 dark:bg-slate-800/95 text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider backdrop-blur-xs select-none z-10">
                <tr>
                  {#if multiple}
                    <th class="py-2 px-2.5 w-8 text-center">✓</th>
                  {/if}
                  {#each columns as col}
                    <th
                      style={col.width ? `width: ${col.width};` : (col.minWidth ? `min-width: ${col.minWidth};` : "")}
                      onclick={() => toggleColumnSort(col.key)}
                      class={cn(
                        "py-2 px-3 transition-colors",
                        sortable && "cursor-pointer hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-200/50 dark:hover:bg-slate-700/50",
                        col.align === "right" && "text-right",
                        col.align === "center" && "text-center"
                      )}
                    >
                      <span class="inline-flex items-center gap-1">
                        <span>{col.label}</span>
                        {#if sortable}
                          {#if sortKey === col.key}
                            {#if sortDirection === "asc"}
                              <ArrowUp class="size-3 text-indigo-600 dark:text-indigo-400" />
                            {:else}
                              <ArrowDown class="size-3 text-indigo-600 dark:text-indigo-400" />
                            {/if}
                          {:else}
                            <ArrowUpDown class="size-2.5 text-slate-300 dark:text-slate-600 opacity-0 group-hover:opacity-100" />
                          {/if}
                        {/if}
                      </span>
                    </th>
                  {/each}
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
                {#if groupedSections}
                  <!-- Grouped Table Rows -->
                  {#each groupedSections as grp}
                    <tr class="bg-slate-50/80 dark:bg-slate-800/70 text-[10px] font-bold uppercase tracking-wider text-slate-400">
                      <td colspan={columns.length + (multiple ? 1 : 0)} class="py-1 px-3">
                        <span class="flex items-center gap-1">
                          <Layers class="size-2.5" />
                          <span>{grp.name} ({grp.entries.length})</span>
                        </span>
                      </td>
                    </tr>
                    {#each grp.entries as { item, globalIndex }}
                      {@const selected = isItemSelected(item)}
                      {@const active = activeIndex === globalIndex}
                      <tr
                        data-combobox-index={globalIndex}
                        onclick={() => selectItem(item)}
                        onmouseenter={() => {
                          activeIndex = globalIndex;
                          hoveredItem = item;
                        }}
                        class={cn(
                          "cursor-pointer transition-colors",
                          active
                            ? "bg-indigo-50 dark:bg-indigo-950/50 text-indigo-950 dark:text-indigo-100"
                            : "hover:bg-slate-50 dark:hover:bg-slate-800/50 text-slate-800 dark:text-slate-200",
                          selected && "font-semibold bg-indigo-50/40 dark:bg-indigo-950/20"
                        )}
                      >
                        {#if multiple}
                          <td class="py-2 px-2.5 text-center">
                            <input
                              type="checkbox"
                              checked={selected}
                              class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500 cursor-pointer pointer-events-none"
                            />
                          </td>
                        {/if}
                        {#each columns as col}
                          <td
                            class={cn(
                              "py-2 px-3 whitespace-nowrap",
                              col.mono && "font-mono font-medium",
                              col.primary && "font-semibold text-slate-900 dark:text-slate-100",
                              col.align === "right" && "text-right",
                              col.align === "center" && "text-center",
                              col.format === "currency" && Number(item[col.key]) > 0 && "text-amber-600 dark:text-amber-400"
                            )}
                          >
                            {#if col.format === "badge"}
                              <Badge tone={resolveBadgeTone(item[col.key])} size="sm">
                                {formatCellValue(item[col.key], col.format)}
                              </Badge>
                            {:else}
                              {formatCellValue(item[col.key], col.format)}
                            {/if}
                          </td>
                        {/each}
                      </tr>
                    {/each}
                  {/each}
                {:else}
                  <!-- Standard Flat Rows -->
                  {#each visibleItems as item, idx}
                    {@const selected = isItemSelected(item)}
                    {@const active = activeIndex === idx}
                    <tr
                      data-combobox-index={idx}
                      onclick={() => selectItem(item)}
                      onmouseenter={() => {
                        activeIndex = idx;
                        hoveredItem = item;
                      }}
                      class={cn(
                        "cursor-pointer transition-colors",
                        active
                          ? "bg-indigo-50 dark:bg-indigo-950/50 text-indigo-950 dark:text-indigo-100"
                          : "hover:bg-slate-50 dark:hover:bg-slate-800/50 text-slate-800 dark:text-slate-200",
                        selected && "font-semibold bg-indigo-50/40 dark:bg-indigo-950/20"
                      )}
                    >
                      {#if multiple}
                        <td class="py-2 px-2.5 text-center">
                          <input
                            type="checkbox"
                            checked={selected}
                            class="rounded border-slate-300 text-indigo-600 focus:ring-indigo-500 cursor-pointer pointer-events-none"
                          />
                        </td>
                      {/if}
                      {#each columns as col}
                        <td
                          class={cn(
                            "py-2 px-3 whitespace-nowrap",
                            col.mono && "font-mono font-medium",
                            col.primary && "font-semibold text-slate-900 dark:text-slate-100",
                            col.align === "right" && "text-right",
                            col.align === "center" && "text-center",
                            col.format === "currency" && Number(item[col.key]) > 0 && "text-amber-600 dark:text-amber-400"
                          )}
                        >
                          {#if col.format === "badge"}
                            <Badge tone={resolveBadgeTone(item[col.key])} size="sm">
                              {formatCellValue(item[col.key], col.format)}
                            </Badge>
                          {:else}
                            {formatCellValue(item[col.key], col.format)}
                          {/if}
                        </td>
                      {/each}
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          {:else}
            <!-- Simple Single-Column Mode -->
            <div class="py-1">
              {#each visibleItems as item, idx}
                {@const selected = isItemSelected(item)}
                {@const active = activeIndex === idx}
                <button
                  type="button"
                  data-combobox-index={idx}
                  onclick={() => selectItem(item)}
                  onmouseenter={() => (activeIndex = idx)}
                  class={cn(
                    "w-full text-left py-2 px-3 text-xs flex items-center justify-between cursor-pointer transition-colors",
                    active
                      ? "bg-indigo-50 dark:bg-indigo-950/50 text-indigo-950 dark:text-indigo-100"
                      : "hover:bg-slate-50 dark:hover:bg-slate-800/50 text-slate-800 dark:text-slate-200"
                  )}
                >
                  <span class="font-medium">{item[labelKey] || item[valueKey] || String(item)}</span>
                  {#if selected}
                    <Check class="size-3.5 text-indigo-600 dark:text-indigo-400" />
                  {/if}
                </button>
              {/each}
            </div>
          {/if}

          <!-- Virtual Cap Notice -->
          {#if processedItems.length > maxDisplayItems}
            <div class="py-2 px-3 text-center text-[10px] text-slate-400 bg-slate-50/50 dark:bg-slate-800/30">
              Showing top {maxDisplayItems} of {processedItems.length} records. Type to filter more specifically.
            </div>
          {/if}

          <!-- Empty State -->
          {#if processedItems.length === 0}
            <div class="py-8 text-center text-xs text-slate-400">
              <p>No matching records found for "{searchQuery}"</p>
            </div>
          {/if}
        </div>

        <!-- Optional FactBox Preview Footer (Dynamics BC style) -->
        {#if showFactBox && (hoveredItem || activeItem)}
          {@const previewTarget = hoveredItem || activeItem}
          {@const factBoxDetails = getFactBoxDetails(previewTarget)}
          <div class="p-2.5 bg-slate-50 dark:bg-slate-800/60 border-t border-slate-200 dark:border-slate-800 text-[11px] flex items-center justify-between">
            <div class="flex items-center gap-2 truncate">
              <span class="font-mono font-bold text-indigo-600 dark:text-indigo-400">
                {previewTarget[valueKey]}
              </span>
              <span class="text-slate-600 dark:text-slate-300 truncate font-medium">
                {previewTarget[labelKey]}
              </span>
            </div>
            {#if factBoxDetails}
              <span class="text-slate-400 text-[10px] shrink-0">
                {factBoxDetails}
              </span>
            {/if}
          </div>
        {/if}

        <!-- Bottom Action Bar (Quick-Create & Full-List) -->
        <div class="p-2 bg-white dark:bg-slate-900 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs">
          {#if newRecordLabel || onCreateNew}
            <button
              type="button"
              onclick={() => {
                open = false;
                onCreateNew?.();
              }}
              class="inline-flex items-center gap-1.5 px-2 py-1 rounded text-indigo-600 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/50 font-semibold cursor-pointer transition-colors"
            >
              <Plus class="size-3.5" />
              <span>{newRecordLabel || "+ New Record"}</span>
            </button>
          {:else}
            <div></div>
          {/if}

          {#if onFullList}
            <button
              type="button"
              onclick={() => {
                open = false;
                onFullList?.();
              }}
              class="inline-flex items-center gap-1.5 px-2 py-1 rounded text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors font-medium"
            >
              <span>{fullListLabel}</span>
              <ExternalLink class="size-3" />
            </button>
          {/if}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>

  <!-- 4. Bottom Message / Error / Helper Feedback -->
  {#if errorMessage || warningMessage || helperText}
    <div class="flex items-center gap-1.5 text-xs pt-0.5 animate-in fade-in-50 duration-150">
      {#if resolvedStatus === "error" && errorMessage}
        <AlertCircle class="size-3.5 text-rose-500 shrink-0" />
        <span class="text-rose-600 dark:text-rose-400 font-medium">{errorMessage}</span>
      {:else if resolvedStatus === "warning" && warningMessage}
        <AlertTriangle class="size-3.5 text-amber-500 shrink-0" />
        <span class="text-amber-600 dark:text-amber-400 font-medium">{warningMessage}</span>
      {:else if helperText}
        <Info class="size-3.5 text-slate-400 shrink-0" />
        <span class="text-slate-500 dark:text-slate-400">{helperText}</span>
      {/if}
    </div>
  {/if}
</div>
