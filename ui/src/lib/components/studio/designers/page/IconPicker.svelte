<script>
  /**
   * IconPicker.svelte
   * Enterprise-Grade Searchable Lucide Icon Picker for BOS Studio.
   * - Trigger shows current icon preview & instant clear.
   * - Opens a fully responsive, centered modal overlay (never clipped by sidebars).
   * - Curated category filters (User & Auth, Commerce, Navigation, Data & System).
   * - Fast debounced search across all 1,600+ icons with instant result preview.
   * - Responsive adaptive grid (6 to 10 columns) with crisp vector icons.
   */
  import { X, Search, ChevronDown, Check, Sparkles, Trash2 } from "lucide-svelte";
  import Icon from "$lib/page-kit/Icon.svelte";
  import { ICON_NAMES } from "$lib/page-kit/icons.js";

  let {
    value = null,
    placeholder = "Select icon...",
    disabled = false,
    onChange = null,
    onSelect = null
  } = $props();

  function triggerChange(val) {
    if (onChange) onChange(val);
    if (onSelect) onSelect(val);
  }

  let open = $state(false);
  let query = $state("");
  let selectedCategory = $state("all");
  let hoveredIcon = $state(null);

  function portal(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        if (node.parentNode) {
          node.parentNode.removeChild(node);
        }
      }
    };
  }

  const CATEGORIES = [
    { id: "all", label: "All" },
    {
      id: "auth",
      label: "User & Security",
      icons: [
        "user", "users", "user-check", "user-plus", "user-minus", "user-cog", "user-x",
        "lock", "lock-keyhole", "key", "key-round", "shield", "shield-check", "shield-alert",
        "log-in", "log-out", "fingerprint", "badge-check", "id-card"
      ]
    },
    {
      id: "business",
      label: "Commerce & ERP",
      icons: [
        "credit-card", "wallet", "receipt", "shopping-cart", "shopping-bag", "package",
        "truck", "store", "building-2", "briefcase", "bar-chart-3", "pie-chart", "trending-up",
        "boxes", "badge-percent", "coins", "landmark", "banknote", "calculator", "dollar-sign"
      ]
    },
    {
      id: "general",
      label: "General",
      icons: [
        "check", "x", "plus", "minus", "trash-2", "edit-3", "search", "settings", "sliders",
        "bell", "mail", "home", "calendar", "clock", "alert-triangle", "info", "help-circle",
        "filter", "refresh-cw", "eye", "eye-off", "external-link", "copy", "save"
      ]
    },
    {
      id: "navigation",
      label: "Arrows & Nav",
      icons: [
        "arrow-right", "arrow-left", "arrow-up", "arrow-down", "chevron-right", "chevron-left",
        "chevron-up", "chevron-down", "chevrons-right", "corner-down-right", "arrow-right-left",
        "compass", "map-pin", "navigation", "menu", "more-horizontal", "more-vertical"
      ]
    },
    {
      id: "data",
      label: "Data & Files",
      icons: [
        "database", "server", "cpu", "file-text", "file", "folder", "table", "columns-3",
        "layout-grid", "code-xml", "terminal", "link-2", "paperclip", "layers", "archive"
      ]
    }
  ];

  const LIMIT = 160;

  const matches = $derived.by(() => {
    let pool = ICON_NAMES;
    if (selectedCategory !== "all") {
      const cat = CATEGORIES.find((c) => c.id === selectedCategory);
      if (cat?.icons) {
        pool = cat.icons;
      }
    }
    const q = query.trim().toLowerCase();
    if (q) {
      return pool.filter((n) => n.includes(q)).slice(0, LIMIT);
    }
    return pool.slice(0, LIMIT);
  });

  function handleKeydown(e) {
    if (open && e.key === "Escape") {
      e.stopPropagation();
      open = false;
    }
  }

  const activeDisplayIcon = $derived(hoveredIcon || value || "sparkles");
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="relative flex flex-col gap-1 text-xs">
  <!-- Trigger Button -->
  <div class="flex items-center gap-1.5">
    <button
      type="button"
      data-testid="icon-picker-trigger"
      class={"group flex min-w-0 flex-1 items-center justify-between gap-2 rounded-lg border px-2.5 py-1.5 text-left text-xs transition-colors shadow-2xs " +
        (open
          ? "border-indigo-500 ring-2 ring-indigo-500/20 bg-white dark:bg-slate-900"
          : "border-slate-200 bg-white hover:border-indigo-300 dark:border-slate-800 dark:bg-slate-900 dark:hover:border-indigo-700")}
      aria-expanded={open}
      aria-haspopup="dialog"
      {disabled}
      onclick={() => (open = true)}
    >
      <div class="flex items-center gap-2 min-w-0">
        {#if value}
          <div class="flex size-5 shrink-0 items-center justify-center rounded-md bg-indigo-50 text-indigo-600 dark:bg-indigo-950/70 dark:text-indigo-400 border border-indigo-200/50 dark:border-indigo-800/50">
            <Icon name={value} class="size-3.5" />
          </div>
          <span class="font-mono text-xs font-semibold text-slate-800 dark:text-slate-100 truncate">
            {value}
          </span>
        {:else}
          <div class="flex size-5 shrink-0 items-center justify-center rounded bg-slate-100 text-slate-400 dark:bg-slate-800 dark:text-slate-500">
            <Sparkles class="size-3" />
          </div>
          <span class="text-slate-400 dark:text-slate-500 text-xs truncate">
            {placeholder}
          </span>
        {/if}
      </div>

      <ChevronDown class="size-3.5 text-slate-400 shrink-0 group-hover:text-slate-600 dark:group-hover:text-slate-200" />
    </button>

    {#if value}
      <button
        type="button"
        class="flex size-7 shrink-0 items-center justify-center rounded-lg border border-slate-200 text-slate-400 hover:border-rose-300 hover:bg-rose-50 hover:text-rose-600 dark:border-slate-800 dark:hover:border-rose-800 dark:hover:bg-rose-950/50 dark:hover:text-rose-400 transition-colors"
        title="Remove icon"
        aria-label="Remove icon"
        onclick={() => triggerChange(null)}
      >
        <X class="size-3.5" />
      </button>
    {/if}
  </div>

  <!-- Responsive Modal Overlay (Centered, Full-screen Backdrop, Never Clipped) -->
  {#if open}
    <div
      use:portal
      class="fixed inset-0 z-[999] flex items-center justify-center bg-slate-900/60 backdrop-blur-xs p-3 sm:p-4"
      role="dialog"
      aria-modal="true"
      aria-label="Select Icon"
    >
      <!-- Backdrop button to close on click outside -->
      <button
        type="button"
        class="absolute inset-0 size-full cursor-default bg-transparent"
        aria-label="Close dialog"
        onclick={() => (open = false)}
      ></button>

      <!-- Modal Card -->
      <div
        class="relative flex w-full max-w-xl max-h-[85vh] flex-col rounded-2xl border border-slate-200 bg-white shadow-2xl dark:border-slate-800 dark:bg-slate-900 overflow-hidden animate-in fade-in-50 zoom-in-95 duration-150 z-10"
      >
        <!-- Modal Header -->
        <div class="flex items-center justify-between border-b border-slate-200 px-4 py-3 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-900/70">
          <div class="flex items-center gap-2">
            <div class="flex size-7 items-center justify-center rounded-lg bg-indigo-50 text-indigo-600 dark:bg-indigo-950/80 dark:text-indigo-400 border border-indigo-200/60 dark:border-indigo-800/60 font-semibold">
              <Sparkles class="size-4" />
            </div>
            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100">Select Vector Icon</h3>
              <p class="text-[11px] text-slate-500 dark:text-slate-400">1,686 Lucide vector icons · Click any icon to apply</p>
            </div>
          </div>
          <button
            type="button"
            class="rounded-lg p-1 text-slate-400 hover:bg-slate-200/80 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200 transition-colors"
            onclick={() => (open = false)}
            aria-label="Close"
          >
            <X class="size-4" />
          </button>
        </div>

        <!-- Controls: Search Input & Category Filter Pills -->
        <div class="p-3 border-b border-slate-200 dark:border-slate-800 space-y-2.5 bg-white dark:bg-slate-900">
          <div class="relative flex items-center">
            <Search class="absolute left-3 size-4 text-slate-400 pointer-events-none" />
            <input
              type="text"
              class="w-full rounded-xl border border-slate-200 bg-slate-50/70 pl-9 pr-8 py-2 text-xs text-slate-900 outline-none transition-colors placeholder:text-slate-400 focus:border-indigo-500 focus:bg-white focus:ring-1 focus:ring-indigo-500 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-100 dark:focus:bg-slate-950"
              placeholder={`Search ${ICON_NAMES.length} icons…`}
              bind:value={query}
              autofocus
            />
            {#if query}
              <button
                type="button"
                class="absolute right-2.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300"
                onclick={() => (query = "")}
                aria-label="Clear search"
              >
                <X class="size-4" />
              </button>
            {/if}
          </div>

          <!-- Category Filter Pills -->
          <div class="flex items-center gap-1.5 overflow-x-auto pb-0.5 text-[11px] no-scrollbar">
            {#each CATEGORIES as cat (cat.id)}
              <button
                type="button"
                class={"rounded-full px-3 py-1 font-medium whitespace-nowrap transition-colors " +
                  (selectedCategory === cat.id
                    ? "bg-indigo-600 text-white font-semibold shadow-2xs"
                    : "bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-400 dark:hover:bg-slate-700")}
                onclick={() => {
                  selectedCategory = cat.id;
                  query = "";
                }}
              >
                {cat.label}
              </button>
            {/each}
          </div>
        </div>

        <!-- Live Preview / Selection Bar -->
        <div class="flex items-center justify-between px-3.5 py-2 bg-slate-50/80 dark:bg-slate-950/60 border-b border-slate-200 dark:border-slate-800 text-xs">
          <div class="flex items-center gap-2.5 min-w-0">
            <div class="flex size-7 shrink-0 items-center justify-center rounded-lg bg-white dark:bg-slate-900 border border-indigo-200/80 dark:border-indigo-800/80 text-indigo-600 dark:text-indigo-400 shadow-2xs">
              <Icon name={activeDisplayIcon} class="size-4" />
            </div>
            <div class="flex items-center gap-2 min-w-0">
              <span class="font-mono text-xs font-bold text-slate-900 dark:text-slate-100 truncate">
                {hoveredIcon || value || "Hover or select an icon"}
              </span>
              {#if value && !hoveredIcon}
                <span class="rounded bg-indigo-50 dark:bg-indigo-950/60 px-1.5 py-0.2 text-[10px] font-semibold text-indigo-600 dark:text-indigo-300">
                  Current
                </span>
              {/if}
            </div>
          </div>

          {#if value}
            <button
              type="button"
              class="flex items-center gap-1 text-[11px] font-medium text-rose-600 hover:text-rose-700 dark:text-rose-400 hover:underline shrink-0"
              onclick={() => {
                triggerChange(null);
                open = false;
              }}
            >
              <Trash2 class="size-3" />
              <span>Clear</span>
            </button>
          {/if}
        </div>

        <!-- Responsive Icon Grid -->
        <div
          class="grid flex-1 overflow-y-auto p-3 grid-cols-6 sm:grid-cols-8 md:grid-cols-10 gap-1.5 sm:gap-2 max-h-[46vh] bg-white dark:bg-slate-900"
          role="listbox"
          aria-label="Available Icons"
        >
          {#each matches as name (name)}
            {@const isSelected = value === name}
            <button
              type="button"
              role="option"
              aria-selected={isSelected}
              title={name}
              class={"group relative flex aspect-square items-center justify-center rounded-xl p-1.5 transition-all " +
                (isSelected
                  ? "bg-indigo-600 text-white font-bold ring-2 ring-indigo-600 ring-offset-2 dark:ring-offset-slate-900 shadow-sm"
                  : "border border-slate-200/80 bg-slate-50/50 text-slate-700 hover:border-indigo-400 hover:bg-indigo-50 hover:text-indigo-600 dark:border-slate-800 dark:bg-slate-800/40 dark:text-slate-300 dark:hover:border-indigo-600 dark:hover:bg-indigo-950/60 dark:hover:text-indigo-300 shadow-2xs")}
              onmouseenter={() => (hoveredIcon = name)}
              onmouseleave={() => (hoveredIcon = null)}
              onclick={() => {
                triggerChange(name);
                open = false;
              }}
            >
              <Icon {name} class="size-5 transition-transform group-hover:scale-110" />
              {#if isSelected}
                <div class="absolute -top-1 -right-1 flex size-3.5 items-center justify-center rounded-full bg-white text-indigo-600 shadow-xs border border-indigo-200">
                  <Check class="size-2.5 stroke-[3]" />
                </div>
              {/if}
            </button>
          {/each}

          {#if matches.length === 0}
            <div class="col-span-full py-10 text-center text-slate-400">
              <p class="text-xs font-semibold text-slate-600 dark:text-slate-300">No icons match "{query}"</p>
              <button
                type="button"
                class="mt-1.5 text-xs text-indigo-500 hover:underline font-medium"
                onclick={() => {
                  query = "";
                  selectedCategory = "all";
                }}
              >
                Reset search
              </button>
            </div>
          {/if}
        </div>

        <!-- Footer -->
        <div class="flex items-center justify-between border-t border-slate-200 px-4 py-2.5 bg-slate-50/80 dark:border-slate-800 dark:bg-slate-900/80 text-[11px] text-slate-500 dark:text-slate-400">
          <span>Showing {matches.length} icons</span>
          <button
            type="button"
            class="rounded-lg border border-slate-200 px-3 py-1 font-semibold text-slate-700 hover:bg-slate-100 dark:border-slate-700 dark:text-slate-200 dark:hover:bg-slate-800 transition-colors"
            onclick={() => (open = false)}
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
