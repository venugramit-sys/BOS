<script>
  /**
   * DynamicPageRenderer.svelte (Svelte 5 Runes)
   * Universal runtime renderer for fully resolved BOS Pages + PageExtensions.
   * Fetches the merged layout tree and interconnected meta-object schemas,
   * binds draft record state, and dispatches actions/state machine transitions.
   */
  import { onMount } from "svelte";
  import {
    Button,
    Badge,
    Banner,
    Spinner,
    toastStore
  } from "$lib/bos-ui/index.js";
  import {
    Save,
    Play,
    CheckCircle2,
    RefreshCw,
    AlertCircle,
    ArrowLeft
  } from "lucide-svelte";
  import LayoutNode from "./LayoutNode.svelte";

  let {
    pageName,
    recordId = null,
    mode = "edit", // 'edit' | 'view'
    onBack = null
  } = $props();

  let loading = $state(true);
  let error = $state(null);
  let resolvedData = $state(null);
  let record = $state({});
  let fieldErrors = $state({});
  let isSaving = $state(false);

  // Derived shortcuts
  let page = $derived(resolvedData?.page);
  let resolvedFields = $derived(resolvedData?.resolved_fields || {});
  let stateMachine = $derived(resolvedData?.state_machine);
  let transitions = $derived(resolvedData?.transitions || []);
  let actions = $derived(page?.actions || []);

  onMount(async () => {
    await loadPageDefinition();
  });

  async function loadPageDefinition() {
    loading = true;
    error = null;
    try {
      const res = await fetch(`/api/v1/pages/${encodeURIComponent(pageName)}`);
      if (!res.ok) {
        throw new Error(`Failed to load page '${pageName}': HTTP ${res.status}`);
      }
      resolvedData = await res.json();

      // If recordId is provided, fetch existing row
      if (recordId && page?.source_table) {
        const rowRes = await fetch(`/api/entities/${encodeURIComponent(page.source_table)}/${recordId}`);
        if (rowRes.ok) {
          record = await rowRes.json();
        }
      } else {
        // Initialize defaults from resolved field metadata
        const draft = {};
        for (const [k, meta] of Object.entries(resolvedFields)) {
          if (meta.default_value) {
            draft[k] = meta.default_value;
          }
        }
        record = draft;
      }
    } catch (err) {
      error = err.message;
    } finally {
      loading = false;
    }
  }

  async function handleFieldChange(fieldName, value) {
    // Clear validation error when field changes
    if (fieldErrors[fieldName]) {
      const next = { ...fieldErrors };
      delete next[fieldName];
      fieldErrors = next;
    }
  }

  async function handleSave() {
    if (!page?.source_table) return;
    isSaving = true;
    try {
      const url = recordId
        ? `/api/entities/${encodeURIComponent(page.source_table)}/${recordId}`
        : `/api/entities/${encodeURIComponent(page.source_table)}`;
      const method = recordId ? "PATCH" : "POST";

      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(record)
      });

      if (!res.ok) {
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error || `Save failed with status ${res.status}`);
      }

      const saved = await res.json();
      record = saved;
      toastStore.success(`${page.caption || pageName} saved successfully`);
    } catch (err) {
      toastStore.error(err.message);
    } finally {
      isSaving = false;
    }
  }

  async function handleExecuteAction(action) {
    if (action.run_codeunit) {
      try {
        const res = await fetch(`/api/codeunits/${encodeURIComponent(action.run_codeunit)}/run`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ record, pageName })
        });
        if (res.ok) {
          toastStore.success(`Action '${action.caption || action.name}' executed`);
        } else {
          toastStore.error(`Action execution failed: HTTP ${res.status}`);
        }
      } catch (err) {
        toastStore.error(err.message);
      }
    }
  }

  async function handleTransition(transition) {
    if (!page?.source_table || !recordId) return;
    try {
      const res = await fetch(`/api/statemachines/${encodeURIComponent(stateMachine)}/transition`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          table: page.source_table,
          id: recordId,
          transition: transition.name
        })
      });
      if (res.ok) {
        toastStore.success(`State transitioned to ${transition.to_state}`);
        await loadPageDefinition();
      } else {
        toastStore.error("Failed to execute state transition");
      }
    } catch (err) {
      toastStore.error(err.message);
    }
  }
</script>

<div class="flex flex-col h-full w-full bg-slate-50/50 dark:bg-slate-950/50 overflow-y-auto">
  <!-- Top Command & Action Bar -->
  <header class="flex items-center justify-between px-6 py-3 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 sticky top-0 z-10 shadow-xs">
    <div class="flex items-center gap-3">
      {#if onBack}
        <Button variant="ghost" size="sm" onclick={onBack}>
          <ArrowLeft class="w-4 h-4 mr-1" /> Back
        </Button>
      {/if}
      <div>
        <h1 class="text-base font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
          {page?.caption || pageName}
          {#if page?.page_type}
            <Badge variant="secondary" class="text-[10px] font-normal">{page.page_type}</Badge>
          {/if}
        </h1>
        {#if page?.source_table}
          <p class="text-[11px] text-slate-500">Source: {page.source_table}</p>
        {/if}
      </div>
    </div>

    <!-- Actions & Workflow Transitions -->
    <div class="flex items-center gap-2">
      <!-- State Machine Transitions -->
      {#each transitions as tr (tr.name)}
        <Button
          variant="outline"
          size="sm"
          onclick={() => handleTransition(tr)}
          class="border-indigo-200 text-indigo-700 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/40"
        >
          <Play class="w-3.5 h-3.5 mr-1" />
          {tr.name} &rarr; {tr.to_state}
        </Button>
      {/each}

      <!-- Custom Page Actions -->
      {#each actions as act (act.name)}
        <Button
          variant={act.primary ? "default" : "outline"}
          size="sm"
          onclick={() => handleExecuteAction(act)}
        >
          {act.caption || act.name}
        </Button>
      {/each}

      <!-- Primary Save Action -->
      <Button
        variant="default"
        size="sm"
        disabled={isSaving}
        onclick={handleSave}
        class="bg-emerald-600 hover:bg-emerald-700 text-white"
      >
        {#if isSaving}
          <Spinner size="sm" class="mr-1" /> Saving...
        {:else}
          <Save class="w-4 h-4 mr-1" /> Save
        {/if}
      </Button>
    </div>
  </header>

  <!-- Main Dynamic Layout Body -->
  <main class="flex-1 p-6 max-w-7xl w-full mx-auto space-y-6">
    {#if loading}
      <div class="flex flex-col items-center justify-center py-24 gap-3 text-slate-400">
        <Spinner size="lg" />
        <p class="text-sm">Resolving page layout and metadata...</p>
      </div>
    {:else if error}
      <Banner variant="destructive" title="Page Resolution Error">
        {error}
      </Banner>
    {:else if page}
      <div class="space-y-6">
        {#each page.controls as control (control.name || control.kind + Math.random())}
          <LayoutNode
            node={control}
            bind:record
            {resolvedFields}
            {mode}
            {fieldErrors}
            onFieldChange={handleFieldChange}
          />
        {/each}
      </div>
    {/if}
  </main>
</div>
