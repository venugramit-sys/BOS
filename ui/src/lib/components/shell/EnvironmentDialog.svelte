<script>
  import { shell } from "./shellStore.svelte.js";
  import {
    Database,
    Building2,
    Users,
    Check,
    X,
    Server,
    Shield,
    ArrowRight,
    RefreshCw,
    Sparkles,
    Plus,
    AlertCircle,
    Info
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let selectedDb = $state(shell.activeDatabase || "postgres");
  let selectedTenantId = $state(shell.activeTenant?.tenant_id || shell.activeTenant?.id || "default");
  let selectedCompany = $state(shell.activeCompany || "Default Company");
  let isSubmitting = $state(false);

  // Dynamic context loaded per selected database
  let loadedTenants = $state([]);
  let loadedCompanies = $state([]);
  let isLoadingContext = $state(false);
  let errorNotice = $state("");

  // Creation forms state
  let showCreateTenant = $state(false);
  let showCreateCompany = $state(false);
  let newTenantCode = $state("");
  let newTenantName = $state("");
  let newCompanyCode = $state("");
  let newCompanyName = $state("");
  let isCreatingTenant = $state(false);
  let isCreatingCompany = $state(false);
  let createError = $state("");

  // Live databases from server
  let dbList = $derived.by(() => {
    if (shell.availableDatabases && shell.availableDatabases.length > 0) {
      return shell.availableDatabases.map((db) => (typeof db === "string" ? { name: db } : db));
    }
    if (shell.activeDatabase) {
      return [
        {
          name: shell.activeDatabase,
          storage_strategy: shell.activeStorageStrategy || "shared_rls",
          is_active: true,
        },
      ];
    }
    return [];
  });

  // Effective tenant list: prioritize dynamic fetch for selectedDb
  let tenantList = $derived.by(() => {
    if (loadedTenants.length > 0) return loadedTenants;
    if (selectedDb === shell.activeDatabase && shell.tenants?.length > 0) return shell.tenants;
    return [];
  });

  // Effective company list: prioritize dynamic fetch for selectedDb + tenant
  let companyList = $derived.by(() => {
    if (loadedCompanies.length > 0) return loadedCompanies;
    if (selectedDb === shell.activeDatabase && shell.companies?.length > 0) return shell.companies;
    return [];
  });

  // Selected tenant display object
  let activeTenantObj = $derived(
    tenantList.find((t) => (t.tenant_id || t.id || t.code) === selectedTenantId) || null
  );

  async function loadDatabaseContext(dbName) {
    if (!dbName) return;
    isLoadingContext = true;
    errorNotice = "";
    try {
      // 1. Fetch tenants for the target database
      const tenantRes = await fetch(`/api/tenants?database=${encodeURIComponent(dbName)}`);
      let tenants = [];
      if (tenantRes.ok) {
        const data = await tenantRes.json();
        if (Array.isArray(data)) {
          tenants = data;
        }
      }
      loadedTenants = tenants;

      // Auto-resolve selected tenant
      let activeTId = selectedTenantId;
      const foundTenant = tenants.find((t) => (t.tenant_id || t.id || t.code) === activeTId);
      if (!foundTenant && tenants.length > 0) {
        activeTId = tenants[0].tenant_id || tenants[0].id || tenants[0].code;
        selectedTenantId = activeTId;
      } else if (tenants.length === 0) {
        activeTId = null;
        selectedTenantId = "";
      }

      // 2. Fetch companies for the target database and resolved tenant
      await loadCompaniesForTenant(dbName, activeTId);
    } catch (err) {
      errorNotice = `Could not load context for database '${dbName}'`;
      loadedTenants = [];
      loadedCompanies = [];
    } finally {
      isLoadingContext = false;
    }
  }

  async function loadCompaniesForTenant(dbName, tenantId) {
    try {
      const url = tenantId
        ? `/api/companies?database=${encodeURIComponent(dbName)}&tenant_id=${encodeURIComponent(tenantId)}`
        : `/api/companies?database=${encodeURIComponent(dbName)}`;
      const compRes = await fetch(url);
      let comps = [];
      if (compRes.ok) {
        const data = await compRes.json();
        if (Array.isArray(data)) {
          comps = data;
        }
      }
      loadedCompanies = comps;

      const foundComp = comps.find((c) => (c.name || c.code) === selectedCompany);
      if (!foundComp && comps.length > 0) {
        selectedCompany = comps[0].name || comps[0].code;
      } else if (comps.length === 0) {
        selectedCompany = "";
      }
    } catch {
      loadedCompanies = [];
    }
  }

  function handleSelectDatabase(dbName) {
    selectedDb = dbName;
    showCreateTenant = false;
    showCreateCompany = false;
    createError = "";
    loadDatabaseContext(dbName);
  }

  function handleSelectTenant(tId) {
    selectedTenantId = tId;
    showCreateCompany = false;
    createError = "";
    loadCompaniesForTenant(selectedDb, tId);
  }

  async function handleCreateTenant() {
    if (!newTenantCode.trim() || !newTenantName.trim()) {
      createError = "Tenant Code and Name are required.";
      return;
    }
    isCreatingTenant = true;
    createError = "";
    try {
      const res = await fetch("/api/tenants", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          database: selectedDb,
          code: newTenantCode.trim().toUpperCase(),
          name: newTenantName.trim()
        })
      });
      if (res.ok) {
        const created = await res.json();
        newTenantCode = "";
        newTenantName = "";
        showCreateTenant = false;
        await loadDatabaseContext(selectedDb);
        if (created?.tenant_id) {
          selectedTenantId = created.tenant_id;
        }
      } else {
        const err = await res.json().catch(() => ({}));
        createError = err.error || "Failed to create tenant";
      }
    } catch (err) {
      createError = err.message || "Failed to create tenant";
    } finally {
      isCreatingTenant = false;
    }
  }

  async function handleCreateCompany() {
    if (!newCompanyCode.trim() || !newCompanyName.trim()) {
      createError = "Company Code and Name are required.";
      return;
    }
    const parentTenant = activeTenantObj || loadedTenants[0];
    if (!parentTenant?.tenant_id && !selectedTenantId) {
      createError = "A valid Tenant Partition must be selected first.";
      return;
    }
    isCreatingCompany = true;
    createError = "";
    try {
      const parentTenantId = parentTenant?.tenant_id || selectedTenantId;
      const res = await fetch("/api/companies", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          database: selectedDb,
          tenant_id: parentTenantId,
          code: newCompanyCode.trim().toUpperCase(),
          name: newCompanyName.trim()
        })
      });
      if (res.ok) {
        const created = await res.json();
        newCompanyCode = "";
        newCompanyName = "";
        showCreateCompany = false;
        await loadCompaniesForTenant(selectedDb, selectedTenantId);
        if (created?.name) {
          selectedCompany = created.name;
        }
      } else {
        const err = await res.json().catch(() => ({}));
        createError = err.error || "Failed to create company";
      }
    } catch (err) {
      createError = err.message || "Failed to create company";
    } finally {
      isCreatingCompany = false;
    }
  }

  // Sync state when dialog opens
  $effect(() => {
    if (shell.isEnvironmentDialogOpen) {
      selectedDb = shell.activeDatabase || "postgres";
      selectedTenantId = shell.activeTenant?.tenant_id || shell.activeTenant?.id || "default";
      selectedCompany = shell.activeCompany || "Default Company";
      showCreateTenant = false;
      showCreateCompany = false;
      createError = "";
      loadDatabaseContext(selectedDb);
    }
  });

  async function handleApply() {
    isSubmitting = true;
    try {
      await shell.switchEnvironment({
        database: selectedDb,
        tenantId: selectedTenantId,
        companyName: selectedCompany
      });
    } finally {
      isSubmitting = false;
    }
  }

  function handleKeydown(e) {
    if (e.key === "Escape") {
      shell.closeEnvironmentDialog();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if shell.isEnvironmentDialogOpen}
  <div
    role="presentation"
    class="fixed inset-0 z-50 bg-slate-950/60 backdrop-blur-xs flex items-center justify-center p-4 animate-in fade-in-50 duration-150"
    onclick={() => shell.closeEnvironmentDialog()}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="env-dialog-title"
      tabindex="-1"
      class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl max-w-xl w-full flex flex-col overflow-hidden max-h-[90vh] animate-in zoom-in-95 duration-150"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Dialog Header -->
      <div class="px-5 py-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between bg-slate-50/50 dark:bg-slate-800/40">
        <div class="flex items-center gap-2.5">
          <div class="size-9 rounded-xl bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 flex items-center justify-center">
            <Server class="size-5" />
          </div>
          <div>
            <h2 id="env-dialog-title" class="text-sm font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
              Select Environment Context
              {#if isLoadingContext}
                <RefreshCw class="size-3 text-indigo-500 animate-spin" />
              {/if}
            </h2>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              Select database from server, tenant partition, and legal company.
            </p>
          </div>
        </div>
        <button
          type="button"
          onclick={() => shell.closeEnvironmentDialog()}
          class="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
          title="Close dialog"
          aria-label="Close dialog"
        >
          <X class="size-4" />
        </button>
      </div>

      <!-- Dialog Body (Scrollable) -->
      <div class="p-5 overflow-y-auto space-y-5 text-xs">
        {#if errorNotice}
          <div class="p-3 rounded-xl bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900 text-rose-700 dark:text-rose-300 flex items-center gap-2">
            <AlertCircle class="size-4 shrink-0 text-rose-500" />
            <span>{errorNotice}</span>
          </div>
        {/if}

        <!-- 1. Database Selection -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <label class="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <Database class="size-3.5 text-indigo-500" />
              <span>1. Target Database (Current Server)</span>
            </label>
            <span class="text-[10px] text-slate-400 font-medium">Click to inspect partitions</span>
          </div>
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
            {#each dbList as db}
              {@const isSelected = selectedDb === db.name}
              {@const isCurrent = shell.activeDatabase === db.name}
              <button
                type="button"
                onclick={() => handleSelectDatabase(db.name)}
                class={cn(
                  "p-2.5 rounded-xl border text-left flex flex-col justify-between transition-all cursor-pointer relative",
                  isSelected
                    ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/30 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                    : "border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 hover:border-slate-300 dark:hover:border-slate-700 text-slate-700 dark:text-slate-300"
                )}
              >
                <div class="flex items-center justify-between">
                  <span class="font-bold truncate text-xs">{db.name}</span>
                  {#if isSelected}
                    <Check class="size-3.5 text-indigo-600 dark:text-indigo-400 shrink-0" />
                  {/if}
                </div>
                <div class="mt-2 flex items-center justify-between text-[10px] text-slate-400 font-mono">
                  <span>{db.storage_strategy || "shared_rls"}</span>
                  {#if isCurrent}
                    <span class="px-1 rounded bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 text-[9px] font-sans font-medium">active</span>
                  {/if}
                </div>
              </button>
            {/each}
            {#if dbList.length === 0}
              <div class="col-span-full p-3 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-[11px] text-slate-400 text-center">
                No databases detected on server.
              </div>
            {/if}
          </div>
        </div>

        <!-- 2. Tenant Selection -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <label class="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <Users class="size-3.5 text-indigo-500" />
              <span>2. Tenant Partition</span>
              {#if isLoadingContext}
                <span class="text-[10px] text-indigo-500 font-normal">Loading...</span>
              {/if}
            </label>
            <button
              type="button"
              onclick={() => {
                showCreateTenant = !showCreateTenant;
                createError = "";
              }}
              class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 flex items-center gap-1 cursor-pointer transition-colors"
            >
              <Plus class="size-3" />
              <span>New Tenant</span>
            </button>
          </div>

          <!-- Create Tenant Inline Card -->
          {#if showCreateTenant}
            <div class="p-3.5 rounded-xl bg-slate-50 dark:bg-slate-800/60 border border-indigo-200 dark:border-indigo-900/60 space-y-2.5 animate-in slide-in-from-top-2 duration-150">
              <div class="flex items-center justify-between">
                <span class="font-bold text-xs text-slate-900 dark:text-slate-100">Create Tenant Partition in '{selectedDb}'</span>
                <button
                  type="button"
                  onclick={() => (showCreateTenant = false)}
                  class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                >
                  <X class="size-3.5" />
                </button>
              </div>
              <p class="text-[11px] text-slate-500 dark:text-slate-400">
                Allowed fields: <strong>code</strong> (unique alphanumeric slug) &amp; <strong>name</strong> (display title).
              </p>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <div>
                  <label for="tenant-code-input" class="block text-[10px] font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider mb-1">Tenant Code *</label>
                  <input
                    id="tenant-code-input"
                    type="text"
                    bind:value={newTenantCode}
                    placeholder="e.g. EU_WEST, DEFAULT"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-xs text-slate-900 dark:text-slate-100 uppercase focus:ring-1 focus:ring-indigo-500 outline-hidden font-mono"
                  />
                </div>
                <div>
                  <label for="tenant-name-input" class="block text-[10px] font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider mb-1">Tenant Name *</label>
                  <input
                    id="tenant-name-input"
                    type="text"
                    bind:value={newTenantName}
                    placeholder="e.g. European Operations"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-xs text-slate-900 dark:text-slate-100 focus:ring-1 focus:ring-indigo-500 outline-hidden"
                  />
                </div>
              </div>
              {#if createError && showCreateTenant}
                <div class="text-[11px] text-rose-600 dark:text-rose-400">{createError}</div>
              {/if}
              <div class="flex justify-end gap-2 pt-1">
                <button
                  type="button"
                  onclick={() => (showCreateTenant = false)}
                  class="px-2.5 py-1 rounded-lg text-xs text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700 cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onclick={handleCreateTenant}
                  disabled={isCreatingTenant || !newTenantCode.trim() || !newTenantName.trim()}
                  class="px-3 py-1 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold cursor-pointer disabled:opacity-50 flex items-center gap-1.5"
                >
                  {#if isCreatingTenant}
                    <RefreshCw class="size-3 animate-spin" />
                    <span>Saving...</span>
                  {:else}
                    <span>Create Tenant</span>
                  {/if}
                </button>
              </div>
            </div>
          {/if}

          <!-- Tenant Cards Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
            {#each tenantList as tenant}
              {@const tId = tenant.tenant_id || tenant.id || tenant.name}
              {@const isSelected = selectedTenantId === tId}
              <button
                type="button"
                onclick={() => handleSelectTenant(tId)}
                class={cn(
                  "p-2.5 rounded-xl border text-left flex flex-col justify-between transition-all cursor-pointer",
                  isSelected
                    ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/30 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                    : "border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 hover:border-slate-300 dark:hover:border-slate-700 text-slate-700 dark:text-slate-300"
                )}
              >
                <div class="flex items-center justify-between">
                  <span class="font-bold truncate text-xs">{tenant.name || tenant.id}</span>
                  {#if isSelected}
                    <Check class="size-3.5 text-indigo-600 dark:text-indigo-400 shrink-0" />
                  {/if}
                </div>
                <span class="text-[10px] text-slate-400 font-mono mt-1">
                  code: {tenant.code || tenant.id || "DEFAULT"}
                </span>
              </button>
            {/each}
            {#if tenantList.length === 0}
              <div class="col-span-full p-4 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-[11px] text-slate-400 text-center space-y-1">
                <div>No tenant partitions found in <strong>{selectedDb}</strong>.</div>
                <div class="text-[10px] text-slate-500">Click <strong>+ New Tenant</strong> above to initialize the first partition.</div>
              </div>
            {/if}
          </div>
        </div>

        <!-- 3. Company Selection -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <label class="font-semibold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
              <Building2 class="size-3.5 text-indigo-500" />
              <span>3. Legal Company Entity</span>
              {#if activeTenantObj}
                <span class="text-[10px] text-slate-400 font-normal">
                  (in {activeTenantObj.name || activeTenantObj.code})
                </span>
              {/if}
            </label>
            <button
              type="button"
              onclick={() => {
                showCreateCompany = !showCreateCompany;
                createError = "";
              }}
              class="text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 flex items-center gap-1 cursor-pointer transition-colors"
            >
              <Plus class="size-3" />
              <span>New Company</span>
            </button>
          </div>

          <!-- Create Company Inline Card -->
          {#if showCreateCompany}
            <div class="p-3.5 rounded-xl bg-slate-50 dark:bg-slate-800/60 border border-indigo-200 dark:border-indigo-900/60 space-y-2.5 animate-in slide-in-from-top-2 duration-150">
              <div class="flex items-center justify-between">
                <span class="font-bold text-xs text-slate-900 dark:text-slate-100">
                  Create Legal Company in '{selectedDb}'
                </span>
                <button
                  type="button"
                  onclick={() => (showCreateCompany = false)}
                  class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                >
                  <X class="size-3.5" />
                </button>
              </div>
              <p class="text-[11px] text-slate-500 dark:text-slate-400">
                Allowed fields: <strong>tenant_id</strong> (parent partition), <strong>code</strong> (statutory code), &amp; <strong>name</strong> (legal entity name).
              </p>
              <div class="p-2 rounded-lg bg-indigo-500/5 border border-indigo-500/10 text-[11px] text-indigo-700 dark:text-indigo-300 flex items-center gap-1.5">
                <Info class="size-3.5 shrink-0" />
                <span>Parent Partition: <strong>{activeTenantObj?.name || activeTenantObj?.code || selectedTenantId || "Default"}</strong></span>
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <div>
                  <label for="company-code-input" class="block text-[10px] font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider mb-1">Company Code *</label>
                  <input
                    id="company-code-input"
                    type="text"
                    bind:value={newCompanyCode}
                    placeholder="e.g. CRONUS_US"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-xs text-slate-900 dark:text-slate-100 uppercase focus:ring-1 focus:ring-indigo-500 outline-hidden font-mono"
                  />
                </div>
                <div>
                  <label for="company-name-input" class="block text-[10px] font-semibold text-slate-600 dark:text-slate-400 uppercase tracking-wider mb-1">Company Name *</label>
                  <input
                    id="company-name-input"
                    type="text"
                    bind:value={newCompanyName}
                    placeholder="e.g. Cronus US Inc."
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-xs text-slate-900 dark:text-slate-100 focus:ring-1 focus:ring-indigo-500 outline-hidden"
                  />
                </div>
              </div>
              {#if createError && showCreateCompany}
                <div class="text-[11px] text-rose-600 dark:text-rose-400">{createError}</div>
              {/if}
              <div class="flex justify-end gap-2 pt-1">
                <button
                  type="button"
                  onclick={() => (showCreateCompany = false)}
                  class="px-2.5 py-1 rounded-lg text-xs text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700 cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onclick={handleCreateCompany}
                  disabled={isCreatingCompany || !newCompanyCode.trim() || !newCompanyName.trim()}
                  class="px-3 py-1 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold cursor-pointer disabled:opacity-50 flex items-center gap-1.5"
                >
                  {#if isCreatingCompany}
                    <RefreshCw class="size-3 animate-spin" />
                    <span>Saving...</span>
                  {:else}
                    <span>Create Company</span>
                  {/if}
                </button>
              </div>
            </div>
          {/if}

          <!-- Company List Cards -->
          <div class="space-y-1.5">
            {#each companyList as comp}
              {@const isSelected = selectedCompany === (comp.name || comp.code)}
              <button
                type="button"
                onclick={() => (selectedCompany = comp.name || comp.code)}
                class={cn(
                  "w-full p-2.5 rounded-xl border text-left flex items-center justify-between transition-all cursor-pointer",
                  isSelected
                    ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/30 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                    : "border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 hover:border-slate-300 dark:hover:border-slate-700 text-slate-700 dark:text-slate-300"
                )}
              >
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class={cn("size-7 rounded-lg flex items-center justify-center font-bold text-xs shrink-0", isSelected ? "bg-indigo-600 text-white" : "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400")}>
                    {(comp.name || comp.code || "C")[0]}
                  </div>
                  <div class="truncate">
                    <div class="font-bold text-xs truncate">{comp.name || comp.code}</div>
                    <div class="text-[10px] text-slate-400 font-mono">code: {comp.code || ""} · cur: {comp.currency || "USD"}</div>
                  </div>
                </div>
                {#if isSelected}
                  <Check class="size-4 text-indigo-600 dark:text-indigo-400 shrink-0 ml-2" />
                {/if}
              </button>
            {/each}
            {#if companyList.length === 0}
              <div class="p-4 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-[11px] text-slate-400 text-center space-y-1">
                <div>No legal companies found in <strong>{selectedDb}</strong> under selected partition.</div>
                <div class="text-[10px] text-slate-500">Click <strong>+ New Company</strong> above to create one.</div>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Dialog Footer -->
      <div class="p-4 border-t border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-800/60 flex items-center justify-between gap-3">
        <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate hidden sm:block">
          Switching to: <span class="font-bold text-slate-800 dark:text-slate-200">{selectedCompany || "None"}</span>
          <span class="text-slate-400 font-mono">({selectedDb}{selectedTenantId ? ` / ${activeTenantObj?.code || selectedTenantId}` : ""})</span>
        </div>

        <div class="flex items-center gap-2 ml-auto">
          <button
            type="button"
            onclick={() => shell.closeEnvironmentDialog()}
            class="px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 text-xs font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          >
            Cancel
          </button>
          <button
            type="button"
            onclick={handleApply}
            disabled={isSubmitting || isLoadingContext || !selectedDb}
            class="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold shadow-xs hover:shadow transition-all cursor-pointer flex items-center gap-1.5 disabled:opacity-50"
          >
            {#if isSubmitting}
              <RefreshCw class="size-3.5 animate-spin" />
              <span>Switching...</span>
            {:else}
              <span>Apply & Switch</span>
              <ArrowRight class="size-3.5" />
            {/if}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}
