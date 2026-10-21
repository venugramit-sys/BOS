<script>
  import { onMount, untrack } from "svelte";
  import { shell } from "./shellStore.svelte.js";
  import {
    getRecentServers,
    saveRecentServer,
    getActiveServer,
    detectProvider,
    CLOUD_PRESETS
  } from "$lib/services/serverManager.js";
  import {
    Database,
    Server,
    Building2,
    Users,
    Check,
    CheckCircle2,
    AlertTriangle,
    X,
    RefreshCw,
    Search,
    Plus,
    Wrench,
    Trash2,
    ArrowRight,
    ArrowLeft,
    Activity,
    Cloud,
    Crown,
    ShieldCheck,
    Terminal,
    Lock,
    Sliders,
    Package,
    Sparkles,
    Zap,
    ExternalLink
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  // ==========================================
  // USER PERMISSIONS & AUTHENTICITY
  // ==========================================
  let currentUser = $derived(shell.currentUser);
  let isSuperUser = $derived(
    Boolean(
      currentUser?.role === "SUPER" ||
      currentUser?.role === "Global Admin" ||
      currentUser?.role === "Admin" ||
      (Array.isArray(currentUser?.roles) && currentUser.roles.includes("SUPER"))
    )
  );
  let isAuthenticated = $derived(Boolean(currentUser?.id));

  function authHeaders() {
    const headers = { "Content-Type": "application/json" };
    try {
      const raw = localStorage.getItem("bos_session");
      const token = JSON.parse(raw || "null")?.token || "";
      if (token) headers["Authorization"] = `Bearer ${token}`;
      const env = JSON.parse(localStorage.getItem("bos_environment") || "null");
      if (env?.tenantId) headers["x-bos-tenant-id"] = String(env.tenantId);
      if (env?.companyName) headers["x-bos-company"] = String(env.companyName);
    } catch {}
    return headers;
  }

  // ==========================================
  // STEPPER FLOW: 1. Server -> 2. Database -> 3. Tenant / Company
  // ==========================================
  // currentStep: 'server' | 'database' | 'context' | 'admin'
  let currentStep = $state("server");

  // Step 1: Server State
  let recentServers = $state([]);
  let activeServerObj = $state(getActiveServer());
  let serverConnectionUrl = $state(activeServerObj.url || "postgresql://postgres:postgres@localhost:5432/postgres");
  let isConnectingServer = $state(false);
  let serverConnectionResult = $state(null);
  let showCustomServerInput = $state(false);

  // Step 2: Database State
  let dbSearchQuery = $state("");
  let selectedDb = $state(shell.activeDatabase || "postgres");
  let isSwitchingDb = $state(false);
  let showCreateDbForm = $state(false);

  // Database creation form state (allowed for any user)
  let newDbName = $state("");
  let newStorageStrategy = $state("shared_rls");
  let importBasePackages = $state(true);
  let isCreatingDb = $state(false);

  // Step 3: Tenant & Company Context State
  let loadedTenants = $state([]);
  let loadedCompanies = $state([]);
  let selectedTenantId = $state(shell.activeTenant?.tenant_id || shell.activeTenant?.id || "");
  let selectedCompanyCode = $state(shell.activeCompany || "");
  let isLoadingContext = $state(false);
  let isApplyingEnvironment = $state(false);

  // Superuser Admin Tools State (Restricted to superusers)
  let adminDbTarget = $state(shell.activeDatabase || "");
  let adminAction = $state("vacuum");
  let vacuumFull = $state(true);
  let reindexTable = $state("");
  let strategyUpdate = $state("shared_rls");
  let deleteConfirmation = $state("");
  let isRunningAdmin = $state(false);

  // Notification / Feedback State
  let statusBanner = $state({ type: "", text: "" });

  function formatErrorMessage(err, fallback = "Operation failed") {
    if (!err) return fallback;
    if (typeof err === "string" && err.trim()) return err;
    if (typeof err.message === "string" && err.message.trim()) return err.message;
    if (typeof err.error === "string" && err.error.trim()) return err.error;
    if (typeof err.statusText === "string" && err.statusText.trim()) return err.statusText;
    return fallback;
  }

  let wasOpen = false;

  $effect(() => {
    const isOpen = shell.isDbManagerOpen;
    if (isOpen && !wasOpen) {
      untrack(() => {
        initDialog();
      });
    }
    wasOpen = isOpen;
  });

  function initDialog() {
    recentServers = getRecentServers();
    activeServerObj = getActiveServer();
    serverConnectionUrl = activeServerObj.url || "postgresql://postgres:postgres@localhost:5432/postgres";
    selectedDb = shell.activeDatabase || "postgres";
    adminDbTarget = selectedDb;
    selectedCompanyCode = shell.activeCompany && shell.activeCompany !== "Loading company..." ? shell.activeCompany : "";
    selectedTenantId = shell.activeTenant?.tenant_id || shell.activeTenant?.id || "";
    statusBanner = { type: "", text: "" };

    // If active database is already established, start on Step 2 (Database) or Step 1 (Server)
    if (shell.activeDatabase) {
      currentStep = "database";
      loadDatabases();
    } else {
      currentStep = "server";
    }
  }

  // Filtered databases list
  let filteredDatabases = $derived.by(() => {
    const list = shell.availableDatabases || [];
    if (!dbSearchQuery.trim()) return list;
    const q = dbSearchQuery.toLowerCase();
    return list.filter((db) => {
      const name = typeof db === "string" ? db : db.name || "";
      const strategy = typeof db === "object" ? db.storage_strategy || "" : "";
      return name.toLowerCase().includes(q) || strategy.toLowerCase().includes(q);
    });
  });

  // ==========================================
  // STEP 1: SERVER ACTIONS
  // ==========================================

  // 1-Click Server Connect
  async function connectToServer(server) {
    serverConnectionUrl = server.url;
    await testAndConnectServer(server.url, server.name);
  }

  // Test and connect to server URL
  async function testAndConnectServer(urlToTest, optionalName) {
    const targetUrl = urlToTest || serverConnectionUrl;
    if (!targetUrl?.trim()) return;

    isConnectingServer = true;
    serverConnectionResult = null;
    statusBanner = { type: "", text: "" };

    try {
      // 1. Test probe
      const res = await fetch("/api/databases/test-connection", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: targetUrl.trim() })
      });
      const result = await res.json();
      serverConnectionResult = result;

      if (!result.success) {
        statusBanner = { type: "error", text: `Connection failed: ${result.message}` };
        return;
      }

      // 2. Switch engine pool to this server
      const switchRes = await fetch("/api/databases/switch", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url: targetUrl.trim() })
      });
      const switchData = await switchRes.json().catch(() => ({}));

      if (!switchRes.ok || switchData.error) {
        statusBanner = {
          type: "error",
          text: `Connection test passed, but switching active server failed: ${formatErrorMessage(switchData, switchRes.statusText)}`
        };
        return;
      }

      // 3. Save to recent servers (maintains last 3)
      recentServers = saveRecentServer({
        name: optionalName,
        url: targetUrl.trim(),
        provider: detectProvider(targetUrl.trim())
      });
      activeServerObj = getActiveServer();

      statusBanner = {
        type: "success",
        text: `Connected to ${activeServerObj.name} (${result.latency_ms}ms)`
      };

      if (switchData.active_database) {
        selectedDb = switchData.active_database;
        shell.activeDatabase = switchData.active_database;
      }

      // 4. Refresh databases for target server and advance to Step 2 (Database)
      await loadDatabases(targetUrl.trim());
      currentStep = "database";
    } catch (err) {
      statusBanner = { type: "error", text: "Connection failed: " + formatErrorMessage(err) };
    } finally {
      isConnectingServer = false;
    }
  }

  // ==========================================
  // STEP 2: DATABASE ACTIONS
  // ==========================================

  async function loadDatabases(optionalUrl) {
    try {
      const targetUrl = optionalUrl || activeServerObj?.url || serverConnectionUrl;
      const urlParam = targetUrl ? `?url=${encodeURIComponent(targetUrl)}` : "";
      const res = await fetch(`/api/databases${urlParam}`, {
        headers: { "Content-Type": "application/json" }
      });
      if (res.ok) {
        const data = await res.json();
        if (Array.isArray(data)) {
          shell.availableDatabases = data;
        }
      }
    } catch {
      // Offline fallback
    }
  }

  // Switch database with authenticity verification
  async function selectDatabase(dbName) {
    if (!dbName) return;
    const prevDb = selectedDb;
    selectedDb = dbName;
    adminDbTarget = dbName;
    isSwitchingDb = true;
    statusBanner = { type: "", text: "" };

    try {
      const res = await fetch("/api/databases/switch", {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ name: dbName })
      });

      if (res.ok) {
        const data = await res.json();
        shell.activeDatabase = dbName;
        selectedDb = dbName;
        statusBanner = {
          type: "success",
          text: `Active database: '${dbName}' (${data.authenticated ? "Authenticated" : "Verified"})`
        };
        // Load context (Tenants & Companies) and advance to Step 3
        await loadContextForDatabase(dbName);
        currentStep = "context";
      } else {
        selectedDb = shell.activeDatabase || prevDb;
        const err = await res.json().catch(() => ({}));
        statusBanner = { type: "error", text: formatErrorMessage(err, "Failed to switch database") };
      }
    } catch (err) {
      selectedDb = shell.activeDatabase || prevDb;
      statusBanner = { type: "error", text: formatErrorMessage(err, "Failed to switch database") };
    } finally {
      isSwitchingDb = false;
    }
  }

  // Create database on this server (any user)
  async function createDatabase() {
    const name = newDbName.trim().toLowerCase().replace(/[^a-z0-9_]/g, "_");
    if (!name) return;

    isCreatingDb = true;
    statusBanner = { type: "info", text: `Provisioning '${name}'...` };

    try {
      const res = await fetch("/api/databases", {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({
          name,
          init_extensions: true,
          provision_system: true,
          storage_strategy: newStorageStrategy,
          import_base_packages: importBasePackages,
          packages: importBasePackages ? ["bos.security", "bos.ledger"] : []
        })
      });

      if (res.ok) {
        newDbName = "";
        showCreateDbForm = false;
        await loadDatabases();
        await selectDatabase(name);
      } else {
        const err = await res.json().catch(() => ({}));
        statusBanner = { type: "error", text: formatErrorMessage(err, "Creation failed") };
      }
    } catch (err) {
      statusBanner = { type: "error", text: formatErrorMessage(err, "Creation failed") };
    } finally {
      isCreatingDb = false;
    }
  }

  // ==========================================
  // STEP 3: TENANT OR COMPANY ACTIONS
  // ==========================================

  async function loadContextForDatabase(dbName) {
    isLoadingContext = true;
    try {
      const [compRes, tenRes] = await Promise.all([
        fetch("/api/companies", { headers: authHeaders() }).catch(() => null),
        fetch("/api/tenants", { headers: authHeaders() }).catch(() => null)
      ]);

      if (compRes?.ok) {
        const compData = await compRes.json();
        loadedCompanies = Array.isArray(compData) ? compData : [];
        if (loadedCompanies.length > 0 && (!selectedCompanyCode || selectedCompanyCode === "Loading company...")) {
          selectedCompanyCode = loadedCompanies[0].name || loadedCompanies[0].code;
        }
      }

      if (tenRes?.ok) {
        const tenData = await tenRes.json();
        loadedTenants = Array.isArray(tenData) ? tenData : [];
        if (loadedTenants.length > 0 && !selectedTenantId) {
          selectedTenantId = loadedTenants[0].tenant_id || loadedTenants[0].id || loadedTenants[0].code;
        }
      }
    } catch {
      // Ignored
    } finally {
      isLoadingContext = false;
    }
  }

  async function applyEnvironment() {
    isApplyingEnvironment = true;
    try {
      if (selectedCompanyCode) shell.activeCompany = selectedCompanyCode;
      if (selectedTenantId) {
        const found = loadedTenants.find((t) => (t.tenant_id || t.id || t.code) === selectedTenantId);
        if (found) shell.activeTenant = found;
      }

      // Persist active environment
      localStorage.setItem(
        "bos_environment",
        JSON.stringify({
          database: selectedDb,
          tenantId: selectedTenantId,
          companyName: selectedCompanyCode
        })
      );

      await shell.refreshSystem();
      shell.closeDbManager();
    } finally {
      isApplyingEnvironment = false;
    }
  }

  // ==========================================
  // SUPERUSER ADMIN TOOLS (Vacuum, Reindex, Drop)
  // ==========================================

  async function runAdminMaintenance() {
    if (!isSuperUser) return;
    isRunningAdmin = true;
    try {
      const payload = {
        action: adminAction,
        full: adminAction === "vacuum" ? vacuumFull : undefined,
        table: adminAction === "reindex" && reindexTable.trim() ? reindexTable.trim() : undefined,
        storage_strategy: adminAction === "update_strategy" ? strategyUpdate : undefined
      };

      const res = await fetch(`/api/databases/${encodeURIComponent(adminDbTarget)}/manage`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify(payload)
      });

      if (res.ok) {
        const result = await res.json();
        statusBanner = { type: "success", text: result.message || "Maintenance executed." };
        await loadDatabases();
      } else {
        const err = await res.json().catch(() => ({}));
        statusBanner = { type: "error", text: formatErrorMessage(err, "Operation failed") };
      }
    } catch (err) {
      statusBanner = { type: "error", text: formatErrorMessage(err, "Operation failed") };
    } finally {
      isRunningAdmin = false;
    }
  }

  async function dropDatabase() {
    if (!isSuperUser || deleteConfirmation.trim() !== adminDbTarget.trim()) return;
    isRunningAdmin = true;
    try {
      const res = await fetch(`/api/databases/${encodeURIComponent(adminDbTarget)}?force=true`, {
        method: "DELETE",
        headers: authHeaders()
      });

      if (res.ok) {
        statusBanner = { type: "success", text: `Database '${adminDbTarget}' dropped.` };
        deleteConfirmation = "";
        await loadDatabases();
        currentStep = "database";
        await shell.refreshSystem();
      } else {
        const err = await res.json().catch(() => ({}));
        statusBanner = { type: "error", text: formatErrorMessage(err, "Drop failed") };
      }
    } catch (err) {
      statusBanner = { type: "error", text: formatErrorMessage(err, "Drop failed") };
    } finally {
      isRunningAdmin = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => { if (e.key === "Escape") shell.closeDbManager(); }} />

{#if shell.isDbManagerOpen}
  <div
    role="presentation"
    class="fixed inset-0 z-50 bg-slate-950/70 backdrop-blur-xs flex items-end sm:items-center justify-center p-0 sm:p-4 animate-in fade-in-50 duration-150"
    onclick={() => shell.closeDbManager()}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="db-manager-title"
      tabindex="-1"
      class="bg-white dark:bg-slate-900 border-t sm:border border-slate-200 dark:border-slate-800 rounded-t-2xl sm:rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col overflow-hidden max-h-[94vh] sm:max-h-[85vh] animate-in slide-in-from-bottom-4 sm:zoom-in-95 duration-150 text-xs"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- COMPACT HEADER WITH LIVE ENVIRONMENT PILL -->
      <div class="px-4 py-3 border-b border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-800/50 flex items-center justify-between shrink-0">
        <div class="flex items-center gap-2.5">
          <div class="size-8 rounded-lg bg-indigo-600 text-white flex items-center justify-center shadow-xs">
            <Database class="size-4" />
          </div>
          <div>
            <div class="flex items-center gap-1.5 flex-wrap">
              <h2 id="db-manager-title" class="font-bold text-slate-900 dark:text-slate-100 text-xs sm:text-sm">
                Database Manager
              </h2>
              <span class="inline-flex items-center gap-1 px-1.5 py-0.2 rounded-full text-[10px] font-medium bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400">
                <span class="size-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
                {shell.activeDatabase || "postgres"}
              </span>
              {#if isSuperUser}
                <span class="inline-flex items-center gap-0.5 px-1.5 py-0.2 rounded-full text-[9px] font-semibold bg-purple-100 dark:bg-purple-950/70 text-purple-700 dark:text-purple-300">
                  <Crown class="size-2.5" />
                  SUPER
                </span>
              {/if}
            </div>
            <!-- Dynamic Path Status Strip -->
            <div class="flex items-center gap-1 text-[10px] text-slate-400 font-mono mt-0.5 truncate max-w-[280px] sm:max-w-md">
              <span class="truncate">{activeServerObj.name}</span>
              <span>&rarr;</span>
              <span class="text-indigo-600 dark:text-indigo-400 font-medium truncate">{selectedDb}</span>
              {#if selectedCompanyCode}
                <span>&rarr;</span>
                <span class="truncate">{selectedCompanyCode}</span>
              {/if}
            </div>
          </div>
        </div>

        <button
          type="button"
          onclick={() => shell.closeDbManager()}
          class="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer"
          title="Close dialog"
        >
          <X class="size-4" />
        </button>
      </div>

      <!-- RESPONSIVE 3-STEP NAVIGATION BAR (Server -> Database -> Tenant/Company) -->
      <div class="px-3 sm:px-4 py-2 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex items-center justify-between gap-1 overflow-x-auto scrollbar-none shrink-0">
        <div class="flex items-center gap-1 flex-1">
          <!-- Step 1: Server -->
          <button
            type="button"
            onclick={() => (currentStep = "server")}
            class={cn(
              "flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg font-semibold transition-all cursor-pointer whitespace-nowrap text-[11px]",
              currentStep === "server"
                ? "bg-indigo-600 text-white shadow-2xs"
                : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
            )}
          >
            <Server class="size-3.5" />
            <span>1. Server</span>
          </button>

          <span class="text-slate-300 dark:text-slate-700">&rarr;</span>

          <!-- Step 2: Database -->
          <button
            type="button"
            onclick={() => {
              currentStep = "database";
              loadDatabases();
            }}
            class={cn(
              "flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg font-semibold transition-all cursor-pointer whitespace-nowrap text-[11px]",
              currentStep === "database"
                ? "bg-indigo-600 text-white shadow-2xs"
                : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
            )}
          >
            <Database class="size-3.5" />
            <span>2. Database</span>
            {#if shell.availableDatabases?.length}
              <span class="px-1.5 py-0.2 rounded-full text-[9px] font-mono bg-black/10 dark:bg-white/10">
                {shell.availableDatabases.length}
              </span>
            {/if}
          </button>

          <span class="text-slate-300 dark:text-slate-700">&rarr;</span>

          <!-- Step 3: Tenant or Company -->
          <button
            type="button"
            onclick={() => {
              currentStep = "context";
              loadContextForDatabase(selectedDb);
            }}
            class={cn(
              "flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg font-semibold transition-all cursor-pointer whitespace-nowrap text-[11px]",
              currentStep === "context"
                ? "bg-indigo-600 text-white shadow-2xs"
                : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
            )}
          >
            <Building2 class="size-3.5" />
            <span>3. Tenant & Company</span>
          </button>
        </div>

        <!-- Superuser Admin Tools Tab (Only for Superusers) -->
        {#if isSuperUser}
          <button
            type="button"
            onclick={() => (currentStep = "admin")}
            class={cn(
              "flex items-center gap-1 px-2 py-1 rounded-lg font-semibold text-[10px] transition-all cursor-pointer whitespace-nowrap ml-1 shrink-0",
              currentStep === "admin"
                ? "bg-purple-600 text-white shadow-2xs"
                : "text-purple-600 dark:text-purple-400 bg-purple-50 dark:bg-purple-950/40 hover:bg-purple-100"
            )}
            title="Superuser database alteration and maintenance"
          >
            <Wrench class="size-3" />
            <span>Admin Tools</span>
          </button>
        {/if}
      </div>

      <!-- STATUS BANNER -->
      {#if statusBanner.text}
        <div
          class={cn(
            "px-4 py-2 text-[11px] flex items-center justify-between border-b shrink-0 animate-in fade-in-50",
            statusBanner.type === "error"
              ? "bg-rose-50 dark:bg-rose-950/50 border-rose-200 dark:border-rose-900 text-rose-700 dark:text-rose-300"
              : statusBanner.type === "success"
              ? "bg-emerald-50 dark:bg-emerald-950/50 border-emerald-200 dark:border-emerald-900 text-emerald-700 dark:text-emerald-300"
              : "bg-indigo-50 dark:bg-indigo-950/50 border-indigo-200 dark:border-indigo-900 text-indigo-700 dark:text-indigo-300"
          )}
        >
          <div class="flex items-center gap-1.5 font-medium">
            {#if statusBanner.type === "error"}
              <AlertTriangle class="size-3.5 shrink-0 text-rose-500" />
            {:else if statusBanner.type === "success"}
              <CheckCircle2 class="size-3.5 shrink-0 text-emerald-500" />
            {:else}
              <Activity class="size-3.5 shrink-0 text-indigo-500 animate-spin" />
            {/if}
            <span>{statusBanner.text}</span>
          </div>
          <button
            type="button"
            onclick={() => (statusBanner = { type: "", text: "" })}
            class="p-0.5 rounded hover:bg-black/5 dark:hover:bg-white/5 cursor-pointer ml-2"
          >
            <X class="size-3" />
          </button>
        </div>
      {/if}

      <!-- MAIN BODY CONTENT AREA -->
      <div class="p-4 overflow-y-auto flex-1 space-y-4">
        <!-- ========================================== -->
        <!-- STEP 1: SELECT SERVER (With Last 3 1-Click Connect) -->
        <!-- ========================================== -->
        {#if currentStep === "server"}
          <div class="space-y-3.5 animate-in fade-in-50">
            <!-- 1. Last 3 Successfully Connected Servers -->
            <div class="space-y-1.5">
              <span class="font-bold text-slate-700 dark:text-slate-300 text-[11px] uppercase tracking-wider block">
                Last Connected Servers (1-Click Connect)
              </span>

              <div class="grid grid-cols-1 sm:grid-cols-3 gap-2">
                {#each recentServers as srv}
                  {@const isCurrent = activeServerObj.host === srv.host || activeServerObj.url === srv.url}
                  <button
                    type="button"
                    onclick={() => connectToServer(srv)}
                    disabled={isConnectingServer}
                    class={cn(
                      "p-2.5 rounded-xl border text-left cursor-pointer transition-all flex flex-col justify-between gap-1.5 relative group",
                      isCurrent
                        ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600 shadow-2xs"
                        : "border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 bg-white dark:bg-slate-900/60 text-slate-700 dark:text-slate-300"
                    )}
                  >
                    <div class="flex items-center justify-between gap-1">
                      <span class="font-bold truncate text-xs">{srv.name}</span>
                      <span class="size-2 rounded-full shrink-0 bg-emerald-500"></span>
                    </div>

                    <div class="font-mono text-[10px] text-slate-400 truncate">
                      {srv.host}
                    </div>

                    <div class="flex items-center justify-between pt-1 border-t border-slate-100 dark:border-slate-800/80 text-[10px]">
                      <span class="text-slate-400">{srv.lastConnected}</span>
                      <span class="text-indigo-600 dark:text-indigo-400 font-semibold group-hover:underline flex items-center gap-0.5">
                        <Zap class="size-2.5" />
                        <span>Connect</span>
                      </span>
                    </div>
                  </button>
                {/each}
              </div>
            </div>

            <!-- 2. Connect to New or Remote Cloud Server -->
            <div class="p-3.5 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40 space-y-2.5">
              <div class="flex items-center justify-between">
                <span class="font-bold text-slate-800 dark:text-slate-200 text-xs">
                  Connect Custom or Cloud PostgreSQL
                </span>

                <!-- Cloud Preset Shortcuts -->
                <div class="flex items-center gap-1 text-[10px]">
                  <button
                    type="button"
                    onclick={() => (serverConnectionUrl = "postgresql://postgres:[PASSWORD]@db.[REF].supabase.co:5432/postgres?sslmode=require")}
                    class="px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 hover:underline cursor-pointer"
                  >
                    Supabase
                  </button>
                  <button
                    type="button"
                    onclick={() => (serverConnectionUrl = "postgresql://[USER]:[PASSWORD]@[ENDPOINT].neon.tech/neondb?sslmode=require")}
                    class="px-1.5 py-0.5 rounded bg-cyan-100 dark:bg-cyan-950/60 text-cyan-700 dark:text-cyan-300 hover:underline cursor-pointer"
                  >
                    Neon
                  </button>
                  <button
                    type="button"
                    onclick={() => (serverConnectionUrl = "postgresql://postgres:postgres@localhost:5432/postgres")}
                    class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 hover:underline cursor-pointer"
                  >
                    Local
                  </button>
                </div>
              </div>

              <div class="flex gap-2">
                <input
                  type="text"
                  bind:value={serverConnectionUrl}
                  placeholder="postgresql://user:password@host:5432/postgres?sslmode=require"
                  class="flex-1 px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs focus:ring-1 focus:ring-indigo-500 focus:outline-hidden"
                />

                <button
                  type="button"
                  onclick={() => testAndConnectServer(serverConnectionUrl)}
                  disabled={isConnectingServer}
                  class="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white font-semibold cursor-pointer shrink-0 text-xs flex items-center gap-1.5 shadow-2xs"
                >
                  {#if isConnectingServer}
                    <Activity class="size-3.5 animate-spin" />
                  {:else}
                    <ArrowRight class="size-3.5" />
                  {/if}
                  <span>Test & Next &rarr;</span>
                </button>
              </div>

              {#if serverConnectionResult}
                <div class="text-[11px] font-mono flex items-center gap-2 pt-1 text-slate-600 dark:text-slate-400">
                  <span class="inline-flex items-center gap-1 font-bold text-emerald-600 dark:text-emerald-400">
                    <Check class="size-3" />
                    Online ({serverConnectionResult.latency_ms}ms)
                  </span>
                  {#if serverConnectionResult.server_version}
                    <span class="truncate max-w-xs">{serverConnectionResult.server_version}</span>
                  {/if}
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- ========================================== -->
        <!-- STEP 2: SELECT DATABASE (With Search & +Create) -->
        <!-- ========================================== -->
        {#if currentStep === "database"}
          <div class="space-y-3 animate-in fade-in-50">
            <!-- Header bar with search and +New DB toggle -->
            <div class="flex items-center justify-between gap-2">
              <div class="relative flex-1">
                <Search class="size-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
                <input
                  type="text"
                  bind:value={dbSearchQuery}
                  placeholder="Filter databases..."
                  class="w-full pl-8 pr-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs focus:outline-hidden focus:ring-1 focus:ring-indigo-500"
                />
              </div>

              <button
                type="button"
                onclick={() => (showCreateDbForm = !showCreateDbForm)}
                class="flex items-center gap-1 px-2.5 py-1.5 rounded-lg bg-indigo-50 dark:bg-indigo-950/50 hover:bg-indigo-100 text-indigo-700 dark:text-indigo-300 font-semibold cursor-pointer shrink-0 text-xs"
              >
                <Plus class="size-3.5" />
                <span>{showCreateDbForm ? "Hide" : "+ Create Database"}</span>
              </button>

              <button
                type="button"
                onclick={loadDatabases}
                class="p-1.5 rounded-lg border border-slate-200 dark:border-slate-800 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 cursor-pointer"
                title="Refresh databases"
              >
                <RefreshCw class="size-3.5" />
              </button>
            </div>

            <!-- Inline Compact Database Creation Form -->
            {#if showCreateDbForm}
              <form
                onsubmit={(e) => { e.preventDefault(); createDatabase(); }}
                class="p-3 rounded-xl border border-indigo-200 dark:border-indigo-900/60 bg-indigo-50/30 dark:bg-indigo-950/20 space-y-2 animate-in fade-in-50"
              >
                <div class="flex items-center justify-between">
                  <span class="font-bold text-indigo-950 dark:text-indigo-200 text-xs">
                    Create & Provision New Database
                  </span>
                  <span class="text-[10px] text-slate-400">Available to all users</span>
                </div>

                <div class="flex gap-2">
                  <input
                    type="text"
                    bind:value={newDbName}
                    placeholder="database_name (lowercase, alphanumeric)"
                    class="flex-1 px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs focus:ring-1 focus:ring-indigo-500 focus:outline-hidden"
                    required
                  />
                  <select
                    bind:value={newStorageStrategy}
                    class="px-2 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs font-mono"
                  >
                    <option value="shared_rls">shared_rls (Shared Tables + RLS)</option>
                    <option value="partitioned">partitioned (Native Partitions)</option>
                    <option value="schema_per_company">schema_per_company</option>
                  </select>
                </div>

                <div class="flex items-center justify-between pt-1">
                  <label class="flex items-center gap-1.5 text-[11px] text-slate-600 dark:text-slate-400 cursor-pointer">
                    <input type="checkbox" bind:checked={importBasePackages} class="rounded border-slate-300 text-indigo-600" />
                    <span>Auto-import base packages (bos.security, bos.ledger)</span>
                  </label>

                  <button
                    type="submit"
                    disabled={isCreatingDb}
                    class="px-3 py-1 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs flex items-center gap-1 cursor-pointer shadow-2xs"
                  >
                    {#if isCreatingDb}
                      <Activity class="size-3 animate-spin" />
                    {:else}
                      <Sparkles class="size-3" />
                    {/if}
                    <span>Create & Switch</span>
                  </button>
                </div>
              </form>
            {/if}

            <!-- Database Cards Grid -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 max-h-[320px] overflow-y-auto">
              {#each filteredDatabases as db}
                {@const name = typeof db === "string" ? db : db.name}
                {@const isCurrent = name === (selectedDb || shell.activeDatabase)}
                {@const strategy = typeof db === "object" ? db.storage_strategy || "shared_rls" : "shared_rls"}
                {@const size = typeof db === "object" ? db.size_pretty || "" : ""}

                <button
                  type="button"
                  onclick={() => selectDatabase(name)}
                  disabled={isSwitchingDb}
                  class={cn(
                    "p-2.5 rounded-xl border text-left cursor-pointer transition-all flex items-center justify-between gap-2 group",
                    isCurrent
                      ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600 shadow-2xs"
                      : "border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 bg-white dark:bg-slate-900/60 text-slate-700 dark:text-slate-300"
                  )}
                >
                  <div class="flex items-center gap-2 truncate">
                    <div class={cn(
                      "size-7 rounded-lg flex items-center justify-center shrink-0",
                      isCurrent ? "bg-indigo-600 text-white" : "bg-slate-100 dark:bg-slate-800 text-slate-500"
                    )}>
                      <Database class="size-3.5" />
                    </div>
                    <div class="truncate">
                      <div class="flex items-center gap-1">
                        <span class="font-bold text-xs truncate">{name}</span>
                        {#if isCurrent}
                          <span class="size-1.5 rounded-full bg-emerald-500"></span>
                        {/if}
                      </div>
                      <div class="text-[9px] text-slate-400 font-mono">
                        {strategy} {size ? `• ${size}` : ""}
                      </div>
                    </div>
                  </div>

                  <div class="shrink-0 flex items-center gap-1 text-[11px] font-semibold text-indigo-600 dark:text-indigo-400 group-hover:translate-x-0.5 transition-transform">
                    <span>{isCurrent ? "Active" : "Select"}</span>
                    {#if !isCurrent}
                      <ArrowRight class="size-3" />
                    {/if}
                  </div>
                </button>
              {/each}

              {#if filteredDatabases.length === 0}
                <div class="col-span-full p-6 text-center text-slate-400 border border-dashed border-slate-200 dark:border-slate-800 rounded-xl">
                  No databases match your search. Click "+ Create Database".
                </div>
              {/if}
            </div>
          </div>
        {/if}

        <!-- ========================================== -->
        <!-- STEP 3: SELECT TENANT OR COMPANY -->
        <!-- ========================================== -->
        {#if currentStep === "context"}
          <div class="space-y-3.5 animate-in fade-in-50">
            <div class="p-2.5 rounded-lg bg-indigo-50/40 dark:bg-indigo-950/20 border border-indigo-100 dark:border-indigo-900/40 flex items-center justify-between text-xs">
              <span class="font-medium text-slate-700 dark:text-slate-300">
                Connected Database: <strong class="text-indigo-600 dark:text-indigo-400 font-mono">{selectedDb}</strong>
              </span>
              <button
                type="button"
                onclick={() => (currentStep = "database")}
                class="text-indigo-600 dark:text-indigo-400 hover:underline cursor-pointer text-[11px]"
              >
                Change DB &larr;
              </button>
            </div>

            <!-- Company Selector -->
            <div class="space-y-1.5">
              <label class="font-bold text-slate-700 dark:text-slate-300 flex items-center justify-between text-xs">
                <div class="flex items-center gap-1.5">
                  <Building2 class="size-3.5 text-indigo-500" />
                  <span>Select Active Company</span>
                </div>
                {#if isLoadingContext}
                  <Activity class="size-3 text-indigo-500 animate-spin" />
                {/if}
              </label>

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {#each loadedCompanies as comp}
                  {@const isSelected = selectedCompanyCode === comp.name || selectedCompanyCode === comp.code}
                  <button
                    type="button"
                    onclick={() => (selectedCompanyCode = comp.name || comp.code)}
                    class={cn(
                      "p-2.5 rounded-xl border text-left cursor-pointer transition-all flex items-center justify-between gap-2",
                      isSelected
                        ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                        : "border-slate-200 dark:border-slate-800 hover:border-slate-300 text-slate-700 dark:text-slate-300"
                    )}
                  >
                    <div>
                      <div class="font-bold text-xs">{comp.name}</div>
                      <div class="text-[10px] text-slate-400 font-mono">{comp.code || "CRONUS"}</div>
                    </div>
                    {#if isSelected}
                      <Check class="size-4 text-indigo-600 dark:text-indigo-400" />
                    {/if}
                  </button>
                {/each}

                {#if loadedCompanies.length === 0 && !isLoadingContext}
                  <div class="col-span-full p-3 rounded-lg border border-slate-200 dark:border-slate-800 text-slate-400 text-center">
                    Using system default company (CRONUS)
                  </div>
                {/if}
              </div>
            </div>

            <!-- Tenant Selector (if multi-tenant) -->
            {#if loadedTenants.length > 1}
              <div class="space-y-1.5 pt-1">
                <label for="ctx-tenant-select" class="font-bold text-slate-700 dark:text-slate-300 flex items-center gap-1.5 text-xs">
                  <Users class="size-3.5 text-indigo-500" />
                  <span>Tenant Partition</span>
                </label>
                <select
                  id="ctx-tenant-select"
                  bind:value={selectedTenantId}
                  class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs font-mono"
                >
                  {#each loadedTenants as ten}
                    <option value={ten.tenant_id || ten.id || ten.code}>
                      {ten.name} ({ten.code || "DEFAULT"})
                    </option>
                  {/each}
                </select>
              </div>
            {/if}

            <!-- Apply & Complete Action -->
            <div class="pt-2 border-t border-slate-200 dark:border-slate-800 flex items-center justify-end gap-2">
              <button
                type="button"
                onclick={() => (currentStep = "database")}
                class="px-3 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800 font-medium cursor-pointer"
              >
                &larr; Back to Databases
              </button>

              <button
                type="button"
                onclick={applyEnvironment}
                disabled={isApplyingEnvironment}
                class="px-4 py-1.5 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white font-bold cursor-pointer text-xs flex items-center gap-1.5 shadow-2xs"
              >
                {#if isApplyingEnvironment}
                  <Activity class="size-3.5 animate-spin" />
                {:else}
                  <Check class="size-3.5" />
                {/if}
                <span>Apply & Switch Environment</span>
              </button>
            </div>
          </div>
        {/if}

        <!-- ========================================== -->
        <!-- STEP 4: SUPERUSER ADMIN TOOLS (Vacuum, Reindex, Drop) -->
        <!-- ========================================== -->
        {#if currentStep === "admin" && isSuperUser}
          <div class="space-y-3.5 animate-in fade-in-50">
            <div class="p-2.5 rounded-lg bg-purple-50 dark:bg-purple-950/30 border border-purple-200 dark:border-purple-900/50 flex items-center justify-between text-xs text-purple-950 dark:text-purple-200">
              <span class="font-bold flex items-center gap-1.5">
                <Crown class="size-3.5 text-purple-600 dark:text-purple-400" />
                <span>Superuser Database Administration</span>
              </span>
              <span class="font-mono text-[10px] text-purple-500">Target: {adminDbTarget}</span>
            </div>

            <!-- Target DB Selector -->
            <div class="space-y-1">
              <label for="admin-db-select" class="font-bold text-slate-700 dark:text-slate-300 block text-xs">Target Database</label>
              <select
                id="admin-db-select"
                bind:value={adminDbTarget}
                class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs font-mono"
              >
                {#each shell.availableDatabases as db}
                  {@const name = typeof db === "string" ? db : db.name}
                  <option value={name}>{name}</option>
                {/each}
              </select>
            </div>

            <!-- Action Tabs: Vacuum, Reindex, Strategy, Drop -->
            <div class="flex gap-1 border-b border-slate-200 dark:border-slate-800 pb-2">
              {#each ["vacuum", "reindex", "update_strategy", "drop"] as act}
                <button
                  type="button"
                  onclick={() => (adminAction = act)}
                  class={cn(
                    "px-2.5 py-1 rounded-lg text-xs font-semibold cursor-pointer capitalize transition-all",
                    adminAction === act
                      ? act === "drop"
                        ? "bg-rose-600 text-white shadow-2xs"
                        : "bg-purple-600 text-white shadow-2xs"
                      : "text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800"
                  )}
                >
                  {act === "update_strategy" ? "Alter Strategy" : act}
                </button>
              {/each}
            </div>

            <!-- Action Form Fields -->
            {#if adminAction === "vacuum"}
              <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 space-y-2">
                <label class="flex items-center gap-2 cursor-pointer font-medium text-slate-700 dark:text-slate-300 text-xs">
                  <input type="checkbox" bind:checked={vacuumFull} class="rounded text-purple-600" />
                  <span>Execute VACUUM FULL (Reclaims dead tuple disk storage)</span>
                </label>
                <button
                  type="button"
                  onclick={runAdminMaintenance}
                  disabled={isRunningAdmin}
                  class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-700 text-white font-bold text-xs cursor-pointer shadow-2xs"
                >
                  Execute Vacuum
                </button>
              </div>
            {:else if adminAction === "reindex"}
              <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 space-y-2">
                <input
                  type="text"
                  bind:value={reindexTable}
                  placeholder="Specific table (leave blank for entire database)"
                  class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs"
                />
                <button
                  type="button"
                  onclick={runAdminMaintenance}
                  disabled={isRunningAdmin}
                  class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-700 text-white font-bold text-xs cursor-pointer shadow-2xs"
                >
                  Execute Reindex
                </button>
              </div>
            {:else if adminAction === "update_strategy"}
              <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 space-y-2">
                <select
                  bind:value={strategyUpdate}
                  class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs font-mono"
                >
                  <option value="shared_rls">shared_rls (Shared Tables + RLS)</option>
                  <option value="partitioned">partitioned (Native Partitions)</option>
                  <option value="schema_per_company">schema_per_company</option>
                </select>
                <button
                  type="button"
                  onclick={runAdminMaintenance}
                  disabled={isRunningAdmin}
                  class="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-700 text-white font-bold text-xs cursor-pointer shadow-2xs"
                >
                  Update Storage Strategy
                </button>
              </div>
            {:else if adminAction === "drop"}
              <div class="p-3 rounded-lg border border-rose-200 dark:border-rose-900 bg-rose-50/30 dark:bg-rose-950/20 space-y-2 text-rose-800 dark:text-rose-200">
                <span class="font-bold text-xs block">Danger Zone: Permanent Destruction</span>
                <p class="text-[10px] opacity-90">To drop, type <code class="font-mono font-bold select-all">{adminDbTarget}</code> below:</p>
                <input
                  type="text"
                  bind:value={deleteConfirmation}
                  placeholder="Type exact database name"
                  class="w-full px-2.5 py-1.5 rounded-lg border border-rose-300 dark:border-rose-900 bg-white dark:bg-slate-950 font-mono text-xs"
                />
                <button
                  type="button"
                  onclick={dropDatabase}
                  disabled={isRunningAdmin || deleteConfirmation.trim() !== adminDbTarget.trim()}
                  class={cn(
                    "px-3 py-1.5 rounded-lg text-white font-bold text-xs cursor-pointer shadow-2xs",
                    deleteConfirmation.trim() === adminDbTarget.trim()
                      ? "bg-rose-600 hover:bg-rose-700"
                      : "bg-slate-300 dark:bg-slate-800 text-slate-500 cursor-not-allowed"
                  )}
                >
                  Permanently Drop Database
                </button>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- COMPACT FOOTER -->
      <div class="px-4 py-2.5 border-t border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-800/30 flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400 shrink-0">
        <div class="flex items-center gap-1.5 font-mono">
          <Server class="size-3 text-slate-400" />
          <span class="truncate max-w-[200px] sm:max-w-xs">{activeServerObj.host}</span>
        </div>

        <button
          type="button"
          onclick={() => shell.closeDbManager()}
          class="px-3 py-1 rounded-lg border border-slate-200 dark:border-slate-800 hover:bg-slate-100 dark:hover:bg-slate-800 font-medium cursor-pointer text-xs"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
