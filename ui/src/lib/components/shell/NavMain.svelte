<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    LayoutDashboard,
    Database,
    Users,
    ShieldCheck,
    Activity,
    FileText,
    Sliders,
    Settings,
    Table,
    ChevronRight,
    Building2,
    ArrowRightLeft,
    Layers,
    Compass,
    Sparkles
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { collapsed = false, onNavigate = undefined } = $props();

  // Static icon lookup map for fast rendering
  const ICON_MAP = {
    LayoutDashboard,
    Database,
    Users,
    ShieldCheck,
    Activity,
    FileText,
    Sliders,
    Settings,
    Table,
    Building2,
    ArrowRightLeft,
    Layers,
    Compass,
    Sparkles
  };

  function getIcon(name) {
    if (!name) return LayoutDashboard;
    const clean = String(name).trim();
    return ICON_MAP[clean] || LayoutDashboard;
  }

  // 1. Resolve registered menu meta-objects from active DB
  let menuMetaObjects = $derived(
    shell.metaObjects.filter((o) => o.object_type === "menu")
  );

  // 2. Resolve Role-Tailored Navigation Hierarchy - 100% Real Database Derived (Zero Mocks)
  let activeMenu = $derived.by(() => {
    const userRoles = shell.currentUser?.roles || (shell.currentUser?.role ? [shell.currentUser.role] : ["User"]);
    const isSuper = userRoles.includes("SUPER") || shell.currentUser?.role === "SUPER" || shell.currentUser?.role === "Global Admin";

    // 1. Match user's primary/active role against real database menus
    for (const r of userRoles) {
      const match = menuMetaObjects.find((m) => {
        const def = m.definition || m;
        return String(def.role || "").toLowerCase() === String(r).toLowerCase();
      });
      if (match?.definition?.items?.length) return match.definition;
      if (match?.items?.length) return match;
    }

    // 2. If Superuser, match any SuperAdmin menu from database
    if (isSuper) {
      const superMatch = menuMetaObjects.find((m) => {
        const def = m.definition || m;
        return (
          String(def.name || m.object_name).toLowerCase().includes("super") ||
          String(def.role || "").toLowerCase() === "super"
        );
      });
      if (superMatch?.definition?.items?.length) return superMatch.definition;
      if (superMatch?.items?.length) return superMatch;
    }

    // 3. Match any registered database menu that has loaded items
    for (const m of menuMetaObjects) {
      if (m.definition?.items?.length) return m.definition;
      if (m.items?.length) return m;
    }

    // Zero mocks, zero fake fallbacks
    return null;
  });

  // Groups derived strictly from the real database menu
  let primaryGroups = $derived(
    (activeMenu?.items || []).filter(
      (g) => String(g.name || "").toLowerCase() !== "secondary"
    )
  );

  // Collapsed state for groups/items
  let openSubmenus = $state({});

  function toggleSubmenu(id) {
    openSubmenus[id] = !openSubmenus[id];
  }

  function handleItemClick(item) {
    const pageTarget = item.page || item.id || item.name;

    if (pageTarget === "databases") {
      shell.openDbManager();
      onNavigate?.(item);
      return;
    }

    if (pageTarget === "dev-studio") {
      shell.navigate("/studio");
      onNavigate?.(item);
      return;
    }

    if (pageTarget === "RoleCenter") {
      shell.activeTabId = "";
      onNavigate?.(item);
      return;
    }

    shell.openTab({
      id: pageTarget,
      title: item.label || item.caption || item.name,
      type: "page"
    });
    onNavigate?.(item);
  }
</script>

<div class="flex-1 overflow-y-auto px-2 py-3 space-y-4">
  {#each primaryGroups as group}
    <div class="space-y-1">
      {#if !collapsed}
        <div class="px-2.5 py-1 text-[11px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider select-none">
          {group.label || group.caption || group.name}
        </div>
      {/if}

      <div class="space-y-0.5">
        {#each group.items || [] as item}
          {@const hasChildren = Array.isArray(item.items) && item.items.length > 0}
          {@const targetPage = item.page || item.id || item.name}
          {@const isActive = shell.activeTabId === targetPage || (!shell.activeTabId && targetPage === "RoleCenter")}
          {@const IconCmp = getIcon(item.icon)}

          <div class="relative">
            <button
              type="button"
              onclick={() => (hasChildren ? toggleSubmenu(item.name) : handleItemClick(item))}
              class={cn(
                "w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all text-left cursor-pointer select-none group",
                isActive
                  ? "bg-slate-200/90 dark:bg-slate-800/90 text-slate-900 dark:text-slate-100 shadow-2xs font-semibold"
                  : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-200/50 dark:hover:bg-slate-800/50",
                collapsed && "justify-center px-0 py-2"
              )}
              title={collapsed ? item.label || item.name : undefined}
            >
              <IconCmp class={cn("size-4 shrink-0 transition-colors", isActive ? "text-indigo-600 dark:text-indigo-400" : "text-slate-400 dark:text-slate-500 group-hover:text-slate-600 dark:group-hover:text-slate-300")} />

              {#if !collapsed}
                <span class="truncate flex-1">{item.label || item.caption || item.name}</span>

                {#if item.badge_field || item.badge}
                  <span class="px-1.5 py-0.2 rounded text-[10px] font-semibold uppercase tracking-wider bg-indigo-100 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300">
                    {item.badge_field || item.badge}
                  </span>
                {/if}

                {#if hasChildren}
                  <ChevronRight
                    class={cn(
                      "size-3.5 text-slate-400 transition-transform duration-200 shrink-0",
                      openSubmenus[item.name] && "rotate-90"
                    )}
                  />
                {/if}
              {/if}
            </button>

            <!-- Nested Submenu -->
            {#if hasChildren && openSubmenus[item.name] && !collapsed}
              <div class="ml-4 pl-2 border-l border-slate-200 dark:border-slate-800 space-y-0.5 mt-0.5">
                {#each item.items as subItem}
                  {@const subActive = shell.activeTabId === (subItem.page || subItem.name)}
                  <button
                    type="button"
                    onclick={() => handleItemClick(subItem)}
                    class={cn(
                      "w-full flex items-center gap-2 px-2 py-1 rounded-md text-[11px] font-medium transition-colors text-left cursor-pointer",
                      subActive
                        ? "text-indigo-600 dark:text-indigo-400 font-semibold bg-indigo-50/50 dark:bg-indigo-950/30"
                        : "text-slate-500 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-800/50"
                    )}
                  >
                    <span class="truncate">{subItem.label || subItem.name}</span>
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/each}

  <!-- Ready-Made Templates Section -->
  <div class="space-y-1 pt-2 border-t border-slate-200/60 dark:border-slate-800/60">
    {#if !collapsed}
      <div class="px-2.5 py-1 text-[11px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider select-none flex items-center justify-between">
        <span>Templates Gallery</span>
        <span class="text-[9px] px-1 rounded bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 font-mono">14 READY</span>
      </div>
    {/if}

    <button
      type="button"
      onclick={() => { shell.activeTabId = ""; onNavigate?.({ name: "TemplatesGallery" }); }}
      class={cn(
        "w-full flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all text-left cursor-pointer select-none group",
        shell.activeTabId === ""
          ? "bg-slate-200/90 dark:bg-slate-800/90 text-slate-900 dark:text-slate-100 shadow-2xs font-semibold"
          : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-200/50 dark:hover:bg-slate-800/50",
        collapsed && "justify-center px-0 py-2"
      )}
      title="Ready-Made Modern Templates Gallery"
    >
      <Sparkles class="size-4 shrink-0 text-indigo-600 dark:text-indigo-400" />
      {#if !collapsed}
        <span class="truncate flex-1">Template Showcase</span>
      {/if}
    </button>
  </div>
</div>
