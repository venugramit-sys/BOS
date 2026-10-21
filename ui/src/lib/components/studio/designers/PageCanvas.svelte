<script>
  /**
   * PageCanvas.svelte (Svelte 5 Runes)
   * Microsoft Access-Style WYSIWYG Form / Page Designer Canvas & Live Interactive Preview
   * Supports all toolbox control types (fasttab, grid, subform, text, decimal, date, lookup, toggle, tile, step, factbox, action)
   * and all 8 page layouts (List, Record, Document, Board, Workspace, Split, Timeline, Wizard).
   */
  import {
    Trash2,
    ChevronDown,
    Layers,
    LayoutGrid,
    Search,
    BarChart3,
    ListOrdered,
    PanelRight,
    Plus,
    Play,
    Lock,
    User,
    Eye,
    EyeOff,
    ArrowRight,
    CheckCircle2,
    AlertCircle,
    Loader2,
  } from "lucide-svelte";
  import { Button, Input, DatePicker, Switch, Badge, Select } from "$lib/bos-ui/index.js";

  let {
    elements = $bindable([]),
    selectedElement = $bindable(null),
    layoutType = "Record",
    pageCaption = "Page Layout",
    sourceTable = "",
    previewMode = false,
    onSelect = null,
  } = $props();

  let previewValues = $state({});
  let activeWizardStep = $state(0);
  let showPreviewPassword = $state(false);
  let previewRememberMe = $state(true);
  let previewBusy = $state(false);
  let previewSuccess = $state(false);

  let actionElements = $derived(elements.filter((el) => el.type === "action"));
  let sideElements = $derived(elements.filter((el) => el.type === "factbox"));
  let stepElements = $derived(elements.filter((el) => el.type === "step"));
  let tileElements = $derived(elements.filter((el) => el.type === "tile"));
  let dataFieldElements = $derived(
    elements.filter((el) =>
      ["text", "decimal", "date", "lookup", "toggle", "password"].includes(el.type)
    )
  );

  function handleSelect(el, e) {
    if (previewMode) return;
    e.stopPropagation();
    selectedElement = el;
    if (onSelect) onSelect(el);
  }

  function handleDelete(id, e) {
    e.stopPropagation();
    elements = elements.filter((el) => el.id !== id);
    if (selectedElement && selectedElement.id === id) {
      selectedElement = null;
    }
  }
</script>

<div
  onclick={() => {
    if (previewMode) return;
    selectedElement = null;
    if (onSelect) onSelect(null);
  }}
  role="presentation"
  class="flex-1 p-6 overflow-auto bg-slate-100/50 dark:bg-slate-950 flex justify-center cursor-default"
>
  <div class="w-full max-w-5xl bg-white dark:bg-slate-900 rounded-xl border border-slate-300 dark:border-slate-800 shadow-sm p-6 space-y-6 min-h-[520px]">
    <!-- Top Header & Command Bar -->
    <div class="border-b border-slate-200 dark:border-slate-800 pb-3 flex flex-wrap items-center justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <h3 class="text-sm font-bold text-slate-800 dark:text-slate-200">
            {pageCaption}
          </h3>
          <Badge tone="info">{layoutType}</Badge>
          {#if sourceTable}
            <span class="text-[11px] font-mono text-slate-500">source: {sourceTable}</span>
          {/if}
        </div>
        {#if !previewMode}
          <p class="text-[11px] text-slate-400 mt-0.5">
            WYSIWYG layout synchronized with declarative .bos page definition.
          </p>
        {/if}
      </div>

      <div class="flex items-center gap-2">
        {#if previewMode && actionElements.length > 0 && layoutType !== "Dialog" && pageCaption !== "Sign In"}
          {#each actionElements as act (act.id)}
            <Button size="xs" icon={Play}>
              {act.caption || act.label}
            </Button>
          {/each}
        {/if}
        {#if !previewMode}
          <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-slate-500">
            Snap to Grid: ON
          </span>
        {:else}
          <Badge tone="success">Live Interactive Preview</Badge>
        {/if}
      </div>
    </div>

    {#if elements.length === 0}
      <div class="py-16 text-center text-xs text-slate-400 italic border-2 border-dashed border-slate-200 dark:border-slate-800 rounded-xl">
        Click items in the Control Toolbox on the left to add controls to this form canvas.
      </div>
    {:else if previewMode && layoutType === "List"}
      <!-- Clean Live Preview: List Layout -->
      <div class="space-y-3">
        <div class="flex items-center justify-between gap-2">
          <Input placeholder="Filter records..." class="max-w-xs" />
          <Button size="xs" icon={Plus}>New {sourceTable || "Record"}</Button>
        </div>
        <div class="rounded-lg border border-slate-200 dark:border-slate-800 overflow-hidden">
          <table class="w-full text-xs text-left">
            <thead class="bg-slate-50 dark:bg-slate-800/70 text-slate-600 dark:text-slate-300 border-b border-slate-200 dark:border-slate-800">
              <tr>
                {#each dataFieldElements as col (col.id)}
                  <th class="px-3 py-2 font-semibold">{col.caption || col.label}</th>
                {/each}
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
              {#each [1, 2, 3] as rowIdx}
                <tr class="hover:bg-slate-50 dark:hover:bg-slate-800/40">
                  {#each dataFieldElements as col (col.id)}
                    <td class="px-3 py-2 text-slate-700 dark:text-slate-300">
                      {#if col.type === "decimal"}
                        {(rowIdx * 125.5).toFixed(2)}
                      {:else if col.type === "toggle"}
                        <Badge tone={rowIdx % 2 === 1 ? "success" : "neutral"}>{rowIdx % 2 === 1 ? "Yes" : "No"}</Badge>
                      {:else if col.type === "date"}
                        2026-10-1{rowIdx}
                      {:else}
                        {col.sourceField || col.label} #{rowIdx}
                      {/if}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {:else if previewMode && layoutType === "Board"}
      <!-- Clean Live Preview: Kanban Board Layout -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        {#each ["Draft", "In Progress", "Released"] as lane}
          <div class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-slate-800 space-y-2.5">
            <div class="flex items-center justify-between text-xs font-bold text-slate-700 dark:text-slate-300">
              <span>{lane}</span>
              <Badge>{lane === "Draft" ? 2 : 1}</Badge>
            </div>
            <div class="p-3 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-2xs space-y-1.5 text-xs">
              <div class="font-semibold text-slate-800 dark:text-slate-200">{sourceTable || "Record"} · {lane}</div>
              {#each dataFieldElements.slice(0, 3) as fld (fld.id)}
                <div class="flex justify-between text-[11px] text-slate-500">
                  <span>{fld.caption || fld.label}:</span>
                  <span class="font-mono text-slate-700 dark:text-slate-300">{previewValues[fld.id] || "—"}</span>
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {:else if previewMode && layoutType === "Timeline"}
      <!-- Clean Live Preview: Timeline Layout -->
      <div class="space-y-4 pl-4 border-l-2 border-indigo-500/40">
        {#each ["Created", "Validated", "Posted"] as ev, idx}
          <div class="relative pl-4 space-y-1">
            <span class="absolute -left-[21px] top-1 size-3 rounded-full bg-indigo-600 ring-4 ring-white dark:ring-slate-900"></span>
            <div class="text-xs font-bold text-slate-800 dark:text-slate-200">{ev} · {sourceTable || "Document"}</div>
            <div class="text-[11px] text-slate-500">Step {idx + 1} recorded with {dataFieldElements.length} tracked fields.</div>
          </div>
        {/each}
      </div>
    {:else if previewMode && layoutType === "Wizard"}
      <!-- Clean Live Preview: Wizard Steps Layout -->
      <div class="space-y-4">
        <div class="flex items-center gap-2 border-b border-slate-200 dark:border-slate-800 pb-3">
          {#each (stepElements.length > 0 ? stepElements : [{ id: "s1", caption: "Step 1: Details" }, { id: "s2", caption: "Step 2: Confirm" }]) as st, sIdx}
            <button
              type="button"
              onclick={() => (activeWizardStep = sIdx)}
              class="px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer {activeWizardStep === sIdx ? 'bg-indigo-600 text-white' : 'bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400'}"
            >
              {sIdx + 1}. {st.caption || st.label}
            </button>
          {/each}
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          {#each dataFieldElements as el (el.id)}
            <div class="space-y-1">
              <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 block">{el.caption || el.label}</span>
              <Input
                value={previewValues[el.id] ?? ""}
                oninput={(e) => (previewValues[el.id] = e.currentTarget.value)}
                placeholder={el.sourceField || el.label}
              />
            </div>
          {/each}
        </div>
      </div>
    {:else if layoutType === "Dialog" || pageCaption === "Sign In" || (sourceTable === "User" && elements.some((e) => e.name === "password"))}
      <!-- UNIFIED TRUE WYSIWYG DIALOG / PORTAL LAYOUT (Design & Preview share the EXACT same visual structure) -->
      {@const headerGrp = elements.find((e) => e.type === "fasttab" && (e.name === "Branding" || e.name === "Header"))}
      {@const titleEl = elements.find((e) => e.name === "applicationTitle") || headerGrp}
      {@const subtitleEl = elements.find((e) => e.name === "welcomeMessage")}
      {@const credGrp = elements.find((e) => e.type === "fasttab" && (e.name === "Credentials" || e.name === "General" || e.name === "Card"))}
      {@const userEl = elements.find((e) => e.name === "username")}
      {@const passEl = elements.find((e) => e.name === "password" || e.type === "password")}
      {@const rememberEl = elements.find((e) => e.name === "rememberMe" || e.type === "toggle")}
      {@const actEl = actionElements[0]}
      {@const compGrp = elements.find((e) => e.type === "fasttab" && (e.name === "Compliance" || e.name === "Footer" || e.name === "Security"))}
      {@const discEl = elements.find((e) => e.name === "customDisclaimer") || compGrp}
      {@const supEl = elements.find((e) => e.name === "supportEmail")}

      <div class="py-8 flex flex-col items-center justify-center select-none w-full bg-slate-50/60 dark:bg-slate-950/40 rounded-2xl p-4">
        <div class="w-full max-w-[380px] space-y-6">
          <!-- 1. Brand & Node Header Zone -->
          <div
            onclick={(e) => headerGrp && handleSelect(headerGrp, e)}
            role="presentation"
            class="text-center space-y-2 p-2 rounded-xl transition-all cursor-pointer {previewMode
              ? ''
              : selectedElement && (selectedElement.id === headerGrp?.id || selectedElement.id === titleEl?.id)
                ? 'ring-2 ring-indigo-500 bg-indigo-50/20 dark:bg-indigo-950/20'
                : 'hover:ring-1 hover:ring-slate-300 dark:hover:ring-slate-700'}"
          >
            <div class="size-12 rounded-2xl bg-indigo-600 text-white flex items-center justify-center mx-auto shadow-md transition-all">
              <Layers class="size-6 stroke-[2]" />
            </div>
            <h1 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
              {headerGrp?.caption || titleEl?.caption || "BOS Enterprise Platform"}
            </h1>
            {#if subtitleEl}
              <p class="text-xs text-slate-400">
                {subtitleEl.caption || subtitleEl.label || "Welcome Subtitle"}
              </p>
            {/if}
            <div class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-[11px] text-slate-600 dark:text-slate-300 font-medium">
              <span class="size-2 rounded-full bg-emerald-500 animate-pulse"></span>
              <span>BOS Enterprise Primary Node (allinone)</span>
            </div>
          </div>

          <!-- 2. Elevated Login Card -->
          <div
            onclick={(e) => credGrp && handleSelect(credGrp, e)}
            role="presentation"
            class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-6 shadow-xl space-y-4 text-xs transition-all {previewMode
              ? ''
              : selectedElement && selectedElement.id === credGrp?.id
                ? 'ring-2 ring-indigo-500'
                : ''}"
          >
            {#if previewMode && previewSuccess}
              <div class="p-3 rounded-xl bg-emerald-50 dark:bg-emerald-950/40 border border-emerald-200 dark:border-emerald-900/60 text-emerald-800 dark:text-emerald-200 text-xs flex items-center gap-2">
                <CheckCircle2 class="size-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
                <span>Authentication successful! Active session verified.</span>
              </div>
            {/if}

            <form
              onsubmit={(e) => {
                e.preventDefault();
                if (!previewMode) return;
                previewBusy = true;
                setTimeout(() => {
                  previewBusy = false;
                  previewSuccess = true;
                  setTimeout(() => (previewSuccess = false), 4000);
                }, 600);
              }}
              class="space-y-4"
            >
              <!-- User ID Input -->
              {#if userEl}
                <div
                  onclick={(e) => handleSelect(userEl, e)}
                  role="presentation"
                  class="space-y-1.5 p-1 rounded-lg transition-all {previewMode
                    ? ''
                    : selectedElement && selectedElement.id === userEl.id
                      ? 'ring-2 ring-indigo-500 bg-indigo-50/30 dark:bg-indigo-950/30'
                      : 'hover:ring-1 hover:ring-slate-300 dark:hover:ring-slate-700'}"
                >
                  <div class="flex items-center justify-between">
                    <label for="dialog-user-id" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                      {userEl.caption || userEl.label || "User ID"}
                    </label>
                    {#if !previewMode}
                      <span class="text-[10px] text-slate-400 font-mono">Bound: {userEl.sourceField || userEl.name}</span>
                    {/if}
                  </div>
                  <div class="relative">
                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
                      <User class="size-4" />
                    </div>
                    <input
                      id="dialog-user-id"
                      type="text"
                      disabled={!previewMode}
                      value={previewMode ? (previewValues["username"] ?? "admin") : `Bound: ${userEl.sourceField || userEl.name}`}
                      oninput={(e) => (previewValues["username"] = e.currentTarget.value)}
                      placeholder="e.g. admin or john.doe"
                      class="w-full h-10 pl-9 pr-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-hidden disabled:opacity-90 disabled:cursor-pointer"
                    />
                  </div>
                </div>
              {/if}

              <!-- Password Input -->
              {#if passEl}
                <div
                  onclick={(e) => handleSelect(passEl, e)}
                  role="presentation"
                  class="space-y-1.5 p-1 rounded-lg transition-all {previewMode
                    ? ''
                    : selectedElement && selectedElement.id === passEl.id
                      ? 'ring-2 ring-indigo-500 bg-indigo-50/30 dark:bg-indigo-950/30'
                      : 'hover:ring-1 hover:ring-slate-300 dark:hover:ring-slate-700'}"
                >
                  <div class="flex items-center justify-between">
                    <label for="dialog-password" class="block text-xs font-semibold text-slate-700 dark:text-slate-300">
                      {passEl.caption || passEl.label || "Password"}
                    </label>
                    {#if !previewMode}
                      <span class="text-[10px] text-slate-400 font-mono">style: password</span>
                    {/if}
                  </div>
                  <div class="relative">
                    <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none text-slate-400">
                      <Lock class="size-4" />
                    </div>
                    <input
                      id="dialog-password"
                      type={showPreviewPassword ? "text" : "password"}
                      disabled={!previewMode}
                      value={previewMode ? (previewValues["password"] ?? "••••••••") : "••••••••"}
                      oninput={(e) => (previewValues["password"] = e.currentTarget.value)}
                      placeholder="••••••••"
                      class="w-full h-10 pl-9 pr-10 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/50 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-hidden disabled:opacity-90 disabled:cursor-pointer"
                    />
                    <button
                      type="button"
                      onclick={() => (showPreviewPassword = !showPreviewPassword)}
                      class="absolute inset-y-0 right-0 pr-3 flex items-center text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer"
                    >
                      {#if showPreviewPassword}
                        <EyeOff class="size-4" />
                      {:else}
                        <Eye class="size-4" />
                      {/if}
                    </button>
                  </div>
                </div>
              {/if}

              <!-- Remember User ID Checkbox -->
              {#if rememberEl}
                <div
                  onclick={(e) => handleSelect(rememberEl, e)}
                  role="presentation"
                  class="flex items-center justify-between pt-0.5 p-1 rounded-lg transition-all {previewMode
                    ? ''
                    : selectedElement && selectedElement.id === rememberEl.id
                      ? 'ring-2 ring-indigo-500 bg-indigo-50/30 dark:bg-indigo-950/30'
                      : 'hover:ring-1 hover:ring-slate-300 dark:hover:ring-slate-700'}"
                >
                  <div class="flex items-center gap-2 select-none">
                    <input
                      id="dialog-remember-me-toggle"
                      type="checkbox"
                      checked={previewRememberMe}
                      onchange={(e) => (previewRememberMe = e.currentTarget.checked)}
                      class="size-3.5 rounded border-slate-300 dark:border-slate-700 text-indigo-600 focus:ring-indigo-500 dark:bg-slate-950"
                    />
                    <label for="dialog-remember-me-toggle" class="text-xs text-slate-600 dark:text-slate-400 cursor-pointer">
                      {rememberEl.caption || rememberEl.label || "Remember User ID"}
                    </label>
                  </div>
                  {#if !previewMode}
                    <span class="text-[10px] text-slate-400 font-mono">style: switch</span>
                  {/if}
                </div>
              {/if}

              <!-- Sign In Action Button -->
              {#if actEl}
                <div
                  onclick={(e) => handleSelect(actEl, e)}
                  role="presentation"
                  class="pt-1 rounded-lg transition-all {previewMode
                    ? ''
                    : selectedElement && selectedElement.id === actEl.id
                      ? 'ring-2 ring-indigo-500 p-0.5'
                      : ''}"
                >
                  <button
                    type={previewMode ? "submit" : "button"}
                    disabled={previewMode && previewBusy}
                    class="w-full h-10 rounded-lg bg-indigo-600 hover:bg-indigo-700 active:bg-indigo-800 text-white font-medium text-xs flex items-center justify-center gap-2 shadow-sm cursor-pointer transition-all active:scale-[0.99] disabled:opacity-50"
                  >
                    {#if previewBusy}
                      <Loader2 class="size-4 animate-spin" />
                      <span>Verifying Credentials...</span>
                    {:else}
                      <span>{actEl.caption || actEl.label || "Sign In"}</span>
                      <ArrowRight class="size-4" />
                    {/if}
                  </button>
                </div>
              {/if}
            </form>
          </div>

          <!-- 3. Compliance & Security Footer Zone -->
          <div
            onclick={(e) => compGrp && handleSelect(compGrp, e)}
            role="presentation"
            class="text-center text-[10px] text-slate-400 dark:text-slate-500 space-y-1.5 leading-relaxed p-2 rounded-xl transition-all cursor-pointer {previewMode
              ? ''
              : selectedElement && (selectedElement.id === compGrp?.id || selectedElement.id === discEl?.id)
                ? 'ring-2 ring-indigo-500 bg-indigo-50/20 dark:bg-indigo-950/20'
                : 'hover:ring-1 hover:ring-slate-300 dark:hover:ring-slate-700'}"
          >
            <p class="max-w-xs mx-auto">
              {discEl?.caption || "Authorized personnel only. All access attempts are monitored and audited."}
            </p>
            <p>
              Need support? <span class="underline text-indigo-500 cursor-pointer">{supEl?.caption || "admin@bos.local"}</span>
            </p>
            <p class="font-mono text-slate-500 dark:text-slate-600">
              3-Tier Architecture · Controlled by bos.auth Codeunit
            </p>
          </div>
        </div>
      </div>
    {:else}
      <!-- Main + Side FactBox Responsive Grid (Record, Document, Workspace, Split, or Design Mode) -->
      <div class="{layoutType === 'Dialog' || pageCaption === 'Sign In' ? 'max-w-md mx-auto space-y-4' : 'grid grid-cols-1 ' + (sideElements.length > 0 && previewMode ? 'lg:grid-cols-4' : '') + ' gap-6'}">
        {#if previewMode && layoutType === "Split"}
          <div class="lg:col-span-1 border-r border-slate-200 dark:border-slate-800 pr-4 space-y-2">
            <div class="text-xs font-bold uppercase tracking-wider text-slate-400">Master List</div>
            {#each [1, 2, 3] as itemIdx}
              <div class="p-2.5 rounded-lg border {itemIdx === 1 ? 'border-indigo-500 bg-indigo-50/30 dark:bg-indigo-950/30' : 'border-slate-200 dark:border-slate-800'} text-xs">
                <div class="font-semibold">{sourceTable || "Record"} #{itemIdx}</div>
                <div class="text-[10px] text-slate-400">Active</div>
              </div>
            {/each}
          </div>
        {/if}

        <div class="{sideElements.length > 0 && previewMode ? 'lg:col-span-3' : ''} space-y-4">
          {#if previewMode && tileElements.length > 0}
            <div class="grid grid-cols-2 sm:grid-cols-3 gap-3">
              {#each tileElements as tile (tile.id)}
                <div class="p-3.5 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-950/50 space-y-1">
                  <span class="text-[11px] text-slate-500 font-medium">{tile.caption || tile.label}</span>
                  <div class="text-xl font-extrabold text-indigo-600 dark:text-indigo-400">12</div>
                </div>
              {/each}
            </div>
          {/if}

          {#each elements as el (el.id)}
            {#if !previewMode || (el.type !== "action" && el.type !== "factbox" && el.type !== "tile")}
              <div
                onclick={(e) => handleSelect(el, e)}
                role="presentation"
                class="relative rounded-lg transition-all {previewMode
                  ? ''
                  : selectedElement && selectedElement.id === el.id
                    ? 'border-2 border-indigo-600 bg-indigo-50/20 dark:bg-indigo-950/20 ring-2 ring-indigo-500/20'
                    : 'border-2 border-transparent hover:border-slate-300 dark:hover:border-slate-700'}"
              >
                <!-- Element Type: FastTab Group -->
                {#if el.type === "fasttab"}
                  <div class="p-4 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 space-y-3">
                    <div class="flex items-center justify-between font-bold text-xs text-slate-800 dark:text-slate-200 pb-2 border-b border-slate-200 dark:border-slate-800">
                      <span class="flex items-center gap-1.5">
                        <Layers class="size-3.5 text-indigo-500" />
                        <span>{el.caption || el.label}</span>
                      </span>
                      <ChevronDown class="size-3 text-slate-400" />
                    </div>
                    {#if !previewMode}
                      <div class="grid grid-cols-2 gap-3 text-xs">
                        <div class="p-2.5 rounded border border-dashed border-slate-200 dark:border-slate-800 text-slate-400 text-center text-[11px]">
                          Section Column 1
                        </div>
                        <div class="p-2.5 rounded border border-dashed border-slate-200 dark:border-slate-800 text-slate-400 text-center text-[11px]">
                          Section Column 2
                        </div>
                      </div>
                    {/if}
                  </div>

                <!-- Element Type: 2-Column Grid Layout -->
                {:else if el.type === "grid"}
                  <div class="p-3.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-2">
                    <div class="flex items-center gap-1.5 text-xs font-semibold text-slate-700 dark:text-slate-300">
                      <LayoutGrid class="size-3.5 text-indigo-500" />
                      <span>{el.caption || el.label} (2-Column Matrix)</span>
                    </div>
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                      <Input disabled={!previewMode} placeholder="Column A" />
                      <Input disabled={!previewMode} placeholder="Column B" />
                    </div>
                  </div>

                <!-- Element Type: Subform (Child Lines Grid) -->
                {:else if el.type === "subform"}
                  <div class="p-3.5 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-2">
                    <div class="flex items-center justify-between">
                      <span class="text-xs font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                        <Layers class="size-3.5 text-indigo-500" />
                        <span>{el.caption || el.label} (Lines Part: {el.sourceField || "SalesLine"})</span>
                      </span>
                      <Badge>part</Badge>
                    </div>
                    <div class="rounded border border-slate-200 dark:border-slate-800 overflow-hidden">
                      <table class="w-full text-[11px] text-left">
                        <thead class="bg-slate-50 dark:bg-slate-800 text-slate-500">
                          <tr>
                            <th class="px-2.5 py-1.5">Item / Code</th>
                            <th class="px-2.5 py-1.5">Description</th>
                            <th class="px-2.5 py-1.5 text-right">Quantity</th>
                            <th class="px-2.5 py-1.5 text-right">Amount</th>
                          </tr>
                        </thead>
                        <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
                          <tr>
                            <td class="px-2.5 py-1.5 font-mono">ITEM-100</td>
                            <td class="px-2.5 py-1.5">Line Item</td>
                            <td class="px-2.5 py-1.5 text-right font-mono">1.00</td>
                            <td class="px-2.5 py-1.5 text-right font-mono">100.00</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>

                <!-- Element Type: Text Box -->
                {:else if el.type === "text"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate">
                      {el.caption || el.label}:
                    </span>
                    <Input
                      class="flex-1"
                      disabled={!previewMode || el.editable === false}
                      value={previewValues[el.id] ?? ""}
                      oninput={(e) => (previewValues[el.id] = e.currentTarget.value)}
                      placeholder={previewMode ? `Enter ${el.caption || el.label}` : `Bound: ${el.sourceField || "unbound"}`}
                    />
                  </div>

                <!-- Element Type: Password -->
                {:else if el.type === "password"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate flex items-center gap-1.5">
                      <Lock class="size-3 text-slate-400 shrink-0" />
                      <span>{el.caption || el.label}:</span>
                    </span>
                    <Input
                      class="flex-1"
                      type="password"
                      disabled={!previewMode || el.editable === false}
                      value={previewValues[el.id] ?? ""}
                      oninput={(e) => (previewValues[el.id] = e.currentTarget.value)}
                      placeholder={previewMode ? "••••••••" : `Bound: ${el.sourceField || "unbound"}`}
                    />
                  </div>

                <!-- Element Type: Decimal -->
                {:else if el.type === "decimal"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate">
                      {el.caption || el.label}:
                    </span>
                    <Input
                      class="flex-1"
                      type="currency"
                      disabled={!previewMode || el.editable === false}
                      value={previewValues[el.id] ?? ""}
                      oninput={(e) => (previewValues[el.id] = e.currentTarget.value)}
                      placeholder="0.00"
                    />
                  </div>

                <!-- Element Type: Date -->
                {:else if el.type === "date"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate">
                      {el.caption || el.label}:
                    </span>
                    <DatePicker
                      class="flex-1"
                      disabled={!previewMode || el.editable === false}
                      value={previewValues[el.id] ?? ""}
                      onValueChange={(val) => (previewValues[el.id] = val)}
                    />
                  </div>

                <!-- Element Type: Lookup Dropdown -->
                {:else if el.type === "lookup"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate flex items-center gap-1">
                      <Search class="size-3 text-indigo-500 shrink-0" />
                      <span>{el.caption || el.label}:</span>
                    </span>
                    <div class="flex-1">
                      <Select
                        disabled={!previewMode || el.editable === false}
                        value={previewValues[el.id] ?? ""}
                        onValueChange={(val) => (previewValues[el.id] = val)}
                        placeholder={`Lookup ${el.sourceField || el.label}...`}
                        options={[
                          { value: "REC-001", label: "REC-001 · Primary Reference" },
                          { value: "REC-002", label: "REC-002 · Secondary Reference" },
                        ]}
                      />
                    </div>
                  </div>

                <!-- Element Type: Boolean Toggle -->
                {:else if el.type === "toggle"}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between gap-4">
                    <span class="text-xs font-semibold text-slate-700 dark:text-slate-300 w-36 truncate">
                      {el.caption || el.label}:
                    </span>
                    <div class="flex-1 flex items-center">
                      <Switch
                        disabled={!previewMode || el.editable === false}
                        checked={Boolean(previewValues[el.id])}
                        onCheckedChange={(val) => (previewValues[el.id] = val)}
                        label={previewValues[el.id] ? "Yes" : "No"}
                      />
                    </div>
                  </div>

                <!-- Element Type: KPI Cue Tile -->
                {:else if el.type === "tile"}
                  <div class="p-3.5 bg-indigo-50/40 dark:bg-indigo-950/30 rounded-lg border border-indigo-200 dark:border-indigo-800/70 flex items-center justify-between">
                    <div class="flex items-center gap-2">
                      <BarChart3 class="size-4 text-indigo-500" />
                      <div>
                        <div class="text-xs font-bold text-slate-800 dark:text-slate-200">{el.caption || el.label}</div>
                        <div class="text-[10px] text-slate-400 font-mono">source: {el.sourceField || "query.OpenOrders"}</div>
                      </div>
                    </div>
                    <span class="text-lg font-extrabold text-indigo-600 dark:text-indigo-400">12</span>
                  </div>

                <!-- Element Type: Wizard Step -->
                {:else if el.type === "step"}
                  <div class="p-3.5 bg-slate-50 dark:bg-slate-900/60 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between">
                    <span class="text-xs font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                      <ListOrdered class="size-3.5 text-indigo-500" />
                      <span>Step: {el.caption || el.label}</span>
                    </span>
                    <Badge>step</Badge>
                  </div>

                <!-- Element Type: FactBox Side Part -->
                {:else if el.type === "factbox"}
                  <div class="p-3.5 bg-slate-50 dark:bg-slate-900/60 rounded-lg border border-slate-200 dark:border-slate-800 flex items-center justify-between">
                    <span class="text-xs font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                      <PanelRight class="size-3.5 text-indigo-500" />
                      <span>FactBox Side Part: {el.caption || el.label}</span>
                    </span>
                    <Badge tone="info">side</Badge>
                  </div>

                <!-- Element Type: Action Button -->
                {:else if el.type === "action"}
                  <div class="p-2 flex justify-end">
                    <Button size="sm" icon={Play}>{el.caption || el.label}</Button>
                  </div>

                <!-- Default / Fallback -->
                {:else}
                  <div class="p-3 bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 text-xs">
                    {el.caption || el.label} ({el.type})
                  </div>
                {/if}

                <!-- Selection Delete Handle (when selected in design mode) -->
                {#if !previewMode && selectedElement && selectedElement.id === el.id}
                  <Button
                    variant="destructive"
                    size="xs"
                    iconOnly
                    icon={Trash2}
                    aria-label="Delete control"
                    class="absolute -top-2.5 -right-2.5 z-10"
                    onclick={(event) => handleDelete(el.id, event)}
                  />
                {/if}
              </div>
            {/if}
          {/each}
        </div>

        {#if previewMode && sideElements.length > 0}
          <aside class="lg:col-span-1 border-l border-slate-200 dark:border-slate-800 pl-4 space-y-3">
            <div class="text-xs font-bold uppercase tracking-wider text-slate-400">FactBox Side Panel</div>
            {#each sideElements as fb (fb.id)}
              <div class="p-3 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-950/60 space-y-1.5 text-xs">
                <div class="font-bold text-slate-800 dark:text-slate-200">{fb.caption || fb.label}</div>
                <div class="text-[11px] text-slate-500">Related summary facts linked to current {sourceTable || "record"}.</div>
              </div>
            {/each}
          </aside>
        {/if}
      </div>
    {/if}
  </div>
</div>
