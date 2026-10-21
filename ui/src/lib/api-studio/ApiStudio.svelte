<script>
  import { onMount } from 'svelte';
  import {
    PRESET_ENDPOINTS,
    AUTOMATED_FLOW_STEPS
  } from './catalog.js';
  import {
    sqlTableName,
    isUuid,
    parseEntityRequest,
    fillKindFor,
    createPayloadFromSchema,
    batchPayloadFromSchema,
    updatePayloadFromSchema,
    liveEntityEndpoints
  } from './entitySchema.js';
  import {
    Play,
    CheckCircle2,
    XCircle,
    Clock,
    RefreshCw,
    Search,
    Copy,
    Check,
    Send,
    Database,
    Sliders,
    Layers,
    Shield,
    Activity,
    Settings,
    FileCode,
    Radio,
    Code2,
    Cpu,
    ExternalLink,
    Terminal,
    Sparkles,
    Trash2,
    Sun,
    Moon
  } from 'lucide-svelte';

  // --- Props ---
  let { isStandalone = true } = $props();

  // --- Active State ---
  let searchQuery = $state('');
  let apiBase = $state('');
  let isOnline = $state(false);
  let healthInfo = $state(null);
  let pingLatency = $state(null);

  // Active Request Configuration
  let activeMethod = $state('GET');
  let activePath = $state('/api/health');
  let activeBody = $state('');
  let activeName = $state('System Health Check');
  let activeDescription = $state('Verify BOS core Axum server liveness and service status.');

  // Session Context Headers

  // Response Inspection State
  let isLoading = $state(false);
  let lastResponse = $state(null);
  let lastStatus = $state(null);
  let lastDuration = $state(null);
  let responseHeaders = $state([]);
  let copyFeedback = $state(false);

  // Automated Integration Suite State
  let isRunningAutoSuite = $state(false);
  let autoSuiteProgress = $state({ current: 0, total: AUTOMATED_FLOW_STEPS.length, passed: 0, failed: 0 });
  let autoSuiteResults = $state([]);

  // Theme
  let isDarkMode = $state(false);

  // Live tables discovered from the catalog. Create and Update are generated from these.
  let liveTables = $state([]);
  let schemaNotice = $state('');
  let schemaBusy = $state(false);
  let activeEndpointId = $state('');

  let liveEndpoints = $derived(liveEntityEndpoints(liveTables));
  let allEndpoints = $derived([...liveEndpoints, ...PRESET_ENDPOINTS]);

  let entityRequest = $derived(parseEntityRequest(activePath));
  let schemaFillKind = $derived(fillKindFor(activeMethod, entityRequest));

  // Filtered Endpoints
  let filteredEndpoints = $derived(
    allEndpoints.filter(ep => {
      const q = searchQuery.toLowerCase();
      return !q ||
        ep.name.toLowerCase().includes(q) ||
        ep.path.toLowerCase().includes(q) ||
        ep.method.toLowerCase().includes(q) ||
        ep.description.toLowerCase().includes(q);
    })
  );

  function sessionHeaders() {
    const headers = { 'Content-Type': 'application/json' };
    try {
      const raw = localStorage.getItem('bos_session');
      const token = JSON.parse(raw || 'null')?.token || '';
      if (token) {
        headers['Authorization'] = `Bearer ${token}`;
      }
      const env = JSON.parse(localStorage.getItem('bos_environment') || 'null');
      if (env?.tenantId) headers['x-bos-tenant-id'] = String(env.tenantId);
      if (env?.companyName) headers['x-bos-company'] = String(env.companyName);
    } catch {
      // ignore
    }
    return headers;
  }

  async function loadLiveTables() {
    try {
      const res = await fetch(`${apiBase}/api/meta/objects?type=table`, { headers: sessionHeaders() });
      if (!res.ok) return;
      const rows = await res.json();
      if (!Array.isArray(rows)) return;
      const seen = new Set();
      liveTables = rows
        .filter((row) => String(row.object_type || '').toLowerCase() === 'table' && row.object_name)
        .map((row) => ({
          name: row.object_name,
          sql: sqlTableName(row.object_name),
          packageName: row.package_name || '',
          caption: row.caption || row.object_name
        }))
        .filter((table) => {
          if (seen.has(table.sql)) return false;
          seen.add(table.sql);
          return true;
        });
    } catch {
      liveTables = [];
    }
  }

  async function readJson(url) {
    const res = await fetch(url, { headers: sessionHeaders() });
    const text = await res.text();
    let data = null;
    try { data = text ? JSON.parse(text) : null; } catch { data = { message: text }; }
    return { ok: res.ok, status: res.status, data };
  }

  /**
   * Build the create or update body from GET /api/entities/{table}/schema.
   * Update also reads one row so the path id and _system_version match a real record.
   */
  async function fillFromSchema(table, kind) {
    const target = table || entityRequest?.table;
    if (!target || !kind) return;
    schemaBusy = true;
    schemaNotice = `Reading ${target} schema…`;
    try {
      const schemaRes = await readJson(`${apiBase}/api/entities/${encodeURIComponent(target)}/schema`);
      if (!schemaRes.ok || !schemaRes.data?.columns) {
        schemaNotice = schemaRes.data?.message || schemaRes.data?.error || `No schema for ${target}.`;
        return;
      }
      const schema = schemaRes.data;
      const physical = schema.physical_table || target;
      const columnCount = schema.columns.length;

      if (kind === 'create') {
        activeMethod = 'POST';
        activePath = `/api/entities/${physical}`;
        activeBody = JSON.stringify(createPayloadFromSchema(schema), null, 2);
        activeName = `Create ${schema.table_name || physical}`;
        activeDescription = `Insert one ${schema.table_name || physical} record. Fields come from the live schema.`;
        schemaNotice = `Create body filled from ${schema.table_name || physical} (${columnCount} columns).`;
      } else if (kind === 'batch') {
        activeMethod = 'POST';
        activePath = `/api/entities/${physical}/batch`;
        activeBody = JSON.stringify(batchPayloadFromSchema(schema), null, 2);
        activeName = `Batch ${schema.table_name || physical}`;
        activeDescription = `Insert several ${schema.table_name || physical} records. Each row follows the live schema.`;
        schemaNotice = `Batch body filled from ${schema.table_name || physical} (${columnCount} columns).`;
      } else {
        const existingId = entityRequest?.id && isUuid(entityRequest.id) ? entityRequest.id : '';
        let row = null;
        if (existingId) {
          const one = await readJson(`${apiBase}/api/entities/${encodeURIComponent(physical)}/${existingId}`);
          if (one.ok && one.data && typeof one.data === 'object' && !Array.isArray(one.data)) row = one.data;
        }
        if (!row) {
          const page = await readJson(`${apiBase}/api/entities/${encodeURIComponent(physical)}?page=1&limit=1`);
          row = page.data?.records?.[0] || null;
        }
        const id = row?._id || existingId || '00000000-0000-0000-0000-000000000000';
        activeMethod = activeMethod === 'PATCH' ? 'PATCH' : 'PUT';
        activePath = `/api/entities/${physical}/${id}`;
        activeBody = JSON.stringify(updatePayloadFromSchema(schema, row), null, 2);
        activeName = `Update ${schema.table_name || physical}`;
        activeDescription = row
          ? `Update ${schema.table_name || physical} ${id}. _system_version is the value just read, and the other fields are the current row.`
          : `Update body from the ${schema.table_name || physical} schema. No row exists yet, so the id is a placeholder. Create a record first.`;
        schemaNotice = row
          ? `Update body filled from ${schema.table_name || physical} (${columnCount} columns) using record ${id}.`
          : `Update body filled from ${schema.table_name || physical}. No existing record was found.`;
      }
    } catch (err) {
      schemaNotice = err?.message || 'Could not read the entity schema.';
    } finally {
      schemaBusy = false;
    }
  }

  function onEntityPicked(event) {
    const table = event.currentTarget.value;
    const parsed = parseEntityRequest(activePath);
    const kind = fillKindFor(activeMethod, parsed);
    if (parsed?.action === 'schema') {
      activePath = `/api/entities/${table}/schema`;
      schemaNotice = '';
      return;
    }
    if (parsed?.action === 'collection' && activeMethod === 'GET') {
      activePath = `/api/entities/${table}?page=1&limit=25`;
      schemaNotice = '';
      return;
    }
    if (kind === 'batch') {
      activePath = `/api/entities/${table}/batch`;
      fillFromSchema(table, 'batch');
      return;
    }
    if (kind === 'update') {
      const id = parsed?.id && isUuid(parsed.id) ? parsed.id : '{id}';
      activePath = `/api/entities/${table}/${id}`;
      fillFromSchema(table, 'update');
      return;
    }
    activeMethod = 'POST';
    activePath = `/api/entities/${table}`;
    fillFromSchema(table, 'create');
  }

  // Check Server Health
  async function checkHealth() {
    const t0 = performance.now();
    try {
      const res = await fetch(`${apiBase}/api/health`, { method: 'GET' });
      pingLatency = Math.round(performance.now() - t0);
      if (res.ok) {
        healthInfo = await res.json();
        isOnline = true;
        if (liveTables.length === 0) loadLiveTables();
      } else {
        isOnline = false;
        healthInfo = null;
      }
    } catch (e) {
      pingLatency = null;
      isOnline = false;
      healthInfo = null;
    }
  }

  // Load Preset. Live create/update endpoints replace the sample with the table schema.
  function selectPreset(ep) {
    activeEndpointId = ep.id;
    activeMethod = ep.method;
    activePath = ep.path;
    activeBody = ep.sampleBody || '';
    activeName = ep.name;
    activeDescription = ep.description;
    schemaNotice = '';
    if (ep.fill && ep.table) fillFromSchema(ep.table, ep.fill);
  }

  // Execute Current Request
  async function executeRequest() {
    isLoading = true;
    lastResponse = null;
    lastStatus = null;
    lastDuration = null;
    responseHeaders = [];

    const url = `${apiBase}${activePath}`;
    const t0 = performance.now();

    const options = {
      method: activeMethod,
      headers: sessionHeaders()
    };

    if (['POST', 'PUT', 'PATCH'].includes(activeMethod) && activeBody.trim()) {
      options.body = activeBody;
    }

    try {
      const res = await fetch(url, options);
      const duration = Math.round(performance.now() - t0);
      lastStatus = res.status;
      lastDuration = duration;

      const headerEntries = [];
      res.headers.forEach((v, k) => headerEntries.push({ key: k, value: v }));
      responseHeaders = headerEntries;

      const text = await res.text();
      try {
        lastResponse = JSON.parse(text);
      } catch {
        lastResponse = text;
      }
    } catch (err) {
      lastStatus = 0;
      lastDuration = Math.round(performance.now() - t0);
      lastResponse = { error: true, message: err.message };
    } finally {
      isLoading = false;
    }
  }

  // Run Automated 15-Step Test Suite
  async function runAutomatedSuite() {
    isRunningAutoSuite = true;
    autoSuiteResults = [];
    autoSuiteProgress = { current: 0, total: AUTOMATED_FLOW_STEPS.length, passed: 0, failed: 0 };

    for (let i = 0; i < AUTOMATED_FLOW_STEPS.length; i++) {
      const s = AUTOMATED_FLOW_STEPS[i];
      autoSuiteProgress.current = i + 1;

      const t0 = performance.now();
      const url = `${apiBase}${s.path}`;
      const options = { method: s.method, headers: sessionHeaders() };
      if (s.body) options.body = JSON.stringify(s.body);

      try {
        const res = await fetch(url, options);
        const duration = Math.round(performance.now() - t0);
        const isSuccess = res.status >= 200 && res.status < 300;

        const resData = await res.json().catch(() => ({}));
        if (isSuccess) autoSuiteProgress.passed++;
        else autoSuiteProgress.failed++;

        autoSuiteResults = [
          ...autoSuiteResults,
          {
            step: s.step,
            name: s.name,
            method: s.method,
            path: s.path,
            status: res.status,
            duration,
            success: isSuccess,
            data: resData
          }
        ];
      } catch (err) {
        autoSuiteProgress.failed++;
        autoSuiteResults = [
          ...autoSuiteResults,
          {
            step: s.step,
            name: s.name,
            method: s.method,
            path: s.path,
            status: 0,
            duration: Math.round(performance.now() - t0),
            success: false,
            data: { error: err.message }
          }
        ];
      }
    }

    isRunningAutoSuite = false;
  }

  function copyResponse() {
    if (!lastResponse) return;
    const text = typeof lastResponse === 'string' ? lastResponse : JSON.stringify(lastResponse, null, 2);
    navigator.clipboard.writeText(text);
    copyFeedback = true;
    setTimeout(() => { copyFeedback = false; }, 2000);
  }

  function getMethodBadgeClass(m) {
    switch (m) {
      case 'GET': return 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20';
      case 'POST': return 'bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/20';
      case 'PUT': return 'bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20';
      case 'PATCH': return 'bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20';
      case 'DELETE': return 'bg-rose-500/10 text-rose-600 dark:text-rose-400 border-rose-500/20';
      default: return 'bg-slate-500/10 text-slate-600 border-slate-500/20';
    }
  }

  function getStatusClass(s) {
    if (!s) return 'bg-rose-500/10 text-rose-600 border-rose-500/20';
    if (s >= 200 && s < 300) return 'bg-emerald-500/10 text-emerald-600 border-emerald-500/20';
    if (s >= 400 && s < 500) return 'bg-amber-500/10 text-amber-600 border-amber-500/20';
    return 'bg-rose-500/10 text-rose-600 border-rose-500/20';
  }

  onMount(() => {
    checkHealth();
    const interval = setInterval(checkHealth, 10000);
    return () => clearInterval(interval);
  });
</script>

<div class={`h-screen h-[100dvh] max-h-screen flex flex-col font-sans overflow-hidden transition-colors ${isDarkMode ? 'dark bg-slate-950 text-slate-100' : 'bg-slate-50 text-slate-900'}`}>

  <!-- Top App Navigation Bar -->
  <header class="h-14 border-b border-slate-200 dark:border-slate-800 bg-white/80 dark:bg-slate-900/80 backdrop-blur-md px-4 flex items-center justify-between sticky top-0 z-40 shrink-0">
    <div class="flex items-center gap-3">
      <div class="size-8 rounded-lg bg-indigo-600 text-white flex items-center justify-center font-bold text-sm shadow-xs">
        BOS
      </div>
      <div>
        <div class="font-bold text-sm flex items-center gap-2">
          <span>API Testing Studio</span>
          <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 font-semibold">
            v2.0 Automated
          </span>
        </div>
      </div>
    </div>

    <!-- Health / Connection Pill & Automated Suite Launcher -->
    <div class="flex items-center gap-2.5">
      <!-- Live Server Indicator -->
      <div class="flex items-center gap-2 px-3 py-1.5 rounded-full border border-slate-200 dark:border-slate-800 text-xs font-mono">
        <span class={`size-2 rounded-full ${isOnline ? 'bg-emerald-500 animate-pulse' : 'bg-rose-500'}`}></span>
        <span>{isOnline ? 'LIVE 3000' : 'OFFLINE'}</span>
        {#if pingLatency !== null}
          <span class="text-[10px] text-slate-400">({pingLatency}ms)</span>
        {/if}
      </div>

      <!-- Single-Click Automated Test Runner -->
      <button
        onclick={runAutomatedSuite}
        disabled={isRunningAutoSuite || !isOnline}
        class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-xs font-semibold cursor-pointer shadow-xs transition-all"
      >
        {#if isRunningAutoSuite}
          <RefreshCw class="size-3.5 animate-spin" />
          <span>Running ({autoSuiteProgress.current}/{autoSuiteProgress.total})...</span>
        {:else}
          <Play class="size-3.5 fill-current" />
          <span>Run Automated Suite</span>
        {/if}
      </button>

      <!-- Theme Switcher -->
      <button
        onclick={() => (isDarkMode = !isDarkMode)}
        class="size-8 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-center text-slate-500 hover:text-slate-900 dark:hover:text-slate-100 cursor-pointer"
        title="Toggle Theme"
      >
        {#if isDarkMode}
          <Sun class="size-4" />
        {:else}
          <Moon class="size-4" />
        {/if}
      </button>

      {#if !isStandalone}
        <a
          href="/api-studio.html"
          target="_blank"
          class="size-8 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-center text-slate-500 hover:text-slate-900 cursor-pointer"
          title="Open in new standalone tab"
        >
          <ExternalLink class="size-3.5" />
        </a>
      {/if}
    </div>
  </header>

  <!-- Automated Suite Summary Banner (Visible when run) -->
  {#if autoSuiteResults.length > 0}
    <div class="bg-indigo-900/10 border-b border-indigo-500/20 px-4 py-2.5 flex items-center justify-between text-xs shrink-0">
      <div class="flex items-center gap-3">
        <span class="font-semibold text-indigo-700 dark:text-indigo-400">Automated Integration Verification:</span>
        <span class="text-emerald-600 dark:text-emerald-400 font-mono font-medium">✓ {autoSuiteProgress.passed} Passed</span>
        {#if autoSuiteProgress.failed > 0}
          <span class="text-rose-600 dark:text-rose-400 font-mono font-medium">✕ {autoSuiteProgress.failed} Failed</span>
        {/if}
        <span class="text-slate-500 font-mono">({autoSuiteProgress.total} steps executed)</span>
      </div>
      <button
        onclick={() => (autoSuiteResults = [])}
        class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer font-medium"
      >
        Dismiss
      </button>
    </div>
  {/if}

  <!-- 3-Column Studio Workspace -->
  <div class="flex-1 min-h-0 flex overflow-hidden">

    <!-- Column 1: Categories & Endpoint Presets -->
    <aside class="w-80 border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex flex-col shrink-0 min-h-0">
      <!-- Search Box & Header -->
      <div class="p-3 border-b border-slate-200 dark:border-slate-800 shrink-0">
        <div class="relative">
          <Search class="size-3.5 absolute left-2.5 top-2.5 text-slate-400" />
          <input
            type="text"
            bind:value={searchQuery}
            placeholder={`Filter ${allEndpoints.length} endpoints...`}
            class="w-full pl-8 pr-3 py-1.5 bg-slate-100 dark:bg-slate-800 text-xs rounded-md border-0 focus:ring-1 focus:ring-indigo-500 outline-none"
          />
        </div>
        <div class="flex items-center justify-between mt-2 px-0.5 text-[10px] text-slate-400 font-mono">
          <span class="font-semibold uppercase tracking-wider text-slate-500">Endpoints</span>
          <button
            type="button"
            onclick={loadLiveTables}
            class="text-indigo-600 dark:text-indigo-400 hover:underline cursor-pointer"
            title="Reload tables and rebuild Create and Update endpoints"
          >
            Refresh entities
          </button>
          <span class="bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 rounded font-bold text-slate-700 dark:text-slate-300">
            {filteredEndpoints.length} of {allEndpoints.length}
          </span>
        </div>
      </div>

      <!-- Endpoint List -->
      <div class="flex-1 min-h-0 overflow-y-auto divide-y divide-slate-100 dark:divide-slate-800/50">
        {#each filteredEndpoints as ep}
          {@const isSelected = activeEndpointId === ep.id}
          <button
            onclick={() => selectPreset(ep)}
            class={`w-full p-3 text-left transition-all cursor-pointer block border-l-4 ${
              isSelected
                ? 'bg-indigo-50/90 dark:bg-indigo-950/70 border-indigo-600 shadow-sm relative z-10'
                : 'border-transparent hover:bg-slate-100/70 dark:hover:bg-slate-800/40 text-slate-600 dark:text-slate-400'
            }`}
          >
            <div class="flex items-center justify-between gap-1.5">
              <span class={`text-xs truncate ${isSelected ? 'font-bold text-indigo-950 dark:text-indigo-100' : 'font-medium text-slate-800 dark:text-slate-200'}`}>
                {ep.name}
              </span>
              <span class="flex items-center gap-1 shrink-0">
                {#if ep.live}
                  <span class="text-[9px] font-semibold uppercase tracking-wide px-1 py-0.5 rounded bg-emerald-500/10 text-emerald-700 dark:text-emerald-400 border border-emerald-500/20">Schema</span>
                {/if}
                <span class={`text-[10px] font-mono px-1.5 py-0.5 rounded font-bold border ${getMethodBadgeClass(ep.method)}`}>
                  {ep.method}
                </span>
              </span>
            </div>
            <div class="flex items-center justify-between mt-1 gap-1">
              <p class={`font-mono text-[11px] truncate ${isSelected ? 'text-indigo-600 dark:text-indigo-400 font-semibold' : 'text-slate-400 dark:text-slate-500'}`}>
                {ep.path}
              </p>
              {#if isSelected}
                <span class="inline-flex items-center justify-center size-2 rounded-full bg-indigo-600 dark:bg-indigo-400 shrink-0"></span>
              {/if}
            </div>
          </button>
        {/each}
      </div>
    </aside>

    <!-- Column 2: Interactive Request Composer -->
    <main class="flex-1 min-h-0 flex flex-col border-r border-slate-200 dark:border-slate-800 overflow-y-auto bg-white dark:bg-slate-900">
      <!-- Request Bar -->
      <div class="p-4 border-b border-slate-200 dark:border-slate-800 space-y-3">
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-base font-bold text-slate-900 dark:text-slate-100">{activeName}</h2>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">{activeDescription}</p>
          </div>
          <button
            onclick={executeRequest}
            disabled={isLoading || !isOnline}
            class="flex items-center gap-1.5 px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white text-xs font-semibold cursor-pointer shadow-xs"
          >
            {#if isLoading}
              <RefreshCw class="size-3.5 animate-spin" />
              <span>Executing...</span>
            {:else}
              <Send class="size-3.5" />
              <span>Send Request</span>
            {/if}
          </button>
        </div>

        <!-- URL Input Row -->
        <div class="flex items-center gap-2">
          <select
            bind:value={activeMethod}
            class={`font-mono text-xs font-bold px-2.5 py-2 rounded-lg border outline-none cursor-pointer ${getMethodBadgeClass(activeMethod)}`}
          >
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="PATCH">PATCH</option>
            <option value="DELETE">DELETE</option>
          </select>
          <div class="flex-1 flex items-center bg-slate-100 dark:bg-slate-800 rounded-lg px-3 py-2 border border-slate-200 dark:border-slate-700">
            <span class="text-xs font-mono text-slate-400 select-none mr-1">{apiBase}</span>
            <input
              type="text"
              bind:value={activePath}
              class="flex-1 font-mono text-xs bg-transparent border-0 outline-none text-slate-800 dark:text-slate-200"
            />
          </div>
        </div>

        {#if entityRequest}
          <div class="flex items-center gap-2 flex-wrap">
            <label for="entity-picker" class="text-[10px] font-semibold uppercase tracking-wider text-slate-500">Entity</label>
            <select
              id="entity-picker"
              value={entityRequest.table}
              onchange={onEntityPicked}
              class="font-mono text-xs px-2 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-100 dark:bg-slate-800 outline-none cursor-pointer"
            >
              {#if !liveTables.some((table) => table.sql === entityRequest.table)}
                <option value={entityRequest.table}>{entityRequest.table}</option>
              {/if}
              {#each liveTables as table}
                <option value={table.sql}>{table.caption || table.name}</option>
              {/each}
            </select>
            {#if schemaFillKind}
              <button
                type="button"
                onclick={() => fillFromSchema(entityRequest.table, schemaFillKind)}
                disabled={schemaBusy || !isOnline}
                class="text-[11px] font-semibold px-2.5 py-1.5 rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300 hover:bg-emerald-500/20 disabled:opacity-50 cursor-pointer"
              >
                {schemaBusy ? 'Reading schema…' : schemaFillKind === 'update' ? 'Fill update from schema' : schemaFillKind === 'batch' ? 'Fill batch from schema' : 'Fill create from schema'}
              </button>
            {/if}
          </div>
        {/if}

        <p class="text-xs text-slate-500">Requests use the signed-in session.</p>
      </div>

      <!-- JSON Body Editor -->
      <div class="flex-1 flex flex-col p-4">
        <div class="flex items-center justify-between pb-1">
          <span class="text-xs font-semibold text-slate-700 dark:text-slate-300">Request Body (JSON / DSL)</span>
          {#if activeBody.trim()}
            <button
              onclick={() => (activeBody = '')}
              class="text-[11px] text-slate-400 hover:text-slate-600 cursor-pointer"
            >
              Clear Body
            </button>
          {/if}
        </div>
        {#if schemaNotice}
          <p class="text-[11px] text-emerald-700 dark:text-emerald-400 pb-2">{schemaNotice}</p>
        {/if}
        <textarea
          bind:value={activeBody}
          placeholder="No request body required for this endpoint (or enter JSON payload)..."
          class="flex-1 font-mono text-xs p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 text-slate-900 dark:text-slate-100 resize-none outline-none focus:border-indigo-500"
          rows="14"
        ></textarea>
      </div>
    </main>

    <!-- Column 3: Live Response Inspector & Automated Test Logs -->
    <aside class="w-[460px] flex flex-col shrink-0 bg-white dark:bg-slate-900 min-h-0">
      <!-- Response Status Header -->
      <div class="p-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="text-xs font-semibold text-slate-700 dark:text-slate-300">Response</span>
          {#if lastStatus !== null}
            <span class={`text-[10px] font-mono px-2 py-0.5 rounded font-bold border ${getStatusClass(lastStatus)}`}>
              {lastStatus || 'NETWORK ERROR'}
            </span>
          {/if}
          {#if lastDuration !== null}
            <span class="text-[10px] font-mono text-slate-400 flex items-center gap-1">
              <Clock class="size-3" />
              {lastDuration}ms
            </span>
          {/if}
        </div>
        <button
          onclick={copyResponse}
          disabled={!lastResponse}
          class="flex items-center gap-1 text-xs text-slate-500 hover:text-slate-900 dark:hover:text-slate-100 cursor-pointer disabled:opacity-30"
        >
          {#if copyFeedback}
            <Check class="size-3.5 text-emerald-500" />
            <span class="text-emerald-500">Copied</span>
          {:else}
            <Copy class="size-3.5" />
            <span>Copy</span>
          {/if}
        </button>
      </div>

      <!-- Response Content Surface -->
      <div class="flex-1 p-3 overflow-y-auto">
        {#if isLoading}
          <div class="flex flex-col items-center justify-center h-full text-slate-400 space-y-2">
            <RefreshCw class="size-6 animate-spin text-indigo-500" />
            <span class="text-xs">Awaiting backend response...</span>
          </div>
        {:else if lastResponse !== null}
          <pre class="font-mono text-xs leading-relaxed p-3 rounded-lg bg-slate-900 text-slate-100 overflow-x-auto select-all">{typeof lastResponse === 'string' ? lastResponse : JSON.stringify(lastResponse, null, 2)}</pre>
        {:else}
          <div class="flex flex-col items-center justify-center h-full text-center p-6 text-slate-400 space-y-3">
            <Terminal class="size-8 stroke-1 text-slate-300 dark:text-slate-700" />
            <p class="text-xs leading-normal">
              Select any endpoint from the left directory or click <strong>Run Automated Suite</strong> to inspect real-time responses.
            </p>
          </div>
        {/if}
      </div>

      <!-- Response Headers Inspector -->
      {#if responseHeaders.length > 0}
        <div class="p-3 border-t border-slate-200 dark:border-slate-800 max-h-40 overflow-y-auto bg-slate-50 dark:bg-slate-950">
          <span class="text-[10px] font-bold text-slate-400 uppercase tracking-wider block mb-1">Response Headers</span>
          <div class="space-y-0.5">
            {#each responseHeaders as h}
              <div class="flex text-[10px] font-mono">
                <span class="text-slate-500 w-36 truncate">{h.key}:</span>
                <span class="text-slate-800 dark:text-slate-200 truncate">{h.value}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </aside>

  </div>

</div>
