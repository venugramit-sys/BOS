<script>
  import { onMount } from "svelte";
  import { shell } from "./shellStore.svelte.js";
  import { notificationStore, ToastContainer, AlertModal } from "$lib/components/notifications/index.js";
  import TopNavBar from "./TopNavBar.svelte";
  import SidebarNav from "./SidebarNav.svelte";
  import WorkspaceTabs from "./WorkspaceTabs.svelte";
  import FactBoxDrawer from "./FactBoxDrawer.svelte";
  import MobileBottomNav from "./MobileBottomNav.svelte";
  import StatusBar from "./StatusBar.svelte";
  import { X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    children,
    menus = [],
    user = null,
    searchQuery = $bindable(""),
    searchPlaceholder = "Search (⌘K)...",
    onSearch,
    menubarSnippet,
    factboxSnippet,
    onOpenSearch,
    onOpenQuickCreate,
    forceMobile = false
  } = $props();

  onMount(() => {
    shell.initTheme();
    shell.refreshSystem();
    notificationStore.initSse();
    const interval = setInterval(() => {
      shell.refreshSystem();
    }, 10000);
    return () => {
      clearInterval(interval);
      notificationStore.destroy();
    };
  });
</script>

<div class="h-full w-full flex flex-col overflow-hidden bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 font-sans transition-colors">
  <!-- 1. Top Navigation Bar (Dynamic Menubar + Search Input + User Widget) -->
  <TopNavBar
    {menus}
    {user}
    bind:searchQuery
    {searchPlaceholder}
    {onSearch}
    {onOpenSearch}
  />

  <!-- 2. Context Action Menubar (Optional snippet slot) -->
  {#if menubarSnippet && !forceMobile}
    <div class="border-b border-slate-200 dark:border-slate-800 bg-white/90 dark:bg-slate-900/90 px-3 py-1 shrink-0">
      {@render menubarSnippet()}
    </div>
  {/if}

  <!-- 3. Multi-Document Workspace Tab Bar -->
  <WorkspaceTabs />

  <!-- 4. Main Operational Layout (Sidebar + Document Canvas + FactBox) -->
  <div class="flex-1 min-h-0 flex overflow-hidden relative">
    <!-- Desktop Left Sidebar -->
    <div class={cn("h-full", forceMobile ? "hidden" : "hidden md:flex")}>
      <SidebarNav />
    </div>

    <!-- Center Active Document Canvas Surface -->
    <main class="flex-1 overflow-y-auto flex flex-col bg-slate-50 dark:bg-slate-950 relative">
      {@render children?.()}
    </main>

    <!-- Context FactBox Sidecar (Desktop) -->
    {#if !forceMobile}
      <FactBoxDrawer {factboxSnippet} />
    {/if}
  </div>

  <!-- 5. Desktop System Status Bar -->
  {#if !forceMobile}
    <StatusBar />
  {/if}

  <!-- 6. Mobile-First Bottom Navigation Bar (Phone viewports) -->
  <MobileBottomNav
    {forceMobile}
    {onOpenSearch}
    {onOpenQuickCreate}
    onOpenTabs={() => {
      if (onOpenSearch) onOpenSearch();
      else shell.isSearchOpen = true;
    }}
  />

  <!-- Mobile Off-Canvas Navigation Drawer -->
  {#if shell.isMobileDrawerOpen}
    <div
      role="presentation"
      class="fixed inset-0 bg-slate-950/60 backdrop-blur-xs z-50 md:hidden flex"
      onclick={() => (shell.isMobileDrawerOpen = false)}
    >
      <div
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        class="w-4/5 max-w-xs h-full bg-white dark:bg-slate-900 shadow-2xl flex flex-col focus:outline-hidden"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div class="p-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <span class="font-bold text-sm">Navigation Modules</span>
          <button
            onclick={() => (shell.isMobileDrawerOpen = false)}
            class="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer"
          >
            <X class="size-4" />
          </button>
        </div>
        <div class="flex-1 overflow-y-auto">
          <SidebarNav onNavigate={() => (shell.isMobileDrawerOpen = false)} />
        </div>
      </div>
    </div>
  {/if}

  <!-- Notifications Slide-Over Drawer -->
  {#if shell.isNotificationsOpen}
    <div
      role="presentation"
      class="fixed inset-0 bg-slate-950/40 z-50 flex justify-end"
      onclick={() => (shell.isNotificationsOpen = false)}
    >
      <div
        role="dialog"
        aria-modal="true"
        tabindex="-1"
        class="w-84 max-w-full h-full bg-white dark:bg-slate-900 border-l border-slate-200 dark:border-slate-800 shadow-2xl flex flex-col focus:outline-hidden"
        onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.stopPropagation()}
      >
        <div class="p-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="font-bold text-xs uppercase tracking-wider">Notifications Center</span>
            {#if notificationStore.isSseConnected}
              <span class="size-2 rounded-full bg-emerald-500" title="Connected to real-time notification stream"></span>
            {:else}
              <span class="size-2 rounded-full bg-amber-500" title="Reconnecting notification stream..."></span>
            {/if}
          </div>
          <div class="flex items-center gap-2">
            {#if notificationStore.history.length > 0}
              <button
                type="button"
                onclick={() => notificationStore.clearHistory()}
                class="text-[11px] font-medium text-indigo-600 dark:text-indigo-400 hover:underline cursor-pointer"
              >
                Clear All
              </button>
            {/if}
            <button
              onclick={() => (shell.isNotificationsOpen = false)}
              class="p-1 rounded text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 cursor-pointer"
            >
              <X class="size-4" />
            </button>
          </div>
        </div>
        <div class="flex-1 overflow-y-auto p-4 space-y-3 text-xs">
          {#if notificationStore.history.length === 0}
            <div class="py-12 text-center text-slate-400 space-y-1">
              <div class="font-semibold text-slate-600 dark:text-slate-300">No new notifications</div>
              <div class="text-[11px]">System alerts, background job updates, and validations appear here.</div>
            </div>
          {:else}
            {#each notificationStore.history as item (item.id)}
              <div
                class={cn(
                  "p-3 rounded-xl border transition-colors",
                  item.severity === "error"
                    ? "border-rose-200 dark:border-rose-900/50 bg-rose-50/50 dark:bg-rose-950/20"
                    : item.severity === "warning"
                    ? "border-amber-200 dark:border-amber-900/50 bg-amber-50/50 dark:bg-amber-950/20"
                    : item.severity === "success"
                    ? "border-emerald-200 dark:border-emerald-900/50 bg-emerald-50/50 dark:bg-emerald-950/20"
                    : "border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60"
                )}
              >
                <div class="flex items-center justify-between gap-2 mb-1">
                  <div class="font-bold text-slate-800 dark:text-slate-200 truncate">
                    {item.title || (item.severity.toUpperCase())}
                  </div>
                  <span
                    class={cn(
                      "px-1.5 py-0.2 rounded text-[9px] font-mono uppercase tracking-wider",
                      item.severity === "error"
                        ? "bg-rose-100 dark:bg-rose-900/50 text-rose-700 dark:text-rose-300"
                        : item.severity === "warning"
                        ? "bg-amber-100 dark:bg-amber-900/50 text-amber-700 dark:text-amber-300"
                        : item.severity === "success"
                        ? "bg-emerald-100 dark:bg-emerald-900/50 text-emerald-700 dark:text-emerald-300"
                        : "bg-indigo-100 dark:bg-indigo-900/50 text-indigo-700 dark:text-indigo-300"
                    )}
                  >
                    {item.target || item.severity}
                  </span>
                </div>
                {#if item.message}
                  <div class="text-slate-600 dark:text-slate-400 leading-relaxed break-words">{item.message}</div>
                {/if}
                {#if item.field}
                  <div class="mt-1 text-[11px] font-mono text-rose-600 dark:text-rose-400">
                    Field: {item.field}
                  </div>
                {/if}
                <div class="text-[10px] text-slate-400 font-mono mt-2 flex items-center justify-between">
                  <span>{new Date(item.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}</span>
                  {#if item.code}
                    <span>{item.code}</span>
                  {/if}
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Unified Notification Overlays -->
  <ToastContainer />
  <AlertModal />
</div>
