<script>
  import { onMount } from "svelte";
  import { AppShell, UserProfilePage, SignIn, PageSurface, TemplatesGallery, EnvironmentDialog, DatabaseManagerDialog, shell } from "$lib/components/shell/index.js";
  import { StudioWorkspace } from "$lib/components/studio/index.js";
  import { ToastContainer, AlertModal } from "$lib/components/notifications/index.js";
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
  let currentHash = $state(
    typeof window !== "undefined" ? window.location.hash : ""
  );

  let isDevStudio = $derived(
    currentPath === "/studio" ||
    currentPath.startsWith("/studio") ||
    currentPath === "/dev-studio" ||
    currentPath.startsWith("/dev-studio") ||
    currentPath.includes("dev-studio") ||
    currentHash.startsWith("#/studio") ||
    currentHash.startsWith("#/dev-studio")
  );

  onMount(() => {
    const handleUrlChange = () => {
      currentPath = window.location.pathname.replace(/\/$/, "") || "/";
      currentHash = window.location.hash;
    };
    window.addEventListener("popstate", handleUrlChange);
    window.addEventListener("hashchange", handleUrlChange);
    return () => {
      window.removeEventListener("popstate", handleUrlChange);
      window.removeEventListener("hashchange", handleUrlChange);
    };
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
      : `${shell.companyDisplayName} · Workspace`}
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
      {#if shell.openTabs.length > 0}
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
      {:else}
        <!-- Modern Executive Dashboard & Ready-Made Templates Gallery -->
        <TemplatesGallery />
      {/if}
    </div>
  </AppShell>
  {/if}
{/if}

<!-- Global Modals & Notifications Accessible Everywhere (Sign-in, Studio, Workspace) -->
<ToastContainer />
<AlertModal />
<EnvironmentDialog />

