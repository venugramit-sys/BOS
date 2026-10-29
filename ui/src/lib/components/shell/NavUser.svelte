<script>
  import { shell } from "./shellStore.svelte.js";
  import { ChevronsUpDown, Sun, Moon, Laptop, Code2, ExternalLink, LogOut, Shield } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { collapsed = false } = $props();

  let isOpen = $state(false);

  function getInitials(name) {
    if (!name) return "US";
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
  }

  function handleSignOut() {
    isOpen = false;
    shell.signOut();
  }

  function closeDropdown(e) {
    if (!e.target.closest(".nav-user-container")) {
      isOpen = false;
    }
  }
</script>

<svelte:window onclick={closeDropdown} />

<div class="relative nav-user-container p-2 border-t border-slate-200/80 dark:border-slate-800/80 shrink-0">
  <button
    type="button"
    onclick={() => (isOpen = !isOpen)}
    class={cn(
      "w-full flex items-center gap-2.5 p-2 rounded-lg text-left transition-colors cursor-pointer select-none",
      "hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-900 dark:text-slate-100",
      isOpen && "bg-slate-200/80 dark:bg-slate-800/80",
      collapsed && "justify-center px-0 py-2"
    )}
    title={collapsed ? shell.currentUser?.name || "Account" : "Account details"}
  >
    <!-- Avatar Initials -->
    <div class="size-8 rounded-lg bg-indigo-600/10 dark:bg-indigo-400/20 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 flex items-center justify-center font-bold text-xs shrink-0">
      {getInitials(shell.currentUser?.name || shell.currentUser?.username)}
    </div>

    {#if !collapsed}
      <div class="flex-1 min-w-0 text-left">
        <div class="text-xs font-semibold truncate leading-tight">
          {shell.currentUser?.name || shell.currentUser?.username || "Enterprise User"}
        </div>
        <div class="text-[10px] text-slate-500 dark:text-slate-400 truncate leading-tight mt-0.5">
          {shell.currentUser?.email || (shell.currentUser?.roles?.[0] ? `${shell.currentUser.roles[0]} Role` : "System User")}
        </div>
      </div>

      <ChevronsUpDown class="size-3.5 text-slate-400 shrink-0 ml-auto" />
    {/if}
  </button>

  <!-- User Dropdown Popover -->
  {#if isOpen}
    <div
      role="menu"
      tabindex="-1"
      class="absolute left-2 bottom-full mb-1.5 w-64 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-xl p-1.5 z-50 text-xs animate-in fade-in-0 zoom-in-95"
    >
      <!-- User Summary Header -->
      <div class="px-2.5 py-2 border-b border-slate-100 dark:border-slate-800 mb-1">
        <div class="font-semibold text-slate-900 dark:text-slate-100 truncate">
          {shell.currentUser?.name || shell.currentUser?.username}
        </div>
        <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate">
          {shell.currentUser?.email || "Enterprise Account"}
        </div>
        <div class="mt-1 flex items-center gap-1.5">
          <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-semibold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300">
            <Shield class="size-2.5" />
            <span>{shell.currentUser?.role || shell.currentUser?.roles?.[0] || "User"}</span>
          </span>
          {#if shell.isEmergencySession}
            <span class="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-500 text-amber-950">
              EMERGENCY
            </span>
          {/if}
        </div>
      </div>

      <!-- Complete Theme Selector -->
      <div class="px-2.5 py-1.5">
        <div class="text-[10px] font-semibold text-slate-400 dark:text-slate-500 uppercase tracking-wider mb-1.5">
          Theme
        </div>
        <div class="grid grid-cols-3 gap-1 p-0.5 rounded-lg bg-slate-100 dark:bg-slate-800/80">
          <button
            type="button"
            onclick={() => shell.setTheme("light")}
            class={cn(
              "flex items-center justify-center gap-1 py-1 px-1.5 rounded-md text-[11px] font-medium transition-all cursor-pointer",
              shell.theme === "light"
                ? "bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 shadow-2xs font-semibold"
                : "text-slate-500 hover:text-slate-900 dark:hover:text-slate-200"
            )}
            title="Light Mode"
          >
            <Sun class="size-3 text-amber-500" />
            <span>Light</span>
          </button>

          <button
            type="button"
            onclick={() => shell.setTheme("dark")}
            class={cn(
              "flex items-center justify-center gap-1 py-1 px-1.5 rounded-md text-[11px] font-medium transition-all cursor-pointer",
              shell.theme === "dark"
                ? "bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 shadow-2xs font-semibold"
                : "text-slate-500 hover:text-slate-900 dark:hover:text-slate-200"
            )}
            title="Dark Mode"
          >
            <Moon class="size-3 text-indigo-400" />
            <span>Dark</span>
          </button>

          <button
            type="button"
            onclick={() => shell.setTheme("system")}
            class={cn(
              "flex items-center justify-center gap-1 py-1 px-1.5 rounded-md text-[11px] font-medium transition-all cursor-pointer",
              shell.theme === "system"
                ? "bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 shadow-2xs font-semibold"
                : "text-slate-500 hover:text-slate-900 dark:hover:text-slate-200"
            )}
            title="System Mode"
          >
            <Laptop class="size-3 text-slate-400" />
            <span>System</span>
          </button>
        </div>
      </div>

      <!-- Dev Studio Option (Beneath Theme) -->
      <a
        href="/studio"
        onclick={() => (isOpen = false)}
        class="w-full flex items-center justify-between px-2.5 py-1.5 rounded-lg text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer font-medium text-xs mt-1"
      >
        <div class="flex items-center gap-2">
          <Code2 class="size-3.5 text-indigo-600 dark:text-indigo-400" />
          <span>Dev Studio</span>
        </div>
        <ExternalLink class="size-3 text-slate-400" />
      </a>

      <div class="border-t border-slate-100 dark:border-slate-800 my-1"></div>

      <!-- Log Out -->
      <button
        type="button"
        onclick={handleSignOut}
        class="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 transition-colors cursor-pointer text-left font-medium"
      >
        <LogOut class="size-3.5" />
        <span>Log Out</span>
      </button>
    </div>
  {/if}
</div>
