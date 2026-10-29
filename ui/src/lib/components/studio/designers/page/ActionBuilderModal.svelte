<script>
  /**
   * ActionBuilderModal.svelte
   * Enterprise Page Action & Menu Configurator for BOS Page Designer.
   * Gives developers full control to define, organize, and configure page actions:
   * - Hierarchical Area assignment (Processing, Navigation, Reporting, Creation, Promoted)
   * - Dropdown Menu Group assignment (e.g. Posting, Approval)
   * - Execution target (Run Codeunit, Open Page with filters, or custom OnAction script)
   * - Enterprise behaviors (Promoted ribbon, primary styling, confirmation prompts, shortcuts)
   */
  import { Play, Link2, FileText, PlusCircle, Sparkles, FolderPlus, Zap, Check, AlertCircle } from "lucide-svelte";
  import { Dialog, Button, Input, Banner } from "$lib/bos-ui/index.js";
  import IconPicker from "./IconPicker.svelte";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";

  let {
    open = $bindable(false),
    model,
    defaultArea = "Processing",
    defaultGroup = ""
  } = $props();

  let mode = $state("action"); // "action" | "group"
  let actionName = $state("");
  let caption = $state("");
  let area = $state("Processing");
  let group = $state("");
  let isNewGroup = $state(false);
  let newGroupName = $state("");
  let newGroupCaption = $state("");
  let executionType = $state("codeunit"); // "codeunit" | "page" | "script" | "standard"
  let runCodeunit = $state("");
  let openPage = $state("");
  let runPageMode = $state("view"); // "view" | "edit" | "create"
  let icon = $state("");
  let isPrimary = $state(false);
  let isPromoted = $state(true);
  let confirmMessage = $state("");
  let shortcut = $state("");
  let tooltip = $state("");
  let errorMessage = $state("");

  const AREAS = [
    { id: "Processing", label: "Processing (Actions)", icon: Play, desc: "Operational tasks, posting, releasing" },
    { id: "Navigation", label: "Navigation (Related)", icon: Link2, desc: "Related records, ledger entries, history" },
    { id: "Reporting", label: "Reporting (Reports)", icon: FileText, desc: "Statements, printouts, PDF/Excel export" },
    { id: "Creation", label: "Creation (New Doc)", icon: PlusCircle, desc: "Create related document or card" },
    { id: "Promoted", label: "Promoted (Top Ribbon)", icon: Sparkles, desc: "Direct prominent toolbar action" }
  ];

  // Existing groups in the page outline
  const existingGroups = $derived.by(() => {
    const list = [];
    const walk = (nodes) => {
      for (const n of nodes ?? []) {
        if (n.kind === "group") list.push({ name: n.name, caption: n.props?.find((p) => p.name === "caption")?.value?.replace(/^"|"$/g, "") ?? n.name });
        if (n.children) walk(n.children);
      }
    };
    walk(model?.actionNodes ?? []);
    return list;
  });

  const availablePages = $derived(
    Object.keys(pageManifest.manifest?.pages ?? {}).sort()
  );

  $effect(() => {
    if (open) {
      area = defaultArea || "Processing";
      group = defaultGroup || "";
      isNewGroup = false;
      errorMessage = "";
    }
  });

  function validateIdentifier(id) {
    if (!id.trim()) return "Action Name / Identifier is required.";
    if (!/^[a-zA-Z][a-zA-Z0-9_]*$/.test(id.trim())) {
      return "Identifier must be alphanumeric without spaces (e.g. PostInvoice, ReleaseOrder).";
    }
    return "";
  }

  async function handleCreateAction() {
    errorMessage = "";
    const nameErr = validateIdentifier(actionName);
    if (nameErr) {
      errorMessage = nameErr;
      return;
    }

    const finalGroup = isNewGroup ? newGroupName.trim() : group;

    const actionData = {
      name: actionName.trim(),
      caption: caption.trim() || actionName.trim(),
      area,
      group: finalGroup || null,
      groupCaption: isNewGroup && newGroupCaption.trim() ? newGroupCaption.trim() : null,
      icon: icon || null,
      primary: isPrimary,
      promoted: isPromoted,
      confirm: confirmMessage.trim() || null,
      shortcut: shortcut.trim() || null,
      tooltip: tooltip.trim() || null,
      runCodeunit: executionType === "codeunit" && runCodeunit.trim() ? runCodeunit.trim() : null,
      openPage: executionType === "page" && openPage.trim() ? openPage.trim() : null,
      runPageMode: executionType === "page" ? runPageMode : null,
      script: executionType === "script" ? 'Message("Action executed.");' : null
    };

    try {
      await model.addAction(actionData);
      open = false;
      resetForm();
    } catch (e) {
      errorMessage = e.message ?? String(e);
    }
  }

  async function handleCreateGroup() {
    errorMessage = "";
    const nameErr = validateIdentifier(newGroupName);
    if (nameErr) {
      errorMessage = nameErr;
      return;
    }

    try {
      await model.addActionGroup({
        name: newGroupName.trim(),
        caption: newGroupCaption.trim() || newGroupName.trim(),
        area
      });
      open = false;
      resetForm();
    } catch (e) {
      errorMessage = e.message ?? String(e);
    }
  }

  function resetForm() {
    actionName = "";
    caption = "";
    newGroupName = "";
    newGroupCaption = "";
    isNewGroup = false;
    runCodeunit = "";
    openPage = "";
    icon = "";
    confirmMessage = "";
    shortcut = "";
    tooltip = "";
    errorMessage = "";
  }
</script>

<Dialog
  bind:open
  title={mode === "action" ? "Add Page Action / Menu Item" : "Create Action Menu Group"}
  description="Configure enterprise page actions, menus, and execution targets."
  size="lg"
>
  {#snippet children()}
    <div class="space-y-4 text-xs">
      <!-- Mode Toggle -->
      <div class="flex items-center gap-2 border-b border-slate-200 pb-3 dark:border-slate-800">
        <button
          type="button"
          class={"flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors " +
            (mode === "action"
              ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 ring-1 ring-indigo-500"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800")}
          onclick={() => { mode = "action"; errorMessage = ""; }}
        >
          <Zap class="size-3.5" />
          <span>New Action Button</span>
        </button>
        <button
          type="button"
          class={"flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-semibold transition-colors " +
            (mode === "group"
              ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/80 dark:text-indigo-300 ring-1 ring-indigo-500"
              : "text-slate-600 hover:bg-slate-100 dark:text-slate-400 dark:hover:bg-slate-800")}
          onclick={() => { mode = "group"; errorMessage = ""; }}
        >
          <FolderPlus class="size-3.5" />
          <span>New Dropdown Menu Group</span>
        </button>
      </div>

      {#if errorMessage}
        <Banner variant="danger" title="Validation Error">
          {errorMessage}
        </Banner>
      {/if}

      {#if mode === "action"}
        <!-- 1. Identity & Naming -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Action Identifier (ID) <span class="text-rose-500">*</span>
            </label>
            <input
              type="text"
              class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              placeholder="PostInvoice, ReleaseOrder"
              bind:value={actionName}
            />
          </div>
          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Display Caption
            </label>
            <input
              type="text"
              class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              placeholder='e.g. "Post Invoice"'
              bind:value={caption}
            />
          </div>
        </div>

        <!-- 2. Menu Area Selector -->
        <div>
          <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
            Functional Menu Area
          </label>
          <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
            {#each AREAS as a}
              {@const Icon = a.icon}
              <button
                type="button"
                class={"flex flex-col items-start gap-1 rounded-md border p-2 text-left transition-colors " +
                  (area === a.id
                    ? "border-indigo-600 bg-indigo-50/70 text-indigo-900 dark:border-indigo-500 dark:bg-indigo-950/50 dark:text-indigo-200 ring-1 ring-indigo-500"
                    : "border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")}
                onclick={() => (area = a.id)}
              >
                <div class="flex items-center gap-1.5 font-semibold text-xs">
                  <Icon class="size-3.5 text-indigo-500" />
                  <span>{a.id}</span>
                  {#if area === a.id}<Check class="size-3 ml-auto text-indigo-600" />{/if}
                </div>
                <span class="text-[10px] text-slate-500 dark:text-slate-400 line-clamp-1">{a.desc}</span>
              </button>
            {/each}
          </div>
        </div>

        <!-- 3. Menu Grouping (Dropdown Container) -->
        <div class="rounded-md border border-slate-200 bg-slate-50/50 p-2.5 dark:border-slate-800 dark:bg-slate-900/50 space-y-2">
          <div class="flex items-center justify-between">
            <span class="font-semibold text-slate-700 dark:text-slate-300 text-[11px]">Dropdown Menu Group</span>
            <label class="inline-flex items-center gap-1.5 text-[11px] text-slate-600 dark:text-slate-400 cursor-pointer">
              <input type="checkbox" bind:checked={isNewGroup} class="rounded border-slate-300 text-indigo-600" />
              <span>Create new group</span>
            </label>
          </div>

          {#if isNewGroup}
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2 pt-1">
              <div>
                <label class="block text-[10px] text-slate-500 mb-0.5">Group ID</label>
                <input
                  type="text"
                  placeholder="PostingGroup"
                  class="w-full rounded border border-slate-300 bg-white px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-800"
                  bind:value={newGroupName}
                />
              </div>
              <div>
                <label class="block text-[10px] text-slate-500 mb-0.5">Group Caption</label>
                <input
                  type="text"
                  placeholder='e.g. "Posting"'
                  class="w-full rounded border border-slate-300 bg-white px-2 py-1 text-xs dark:border-slate-700 dark:bg-slate-800"
                  bind:value={newGroupCaption}
                />
              </div>
            </div>
          {:else}
            <select
              class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
              bind:value={group}
            >
              <option value="">(None - Top-level in {area} menu)</option>
              {#each existingGroups as g}
                <option value={g.name}>{g.caption} ({g.name})</option>
              {/each}
            </select>
          {/if}
        </div>

        <!-- 4. Execution Target -->
        <div class="space-y-2">
          <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300">
            Action Target / Execution
          </label>
          <div class="grid grid-cols-4 gap-1.5 p-1 bg-slate-100 rounded-md dark:bg-slate-800">
            {#each [["codeunit", "Run Codeunit"], ["page", "Open Page"], ["script", "Custom Script"], ["standard", "Standard"]] as [val, lbl]}
              <button
                type="button"
                class={"py-1 text-center rounded text-[11px] font-medium transition-colors " +
                  (executionType === val
                    ? "bg-white text-indigo-600 shadow-xs dark:bg-slate-900 dark:text-indigo-300 font-semibold"
                    : "text-slate-600 dark:text-slate-400 hover:text-slate-800")}
                onclick={() => (executionType = val)}
              >
                {lbl}
              </button>
            {/each}
          </div>

          {#if executionType === "codeunit"}
            <div>
              <label class="block text-[10px] text-slate-500 mb-0.5">Target Codeunit Identifier</label>
              <input
                type="text"
                placeholder="SalesPostHandler, ApproveOrderWorkflow"
                class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
                bind:value={runCodeunit}
              />
            </div>
          {:else if executionType === "page"}
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
              <div>
                <label class="block text-[10px] text-slate-500 mb-0.5">Target Page</label>
                <select
                  class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
                  bind:value={openPage}
                >
                  <option value="">(Select target page)</option>
                  {#each availablePages as p}
                    <option value={p}>{p}</option>
                  {/each}
                </select>
              </div>
              <div>
                <label class="block text-[10px] text-slate-500 mb-0.5">Page Mode</label>
                <select
                  class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
                  bind:value={runPageMode}
                >
                  <option value="view">View</option>
                  <option value="edit">Edit</option>
                  <option value="create">Create</option>
                </select>
              </div>
            </div>
          {:else if executionType === "script"}
            <div class="rounded bg-indigo-50/50 p-2 text-[11px] text-indigo-700 dark:bg-indigo-950/40 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-900">
              A standard <code>trigger onAction()</code> scaffold will be inserted. You can jump directly to Monaco editor to author the business logic after creation.
            </div>
          {/if}
        </div>

        <!-- 5. Visuals & Enterprise Options -->
        <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-1">
          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Button Icon
            </label>
            <IconPicker value={icon} onChange={(i) => (icon = i)} />
          </div>
          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Keyboard Shortcut
            </label>
            <input
              type="text"
              placeholder="Ctrl+F9, F9, Ctrl+Shift+P"
              class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
              bind:value={shortcut}
            />
          </div>
        </div>

        <div class="space-y-2 pt-1 border-t border-slate-100 dark:border-slate-800">
          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-0.5">
              Confirmation Message
            </label>
            <input
              type="text"
              placeholder='e.g. "Are you sure you want to post this invoice?"'
              class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
              bind:value={confirmMessage}
            />
          </div>

          <div class="flex flex-wrap items-center gap-4 pt-1">
            <label class="inline-flex items-center gap-1.5 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
              <input type="checkbox" bind:checked={isPromoted} class="rounded border-slate-300 text-indigo-600" />
              <span class="font-medium">Promoted (Show on top ribbon)</span>
            </label>
            <label class="inline-flex items-center gap-1.5 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
              <input type="checkbox" bind:checked={isPrimary} class="rounded border-slate-300 text-indigo-600" />
              <span class="font-medium">Primary Action Style (Accent)</span>
            </label>
          </div>
        </div>

      {:else}
        <!-- New Menu Group Mode -->
        <div class="space-y-3">
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div>
              <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Group Identifier (ID) <span class="text-rose-500">*</span>
              </label>
              <input
                type="text"
                class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
                placeholder="PostingGroup, ApprovalMenu"
                bind:value={newGroupName}
              />
            </div>
            <div>
              <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1">
                Menu Caption
              </label>
              <input
                type="text"
                class="w-full rounded border border-slate-300 bg-white px-2.5 py-1.5 text-xs dark:border-slate-700 dark:bg-slate-800"
                placeholder='e.g. "Posting", "Approvals"'
                bind:value={newGroupCaption}
              />
            </div>
          </div>

          <div>
            <label class="block text-[11px] font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
              Host Menu Area
            </label>
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {#each AREAS.filter(a => a.id !== "Promoted") as a}
                {@const Icon = a.icon}
                <button
                  type="button"
                  class={"flex items-center gap-1.5 rounded-md border p-2 text-left transition-colors " +
                    (area === a.id
                      ? "border-indigo-600 bg-indigo-50 text-indigo-900 dark:border-indigo-500 dark:bg-indigo-950 dark:text-indigo-200 ring-1 ring-indigo-500"
                      : "border-slate-200 hover:bg-slate-50 dark:border-slate-700 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300")}
                  onclick={() => (area = a.id)}
                >
                  <Icon class="size-3.5 text-indigo-500 shrink-0" />
                  <span class="font-semibold text-xs truncate">{a.id}</span>
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}

      <!-- Footer Buttons -->
      <div class="flex items-center justify-end gap-2 pt-3 border-t border-slate-200 dark:border-slate-800">
        <Button variant="ghost" onclick={() => (open = false)}>
          Cancel
        </Button>
        <Button
          variant="primary"
          onclick={mode === "action" ? handleCreateAction : handleCreateGroup}
        >
          {mode === "action" ? "Add Action" : "Create Menu Group"}
        </Button>
      </div>
    </div>
  {/snippet}
</Dialog>
