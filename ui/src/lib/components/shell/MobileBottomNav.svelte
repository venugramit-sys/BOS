<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    LayoutDashboard,
    Search,
    Plus,
    Layers,
    Menu,
    X
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { onOpenSearch, onOpenQuickCreate, onOpenTabs, forceMobile = false } = $props();
</script>

<!-- Thumb-Zone Bottom Navigation Bar -->
<nav class={cn("h-14 border-t border-slate-200 dark:border-slate-800 bg-white/95 dark:bg-slate-900/95 backdrop-blur-md px-3 flex items-center justify-around z-30 shrink-0 select-none", forceMobile ? "flex" : "flex md:hidden")}>
  <!-- Home Tab -->
  <button
    onclick={() => (shell.activeTabId = "dashboard")}
    class={cn(
      "flex flex-col items-center justify-center gap-1 text-[10px] font-medium transition-colors cursor-pointer",
      shell.activeTabId === "dashboard"
        ? "text-indigo-600 dark:text-indigo-400"
        : "text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
    )}
  >
    <LayoutDashboard class="size-5" />
    <span>Home</span>
  </button>

  <!-- Search Button -->
  <button
    onclick={() => onOpenSearch?.()}
    class="flex flex-col items-center justify-center gap-1 text-[10px] font-medium text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors cursor-pointer"
  >
    <Search class="size-5" />
    <span>Search</span>
  </button>

  <!-- Centered Floating Action Button '+' -->
  <button
    onclick={() => onOpenQuickCreate?.()}
    class="size-11 -mt-5 rounded-full bg-indigo-600 text-white flex items-center justify-center shadow-lg hover:bg-indigo-700 transition-all cursor-pointer active:scale-95"
    title="Create New Document"
  >
    <Plus class="size-6 stroke-[2.5]" />
  </button>

  <!-- Open Tabs Switcher -->
  <button
    onclick={() => onOpenTabs?.()}
    class="flex flex-col items-center justify-center gap-1 text-[10px] font-medium text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors cursor-pointer relative"
  >
    <Layers class="size-5" />
    <span>Tabs</span>
    {#if shell.openTabs.length > 1}
      <span class="absolute top-0 right-1 size-3.5 rounded-full bg-indigo-600 text-white text-[9px] font-bold flex items-center justify-center">
        {shell.openTabs.length}
      </span>
    {/if}
  </button>

  <!-- Module Menu Drawer Toggle -->
  <button
    onclick={() => shell.toggleMobileDrawer()}
    class="flex flex-col items-center justify-center gap-1 text-[10px] font-medium text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors cursor-pointer"
  >
    <Menu class="size-5" />
    <span>Menu</span>
  </button>
</nav>
