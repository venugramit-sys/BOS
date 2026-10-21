<script>
  import { onMount } from "svelte";
  import { AppShell, UserProfilePage, SignIn, PageSurface, EnvironmentDialog, shell } from "$lib/components/shell/index.js";
  import { StudioWorkspace } from "$lib/components/studio/index.js";
  import { MenuHelpers } from "$lib/bos-ui/menu/index.js";
  import {
    Search,
    Layers,
    Table,
    ShieldAlert,
    ArrowRight,
  } from "lucide-svelte";

  // URL Path State for Superuser Dev-Studio vs Business User Workspace
  let currentPath = $state(
    typeof window !== "undefined"
      ? (window.location.pathname.replace(/\/$/, "") || "/")
      : "/"
  );

  let isDevStudio = $derived(
    currentPath === "/studio" ||
    currentPath.startsWith("/studio") ||
    currentPath === "/dev-studio" ||
    currentPath.startsWith("/dev-studio") ||
    currentPath.includes("dev-studio")
  );

  onMount(() => {
    const handlePopState = () => {
      currentPath = window.location.pathname.replace(/\/$/, "") || "/";
    };
    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  });

  // Clean End-User Menubar Actions (Zero Developer Clutter on '/')
  let liveMenus = $derived([
    {
      id: "file",
      label: "File",
      items: [
        {
          id: "search",
          label: "Command Palette...",
          icon: Search,
          shortcut: "Ctrl+K",
          action: () => (shell.isSearchOpen = true)
        },
        MenuHelpers.separator(),
        {
          id: "env-selector",
          label: "Switch Company / Tenant...",
          action: () => shell.openEnvironmentDialog()
        },
        MenuHelpers.separator(),
        {
          id: "close-tab",
          label: "Close Current Tab",
          shortcut: "Ctrl+W",
          disabled: shell.openTabs.length === 0,
          action: () => {
            if (shell.activeTabId) shell.closeTab(shell.activeTabId);
          }
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
        },
        ...(shell.currentUser?.role === "SUPER" || shell.currentUser?.role === "Global Admin" || shell.currentUser?.role === "Admin" ? [
          MenuHelpers.separator(),
          {
            id: "open-dev-studio",
            label: "Open Developer Studio (/studio)",
            icon: Table,
            action: () => {
              shell.navigate("/studio");
            }
          }
        ] : [])
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
  ]);
</script>

<svelte:head>
  <title>
    {isDevStudio
      ? "BOS Developer Studio · Superuser Meta-Objects & Monaco IDE"
      : shell.activeTab
      ? `${shell.activeTab.title} · BOS`
      : `${shell.activeCompany || 'BOS'} · Workspace`}
  </title>
</svelte:head>

{#if !shell.currentUser}
  <SignIn />
{:else}
  {#if shell.isEmergencySession || shell.maintenanceMode}
    <div class="w-full bg-amber-500 text-amber-950 px-4 py-2 text-xs font-medium flex items-center justify-between shrink-0 shadow-sm z-50">
      <div class="flex items-center gap-2">
        <ShieldAlert class="size-4 shrink-0 text-amber-900" />
        <span>
          <strong>Emergency Super Admin Mode:</strong>
          {shell.emergencyWarning || (shell.maintenanceMode ? "Node is running in Maintenance Mode." : "Primary auth fallback active.")}
        </span>
      </div>
      {#if !isDevStudio}
        <button
          type="button"
          onclick={() => shell.navigate("/studio")}
          class="inline-flex items-center gap-1.5 px-3 py-1 rounded-md bg-amber-900 text-amber-100 hover:bg-amber-950 text-xs font-semibold cursor-pointer transition-colors shadow-xs"
        >
          <span>Open Dev Studio to Repair .BOS Scripts</span>
          <ArrowRight class="size-3.5" />
        </button>
      {/if}
    </div>
  {/if}

  {#if isDevStudio}
    <!-- DEDICATED SUPERUSER ROUTE: /dev-studio -->
    <div class="w-screen h-screen overflow-hidden flex flex-col">
      <StudioWorkspace theme={shell.theme} />
    </div>
  {:else}
    <!-- CLEAN BUSINESS USER APPLICATION: '/' -->
    <AppShell menus={liveMenus}>
    <div class="flex-1 flex flex-col p-4 sm:p-6 lg:p-8 max-w-6xl w-full mx-auto justify-center">
      {#if shell.openTabs.length === 0}
        <!-- 100% Blank Clean Workspace Ready for User Records -->
        <div class="flex flex-col items-center justify-center text-center py-24 px-4 space-y-4 max-w-md mx-auto select-none">
          <!-- Enterprise Logo -->
          <div class="size-14 rounded-2xl bg-indigo-600/10 dark:bg-indigo-500/20 text-indigo-600 dark:text-indigo-400 flex items-center justify-center shadow-xs">
            <Layers class="size-7 stroke-[1.75]" />
          </div>

          <div class="space-y-1">
            <h1 class="text-2xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
              {shell.activeCompany || "BOS Platform"}
            </h1>
            <p class="text-xs text-slate-400">
              {#if shell.isOnline}
                Connected to <span class="font-mono font-medium text-slate-700 dark:text-slate-300">{shell.activeDatabase}</span> ({shell.runtimeStatus}). No open documents.
              {:else}
                Connecting to backend runtime...
              {/if}
            </p>
          </div>

          <p class="text-xs text-slate-400 max-w-sm leading-relaxed">
            Workspace ready for business operations. Open an operational document or card from the left navigation, or press <kbd class="font-mono px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-[10px]">Ctrl+K</kbd> to search records.
          </p>
        </div>
      {:else}
        <!-- Active Open Document View (Business Records / Profile) -->
        {#if shell.activeTab.type === "profile" || shell.activeTab.id === "user-profile"}
          <UserProfilePage />
        {:else if shell.activeTab.type === "page" || shell.activeTab.type === "report"}
          <PageSurface tab={shell.activeTab} />
        {:else}
          <div class="space-y-4">
            <div class="flex items-center justify-between pb-3 border-b border-slate-200 dark:border-slate-800">
              <div>
                <h2 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
                  {shell.activeTab.title}
                </h2>
                <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5 font-mono">
                  id: {shell.activeTab.id}
                </p>
              </div>
              <button
                onclick={() => shell.closeTab(shell.activeTab.id)}
                class="px-2.5 py-1 rounded-md text-xs font-medium text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer"
              >
                Close Document
              </button>
            </div>

            <div class="p-8 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-center text-xs text-slate-400">
              Document workspace ready for real records.
            </div>
          </div>
        {/if}
      {/if}
    </div>
  </AppShell>
  {/if}
{/if}

<!-- Global Modals Accessible Everywhere (Sign-in, Studio, Workspace) -->
<EnvironmentDialog />
