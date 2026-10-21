<script>
  import { Menubar } from "$lib/bos-ui/menu/index.js";
  import { Search, X, Bell, Database } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import { shell } from "./shellStore.svelte.js";
  import { notificationStore } from "$lib/services/notificationStore.svelte.js";
  import UserWidget from "./UserWidget.svelte";

  let {
    menus = [],
    user = null,
    searchQuery = $bindable(""),
    searchPlaceholder = "Search (⌘K)...",
    onSearch,
    onOpenSearch,
    class: className = ""
  } = $props();

  let inputElement = $state(null);

  // Fallback real menus if none provided
  let resolvedMenus = $derived(
    menus.length > 0
      ? menus
      : [
          {
            id: "file",
            label: "File",
            items: [
              {
                id: "db-manager",
                label: "Database Manager...",
                icon: Database,
                action: () => shell.openDbManager()
              },
              {
                id: "close-tab",
                label: "Close Tab",
                shortcut: "Ctrl+W",
                disabled: shell.openTabs.length === 0,
                action: () => {
                  if (shell.activeTabId) shell.closeTab(shell.activeTabId);
                }
              }
            ]
          },
          {
            id: "database",
            label: "Database",
            items: [
              {
                id: "open-db-mgr",
                label: "Database Manager...",
                icon: Database,
                action: () => shell.openDbManager()
              },
              {
                id: "open-env-dlg",
                label: "Switch Environment...",
                action: () => shell.openEnvironmentDialog()
              }
            ]
          },
          {
            id: "view",
            label: "View",
            items: [
              {
                id: "toggle-sidebar",
                label: shell.isSidebarCollapsed ? "Expand Sidebar" : "Collapse Sidebar",
                shortcut: "Ctrl+B",
                action: () => shell.toggleSidebar()
              },
              {
                id: "toggle-theme",
                label: shell.theme === "dark" ? "Switch to Light Mode" : "Switch to Dark Mode",
                action: () => shell.toggleTheme()
              }
            ]
          },
          {
            id: "help",
            label: "Help",
            items: [
              {
                id: "refresh-sys",
                label: "Refresh System Health",
                action: () => shell.refreshSystem()
              }
            ]
          }
        ]
  );

  function handleKeyDown(e) {
    if (e.key === "Enter") {
      e.preventDefault();
      onSearch?.(searchQuery);
    } else if (e.key === "Escape") {
      inputElement?.blur();
    }
  }

  function handleGlobalKeydown(e) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      inputElement?.focus();
      onOpenSearch?.();
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<header
  class={cn(
    "h-10 border-b border-slate-200 dark:border-slate-800 bg-white/95 dark:bg-slate-900/95 backdrop-blur-xs px-2 sm:px-3 flex items-center justify-between gap-2 sm:gap-4 z-30 shrink-0 select-none transition-colors",
    className
  )}
>
  <!-- 1. Left: Dynamic Menubar -->
  <div class="flex items-center min-w-0 overflow-x-auto scrollbar-none">
    <Menubar
      menus={resolvedMenus}
      class="border-0 bg-transparent shadow-none h-8 px-0 rounded-none gap-0.5"
      triggerClass="h-7 px-2.5 text-xs text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 rounded-md transition-colors"
    />
  </div>

  <!-- 2. Right: Search Input + System Notifications + Dedicated User Widget -->
  <div class="flex items-center gap-1.5 sm:gap-2.5 shrink-0">
    <!-- Search Input -->
    <div class="relative w-36 sm:w-52 md:w-64 lg:w-72">
      <Search class="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-slate-400 dark:text-slate-500 pointer-events-none" />
      <input
        bind:this={inputElement}
        type="text"
        role="searchbox"
        autocomplete="off"
        spellcheck="false"
        bind:value={searchQuery}
        onkeydown={handleKeyDown}
        onfocus={() => onOpenSearch?.()}
        placeholder={searchPlaceholder}
        class="w-full h-7 sm:h-8 pl-8 pr-8 rounded-md text-xs bg-slate-100/90 dark:bg-slate-800/90 border border-transparent hover:border-slate-300 dark:hover:border-slate-700 focus:border-indigo-500 dark:focus:border-indigo-400 focus:bg-white dark:focus:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 dark:placeholder-slate-500 outline-none transition-all focus:ring-1 focus:ring-indigo-500/20"
      />
      {#if searchQuery}
        <button
          type="button"
          onclick={() => {
            searchQuery = "";
            onSearch?.("");
          }}
          class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer p-0.5"
          title="Clear search"
          aria-label="Clear search"
        >
          <X class="size-3" />
        </button>
      {:else}
        <kbd class="hidden sm:inline-flex absolute right-2 top-1/2 -translate-y-1/2 px-1.5 py-0.5 text-[10px] font-mono text-slate-400 dark:text-slate-500 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded shadow-2xs pointer-events-none">
          ⌘K
        </kbd>
      {/if}
    </div>

    <!-- Notifications Bell Button -->
    <button
      type="button"
      onclick={() => {
        shell.isNotificationsOpen = !shell.isNotificationsOpen;
        if (shell.isNotificationsOpen) {
          notificationStore.markAllRead();
        }
      }}
      class="relative p-1.5 rounded-lg text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
      title="Notifications Center"
      aria-label="Toggle notifications center"
    >
      <Bell class="size-3.5" />
      {#if notificationStore.unreadCount > 0}
        <span class="absolute -top-1 -right-1 min-w-4 h-4 px-1 rounded-full bg-indigo-600 text-[10px] font-bold text-white flex items-center justify-center shadow-xs">
          {notificationStore.unreadCount > 9 ? "9+" : notificationStore.unreadCount}
        </span>
      {:else if notificationStore.history.length > 0}
        <span class="absolute top-1 right-1 size-1.5 rounded-full bg-slate-400"></span>
      {/if}
    </button>

    <!-- Subtle Divider -->
    <div class="h-4 w-px bg-slate-200 dark:bg-slate-800 hidden sm:block"></div>

    <!-- Dedicated User Widget (Icon with Dropdown: Name, Designation, Theme Changer) -->
    <UserWidget {user} />
  </div>
</header>
