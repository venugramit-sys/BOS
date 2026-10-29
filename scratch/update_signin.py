with open('ui/src/lib/components/shell/SignIn.svelte', 'r', encoding='utf-8') as f:
    c = f.read()

script_target = """  import { notificationStore, notify } from "$lib/components/notifications/index.js";
  import { ShieldAlert } from "lucide-svelte";

  let nodeInfo = $state(null);
  let initialUsername = $state("");"""

script_repl = """  import { notificationStore, notify } from "$lib/components/notifications/index.js";
  import { ShieldAlert, Layers, Lock, User, Eye, EyeOff, ArrowRight, Loader2, RotateCcw } from "lucide-svelte";

  let nodeInfo = $state(null);
  let initialUsername = $state("");
  let pageHostError = $state(null);
  let fallbackUsername = $state("");
  let fallbackPassword = $state("");
  let fallbackRememberMe = $state(true);
  let fallbackShowPassword = $state(false);
  let fallbackBusy = $state(false);
  let fallbackError = $state("");
  let pageHostKey = $state(0);"""

onmount_target = """        const saved = localStorage.getItem("bos_remember_username");
        if (saved) initialUsername = saved;"""

onmount_repl = """        const saved = localStorage.getItem("bos_remember_username");
        if (saved) {
          initialUsername = saved;
          fallbackUsername = saved;
        }"""

fallback_fn = """  async function handleFallbackSubmit(e) {
    e?.preventDefault();
    if (!fallbackUsername.trim()) {
      fallbackError = "Please enter your User ID.";
      return;
    }
    if (!fallbackPassword) {
      fallbackError = "Please enter your password.";
      return;
    }
    fallbackError = "";
    fallbackBusy = true;
    try {
      const resp = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          username: fallbackUsername.trim(),
          password: fallbackPassword,
        }),
        signal: AbortSignal.timeout(10000),
      });
      const data = await resp.json().catch(() => ({}));
      if (!resp.ok) {
        fallbackError = data.message || data.error || "Invalid User ID or Password.";
        return;
      }
      if (fallbackRememberMe) {
        try { localStorage.setItem("bos_remember_username", fallbackUsername.trim()); } catch {}
      } else {
        try { localStorage.removeItem("bos_remember_username"); } catch {}
      }
      shell.signIn(data);
      notify.success(`Welcome to BOS, ${data.user?.name || fallbackUsername}!`, { title: "Signed In" });
      if (nodeInfo?.activeDatabase) shell.activeDatabase = nodeInfo.activeDatabase;
      if (nodeInfo?.defaultCompany) shell.activeCompany = nodeInfo.defaultCompany;
      await shell.refreshSystem();
    } catch (err) {
      fallbackError = err instanceof Error ? err.message : "Connection failed. Please ensure the server node is running.";
    } finally {
      fallbackBusy = false;
    }
  }

  function retryPageHost() {
    pageHostError = null;
    pageHostKey += 1;
  }
"""

template_target = """    <!-- The 100% Real BOS LoginPage Meta-Object (Compiled from packages/bos.auth) -->
    <div class="bg-white dark:bg-slate-900 rounded-2xl shadow-xl border border-slate-200/80 dark:border-slate-800/80 p-6 sm:p-8">
      <PageHost
        page="LoginPage"
        params={{
          mode: "view",
          variables: {
            username: initialUsername || "admin",
            rememberMe: true
          }
        }}
        onActionComplete={handleActionComplete}
        onActionError={handleActionError}
      />
    </div>"""

template_repl = """    <div class="bg-white dark:bg-slate-900 rounded-2xl shadow-xl border border-slate-200/80 dark:border-slate-800/80 p-6 sm:p-8">
      {#if pageHostError}
        <!-- Seamless Direct Login Form Fallback if PageHost is loading or unavailable -->
        <div class="space-y-4">
          <div class="space-y-1">
            <h1 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100 flex items-center gap-2">
              <Layers class="size-5 text-indigo-600 dark:text-indigo-400" />
              <span>Sign In</span>
            </h1>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              Enter your enterprise credentials to access your workspace.
            </p>
          </div>

          <div class="p-2.5 rounded-lg bg-slate-100 dark:bg-slate-800/60 border border-slate-200 dark:border-slate-700/60 flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400">
            <span class="truncate max-w-[240px]">LoginPage host warming: direct auth active</span>
            <button
              type="button"
              onclick={retryPageHost}
              class="inline-flex items-center gap-1 text-indigo-600 dark:text-indigo-400 hover:underline font-medium cursor-pointer"
            >
              <RotateCcw class="size-3" />
              <span>Retry</span>
            </button>
          </div>

          {#if fallbackError}
            <div class="p-3 rounded-lg bg-red-50 dark:bg-red-950/40 border border-red-200 dark:border-red-900/40 text-red-600 dark:text-red-400 text-xs">
              {fallbackError}
            </div>
          {/if}

          <form onsubmit={handleFallbackSubmit} class="space-y-3.5 pt-1">
            <div class="space-y-1">
              <label for="fb-user" class="text-xs font-medium text-slate-700 dark:text-slate-300">User ID</label>
              <div class="relative">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
                  <User class="size-4" />
                </div>
                <input
                  id="fb-user"
                  type="text"
                  bind:value={fallbackUsername}
                  disabled={fallbackBusy}
                  placeholder="admin"
                  required
                  class="w-full h-10 pl-9 pr-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-hidden"
                />
              </div>
            </div>

            <div class="space-y-1">
              <label for="fb-pwd" class="text-xs font-medium text-slate-700 dark:text-slate-300">Password</label>
              <div class="relative">
                <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
                  <Lock class="size-4" />
                </div>
                <input
                  id="fb-pwd"
                  type={fallbackShowPassword ? "text" : "password"}
                  bind:value={fallbackPassword}
                  disabled={fallbackBusy}
                  placeholder="••••••••"
                  required
                  class="w-full h-10 pl-9 pr-10 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-hidden"
                />
                <button
                  type="button"
                  onclick={() => (fallbackShowPassword = !fallbackShowPassword)}
                  class="absolute inset-y-0 right-0 pr-3 flex items-center text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer"
                  tabindex="-1"
                >
                  {#if fallbackShowPassword}
                    <EyeOff class="size-4" />
                  {:else}
                    <Eye class="size-4" />
                  {/if}
                </button>
              </div>
            </div>

            <div class="flex items-center justify-between pt-0.5">
              <label class="flex items-center gap-2 cursor-pointer text-xs text-slate-600 dark:text-slate-400 select-none">
                <input
                  type="checkbox"
                  bind:checked={fallbackRememberMe}
                  class="size-3.5 rounded border-slate-300 dark:border-slate-700 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-950"
                />
                <span>Remember User ID</span>
              </label>
            </div>

            <button
              type="submit"
              disabled={fallbackBusy}
              class="w-full h-10 mt-2 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white font-medium text-xs flex items-center justify-center gap-2 shadow-xs cursor-pointer transition-all disabled:opacity-50"
            >
              {#if fallbackBusy}
                <Loader2 class="size-4 animate-spin" />
                <span>Verifying Credentials...</span>
              {:else}
                <span>Sign In</span>
                <ArrowRight class="size-4" />
              {/if}
            </button>
          </form>
        </div>
      {:else}
        <!-- The 100% Real BOS LoginPage Meta-Object (Compiled from packages/bos.auth) -->
        {#key pageHostKey}
          <PageHost
            page="LoginPage"
            params={{
              mode: "view",
              variables: {
                username: initialUsername || "admin",
                rememberMe: true
              }
            }}
            onHostError={(err) => (pageHostError = err)}
            onActionComplete={handleActionComplete}
            onActionError={handleActionError}
          />
        {/key}
      {/if}
    </div>"""

is_crlf = '\r\n' in c
if is_crlf:
    script_target = script_target.replace('\n', '\r\n')
    script_repl = script_repl.replace('\n', '\r\n')
    onmount_target = onmount_target.replace('\n', '\r\n')
    onmount_repl = onmount_repl.replace('\n', '\r\n')
    fallback_fn = fallback_fn.replace('\n', '\r\n')
    template_target = template_target.replace('\n', '\r\n')
    template_repl = template_repl.replace('\n', '\r\n')

assert script_target in c, "script_target not found"
c = c.replace(script_target, script_repl, 1)

assert onmount_target in c, "onmount_target not found"
c = c.replace(onmount_target, onmount_repl, 1)

# insert fallback_fn before </script>
script_close = "</script>"
assert script_close in c, "script_close not found"
c = c.replace(script_close, fallback_fn + "\n" + script_close, 1)

assert template_target in c, "template_target not found"
c = c.replace(template_target, template_repl, 1)

with open('ui/src/lib/components/shell/SignIn.svelte', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated SignIn.svelte successfully")
