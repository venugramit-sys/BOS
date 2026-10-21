<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    User,
    Mail,
    Briefcase,
    Building2,
    Shield,
    Key,
    Check,
    Save,
    RotateCcw,
    Database,
    Clock,
    Laptop,
    Lock,
    Sparkles,
    ShieldCheck,
    ExternalLink,
    Sliders,
    Palette
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import { themeStore } from "$lib/bos-ui/theme/index.js";

  let { user = null } = $props();

  let activeUser = $derived(user || shell.currentUser);

  // Editable form fields
  let formName = $state(shell.currentUser?.name || "");
  let formDesignation = $state(shell.currentUser?.designation || "");
  let formEmail = $state(shell.currentUser?.email || "");
  let formDepartment = $state(shell.currentUser?.department || "");
  let formRole = $state(shell.currentUser?.role || "");

  let isSaved = $state(false);

  // Sync when active user changes
  $effect(() => {
    if (!activeUser) return;
    formName = activeUser.name || "";
    formDesignation = activeUser.designation || "";
    formEmail = activeUser.email || "";
    formDepartment = activeUser.department || "";
    formRole = activeUser.role || "";
  });

  let initials = $derived.by(() => {
    if (!formName) return "U";
    return formName
      .split(" ")
      .filter(Boolean)
      .map((n) => n[0])
      .slice(0, 2)
      .join("")
      .toUpperCase();
  });

  function handleSaveProfile(e) {
    e?.preventDefault();
    shell.updateUser({
      name: formName,
      designation: formDesignation,
      email: formEmail,
      department: formDepartment,
      role: formRole
    });
    isSaved = true;
    setTimeout(() => {
      isSaved = false;
    }, 3000);
  }

  function handleReset() {
    formName = shell.currentUser?.name || "";
    formDesignation = shell.currentUser?.designation || "";
    formEmail = shell.currentUser?.email || "";
    formDepartment = shell.currentUser?.department || "";
    formRole = shell.currentUser?.role || "";
  }
</script>

<div class="flex-1 w-full max-w-5xl mx-auto p-4 sm:p-6 lg:p-8 space-y-6">
  <!-- Profile Hero Header Banner -->
  <div class="relative rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-xs overflow-hidden">
    <!-- Top Decorative Gradient -->
    <div class="h-28 sm:h-36 bg-gradient-to-r from-indigo-600 via-violet-600 to-indigo-800 relative">
      <div class="absolute inset-0 bg-[radial-gradient(ellipse_at_top,_var(--tw-gradient-stops))] from-white/10 to-transparent"></div>
    </div>

    <!-- User Information Bar -->
    <div class="px-5 sm:px-8 pb-6 pt-0 relative flex flex-col sm:flex-row items-start sm:items-end justify-between gap-4 -mt-12 sm:-mt-14">
      <div class="flex flex-col sm:flex-row items-start sm:items-end gap-4">
        <!-- Large Avatar Circle -->
        <div class="relative shrink-0">
          <div
            class="size-20 sm:size-24 rounded-2xl bg-gradient-to-tr from-indigo-600 via-indigo-500 to-violet-600 text-white font-extrabold text-2xl sm:text-3xl flex items-center justify-center shadow-lg ring-4 ring-white dark:ring-slate-900"
          >
            <span>{initials}</span>
          </div>
          <span
            class="absolute bottom-1 right-1 size-4 rounded-full bg-emerald-500 ring-2 ring-white dark:ring-slate-900"
            title="Account Status: Online & Active"
          ></span>
        </div>

        <!-- Identity Details -->
        <div class="space-y-1">
          <div class="flex items-center gap-2.5 flex-wrap">
            <h1 class="text-xl sm:text-2xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
              {formName}
            </h1>
            <span class="px-2 py-0.5 rounded-full text-[11px] font-semibold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800/80">
              {formRole}
            </span>
            <span class="px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-50 dark:bg-emerald-950/60 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800/80 flex items-center gap-1">
              <span class="size-1.5 rounded-full bg-emerald-500"></span>
              Active
            </span>
          </div>
          <p class="text-xs sm:text-sm font-medium text-slate-500 dark:text-slate-400">
            {formDesignation} · <span class="font-normal">{formDepartment}</span>
          </p>
          <p class="text-xs text-slate-400 dark:text-slate-500 font-mono">
            {formEmail}
          </p>
        </div>
      </div>

      <!-- Quick Action Buttons -->
      <div class="flex items-center gap-2 shrink-0">
        <button
          type="button"
          onclick={() => shell.openEnvironmentDialog()}
          class="px-3 py-2 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 hover:bg-slate-50 dark:hover:bg-slate-800/80 text-xs font-semibold text-slate-700 dark:text-slate-200 shadow-2xs transition-all cursor-pointer flex items-center gap-1.5"
        >
          <Building2 class="size-3.5 text-indigo-500" />
          <span>Switch Environment</span>
        </button>

        <button
          type="button"
          onclick={handleSaveProfile}
          class="px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold shadow-xs transition-all cursor-pointer flex items-center gap-1.5"
        >
          {#if isSaved}
            <Check class="size-3.5 text-emerald-300" />
            <span>Saved!</span>
          {:else}
            <Save class="size-3.5" />
            <span>Save Profile</span>
          {/if}
        </button>
      </div>
    </div>
  </div>

  {#if isSaved}
    <div class="p-3 rounded-xl bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-800 text-emerald-800 dark:text-emerald-200 text-xs flex items-center gap-2 animate-in fade-in duration-150">
      <Check class="size-4 text-emerald-500 shrink-0" />
      <span>Profile changes have been saved and applied to your current session.</span>
    </div>
  {/if}

  <!-- Grid: Edit Details & Security Context -->
  <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
    <!-- Left Column: Personal Identity & Organization (2 cols) -->
    <div class="lg:col-span-2 space-y-6">
      <!-- Edit Profile Details Card -->
      <div class="p-5 sm:p-6 rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-5">
        <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100">
              Identity & Profile Information
            </h2>
            <p class="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
              Update your public name, job designation, and organizational department.
            </p>
          </div>
        </div>

        <form onsubmit={handleSaveProfile} class="space-y-4 text-xs">
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label for="prof-name" class="block font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Full Name
              </label>
              <input
                id="prof-name"
                type="text"
                bind:value={formName}
                class="w-full h-9 px-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 focus:outline-hidden focus:border-indigo-500 font-medium"
              />
            </div>

            <div>
              <label for="prof-desig" class="block font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Designation / Job Title
              </label>
              <input
                id="prof-desig"
                type="text"
                bind:value={formDesignation}
                class="w-full h-9 px-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 focus:outline-hidden focus:border-indigo-500 font-medium"
              />
            </div>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label for="prof-email" class="block font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Email Address
              </label>
              <input
                id="prof-email"
                type="email"
                bind:value={formEmail}
                class="w-full h-9 px-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 focus:outline-hidden focus:border-indigo-500 font-mono text-xs"
              />
            </div>

            <div>
              <label for="prof-dept" class="block font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Department
              </label>
              <input
                id="prof-dept"
                type="text"
                bind:value={formDepartment}
                class="w-full h-9 px-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 focus:outline-hidden focus:border-indigo-500 font-medium"
              />
            </div>
          </div>

          <div class="pt-2 flex items-center justify-end gap-2">
            <button
              type="button"
              onclick={handleReset}
              class="px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 text-xs font-medium text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer"
            >
              Reset
            </button>
            <button
              type="submit"
              class="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold shadow-xs cursor-pointer flex items-center gap-1.5"
            >
              <Save class="size-3.5" />
              <span>Update Profile</span>
            </button>
          </div>
        </form>
      </div>

      <!-- Interface Personalization Card -->
      <div class="p-5 sm:p-6 rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-4">
        <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100 flex items-center gap-1.5">
              <Sliders class="size-4 text-indigo-500" />
              <span>User Interface & Personalization</span>
            </h2>
            <p class="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
              Customize form component sizing, borders, and simulated accounting work date.
            </p>
          </div>
          <button
            type="button"
            onclick={() => themeStore.resetDefaults()}
            class="px-2.5 py-1 text-xs rounded-lg border border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          >
            Reset
          </button>
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 text-xs">
          <!-- Density Preference -->
          <div class="space-y-1.5">
            <span class="font-semibold text-slate-700 dark:text-slate-300">Default ERP Density</span>
            <div class="flex gap-1.5">
              {#each [
                { id: "sm", label: "Compact (28px)" },
                { id: "md", label: "Standard (36px)" },
                { id: "lg", label: "Touch (44px)" }
              ] as opt}
                <button
                  type="button"
                  onclick={() => themeStore.setDensity(opt.id)}
                  class="flex-1 py-1.5 text-xs font-medium rounded-lg border cursor-pointer transition-colors {themeStore.density === opt.id ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                >
                  {opt.label.split(" ")[0]}
                </button>
              {/each}
            </div>
          </div>

          <!-- Surface Frame Variant -->
          <div class="space-y-1.5">
            <span class="font-semibold text-slate-700 dark:text-slate-300">Surface Frame Variant</span>
            <div class="flex gap-1.5">
              {#each ["outline", "filled", "underline", "ghost"] as v}
                <button
                  type="button"
                  onclick={() => themeStore.setVariant(v)}
                  class="flex-1 py-1.5 text-xs font-medium rounded-lg border cursor-pointer transition-colors capitalize {themeStore.variant === v ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                >
                  {v}
                </button>
              {/each}
            </div>
          </div>

          <!-- Corner Radius -->
          <div class="space-y-1.5">
            <span class="font-semibold text-slate-700 dark:text-slate-300">Corner Radius</span>
            <div class="flex gap-1.5">
              {#each [
                { id: "none", label: "0px" },
                { id: "sm", label: "2px" },
                { id: "md", label: "6px" },
                { id: "pill", label: "Full" }
              ] as r}
                <button
                  type="button"
                  onclick={() => themeStore.setRounded(r.id)}
                  class="flex-1 py-1.5 text-xs font-medium rounded-lg border cursor-pointer transition-colors {themeStore.rounded === r.id ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                >
                  {r.label}
                </button>
              {/each}
            </div>
          </div>

          <!-- Simulated Accounting Work Date -->
          <div class="space-y-1.5">
            <span class="font-semibold text-slate-700 dark:text-slate-300">Simulated Work Date</span>
            <input
              type="date"
              value={themeStore.workDate}
              oninput={(e) => themeStore.setWorkDate(e.currentTarget.value)}
              class="w-full h-8 px-2.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 font-mono text-xs focus:outline-hidden focus:border-indigo-500"
            />
          </div>
        </div>
      </div>

      <!-- Enterprise Permissions & Roles Card -->
      <div class="p-5 sm:p-6 rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-4">
        <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100 flex items-center gap-1.5">
              <ShieldCheck class="size-4 text-indigo-500" />
              <span>Roles & Enterprise Scopes</span>
            </h2>
            <p class="text-xs text-slate-400 dark:text-slate-500 mt-0.5">
              Assigned system permission sets and multi-tenant ledger rights.
            </p>
          </div>
        </div>

        {#if (activeUser?.roles && activeUser.roles.length > 0) || activeUser?.role}
          {@const userRoles = Array.isArray(activeUser?.roles) && activeUser.roles.length > 0 ? activeUser.roles : [activeUser?.role || "User"]}
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs">
            {#each userRoles as roleName (roleName)}
              <div class="p-3 rounded-xl border border-slate-100 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 space-y-1">
                <span class="font-bold text-slate-800 dark:text-slate-200">{roleName}</span>
                <p class="text-[11px] text-slate-400">
                  {#if roleName === "SUPER" || roleName === "Global Admin" || roleName === "Admin"}
                    Full read, write, execute rights on meta-objects and system tables.
                  {:else}
                    Active role permission set assigned to this session.
                  {/if}
                </p>
              </div>
            {/each}
          </div>
        {:else}
          <div class="p-3 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-xs text-slate-400">
            No explicit role permission sets assigned to current session.
          </div>
        {/if}
      </div>
    </div>

    <!-- Right Column: Environment & Active Session Security -->
    <div class="space-y-6">
      <!-- Active Environment Widget -->
      <div class="p-5 rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-3">
        <h3 class="text-xs font-bold uppercase tracking-wider text-slate-400">
          Environment Context
        </h3>

        <div class="space-y-2.5 text-xs">
          <div class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/60 border border-slate-100 dark:border-slate-800 space-y-2">
            <div class="flex items-center justify-between">
              <span class="text-slate-500 dark:text-slate-400">Company:</span>
              <span class="font-bold text-slate-900 dark:text-slate-100">{shell.activeCompany}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-500 dark:text-slate-400">Database:</span>
              <span class="font-mono font-medium text-indigo-600 dark:text-indigo-400">{shell.activeDatabase}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-500 dark:text-slate-400">Strategy:</span>
              <span class="font-mono text-slate-500">{shell.activeStorageStrategy || "shared_rls"}</span>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-slate-500 dark:text-slate-400">Tenant:</span>
              <span class="font-medium text-slate-700 dark:text-slate-300">{shell.activeTenant?.name || "No tenant"}</span>
            </div>
          </div>

          <button
            type="button"
            onclick={() => shell.openEnvironmentDialog()}
            class="w-full py-2 rounded-xl border border-indigo-200 dark:border-indigo-800 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-700 dark:text-indigo-300 text-xs font-semibold hover:bg-indigo-100/60 transition-colors cursor-pointer flex items-center justify-center gap-1.5"
          >
            <Building2 class="size-3.5" />
            <span>Switch Database & Company</span>
          </button>
        </div>
      </div>

      <!-- Security & Session Health Card -->
      <div class="p-5 rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs space-y-3">
        <h3 class="text-xs font-bold uppercase tracking-wider text-slate-400">
          Security & Session
        </h3>

        <div class="space-y-2 text-xs">
          <div class="flex items-center justify-between py-1.5 border-b border-slate-100 dark:border-slate-800">
            <span class="text-slate-500">Authentication</span>
            <span class="font-mono text-[11px] text-emerald-600 dark:text-emerald-400">
              {activeUser?.authMethod || (activeUser?.id ? "Bearer Session" : "Unauthenticated")}
            </span>
          </div>

          <div class="flex items-center justify-between py-1.5 border-b border-slate-100 dark:border-slate-800">
            <span class="text-slate-500">2-Factor Authentication</span>
            <span class="font-medium {activeUser?.mfaEnabled ? 'text-emerald-600 dark:text-emerald-400' : 'text-slate-500 dark:text-slate-400'}">
              {activeUser?.mfaEnabled ? "Enabled" : "Not configured"}
            </span>
          </div>

          <div class="flex items-center justify-between py-1.5 border-b border-slate-100 dark:border-slate-800">
            <span class="text-slate-500">Session Engine</span>
            <span class="font-mono text-[11px] text-slate-700 dark:text-slate-300">QuickJS + Axum</span>
          </div>

          <div class="flex items-center justify-between py-1.5">
            <span class="text-slate-500">Engine Health</span>
            <span class="font-mono text-[11px] text-emerald-600 dark:text-emerald-400">{shell.runtimeStatus}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>
