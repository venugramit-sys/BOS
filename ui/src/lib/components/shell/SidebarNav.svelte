<script>
  import { shell } from "./shellStore.svelte.js";
  import NavSwitcher from "./NavSwitcher.svelte";
  import NavMain from "./NavMain.svelte";
  import NavUser from "./NavUser.svelte";
  import { cn } from "$lib/utils.js";

  let { onNavigate = undefined } = $props();

  let isCollapsed = $derived(shell.isSidebarCollapsed);
</script>

<aside
  class={cn(
    "h-full flex flex-col bg-slate-50 dark:bg-slate-950 border-r border-slate-200/80 dark:border-slate-800/80 shrink-0 select-none transition-all duration-200 ease-in-out relative z-20 overflow-hidden",
    isCollapsed ? "w-0 border-r-0 opacity-0 pointer-events-none" : "w-64 opacity-100"
  )}
>
  <!-- 1. Header: Company / Workspace Switcher -->
  <NavSwitcher collapsed={false} />

  <!-- 2. Main Content: Role-Tailored Navigation Menu -->
  <NavMain collapsed={false} {onNavigate} />

  <!-- 3. Footer: User Profile & Session Controls -->
  <NavUser collapsed={false} />
</aside>
