<script>
  /**
   * CodeunitRunnerPanel.svelte (Svelte 5 Runes)
   * Interactive QuickJS Codeunit & Trigger Debugger / Test Runner
   * Features:
   * - Auto-discovers procedures and typed parameters from the active BOS-Script buffer
   * - Interactive Parameter Form OR Raw JSON Payload Editor
   * - Real-time `rec` Before vs After Diff Inspector
   * - Execution Logs, UI Toast/Message Inspector, and Clickable Error Stack Traces
   */
  import { Play, Terminal, Sparkles, AlertCircle, CheckCircle2, FileDiff, ListChecks } from "lucide-svelte";
  import { Button, Input, Textarea, Banner, Spinner, Badge } from "$lib/bos-ui/index.js";

  let {
    codeunitName = "",
    sourceCode = "",
    isOpen = $bindable(false),
    onJumpToLine = null,
  } = $props();

  let methodName = $state("execute");
  let paramsJson = $state('{\n  "id": "' + (typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : '') + '"\n}');
  let isExecuting = $state(false);
  let executionResult = $state(null);
  let executionError = $state(null);
  let beforeSnapshot = $state(null);
  let durationMs = $state(0);
  let activeResultTab = $state("diff"); // 'diff' | 'json' | 'logs'

  // Auto-discover procedures and their typed parameters from sourceCode
  let discoveredProcedures = $derived.by(() => {
    const procs = [];
    if (!sourceCode) return procs;
    const regex = /(?:async\s+)?(?:procedure|function)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)/g;
    let m;
    while ((m = regex.exec(sourceCode)) !== null) {
      const name = m[1];
      const rawParams = m[2].trim();
      const params = [];
      if (rawParams) {
        for (const part of rawParams.split(",")) {
          const pClean = part.trim();
          if (!pClean) continue;
          const [pNameRaw, pTypeRaw] = pClean.split(":").map((s) => s.trim());
          const pName = (pNameRaw || "").replace(/\?$/, "");
          const pType = pTypeRaw || "any";
          if (pName && pName !== "ctx") {
            params.push({ name: pName, type: pType });
          }
        }
      }
      procs.push({ name, params });
    }
    return procs;
  });

  // Auto-select first discovered procedure when opening if current methodName isn't found
  $effect(() => {
    if (discoveredProcedures.length > 0 && !discoveredProcedures.some((p) => p.name === methodName)) {
      selectProcedure(discoveredProcedures[0]);
    }
  });

  function selectProcedure(proc) {
    if (!proc) return;
    methodName = proc.name;
    const templateObj = {};
    for (const p of proc.params) {
      const t = p.type.toLowerCase();
      if (t.includes("number") || t.includes("int") || t.includes("decimal")) {
        templateObj[p.name] = 100;
      } else if (t.includes("bool")) {
        templateObj[p.name] = true;
      } else if (t.includes("uuid")) {
        templateObj[p.name] = (typeof crypto !== 'undefined' && crypto.randomUUID ? crypto.randomUUID() : '');
      } else if (t.includes("date")) {
        templateObj[p.name] = new Date().toISOString().slice(0, 10);
      } else {
        templateObj[p.name] = `sample_${p.name}`;
      }
    }
    if (Object.keys(templateObj).length > 0) {
      paramsJson = JSON.stringify(templateObj, null, 2);
    }
  }

  let fieldDiffs = $derived.by(() => {
    if (!executionResult || typeof executionResult !== "object") return [];
    const afterObj = executionResult.mutated_data ?? executionResult;
    if (!afterObj || typeof afterObj !== "object") return [];
    const beforeObj = beforeSnapshot && typeof beforeSnapshot === "object" ? beforeSnapshot : {};
    const allKeys = Array.from(new Set([...Object.keys(beforeObj), ...Object.keys(afterObj)])).filter(
      (k) => k !== "_system_messages"
    );
    return allKeys.map((key) => {
      const beforeVal = beforeObj[key];
      const afterVal = afterObj[key];
      const changed = JSON.stringify(beforeVal) !== JSON.stringify(afterVal);
      const status = !(key in beforeObj) ? "added" : !(key in afterObj) ? "removed" : changed ? "modified" : "unchanged";
      return { key, beforeVal, afterVal, status };
    });
  });

  let errorLineNumber = $derived.by(() => {
    if (!executionError) return null;
    const m = String(executionError).match(/:(\d+)(?::\d+)?\]?$/) || String(executionError).match(/line\s+(\d+)/i);
    return m ? parseInt(m[1], 10) : null;
  });

  async function handleRunProcedure() {
    if (!codeunitName) {
      executionError = "No codeunit specified.";
      return;
    }

    let parsedParams = {};
    try {
      if (paramsJson.trim()) {
        parsedParams = JSON.parse(paramsJson);
      }
    } catch (e) {
      executionError = `Invalid JSON parameters: ${e.message}`;
      executionResult = null;
      return;
    }

    beforeSnapshot = JSON.parse(JSON.stringify(parsedParams));
    isExecuting = true;
    executionError = null;
    executionResult = null;
    const startTime = performance.now();

    try {
      const res = await fetch(`/api/codeunits/${encodeURIComponent(codeunitName)}/run`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          method: methodName,
          params: parsedParams,
        }),
      });

      durationMs = Math.round(performance.now() - startTime);
      const data = await res.json();

      if (res.ok) {
        executionResult = data;
      } else {
        executionError = data.error || data.message || `Execution failed with status ${res.status}`;
      }
    } catch (err) {
      durationMs = Math.round(performance.now() - startTime);
      executionError = err.message || "Failed to reach QuickJS execution runtime";
    } finally {
      isExecuting = false;
    }
  }
</script>

{#if isOpen}
  <div class="border-t border-slate-200 dark:border-slate-800 bg-slate-50/95 dark:bg-slate-900/95 p-4 transition-all space-y-3">
    <!-- Header / Title -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <Terminal class="size-4 text-indigo-500" />
        <span class="text-xs font-semibold text-slate-800 dark:text-slate-200 uppercase tracking-wider">
          Interactive QuickJS Debugger & Runner: <span class="font-mono text-indigo-600 dark:text-indigo-400">{codeunitName || "Unsaved"}</span>
        </span>
        {#if discoveredProcedures.length > 0}
          <Badge tone="info">{discoveredProcedures.length} procedures</Badge>
        {/if}
      </div>

      <Button variant="ghost" size="xs" onclick={() => (isOpen = false)}>Close</Button>
    </div>

    <!-- Discovered Procedures Quick-Selector Pills -->
    {#if discoveredProcedures.length > 0}
      <div class="flex items-center gap-1.5 flex-wrap">
        <span class="text-[11px] text-slate-500 dark:text-slate-400 mr-1">Procedures:</span>
        {#each discoveredProcedures as proc}
          <button
            type="button"
            onclick={() => selectProcedure(proc)}
            class="px-2 py-0.5 rounded text-xs font-mono border transition-colors {methodName === proc.name
              ? 'bg-indigo-600 text-white border-indigo-600'
              : 'bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-300 border-slate-200 dark:border-slate-700 hover:border-indigo-400'}"
          >
            {proc.name}({proc.params.map((p) => `${p.name}: ${p.type}`).join(", ")})
          </button>
        {/each}
      </div>
    {/if}

    <!-- Controls Row -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
      <Input label="Procedure Name" type="code" bind:value={methodName} placeholder="execute" />
      <div class="md:col-span-2 space-y-1">
        <div class="flex gap-2 items-end">
          <Textarea class="flex-1" label="Input Record / JSON Parameters" mono rows={2} resize="none" bind:value={paramsJson} />
          <Button icon={Play} loading={isExecuting} disabled={!codeunitName} onclick={handleRunProcedure}>
            {isExecuting ? "Running..." : "Run in QuickJS"}
          </Button>
        </div>
      </div>
    </div>

    <!-- Execution Results & Diff Inspector -->
    {#if isExecuting}
      <Banner tone="info" title="Running">
        <span class="inline-flex items-center gap-2"><Spinner size="xs" /> Executing in the QuickJS sandbox...</span>
      </Banner>
    {:else if executionError}
      <Banner tone="danger" title={`Runtime Exception · ${durationMs}ms`}>
        <div class="flex items-center justify-between gap-2">
          <span class="font-mono text-xs break-all">{executionError}</span>
          {#if errorLineNumber && onJumpToLine}
            <Button size="xs" variant="secondary" onclick={() => onJumpToLine(errorLineNumber)}>
              Jump to Line {errorLineNumber}
            </Button>
          {/if}
        </div>
      </Banner>
    {:else if executionResult !== null}
      <div class="rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 overflow-hidden">
        <div class="px-3 py-1.5 bg-slate-100 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-2 text-xs font-medium text-emerald-600 dark:text-emerald-400">
            <CheckCircle2 class="size-3.5" />
            <span>Completed in {durationMs}ms</span>
          </div>
          <div class="flex items-center gap-1">
            <button
              type="button"
              onclick={() => (activeResultTab = "diff")}
              class="px-2 py-0.5 rounded text-[11px] font-medium {activeResultTab === 'diff'
                ? 'bg-indigo-600 text-white'
                : 'text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800'}"
            >
              State Diff ({fieldDiffs.filter((d) => d.status !== "unchanged").length} mutated)
            </button>
            <button
              type="button"
              onclick={() => (activeResultTab = "logs")}
              class="px-2 py-0.5 rounded text-[11px] font-medium {activeResultTab === 'logs'
                ? 'bg-indigo-600 text-white'
                : 'text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800'}"
            >
              Logs & Toasts ({(executionResult.logs?.length || 0) + (executionResult.messages?.length || 0)})
            </button>
            <button
              type="button"
              onclick={() => (activeResultTab = "json")}
              class="px-2 py-0.5 rounded text-[11px] font-medium {activeResultTab === 'json'
                ? 'bg-indigo-600 text-white'
                : 'text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800'}"
            >
              Raw JSON
            </button>
          </div>
        </div>

        {#if activeResultTab === "diff"}
          <div class="max-h-44 overflow-y-auto divide-y divide-slate-100 dark:divide-slate-800/60 text-xs font-mono">
            {#each fieldDiffs as diff}
              <div class="px-3 py-1.5 flex items-center justify-between gap-4">
                <span class="font-semibold text-slate-700 dark:text-slate-300">{diff.key}</span>
                <div class="flex items-center gap-2 truncate">
                  {#if diff.status === "modified"}
                    <span class="line-through text-rose-500">{JSON.stringify(diff.beforeVal)}</span>
                    <span class="text-slate-400">→</span>
                    <span class="text-emerald-600 dark:text-emerald-400 font-semibold">{JSON.stringify(diff.afterVal)}</span>
                  {:else if diff.status === "added"}
                    <Badge tone="success">+ added</Badge>
                    <span class="text-emerald-600 dark:text-emerald-400 font-semibold">{JSON.stringify(diff.afterVal)}</span>
                  {:else}
                    <span class="text-slate-500">{JSON.stringify(diff.afterVal)}</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {:else if activeResultTab === "logs"}
          <div class="p-3 max-h-44 overflow-y-auto space-y-1.5 text-xs font-mono">
            {#if executionResult.messages?.length}
              {#each executionResult.messages as msg}
                <div class="px-2 py-1 rounded bg-sky-50 dark:bg-sky-950/40 text-sky-700 dark:text-sky-300">
                  [{msg.type?.toUpperCase() || "TOAST"}] <strong>{msg.title}:</strong> {msg.message}
                </div>
              {/each}
            {/if}
            {#if executionResult.logs?.length}
              {#each executionResult.logs as logLine}
                <div class="text-slate-600 dark:text-slate-400">› {logLine}</div>
              {/each}
            {:else if !executionResult.messages?.length}
              <div class="text-slate-400 italic">No logs or UI messages emitted.</div>
            {/if}
          </div>
        {:else}
          <pre class="p-3 text-[11px] font-mono overflow-x-auto max-h-44">{JSON.stringify(executionResult, null, 2)}</pre>
        {/if}
      </div>
    {/if}
  </div>
{/if}
