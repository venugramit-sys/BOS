<script>
  import { onMount } from "svelte";
  import { shell } from "./shellStore.svelte.js";
  import {
    Database,
    Server,
    Shield,
    Building2,
    Users,
    Package,
    Sliders,
    Check,
    CheckCircle2,
    AlertTriangle,
    ArrowRight,
    ArrowLeft,
    Sparkles,
    Activity,
    Layers,
    Lock,
    User,
    Mail,
    Cloud,
    Globe,
    Eye,
    EyeOff,
    Terminal,
    RefreshCw
  } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let { onComplete, onGoToSignIn } = $props();

  // Wizard Step: 1 | 2 | 3 | 4 | 5 (progress)
  let currentStep = $state(1);

  // Form State
  // Step 1: Cluster & Database (Local vs Cloud Provider)
  let providerType = $state("local"); // 'local' | 'cloud'
  let cloudPreset = $state("neon"); // 'neon' | 'supabase' | 'aws' | 'azure' | 'custom'

  // Local settings
  let dbName = $state("bos_enterprise");
  let detectedInstances = $state([]);
  let isCheckingCluster = $state(false);
  let clusterOnline = $state(true);

  // Cloud settings
  let cloudConnectionUrl = $state("");
  let showCloudPassword = $state(false);
  let isTestingCloud = $state(false);
  let cloudTestResult = $state(null);

  // Step 2: Storage Strategy & Packages
  let storageStrategy = $state("shared_rls");
  let importBasePackages = $state(true);
  let selectedPackages = $state(["bos.security", "bos.ledger"]);

  // Step 3: Organization & Scope
  let tenantCode = $state("DEFAULT");
  let tenantName = $state("Default Enterprise Tenant");
  let companyCode = $state("CRONUS");
  let companyName = $state("Cronus International Ltd");

  // Step 4: Superuser Credentials
  let adminUsername = $state("admin");
  let adminDisplayName = $state("System Administrator");
  let adminEmail = $state("admin@cronus.corp");
  let adminPassword = $state("Boss@123");
  let adminPasswordConfirm = $state("Boss@123");

  // Step 5: Provisioning Progress
  let isProvisioning = $state(false);
  let provisioningStage = $state(0); // 1: db, 2: kernel, 3: packages, 4: superuser, 5: done
  let errorMessage = $state("");

  onMount(async () => {
    isCheckingCluster = true;
    try {
      const res = await fetch("/api/detect-db");
      if (res.ok) {
        const data = await res.json();
        detectedInstances = Array.isArray(data) ? data : [data];
        clusterOnline = detectedInstances.some((i) => i.is_running);
      }
    } catch {
      clusterOnline = false;
    } finally {
      isCheckingCluster = false;
    }
  });

  const CLOUD_PRESETS = [
    {
      id: "neon",
      name: "Neon Serverless",
      badge: "Serverless",
      placeholder: "postgresql://user:password@ep-cool-lake-1234.us-east-2.aws.neon.tech/neondb?sslmode=require",
      tip: "Copy connection string from your Neon console. SSL mode `require` is recommended."
    },
    {
      id: "supabase",
      name: "Supabase",
      badge: "Managed",
      placeholder: "postgresql://postgres.[ref]:[password]@aws-0-[region].pooler.supabase.com:6543/postgres",
      tip: "Use the Supabase Session Pooler URL (port 6543 or direct port 5432)."
    },
    {
      id: "aws",
      name: "AWS RDS / Aurora",
      badge: "Enterprise",
      placeholder: "postgresql://postgres:password@database-1.c9ak4z3w.us-east-1.rds.amazonaws.com:5432/bos_db",
      tip: "Ensure your AWS security group allows inbound TCP on port 5432."
    },
    {
      id: "azure",
      name: "Azure PostgreSQL",
      badge: "Flexible Server",
      placeholder: "postgresql://adminuser:password@my-server.postgres.database.azure.com:5432/bos_db?sslmode=require",
      tip: "Ensure 'Allow public access from Azure services and resources' or your IP is whitelisted."
    },
    {
      id: "custom",
      name: "Custom Cloud Host",
      badge: "Remote URI",
      placeholder: "postgresql://username:password@hostname:5432/database_name?sslmode=require",
      tip: "Standard PostgreSQL connection URI format with optional sslmode parameters."
    }
  ];

  function applyPreset(presetId) {
    cloudPreset = presetId;
    cloudTestResult = null;
    const p = CLOUD_PRESETS.find((x) => x.id === presetId);
    if (!cloudConnectionUrl) {
      cloudConnectionUrl = p?.placeholder || "";
    }
  }

  async function testCloudConnection() {
    const url = cloudConnectionUrl.trim();
    if (!url) {
      errorMessage = "Please enter a connection URL to test.";
      return;
    }
    if (!url.startsWith("postgres://") && !url.startsWith("postgresql://")) {
      errorMessage = "Connection string must start with postgres:// or postgresql://";
      return;
    }

    isTestingCloud = true;
    cloudTestResult = null;
    errorMessage = "";

    try {
      const res = await fetch("/api/databases/test-connection", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ url })
      });
      if (res.ok) {
        cloudTestResult = await res.json();
      } else {
        cloudTestResult = { success: false, message: "Backend connection test failed." };
      }
    } catch (err) {
      cloudTestResult = { success: false, message: err.message };
    } finally {
      isTestingCloud = false;
    }
  }

  function handlePackageToggle(pkgId) {
    if (selectedPackages.includes(pkgId)) {
      selectedPackages = selectedPackages.filter((p) => p !== pkgId);
    } else {
      selectedPackages = [...selectedPackages, pkgId];
    }
  }

  function validateStep(step) {
    errorMessage = "";
    if (step === 1) {
      if (providerType === "local") {
        if (!dbName.trim()) {
          errorMessage = "Please specify a target database name.";
          return false;
        }
      } else {
        const url = cloudConnectionUrl.trim();
        if (!url) {
          errorMessage = "Please provide your cloud PostgreSQL connection string.";
          return false;
        }
        if (!url.startsWith("postgres://") && !url.startsWith("postgresql://")) {
          errorMessage = "Cloud connection string must begin with postgres:// or postgresql://";
          return false;
        }
      }
    } else if (step === 3) {
      if (!tenantCode.trim() || !companyCode.trim()) {
        errorMessage = "Tenant Code and Company Code are required.";
        return false;
      }
    } else if (step === 4) {
      if (!adminUsername.trim() || !adminDisplayName.trim() || !adminEmail.trim()) {
        errorMessage = "All administrator fields are required.";
        return false;
      }
      if (!adminEmail.includes("@")) {
        errorMessage = "Please provide a valid administrator email address.";
        return false;
      }
      if (adminPassword.length < 8) {
        errorMessage = "Administrator password must be at least 8 characters long.";
        return false;
      }
      if (adminPassword !== adminPasswordConfirm) {
        errorMessage = "Passwords do not match.";
        return false;
      }
    }
    return true;
  }

  function nextStep() {
    if (validateStep(currentStep)) {
      currentStep += 1;
    }
  }

  function prevStep() {
    errorMessage = "";
    if (currentStep > 1) {
      currentStep -= 1;
    }
  }

  async function executeProvisioning() {
    if (!validateStep(4)) return;

    currentStep = 5;
    isProvisioning = true;
    provisioningStage = 1;
    errorMessage = "";

    try {
      const stageTimer1 = setTimeout(() => { if (isProvisioning) provisioningStage = 2; }, 800);
      const stageTimer2 = setTimeout(() => { if (isProvisioning) provisioningStage = 3; }, 1800);

      const payload = {
        database: providerType === "local" ? dbName.trim().toLowerCase().replace(/[^a-z0-9_]/g, "_") : undefined,
        connection_url: providerType === "cloud" ? cloudConnectionUrl.trim() : undefined,
        storage_strategy: storageStrategy,
        import_base_packages: importBasePackages,
        packages: importBasePackages ? selectedPackages : [],
        tenant_code: tenantCode.trim().toUpperCase(),
        tenant_name: tenantName.trim(),
        company_code: companyCode.trim().toUpperCase(),
        company_name: companyName.trim(),
        admin_username: adminUsername.trim(),
        admin_password: adminPassword,
        admin_display_name: adminDisplayName.trim(),
        admin_email: adminEmail.trim()
      };

      const response = await fetch("/api/setup/provision", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });

      clearTimeout(stageTimer1);
      clearTimeout(stageTimer2);

      provisioningStage = 4;

      const result = await response.json().catch(() => ({}));

      if (!response.ok) {
        throw new Error(result.message || result.error || "Database provisioning failed.");
      }

      provisioningStage = 5;

      // Automatically sign in with generated session
      shell.signIn(result);
      await shell.refreshSystem();

      // Brief delay to allow the user to see the completed state
      setTimeout(() => {
        onComplete?.(result);
      }, 1200);

    } catch (err) {
      errorMessage = err.message || "An unexpected error occurred during system provisioning.";
      currentStep = 4;
    } finally {
      isProvisioning = false;
    }
  }
</script>

<div class="min-h-screen bg-slate-50 dark:bg-slate-950 flex flex-col justify-center items-center p-3 sm:p-6 select-none transition-colors">
  <div class="w-full max-w-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl overflow-hidden flex flex-col">
    <!-- Wizard Header -->
    <div class="px-6 py-5 border-b border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/50 flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="size-10 rounded-xl bg-indigo-600 text-white flex items-center justify-center shadow-xs">
          <Layers class="size-5" />
        </div>
        <div>
          <h1 class="text-base font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <span>BOS Platform Setup</span>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300">
              First-Run Initializer
            </span>
          </h1>
          <p class="text-xs text-slate-500 dark:text-slate-400">
            Provision local or cloud database, install base meta-objects, and configure administrator.
          </p>
        </div>
      </div>

      <!-- Step Indicator -->
      {#if currentStep < 5}
        <div class="flex items-center gap-1.5 text-xs font-mono font-semibold text-slate-400">
          <span class="text-indigo-600 dark:text-indigo-400 font-bold">Step {currentStep}</span>
          <span>/</span>
          <span>4</span>
        </div>
      {/if}
    </div>

    <!-- Progress Steps Bar -->
    {#if currentStep < 5}
      <div class="grid grid-cols-4 border-b border-slate-100 dark:border-slate-800 text-[11px] font-medium text-center">
        <div class={cn("py-2.5 border-b-2 transition-colors", currentStep >= 1 ? "border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold" : "border-transparent text-slate-400")}>
          1. Database
        </div>
        <div class={cn("py-2.5 border-b-2 transition-colors", currentStep >= 2 ? "border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold" : "border-transparent text-slate-400")}>
          2. Packages
        </div>
        <div class={cn("py-2.5 border-b-2 transition-colors", currentStep >= 3 ? "border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold" : "border-transparent text-slate-400")}>
          3. Organization
        </div>
        <div class={cn("py-2.5 border-b-2 transition-colors", currentStep >= 4 ? "border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold" : "border-transparent text-slate-400")}>
          4. Administrator
        </div>
      </div>
    {/if}

    <!-- Error Banner -->
    {#if errorMessage}
      <div class="px-6 py-3 bg-rose-50 dark:bg-rose-950/40 border-b border-rose-200 dark:border-rose-900 text-rose-700 dark:text-rose-300 text-xs flex items-center gap-2">
        <AlertTriangle class="size-4 shrink-0 text-rose-500" />
        <span>{errorMessage}</span>
      </div>
    {/if}

    <!-- Wizard Step Content -->
    <div class="p-6 overflow-y-auto flex-1 text-xs space-y-4">
      <!-- STEP 1: CLUSTER & DATABASE (LOCAL VS CLOUD PROVIDER) -->
      {#if currentStep === 1}
        <div class="space-y-4 animate-in fade-in-50 duration-150">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100">Select Database Host Provider</h2>
            <p class="text-slate-400 text-xs mt-0.5">
              Connect to your local PostgreSQL instance or configure a managed Cloud database.
            </p>
          </div>

          <!-- Provider Switcher Tabs -->
          <div class="grid grid-cols-2 gap-2 p-1 rounded-xl bg-slate-100 dark:bg-slate-800/80">
            <button
              type="button"
              onclick={() => (providerType = "local")}
              class={cn(
                "py-2 px-3 rounded-lg font-semibold text-xs flex items-center justify-center gap-2 transition-all cursor-pointer",
                providerType === "local"
                  ? "bg-white dark:bg-slate-900 text-indigo-600 dark:text-indigo-400 shadow-xs"
                  : "text-slate-600 dark:text-slate-400 hover:text-slate-900"
              )}
            >
              <Server class="size-4" />
              <span>Local / On-Premise</span>
            </button>

            <button
              type="button"
              onclick={() => {
                providerType = "cloud";
                if (!cloudConnectionUrl) applyPreset(cloudPreset);
              }}
              class={cn(
                "py-2 px-3 rounded-lg font-semibold text-xs flex items-center justify-center gap-2 transition-all cursor-pointer",
                providerType === "cloud"
                  ? "bg-white dark:bg-slate-900 text-indigo-600 dark:text-indigo-400 shadow-xs"
                  : "text-slate-600 dark:text-slate-400 hover:text-slate-900"
              )}
            >
              <Cloud class="size-4" />
              <span>Cloud PostgreSQL Provider</span>
            </button>
          </div>

          <!-- LOCAL INSTANCE CONFIGURATION -->
          {#if providerType === "local"}
            <div class="space-y-3 animate-in fade-in-50">
              <!-- Cluster Status Card -->
              <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40 flex items-center justify-between">
                <div class="flex items-center gap-3">
                  <div class={cn(
                    "size-9 rounded-lg flex items-center justify-center shrink-0",
                    clusterOnline ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400" : "bg-amber-500/10 text-amber-600"
                  )}>
                    <Server class="size-5" />
                  </div>
                  <div>
                    <div class="flex items-center gap-2">
                      <span class="font-bold text-slate-800 dark:text-slate-200">Local PostgreSQL Service</span>
                      <span class={cn(
                        "px-2 py-0.5 rounded-full text-[9px] font-semibold",
                        clusterOnline ? "bg-emerald-100 text-emerald-700 dark:bg-emerald-950/60 dark:text-emerald-400" : "bg-amber-100 text-amber-700"
                      )}>
                        {clusterOnline ? "Detected & Listening" : "Probe Pending"}
                      </span>
                    </div>
                    <div class="text-[11px] text-slate-400 font-mono mt-0.5">
                      127.0.0.1:5432 (Ready for Kernel DDL)
                    </div>
                  </div>
                </div>
                {#if isCheckingCluster}
                  <Activity class="size-4 animate-spin text-slate-400" />
                {/if}
              </div>

              <!-- Database Name Field -->
              <div class="space-y-1.5">
                <label for="wizard-db-name" class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                  <Database class="size-3.5 text-indigo-500" />
                  <span>Target Database Name</span>
                </label>
                <input
                  id="wizard-db-name"
                  type="text"
                  bind:value={dbName}
                  placeholder="e.g. bos_enterprise"
                  class="w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs focus:ring-1 focus:ring-indigo-500 focus:outline-hidden"
                />
                <p class="text-[10px] text-slate-400">
                  If this database does not exist, BOS will automatically create it in your cluster during provisioning.
                </p>
              </div>
            </div>
          {/if}

          <!-- CLOUD MANAGED PROVIDER CONFIGURATION -->
          {#if providerType === "cloud"}
            <div class="space-y-3.5 animate-in fade-in-50">
              <!-- Cloud Presets -->
              <div class="space-y-1.5">
                <label class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                  <Globe class="size-3.5 text-indigo-500" />
                  <span>Supported Cloud Providers</span>
                </label>
                <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
                  {#each CLOUD_PRESETS as p}
                    <button
                      type="button"
                      onclick={() => applyPreset(p.id)}
                      class={cn(
                        "p-2.5 rounded-xl border text-left transition-all cursor-pointer flex flex-col justify-between",
                        cloudPreset === p.id
                          ? "border-indigo-600 bg-indigo-50/40 dark:bg-indigo-950/40 text-indigo-900 dark:text-indigo-200 ring-1 ring-indigo-600"
                          : "border-slate-200 dark:border-slate-800 hover:border-slate-300 text-slate-700 dark:text-slate-300"
                      )}
                    >
                      <div class="font-bold text-xs">{p.name}</div>
                      <div class="text-[9px] text-slate-400 font-mono mt-1">{p.badge}</div>
                    </button>
                  {/each}
                </div>
              </div>

              <!-- Connection String Input -->
              <div class="space-y-1.5">
                <div class="flex items-center justify-between">
                  <label for="wizard-cloud-url" class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                    <Terminal class="size-3.5 text-indigo-500" />
                    <span>Cloud PostgreSQL Connection URL</span>
                  </label>
                  <button
                    type="button"
                    onclick={() => (showCloudPassword = !showCloudPassword)}
                    class="text-[11px] text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 flex items-center gap-1 cursor-pointer font-medium"
                  >
                    {#if showCloudPassword}
                      <EyeOff class="size-3.5" />
                      <span>Hide Password</span>
                    {:else}
                      <Eye class="size-3.5" />
                      <span>Show Password</span>
                    {/if}
                  </button>
                </div>

                <div class="relative">
                  <input
                    id="wizard-cloud-url"
                    type={showCloudPassword ? "text" : "password"}
                    bind:value={cloudConnectionUrl}
                    placeholder="postgresql://user:password@host:5432/dbname?sslmode=require"
                    class="w-full px-3 py-2 pr-24 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs focus:ring-1 focus:ring-indigo-500 focus:outline-hidden"
                  />
                  <button
                    type="button"
                    onclick={testCloudConnection}
                    disabled={isTestingCloud || !cloudConnectionUrl.trim()}
                    class="absolute right-1.5 top-1/2 -translate-y-1/2 px-2.5 py-1 rounded-md bg-indigo-600 hover:bg-indigo-700 text-white text-[11px] font-semibold cursor-pointer flex items-center gap-1 transition-colors disabled:opacity-50"
                  >
                    {#if isTestingCloud}
                      <Activity class="size-3 animate-spin" />
                    {:else}
                      <CheckCircle2 class="size-3" />
                    {/if}
                    <span>Test</span>
                  </button>
                </div>

                <!-- Provider Tip -->
                {#if cloudPreset}
                  {@const cur = CLOUD_PRESETS.find((x) => x.id === cloudPreset)}
                  {#if cur?.tip}
                    <p class="text-[10px] text-slate-400 mt-1">
                      💡 <strong class="text-slate-600 dark:text-slate-300">{cur.name}:</strong> {cur.tip}
                    </p>
                  {/if}
                {/if}
              </div>

              <!-- Connection Test Result Box -->
              {#if cloudTestResult}
                <div
                  class={cn(
                    "p-3 rounded-lg border text-xs font-mono animate-in fade-in-50",
                    cloudTestResult.success
                      ? "border-emerald-200 dark:border-emerald-900 bg-emerald-50/50 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-300"
                      : "border-rose-200 dark:border-rose-900 bg-rose-50/50 dark:bg-rose-950/30 text-rose-700 dark:text-rose-300"
                  )}
                >
                  <div class="font-bold flex items-center gap-1.5">
                    {#if cloudTestResult.success}
                      <CheckCircle2 class="size-4 text-emerald-500" />
                      <span>Cloud Connection Verified ({cloudTestResult.latency_ms}ms)</span>
                    {:else}
                      <AlertTriangle class="size-4 text-rose-500" />
                      <span>Connection Test Failed</span>
                    {/if}
                  </div>
                  <div class="mt-1 text-[11px] opacity-90">{cloudTestResult.message}</div>
                  {#if cloudTestResult.server_version}
                    <div class="mt-0.5 text-[10px] text-slate-500">Database Engine: {cloudTestResult.server_version}</div>
                  {/if}
                </div>
              {/if}
            </div>
          {/if}
        </div>
      {/if}

      <!-- STEP 2: MULTI-TENANCY STRATEGY & BASE PACKAGES -->
      {#if currentStep === 2}
        <div class="space-y-4 animate-in fade-in-50 duration-150">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100">Multi-Tenancy Strategy & Base Packages</h2>
            <p class="text-slate-400 text-xs mt-0.5">Select how data is physically partitioned and which modular packages to import.</p>
          </div>

          <!-- Strategy Selection -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <button
              type="button"
              onclick={() => (storageStrategy = "shared_rls")}
              class={cn(
                "p-3 rounded-xl border text-left flex flex-col justify-between transition-all cursor-pointer relative",
                storageStrategy === "shared_rls"
                  ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                  : "border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-300"
              )}
            >
              <div class="flex items-center justify-between">
                <span class="font-bold text-xs">Shared RLS (Recommended)</span>
                {#if storageStrategy === "shared_rls"}
                  <Check class="size-4 text-indigo-600 dark:text-indigo-400" />
                {/if}
              </div>
              <p class="text-[10px] text-slate-500 dark:text-slate-400 mt-1">
                Shared tables with Row-Level Security. Maximum tenant density and instant schema updates.
              </p>
            </button>

            <button
              type="button"
              onclick={() => (storageStrategy = "partitioned")}
              class={cn(
                "p-3 rounded-xl border text-left flex flex-col justify-between transition-all cursor-pointer relative",
                storageStrategy === "partitioned"
                  ? "border-indigo-600 bg-indigo-50/50 dark:bg-indigo-950/40 text-indigo-950 dark:text-indigo-100 ring-1 ring-indigo-600"
                  : "border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-300"
              )}
            >
              <div class="flex items-center justify-between">
                <span class="font-bold text-xs">Partitioned Tables</span>
                {#if storageStrategy === "partitioned"}
                  <Check class="size-4 text-indigo-600 dark:text-indigo-400" />
                {/if}
              </div>
              <p class="text-[10px] text-slate-500 dark:text-slate-400 mt-1">
                PostgreSQL declarative list partitions per company. High transaction throughput.
              </p>
            </button>
          </div>

          <!-- Base Packages Auto-Import -->
          <div class="p-3.5 rounded-xl border border-indigo-200 dark:border-indigo-900/60 bg-indigo-50/30 dark:bg-indigo-950/20 space-y-2.5">
            <div class="flex items-center justify-between">
              <label class="font-bold text-indigo-950 dark:text-indigo-200 flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={importBasePackages}
                  class="rounded border-indigo-300 text-indigo-600 focus:ring-indigo-500"
                />
                <Package class="size-4 text-indigo-600 dark:text-indigo-400" />
                <span>Automatically Import Base .bos Packages (.bospkg)</span>
              </label>
              <span class="text-[10px] font-mono text-indigo-600 dark:text-indigo-400 font-semibold">ADR 12 & ADR 24</span>
            </div>

            {#if importBasePackages}
              <div class="pl-6 space-y-2 text-[11px] animate-in fade-in-50">
                <p class="text-slate-500 dark:text-slate-400 text-[10px]">
                  These packages will be automatically compiled and deployed into your database:
                </p>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <label class="flex items-start gap-2 p-2 rounded-lg border border-indigo-100 dark:border-indigo-900/40 bg-white/80 dark:bg-slate-900/80 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={selectedPackages.includes("bos.security")}
                      onchange={() => handlePackageToggle("bos.security")}
                      class="mt-0.5 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                    />
                    <div>
                      <span class="font-bold text-slate-800 dark:text-slate-200">bos.security</span>
                      <p class="text-[10px] text-slate-400">User, Role, UserRole, Argon2id, Sessions, RBAC</p>
                    </div>
                  </label>

                  <label class="flex items-start gap-2 p-2 rounded-lg border border-indigo-100 dark:border-indigo-900/40 bg-white/80 dark:bg-slate-900/80 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={selectedPackages.includes("bos.ledger")}
                      onchange={() => handlePackageToggle("bos.ledger")}
                      class="mt-0.5 rounded border-slate-300 text-indigo-600 focus:ring-indigo-500"
                    />
                    <div>
                      <span class="font-bold text-slate-800 dark:text-slate-200">bos.ledger</span>
                      <p class="text-[10px] text-slate-400">General Ledger, Dimensions, SAP Storno</p>
                    </div>
                  </label>
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/if}

      <!-- STEP 3: ORGANIZATION SETUP -->
      {#if currentStep === 3}
        <div class="space-y-4 animate-in fade-in-50 duration-150">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100">Organization Hierarchy</h2>
            <p class="text-slate-400 text-xs mt-0.5">Configure your primary tenant partition and initial legal entity / company.</p>
          </div>

          <div class="space-y-3">
            <div class="p-3 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <span class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                <Users class="size-3.5 text-indigo-500" />
                <span>Primary Tenant Partition</span>
              </span>
              <div class="grid grid-cols-3 gap-2">
                <div>
                  <label for="wiz-tenant-code" class="text-[10px] text-slate-400 block mb-0.5">Tenant Code</label>
                  <input
                    id="wiz-tenant-code"
                    type="text"
                    bind:value={tenantCode}
                    placeholder="DEFAULT"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs uppercase"
                  />
                </div>
                <div class="col-span-2">
                  <label for="wiz-tenant-name" class="text-[10px] text-slate-400 block mb-0.5">Tenant Name</label>
                  <input
                    id="wiz-tenant-name"
                    type="text"
                    bind:value={tenantName}
                    placeholder="Default Enterprise Tenant"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                  />
                </div>
              </div>
            </div>

            <div class="p-3 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <span class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                <Building2 class="size-3.5 text-indigo-500" />
                <span>Initial Company / Legal Entity</span>
              </span>
              <div class="grid grid-cols-3 gap-2">
                <div>
                  <label for="wiz-company-code" class="text-[10px] text-slate-400 block mb-0.5">Company Code</label>
                  <input
                    id="wiz-company-code"
                    type="text"
                    bind:value={companyCode}
                    placeholder="CRONUS"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 font-mono text-xs uppercase"
                  />
                </div>
                <div class="col-span-2">
                  <label for="wiz-company-name" class="text-[10px] text-slate-400 block mb-0.5">Company Name</label>
                  <input
                    id="wiz-company-name"
                    type="text"
                    bind:value={companyName}
                    placeholder="Cronus International Ltd"
                    class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- STEP 4: SUPERUSER CREDENTIALS -->
      {#if currentStep === 4}
        <div class="space-y-4 animate-in fade-in-50 duration-150">
          <div>
            <h2 class="text-sm font-bold text-slate-900 dark:text-slate-100">Superuser Administrator Account</h2>
            <p class="text-slate-400 text-xs mt-0.5">
              This account receives permanent `SUPER` and `Admin` permissions to manage the entire platform.
            </p>
          </div>

          <div class="space-y-3">
            <div class="grid grid-cols-2 gap-2.5">
              <div class="space-y-1">
                <label for="wiz-admin-user" class="font-semibold text-slate-700 dark:text-slate-300">Username</label>
                <div class="relative">
                  <User class="size-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
                  <input
                    id="wiz-admin-user"
                    type="text"
                    bind:value={adminUsername}
                    placeholder="admin"
                    class="w-full pl-8 pr-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                  />
                </div>
              </div>

              <div class="space-y-1">
                <label for="wiz-admin-name" class="font-semibold text-slate-700 dark:text-slate-300">Display Name</label>
                <input
                  id="wiz-admin-name"
                  type="text"
                  bind:value={adminDisplayName}
                  placeholder="System Administrator"
                  class="w-full px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                />
              </div>
            </div>

            <div class="space-y-1">
              <label for="wiz-admin-email" class="font-semibold text-slate-700 dark:text-slate-300">Administrator Email</label>
              <div class="relative">
                <Mail class="size-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
                <input
                  id="wiz-admin-email"
                  type="email"
                  bind:value={adminEmail}
                  placeholder="admin@cronus.corp"
                  class="w-full pl-8 pr-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2.5">
              <div class="space-y-1">
                <label for="wiz-admin-pass" class="font-semibold text-slate-700 dark:text-slate-300">Password</label>
                <div class="relative">
                  <Lock class="size-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
                  <input
                    id="wiz-admin-pass"
                    type="password"
                    bind:value={adminPassword}
                    placeholder="At least 8 characters"
                    class="w-full pl-8 pr-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                  />
                </div>
              </div>

              <div class="space-y-1">
                <label for="wiz-admin-confirm" class="font-semibold text-slate-700 dark:text-slate-300">Confirm Password</label>
                <div class="relative">
                  <Lock class="size-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
                  <input
                    id="wiz-admin-confirm"
                    type="password"
                    bind:value={adminPasswordConfirm}
                    placeholder="Repeat password"
                    class="w-full pl-8 pr-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 text-xs"
                  />
                </div>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- STEP 5: PROVISIONING PIPELINE PROGRESS -->
      {#if currentStep === 5}
        <div class="py-8 space-y-6 text-center animate-in fade-in-50 duration-200">
          <div class="size-16 rounded-2xl bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 flex items-center justify-center mx-auto ring-1 ring-indigo-500/20">
            {#if provisioningStage < 5}
              <Activity class="size-8 animate-spin" />
            {:else}
              <CheckCircle2 class="size-8 text-emerald-500" />
            {/if}
          </div>

          <div class="space-y-1">
            <h2 class="text-base font-bold text-slate-900 dark:text-slate-100">
              {provisioningStage < 5 ? "Provisioning BOS Platform..." : "Setup Complete!"}
            </h2>
            <p class="text-xs text-slate-400 max-w-sm mx-auto">
              {provisioningStage < 5
                ? "Applying schema definitions, installing base packages, and creating superuser."
                : "Your enterprise platform has been successfully initialized. Launching workspace..."}
            </p>
          </div>

          <!-- Progress Checklist -->
          <div class="max-w-xs mx-auto text-left space-y-2 font-mono text-[11px]">
            <div class="flex items-center gap-2">
              {#if provisioningStage >= 1}
                <Check class="size-4 text-emerald-500" />
              {:else}
                <span class="size-4 rounded-full border border-slate-300 dark:border-slate-700 shrink-0"></span>
              {/if}
              <span class={cn(provisioningStage >= 1 ? "text-slate-800 dark:text-slate-200" : "text-slate-400")}>
                {providerType === "cloud" ? "Connect to Cloud PostgreSQL Provider" : `Initialize Database '${dbName}'`}
              </span>
            </div>

            <div class="flex items-center gap-2">
              {#if provisioningStage >= 2}
                <Check class="size-4 text-emerald-500" />
              {:else}
                <span class="size-4 rounded-full border border-slate-300 dark:border-slate-700 shrink-0"></span>
              {/if}
              <span class={cn(provisioningStage >= 2 ? "text-slate-800 dark:text-slate-200" : "text-slate-400")}>
                Kernel DDL & Strategy ({storageStrategy})
              </span>
            </div>

            <div class="flex items-center gap-2">
              {#if provisioningStage >= 3}
                <Check class="size-4 text-emerald-500" />
              {:else}
                <span class="size-4 rounded-full border border-slate-300 dark:border-slate-700 shrink-0"></span>
              {/if}
              <span class={cn(provisioningStage >= 3 ? "text-slate-800 dark:text-slate-200" : "text-slate-400")}>
                Deploy Base Packages (bos.security)
              </span>
            </div>

            <div class="flex items-center gap-2">
              {#if provisioningStage >= 4}
                <Check class="size-4 text-emerald-500" />
              {:else}
                <span class="size-4 rounded-full border border-slate-300 dark:border-slate-700 shrink-0"></span>
              {/if}
              <span class={cn(provisioningStage >= 4 ? "text-slate-800 dark:text-slate-200" : "text-slate-400")}>
                Seed Superuser '{adminUsername}'
              </span>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Wizard Navigation Footer -->
    {#if currentStep < 5}
      <div class="px-6 py-4 border-t border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-800/40 flex items-center justify-between">
        {#if currentStep > 1}
          <button
            type="button"
            onclick={prevStep}
            class="px-3.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300 text-xs font-semibold flex items-center gap-1.5 cursor-pointer"
          >
            <ArrowLeft class="size-3.5" />
            <span>Back</span>
          </button>
        {:else}
          <div></div>
        {/if}

        {#if currentStep < 4}
          <button
            type="button"
            onclick={nextStep}
            class="px-4 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-semibold flex items-center gap-1.5 cursor-pointer shadow-xs transition-colors"
          >
            <span>Next</span>
            <ArrowRight class="size-3.5" />
          </button>
        {:else}
          <button
            type="button"
            onclick={executeProvisioning}
            disabled={isProvisioning}
            class="px-5 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-bold flex items-center gap-2 cursor-pointer shadow-xs transition-colors"
          >
            <Sparkles class="size-3.5" />
            <span>Provision & Launch Platform</span>
          </button>
        {/if}
      </div>
    {/if}

    {#if onGoToSignIn && currentStep < 5}
      <div class="mt-3 text-center text-xs">
        <button
          type="button"
          onclick={onGoToSignIn}
          class="text-slate-500 hover:text-indigo-600 dark:hover:text-indigo-400 hover:underline cursor-pointer transition-colors"
        >
          Already have a database or want to sign in directly? Return to Sign In
        </button>
      </div>
    {/if}
  </div>
</div>
