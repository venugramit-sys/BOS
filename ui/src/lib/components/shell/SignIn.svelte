<script>
  import { onMount } from "svelte";
  import { shell } from "./shellStore.svelte.js";
  import { notificationStore } from "$lib/services/notificationStore.svelte.js";
  import {
    Layers,
    User,
    Lock,
    Eye,
    EyeOff,
    ArrowRight,
    AlertCircle,
    CheckCircle2,
    Loader2,
    ShieldAlert,
    Wrench,
  } from "lucide-svelte";

  // Form State - Only User ID & Password
  let username = $state("");
  let password = $state("");
  let showPassword = $state(false);
  let rememberMe = $state(true);

  // Live Login Branding & Configuration from bos.auth
  let authConfig = $state({
    applicationTitle: "BOS Enterprise Platform",
    companyLogoUrl: "",
    welcomeMessage: "Sign in to access your enterprise workspace.",
    primaryColor: "#4f46e5",
    allowRememberMe: true,
    allowEmergencyFallback: true,
    ssoEnabled: false,
    ssoProviderName: "Corporate SSO",
    ssoLoginUrl: "",
    termsOfServiceUrl: "",
    supportEmail: "admin@bos.local",
    customDisclaimer: "Authorized personnel only. All access attempts are monitored and audited."
  });

  // Status & Feedback State
  let busy = $state(false);
  let error = $state("");
  let nodeInfo = $state(null);

  onMount(async () => {
    // 1. Fetch live Login Branding & Configuration from bos.auth
    try {
      const cfgRes = await fetch("/api/auth/config");
      if (cfgRes.ok) {
        const cfgData = await cfgRes.json();
        authConfig = { ...authConfig, ...cfgData };
        if (typeof document !== "undefined" && authConfig.applicationTitle) {
          document.title = `${authConfig.applicationTitle} · Sign In`;
        }
      }
    } catch {
      // Use defaults if offline
    }

    // 2. Fetch connected Server Node metadata (non-blocking)
    try {
      const nodeRes = await fetch("/api/node/info");
      if (nodeRes.ok) {
        nodeInfo = await nodeRes.json();
      }
    } catch {
      // Offline fallback
    }

    // 3. Load remembered username if previously stored
    if (typeof window !== "undefined") {
      try {
        const savedUser = localStorage.getItem("bos_remember_username");
        if (savedUser) {
          username = savedUser;
        }
      } catch {
        // Ignored
      }
    }
  });

  async function handleLogin(event) {
    event?.preventDefault();
    if (!username.trim()) {
      error = "Please enter your User ID.";
      return;
    }
    if (!password) {
      error = "Please enter your password.";
      return;
    }

    error = "";
    busy = true;

    try {
      const response = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          username: username.trim(),
          password
        }),
        signal: AbortSignal.timeout(10000)
      });

      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        error = data.message || data.error || "Invalid User ID or Password.";
        return;
      }

      // Save or clear remembered username
      if (typeof window !== "undefined") {
        try {
          if (rememberMe) {
            localStorage.setItem("bos_remember_username", username.trim());
          } else {
            localStorage.removeItem("bos_remember_username");
          }
        } catch {
          // Ignored
        }
      }

      // Establish session directly into reactive Shell store
      shell.signIn(data);

      if (data.emergency_fallback || data.maintenance_mode) {
        notificationStore.dispatch({
          severity: "warning",
          target: "toast",
          title: data.maintenance_mode ? "Maintenance Mode Active" : "Emergency Super Admin Fallback",
          message:
            data.warning ||
            "Logged in with Emergency Super Admin permissions to repair broken .bos packages.",
          duration_ms: 10000,
        });
      }

      if (nodeInfo?.activeDatabase) {
        shell.activeDatabase = nodeInfo.activeDatabase;
      }
      if (nodeInfo?.defaultCompany) {
        shell.activeCompany = nodeInfo.defaultCompany;
      }

      // Refresh catalog and permissions in background
      await shell.refreshSystem();
    } catch (err) {
      error = err instanceof Error ? err.message : "Connection failed. Please ensure the server node is running.";
    } finally {
      busy = false;
    }
  }

  async function handleEmergencySignIn() {
    error = "";
    busy = true;

    try {
      const response = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          username: "emergency_admin",
          password: "any",
        }),
        signal: AbortSignal.timeout(10000),
      });

      const data = await response.json().catch(() => ({}));

      if (!response.ok) {
        error = data.message || data.error || "Emergency login failed.";
        return;
      }

      shell.signIn(data);

      notificationStore.dispatch({
        severity: "warning",
        target: "toast",
        title: "Maintenance Mode Active",
        message: data.warning || "Logged in with Emergency Super Admin permissions.",
        duration_ms: 10000,
      });

      if (nodeInfo?.activeDatabase) {
        shell.activeDatabase = nodeInfo.activeDatabase;
      }
      if (nodeInfo?.defaultCompany) {
        shell.activeCompany = nodeInfo.defaultCompany;
      }

      await shell.refreshSystem();
    } catch (err) {
      error = err instanceof Error ? err.message : "Emergency login failed.";
    } finally {
      busy = false;
    }
  }
</script>

<div class="min-h-screen bg-slate-50 dark:bg-slate-950 flex flex-col justify-center items-center p-4 select-none transition-colors">
  <div class="w-full max-w-[380px]">
    <!-- Brand & Node Header -->
    <div class="text-center mb-6">
      {#if authConfig.companyLogoUrl}
        <img
          src={authConfig.companyLogoUrl}
          alt={authConfig.applicationTitle}
          class="h-12 w-auto max-w-[220px] object-contain mx-auto mb-3"
        />
      {:else}
        <div
          class="size-12 rounded-2xl text-white flex items-center justify-center mx-auto shadow-md mb-3 transition-all"
          style="background-color: {authConfig.primaryColor || '#4f46e5'};"
        >
          <Layers class="size-6 stroke-[2]" />
        </div>
      {/if}

      <h1 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
        {authConfig.applicationTitle || "Sign In to BOS"}
      </h1>

      {#if nodeInfo}
        <div class="inline-flex items-center gap-1.5 px-2.5 py-1 mt-2 rounded-full bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-[11px] text-slate-600 dark:text-slate-300 font-medium">
          <span class="size-2 rounded-full bg-emerald-500 animate-pulse"></span>
          <span>{nodeInfo.nodeName || nodeInfo.nodeId}</span>
          <span class="text-slate-400 font-normal">({nodeInfo.role})</span>
        </div>
      {:else}
        <p class="text-xs text-slate-400 mt-1">
          {authConfig.welcomeMessage || "Enterprise Business Operating System"}
        </p>
      {/if}
    </div>

    <!-- Maintenance Mode Callout -->
    {#if nodeInfo?.maintenanceMode}
      <div class="mb-4 p-3.5 rounded-2xl bg-amber-500/10 dark:bg-amber-500/15 border border-amber-500/30 text-amber-900 dark:text-amber-200 text-xs space-y-2.5 animate-in fade-in-50">
        <div class="flex items-start gap-2.5">
          <Wrench class="size-4 shrink-0 text-amber-600 dark:text-amber-400 mt-0.5" />
          <div class="leading-relaxed">
            <span class="font-bold">Maintenance Mode Active:</span>
            Authentication bypass is enabled on this node. You may sign in with emergency Super Admin rights to repair .bos packages and scripts.
          </div>
        </div>
        <button
          type="button"
          onclick={handleEmergencySignIn}
          disabled={busy}
          class="w-full py-2 px-3 rounded-xl bg-amber-600 hover:bg-amber-700 active:bg-amber-800 text-white font-medium text-xs flex items-center justify-center gap-2 cursor-pointer shadow-xs shadow-amber-600/30 transition-all disabled:opacity-50"
        >
          <ShieldAlert class="size-4" />
          <span>Sign In as Emergency Super Admin</span>
        </button>
      </div>
    {/if}

    <!-- Login Card -->
    <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-6 shadow-xl space-y-4 text-xs">
      <!-- Error Message Banner -->
      {#if error}
        <div class="p-3 rounded-xl bg-rose-50 dark:bg-rose-950/40 border border-rose-200 dark:border-rose-900/60 text-rose-800 dark:text-rose-200 text-xs flex items-start gap-2.5 animate-in fade-in-50">
          <AlertCircle class="size-4 shrink-0 text-rose-600 dark:text-rose-400 mt-0.5" />
          <div class="flex-1 leading-relaxed">
            <span class="font-semibold">Authentication Failed:</span> {error}
          </div>
        </div>
      {/if}

      <!-- Plain Login Form -->
      <form onsubmit={handleLogin} class="space-y-4">
        <!-- User ID -->
        <div class="space-y-1.5">
          <label for="bos-username" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">
            User ID
          </label>
          <div class="relative">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
              <User class="size-4" />
            </div>
            <input
              id="bos-username"
              type="text"
              bind:value={username}
              disabled={busy}
              placeholder="e.g. admin or john.doe"
              autocomplete="username"
              required
              class="w-full h-10 pl-9 pr-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 text-xs focus:ring-2 focus:ring-indigo-500 focus:border-transparent focus:bg-white dark:focus:bg-slate-950 focus:outline-hidden transition-all disabled:opacity-50"
            />
          </div>
        </div>

        <!-- Password -->
        <div class="space-y-1.5">
          <label for="bos-password" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">
            Password
          </label>
          <div class="relative">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
              <Lock class="size-4" />
            </div>
            <input
              id="bos-password"
              type={showPassword ? "text" : "password"}
              bind:value={password}
              disabled={busy}
              placeholder="••••••••"
              autocomplete="current-password"
              required
              class="w-full h-10 pl-9 pr-10 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 text-xs focus:ring-2 focus:ring-indigo-500 focus:border-transparent focus:bg-white dark:focus:bg-slate-950 focus:outline-hidden transition-all disabled:opacity-50"
            />
            <button
              type="button"
              onclick={() => (showPassword = !showPassword)}
              class="absolute inset-y-0 right-0 pr-3 flex items-center text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer"
              tabindex="-1"
              title={showPassword ? "Hide password" : "Show password"}
            >
              {#if showPassword}
                <EyeOff class="size-4" />
              {:else}
                <Eye class="size-4" />
              {/if}
            </button>
          </div>
        </div>

        <!-- Remember Me Checkbox -->
        {#if authConfig.allowRememberMe}
          <div class="flex items-center justify-between pt-0.5">
            <label class="flex items-center gap-2 cursor-pointer text-xs text-slate-600 dark:text-slate-400 select-none">
              <input
                type="checkbox"
                bind:checked={rememberMe}
                class="size-3.5 rounded border-slate-300 dark:border-slate-700 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-950"
              />
              <span>Remember User ID</span>
            </label>
          </div>
        {/if}

        <!-- Sign In Submit Button -->
        <button
          type="submit"
          disabled={busy}
          style="background-color: {authConfig.primaryColor || '#4f46e5'};"
          class="w-full h-10 mt-2 rounded-lg hover:opacity-95 active:opacity-90 text-white font-medium text-xs flex items-center justify-center gap-2 shadow-sm cursor-pointer transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {#if busy}
            <Loader2 class="size-4 animate-spin" />
            <span>Verifying Credentials...</span>
          {:else}
            <span>Sign In</span>
            <ArrowRight class="size-4" />
          {/if}
        </button>

        <!-- Optional Enterprise Single Sign-On (SSO / SAML) -->
        {#if authConfig.ssoEnabled}
          <div class="relative my-4">
            <div class="absolute inset-0 flex items-center"><div class="w-full border-t border-slate-200 dark:border-slate-800"></div></div>
            <div class="relative flex justify-center text-[10px] uppercase"><span class="bg-white dark:bg-slate-900 px-2 text-slate-400">or</span></div>
          </div>
          <button
            type="button"
            onclick={() => {
              if (authConfig.ssoLoginUrl) {
                window.location.href = authConfig.ssoLoginUrl;
              } else {
                notificationStore.dispatch({
                  severity: "info",
                  target: "toast",
                  title: "SSO Redirect",
                  message: `Redirecting to ${authConfig.ssoProviderName || 'Enterprise Identity Provider'}...`
                });
              }
            }}
            class="w-full h-10 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 hover:bg-slate-100 dark:bg-slate-800 dark:hover:bg-slate-700 text-slate-700 dark:text-slate-200 font-medium text-xs flex items-center justify-center gap-2 transition-colors cursor-pointer"
          >
            <span>Sign In with {authConfig.ssoProviderName || "Enterprise SSO"}</span>
          </button>
        {/if}
      </form>
    </div>

    <!-- Security, Compliance & Architecture Footer -->
    <div class="text-center mt-6 text-[11px] text-slate-400 dark:text-slate-500 space-y-1.5">
      {#if authConfig.customDisclaimer}
        <p class="text-[10px] text-slate-400 dark:text-slate-500 max-w-xs mx-auto leading-relaxed">
          {authConfig.customDisclaimer}
        </p>
      {/if}
      {#if authConfig.supportEmail}
        <p class="text-[10px] text-slate-400">
          Need support? <a href="mailto:{authConfig.supportEmail}" class="underline hover:text-slate-600 dark:hover:text-slate-300">{authConfig.supportEmail}</a>
        </p>
      {/if}
      <p class="text-[10px] font-mono text-slate-500 dark:text-slate-600">
        3-Tier Architecture · Controlled by bos.auth Codeunit
      </p>
    </div>
  </div>
</div>
