<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    LayoutDashboard,
    Database,
    Table,
    FileCode,
    Layers,
    Settings,
    GitBranch,
    Compass,
    ChevronRight,
    Users,
    ShieldCheck,
    Activity,
    FileText,
    Sliders
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { onNavigate } = $props();

  // 100% Real Navigation Sections for Business Users
  let navSections = $derived([
    {
      title: "Core Platform",
      items: [
        { id: "dashboard", label: "Role Center", icon: LayoutDashboard },
        { id: "databases", label: "Databases & Companies", icon: Database }
      ]
    },
    {
      title: "Identity & Access",
      items: [
        { id: "UserList", label: "Users", icon: Users, pageType: "page" },
        { id: "RoleList", label: "Roles & Permissions", icon: ShieldCheck, pageType: "page" },
        { id: "SessionList", label: "Active Sessions", icon: Activity, pageType: "page" },
        { id: "SecurityAuditLogList", label: "Audit Trail", icon: FileText, pageType: "page" },
        { id: "AuthConfigurationCard", label: "Login & Security Branding", icon: Sliders, pageType: "page" }
      ]
    },
    {
      title: "Operations & Setup",
      items: [
        { id: "settings", label: "System Setup", icon: Settings }
      ]
    },
    ...(shell.currentUser?.role === "Global Admin" ? [
      {
        title: "Superuser Tools",
        items: [
          {
            id: "dev-studio",
            label: "Developer Studio",
            icon: Table,
            href: "/dev-studio",
            badge: "Dev"
          }
        ]
      }
    ] : [])
  ]);

  let catalogPages = $derived(
    shell.metaObjects.filter((obj) => obj.object_type === "page" || obj.object_type === "report")
  );

  function openCatalogObject(obj) {
    shell.openTab({
      id: obj.object_name,
      title: obj.caption || obj.object_name,
      type: obj.object_type,
    });
  }

  function handleItemClick(item) {
    if (item.id === "databases") {
      shell.openDbManager();
      onNavigate?.(item);
      return;
    }

    if (item.href) {
      if (item.target === "_blank") {
        window.open(item.href, "_blank");
      } else {
        window.location.href = item.href;
      }
      return;
    }

    if (item.pageType === "page") {
      shell.openTab({ id: item.id, title: item.label, type: "page" });
      onNavigate?.(item);
      return;
    }

    shell.openTab({ id: item.id, title: item.label, type: "document" });
    onNavigate?.(item);
  }
</script>

<aside
  class={cn(
    "h-full border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex flex-col shrink-0 select-none transition-all duration-200",
    shell.isSidebarCollapsed ? "w-14" : "w-60"
  )}
>
  <!-- Navigation Header (Expanded only) -->
  {#if !shell.isSidebarCollapsed}
    <div class="p-3 border-b border-slate-100 dark:border-slate-800/80 flex items-center justify-between">
      <span class="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
        Navigation
      </span>
      {#if shell.metaObjects.length > 0}
        <span class="text-[10px] px-1.5 py-0.2 rounded font-mono bg-slate-100 dark:bg-slate-800 text-slate-500">
          {shell.metaObjects.length} objects
        </span>
      {/if}
    </div>
  {/if}

  <!-- Scrollable Nav List -->
  <div class="flex-1 overflow-y-auto p-2 space-y-4">
    {#each navSections as section}
      <div class="space-y-1">
        {#if !shell.isSidebarCollapsed}
          <div class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-400">
            {section.title}
          </div>
        {/if}

        {#each section.items as item}
          {@const IconComp = item.icon}
          <div>
            <button
              onclick={() => handleItemClick(item)}
              class={cn(
                "w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-xs font-medium cursor-pointer transition-colors",
                shell.activeTabId === item.id
                  ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/60 dark:text-indigo-300 font-semibold"
                  : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 hover:text-slate-900 dark:hover:text-slate-100"
              )}
              title={shell.isSidebarCollapsed ? item.label : undefined}
            >
              <IconComp class="size-4 shrink-0 opacity-80" />

              {#if !shell.isSidebarCollapsed}
                <span class="truncate flex-1 text-left">{item.label}</span>

                {#if item.badge}
                  <span class="px-1.5 py-0.2 rounded text-[10px] font-mono font-medium bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300">
                    {item.badge}
                  </span>
                {/if}
              {/if}
            </button>
          </div>
        {/each}
      </div>
    {/each}
    {#if catalogPages.length && !shell.isSidebarCollapsed}
      <div class="space-y-1">
        <div class="px-2 py-1 text-[10px] font-bold uppercase tracking-wider text-slate-400">Pages</div>
        {#each catalogPages as obj}
          <button
            onclick={() => openCatalogObject(obj)}
            class="w-full text-left px-2.5 py-1.5 rounded-lg text-xs text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
          >
            {obj.caption || obj.object_name}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</aside>
