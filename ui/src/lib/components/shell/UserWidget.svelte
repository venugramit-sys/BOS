<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    DropdownMenu,
    DropdownMenuTrigger,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuGroup,
    DropdownMenuGroupHeading,
    DropdownMenuShortcut
  } from "$lib/components/ui/dropdown-menu/index.js";
  import {
    User,
    Settings,
    Sun,
    Moon,
    Laptop,
    Shield,
    Keyboard,
    LogOut,
    Building2,
    Sparkles,
    ExternalLink,
    ArrowRightLeft
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    user = null,
    class: className = ""
  } = $props();

  let isOpen = $state(false);

  // Active user data (fallback to shell.currentUser)
  let activeUser = $derived(user || shell.currentUser);

  // Compute initials from user name (e.g., "Alex Morgan" -> "AM")
  let initials = $derived.by(() => {
    if (!activeUser?.name) return "U";
    return activeUser.name
      .split(" ")
      .filter(Boolean)
      .map((part) => part[0])
      .slice(0, 2)
      .join("")
      .toUpperCase();
  });

  // Themes list for theme changer
  const themeOptions = [
    { id: "light", label: "Light", icon: Sun },
    { id: "dark", label: "Dark", icon: Moon },
    { id: "system", label: "System", icon: Laptop }
  ];

  function handleSelectTheme(themeId) {
    shell.setTheme(themeId);
  }

  function handleOpenSettings() {
    isOpen = false;
    shell.openTab({
      id: "settings",
      title: "System Setup",
      type: "document"
    });
  }

  function handleOpenShortcuts() {
    isOpen = false;
    shell.isSearchOpen = true;
  }

  function handleOpenUserProfile() {
    isOpen = false;
    shell.openUserProfile();
  }

  function handleOpenEnvironmentDialog() {
    isOpen = false;
    shell.openEnvironmentDialog();
  }
</script>

<DropdownMenu bind:open={isOpen}>
  <!-- Top Bar Trigger: ONLY avatar icon with online dot (Red marked area completely removed) -->
  <DropdownMenuTrigger
    class={cn(
      "group flex items-center justify-center p-0.5 rounded-full hover:ring-2 hover:ring-indigo-500/40 focus-visible:ring-2 focus-visible:ring-indigo-500/50 outline-hidden transition-all duration-150 cursor-pointer shrink-0 select-none",
      className
    )}
    aria-label="User account menu for {activeUser.name}"
  >
    <!-- User Icon / Avatar with Status Dot -->
    <div class="relative flex items-center justify-center shrink-0">
      <div
        class="size-7.5 sm:size-8 rounded-full bg-gradient-to-tr from-indigo-600 via-indigo-500 to-violet-500 text-white font-semibold text-xs flex items-center justify-center shadow-xs ring-1 ring-white/20 dark:ring-slate-900/40 group-hover:scale-105 transition-transform"
      >
        {#if activeUser.avatar}
          <img
            src={activeUser.avatar}
            alt={activeUser.name}
            class="size-full rounded-full object-cover"
          />
        {:else}
          <span>{initials}</span>
        {/if}
      </div>

      <!-- Online Presence Indicator Dot -->
      <span
        class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full bg-emerald-500 ring-1.5 ring-white dark:ring-slate-900"
        title="Status: Online"
      ></span>
    </div>
  </DropdownMenuTrigger>

  <DropdownMenuContent
    align="end"
    sideOffset={8}
    class="w-76 p-2.5 shadow-xl border-slate-200/90 dark:border-slate-800 bg-white/95 dark:bg-slate-900/95 backdrop-blur-md rounded-2xl z-50"
  >
    <!-- Yellow Marked Area: Title [Name], Subtitle [designation] -> Click opens full page user profile -->
    <DropdownMenuItem
      onclick={handleOpenUserProfile}
      class="w-full text-left p-2.5 rounded-xl bg-slate-50 dark:bg-slate-800/60 hover:bg-indigo-50/70 dark:hover:bg-slate-800 border border-slate-100 dark:border-slate-800/80 hover:border-indigo-300 dark:hover:border-indigo-700/60 transition-all duration-150 cursor-pointer group mb-2 block focus:outline-hidden"
    >
      <div class="flex items-center gap-3 w-full">
        <div class="relative shrink-0">
          <div
            class="size-10 rounded-full bg-gradient-to-tr from-indigo-600 to-violet-600 text-white font-bold text-sm flex items-center justify-center shadow-xs group-hover:scale-105 transition-transform"
          >
            {#if activeUser.avatar}
              <img
                src={activeUser.avatar}
                alt={activeUser.name}
                class="size-full rounded-full object-cover"
              />
            {:else}
              <span>{initials}</span>
            {/if}
          </div>
          <span
            class="absolute -bottom-0.5 -right-0.5 size-2.5 rounded-full bg-emerald-500 ring-2 ring-white dark:ring-slate-900"
          ></span>
        </div>

        <div class="flex-1 min-w-0">
          <!-- Title [Name] -->
          <div class="text-xs font-bold text-slate-900 dark:text-slate-100 group-hover:text-indigo-600 dark:group-hover:text-indigo-400 transition-colors flex items-center justify-between">
            <span class="truncate">{activeUser.name}</span>
            <ExternalLink class="size-3 text-slate-400 opacity-0 group-hover:opacity-100 transition-opacity shrink-0 ml-1" />
          </div>
          <!-- Subtitle [designation] -->
          <div class="text-[11px] font-medium text-indigo-600 dark:text-indigo-400 truncate">
            {activeUser.designation}
          </div>
          {#if activeUser.email}
            <div class="text-[10px] text-slate-400 dark:text-slate-500 truncate mt-0.5 font-mono">
              {activeUser.email}
            </div>
          {/if}
        </div>
      </div>
    </DropdownMenuItem>

    <!-- Green Marked Area: Live Enterprise Company & Database -> Click opens Environment Dialog -->
    <DropdownMenuItem
      onclick={handleOpenEnvironmentDialog}
      class="w-full text-left p-2.5 rounded-xl border border-slate-200/80 dark:border-slate-800 bg-white dark:bg-slate-900/90 hover:bg-indigo-50/60 dark:hover:bg-slate-800/80 hover:border-indigo-400 dark:hover:border-indigo-600 transition-all duration-150 cursor-pointer mb-2 flex items-center justify-between group shadow-2xs focus:outline-hidden"
    >
      <div class="flex items-center gap-2 min-w-0">
        <div class="size-6 rounded-lg bg-indigo-50 dark:bg-indigo-950/70 text-indigo-600 dark:text-indigo-400 flex items-center justify-center shrink-0 group-hover:scale-105 transition-transform">
          <Building2 class="size-3.5" />
        </div>
        <div class="truncate text-left leading-tight">
          <div class="text-xs font-semibold text-slate-800 dark:text-slate-100 truncate group-hover:text-indigo-600 dark:group-hover:text-indigo-400">
            {shell.activeCompany || "Default Company"}
          </div>
          <div class="text-[10px] text-slate-400 font-mono truncate">
            tenant: {shell.activeTenant?.name || "No tenant"}
          </div>
        </div>
      </div>

      <div class="flex items-center gap-1.5 shrink-0 ml-2">
        <span class="px-1.5 py-0.5 rounded font-mono text-[9px] bg-indigo-50 dark:bg-indigo-950/80 text-indigo-600 dark:text-indigo-400 border border-indigo-200/50 dark:border-indigo-800/60 font-semibold">
          {shell.activeDatabase || "postgres"}
        </span>
        <ArrowRightLeft class="size-3 text-slate-400 group-hover:text-indigo-500 transition-colors" />
      </div>
    </DropdownMenuItem>

    <!-- Menu Item: Settings => Dedicated Theme Changer -->
    <DropdownMenuGroup>
      <DropdownMenuGroupHeading class="flex items-center justify-between">
        <span>Settings · Appearance</span>
        <span class="text-[9px] font-normal text-slate-400 capitalize">{shell.theme}</span>
      </DropdownMenuGroupHeading>

      <!-- Interactive Theme Changer Options (Light, Dark, System) -->
      <div class="p-1 mb-1 grid grid-cols-3 gap-1 bg-slate-100/80 dark:bg-slate-800/80 rounded-xl">
        {#each themeOptions as opt (opt.id)}
          {@const Icon = opt.icon}
          {@const isActive = shell.theme === opt.id}
          <button
            type="button"
            onclick={() => handleSelectTheme(opt.id)}
            class={cn(
              "flex flex-col items-center justify-center py-1.5 px-2 rounded-lg text-[10px] font-medium transition-all duration-150 cursor-pointer gap-1 select-none",
              isActive
                ? "bg-white dark:bg-slate-900 text-indigo-600 dark:text-indigo-400 shadow-xs font-semibold ring-1 ring-slate-200 dark:ring-slate-700"
                : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200 hover:bg-white/50 dark:hover:bg-slate-700/50"
            )}
            title="Switch theme to {opt.label}"
          >
            <Icon class="size-3.5 shrink-0" />
            <span>{opt.label}</span>
          </button>
        {/each}
      </div>

      <!-- Dedicated Settings Menu Item -->
      <DropdownMenuItem onclick={handleOpenSettings}>
        <Settings class="size-3.5 text-slate-500 dark:text-slate-400" />
        <span>System Setup & Preferences</span>
        <DropdownMenuShortcut>⌘,</DropdownMenuShortcut>
      </DropdownMenuItem>

      <!-- Keyboard Shortcuts Menu Item -->
      <DropdownMenuItem onclick={handleOpenShortcuts}>
        <Keyboard class="size-3.5 text-slate-500 dark:text-slate-400" />
        <span>Command Palette & Shortcuts</span>
        <DropdownMenuShortcut>⌘K</DropdownMenuShortcut>
      </DropdownMenuItem>

      <!-- Roles & Permissions Context -->
      <DropdownMenuItem onclick={() => {
        isOpen = false;
        shell.openTab({ id: "role-center", title: "Role Center", type: "document" });
      }}>
        <Shield class="size-3.5 text-slate-500 dark:text-slate-400" />
        <span>Role Center ({activeUser.role || "Admin"})</span>
      </DropdownMenuItem>
    </DropdownMenuGroup>

    <DropdownMenuSeparator />

    <!-- Superuser Developer Tools -->
    {#if activeUser.role === "SUPER" || activeUser.role === "Global Admin" || activeUser.role === "Admin"}
      <DropdownMenuGroup>
        <DropdownMenuItem onclick={() => {
          isOpen = false;
          shell.navigate("/studio");
        }}>
          <Sparkles class="size-3.5 text-indigo-500" />
          <span>Developer Studio (/studio)</span>
        </DropdownMenuItem>
      </DropdownMenuGroup>
      <DropdownMenuSeparator />
    {/if}

    <!-- Sign Out / Session Lock -->
    <DropdownMenuItem
      class="text-rose-600 dark:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/40 hover:text-rose-700 dark:hover:text-rose-300 focus:bg-rose-50 focus:text-rose-700"
      onclick={() => {
        isOpen = false;
        shell.logout();
      }}
    >
      <LogOut class="size-3.5" />
      <span>Sign Out</span>
    </DropdownMenuItem>
  </DropdownMenuContent>
</DropdownMenu>
