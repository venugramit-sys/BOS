<script>
  /**
   * ArchitectureModal.svelte (Svelte 5 Runes)
   * Enterprise-grade Architecture Management & Creation Card.
   * Guided workflow: Module -> Feature (Optional) -> Meta-Object
   * Complete with real-time validation, color-coded preview cards, and PostgreSQL persistence.
   */
  import {
    Package,
    Sparkles,
    Boxes,
    Table,
    Cpu,
    Layout,
    Workflow,
    GitBranch,
    ListFilter,
    Layers,
    Check,
    ArrowRight,
    Plus,
    X,
    Info,
    Shield,
    FileText,
  } from "lucide-svelte";
  import { Dialog, Button, Input, Banner } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  // Active creation tab: "module" | "feature" | "object"
  let activeTab = $derived(studioStore.architectureModalTab);

  // Tab 1: Module State
  let modPackageId = $state("");
  let modName = $state("");
  let modPublisher = $state("");
  let modVersion = $state("1.0.0");
  let modCategory = $state("Core Platform");
  let modDescription = $state("");
  let modError = $state("");
  let modSuccess = $state("");

  // Tab 2: Feature State
  let featModuleId = $state("");
  let featId = $state("");
  let featName = $state("");
  let featStatus = $state("active");
  let featDescription = $state("");
  let featError = $state("");
  let featSuccess = $state("");

  // Tab 3: Meta-Object State
  let objModuleId = $state("");
  let objFeatureId = $state("");
  let objType = $state("table");
  let objName = $state("");
  let objCaption = $state("");
  let objSubType = $state("Normal");
  let objDescription = $state("");
  let objError = $state("");

  // Synchronize target module when modal opens
  $effect(() => {
    if (studioStore.isArchitectureModalOpen) {
      const defaultMod = studioStore.targetModuleId || (studioStore.packages[0]?.package_id) || "bos.core";
      if (!featModuleId) featModuleId = defaultMod;
      if (!objModuleId) objModuleId = defaultMod;
      if (studioStore.targetFeatureId) objFeatureId = studioStore.targetFeatureId;
    }
  });

  const DOMAIN_CATEGORIES = [
    "Core Platform",
    "Sales & CRM",
    "Financials & Accounting",
    "Supply Chain & Inventory",
    "Human Resources",
    "Manufacturing",
    "Custom Solutions",
  ];

  const OBJECT_TYPES = [
    {
      id: "table",
      name: "Table",
      desc: "Schema, keys, indexes, FlowFields",
      icon: Table,
      color: "text-blue-600 dark:text-blue-400",
      bg: "bg-blue-50 dark:bg-blue-950/40",
      border: "border-blue-200 dark:border-blue-800",
      badge: "border-blue-200 text-blue-700 bg-blue-50",
    },
    {
      id: "tablelogic",
      name: "TableLogic",
      desc: "Companion triggers, AMDP, calculations",
      icon: Cpu,
      color: "text-emerald-600 dark:text-emerald-400",
      bg: "bg-emerald-50 dark:bg-emerald-950/40",
      border: "border-emerald-200 dark:border-emerald-800",
      badge: "border-emerald-200 text-emerald-700 bg-emerald-50",
    },
    {
      id: "page",
      name: "Page",
      desc: "Card, List, Document, or Dialog UI",
      icon: Layout,
      color: "text-indigo-600 dark:text-indigo-400",
      bg: "bg-indigo-50 dark:bg-indigo-950/40",
      border: "border-indigo-200 dark:border-indigo-800",
      badge: "border-indigo-200 text-indigo-700 bg-indigo-50",
    },
    {
      id: "codeunit",
      name: "Codeunit",
      desc: "Domain services & event publishers",
      icon: Cpu,
      color: "text-amber-600 dark:text-amber-400",
      bg: "bg-amber-50 dark:bg-amber-950/40",
      border: "border-amber-200 dark:border-amber-800",
      badge: "border-amber-200 text-amber-700 bg-amber-50",
    },
    {
      id: "statemachine",
      name: "State Machine",
      desc: "Document status workflows & transitions",
      icon: Workflow,
      color: "text-rose-600 dark:text-rose-400",
      bg: "bg-rose-50 dark:bg-rose-950/40",
      border: "border-rose-200 dark:border-rose-800",
      badge: "border-rose-200 text-rose-700 bg-rose-50",
    },
    {
      id: "enum",
      name: "Enum",
      desc: "Strongly typed option sets",
      icon: GitBranch,
      color: "text-teal-600 dark:text-teal-400",
      bg: "bg-teal-50 dark:bg-teal-950/40",
      border: "border-teal-200 dark:border-teal-800",
      badge: "border-teal-200 text-teal-700 bg-teal-50",
    },
    {
      id: "query",
      name: "Query",
      desc: "High-performance joined SQL views",
      icon: ListFilter,
      color: "text-sky-600 dark:text-sky-400",
      bg: "bg-sky-50 dark:bg-sky-950/40",
      border: "border-sky-200 dark:border-sky-800",
      badge: "border-sky-200 text-sky-700 bg-sky-50",
    },
    {
      id: "pageextension",
      name: "Page Extension",
      desc: "Extend page fields, groups & actions",
      icon: Layers,
      color: "text-purple-600 dark:text-purple-400",
      bg: "bg-purple-50 dark:bg-purple-950/40",
      border: "border-purple-200 dark:border-purple-800",
      badge: "border-purple-200 text-purple-700 bg-purple-50",
    },
    {
      id: "tableextension",
      name: "Table Extension",
      desc: "Extend tables with fields & secondary keys",
      icon: Layers,
      color: "text-indigo-600 dark:text-indigo-400",
      bg: "bg-indigo-50 dark:bg-indigo-950/40",
      border: "border-indigo-200 dark:border-indigo-800",
      badge: "border-indigo-200 text-indigo-700 bg-indigo-50",
    },
    {
      id: "enumextension",
      name: "Enum Extension",
      desc: "Append values to extensible enums",
      icon: Layers,
      color: "text-teal-600 dark:text-teal-400",
      bg: "bg-teal-50 dark:bg-teal-950/40",
      border: "border-teal-200 dark:border-teal-800",
      badge: "border-teal-200 text-teal-700 bg-teal-50",
    },
  ];

  let isObjExtension = $derived(
    objType === "pageextension" ||
    objType === "tableextension" ||
    objType === "enumextension"
  );

  let objTargetType = $derived(
    objType === "pageextension" ? "page" :
    objType === "tableextension" ? "table" :
    objType === "enumextension" ? "enum" : null
  );

  let availableObjTargets = $derived(
    objTargetType
      ? (studioStore.catalog || []).filter((o) => o.object_type === objTargetType).map((o) => o.name || o.object_name)
      : []
  );

  $effect(() => {
    if (isObjExtension && availableObjTargets.length > 0 && !objTarget) {
      objTarget = availableObjTargets[0];
    }
  });

  let selectedTypeObj = $derived(OBJECT_TYPES.find((t) => t.id === objType) || OBJECT_TYPES[0]);
  let PreviewIcon = $derived(selectedTypeObj.icon);

  // List of features available for the currently selected module in Tab 3
  let availableFeaturesForObj = $derived.by(() => {
    const list = [];
    const seen = new Set();
    for (const o of studioStore.catalog) {
      if (o.package_name === objModuleId) {
        if (o.object_type === "feature" && !seen.has(o.name)) {
          seen.add(o.name);
          list.push({ id: o.name, name: o.name, caption: o.caption || o.name });
        } else if (o.feature && !seen.has(o.feature)) {
          seen.add(o.feature);
          list.push({ id: o.feature, name: o.feature, caption: o.feature });
        }
      }
    }
    return list;
  });

  // Available packages/modules registered in the catalog
  let registeredModules = $derived(
    studioStore.packages.length > 0
      ? studioStore.packages
      : [{ package_id: "bos.core", name: "BOS Unified Core Platform & Security Kernel", version: "1.0.0", publisher: "BOS Core Architecture Team" }]
  );

  function validateModuleId(id) {
    if (!id.trim()) return "Module ID is required.";
    if (!/^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$/.test(id.trim())) {
      return "Module ID must be dot-namespaced in lowercase (e.g. bos.sales, acme.crm).";
    }
    return "";
  }

  function validatePascalCase(name) {
    if (!name.trim()) return "Name is required.";
    if (!/^[A-Z][a-zA-Z0-9]*$/.test(name.trim())) {
      return "Name must be singular PascalCase (e.g. Customer, SalesHeader, LoginSecurity).";
    }
    if (/^[A-Z]{2,3}[A-Z]/.test(name.trim())) {
      return "Avoid acronym prefixes like SecUser. The module provides the namespace.";
    }
    return "";
  }

  // --- Handlers ---
  async function handleCreateModule(proceedNext = false) {
    modError = "";
    modSuccess = "";
    const err = validateModuleId(modPackageId);
    if (err) {
      modError = err;
      return;
    }
    if (!modName.trim()) {
      modName = modPackageId;
    }

    try {
      await studioStore.createModule({
        package_id: modPackageId.trim(),
        name: modName.trim(),
        version: modVersion.trim() || "1.0.0",
        publisher: modPublisher.trim() || "Workspace",
        status: "active",
        description: modDescription.trim(),
      });

      featModuleId = modPackageId.trim();
      objModuleId = modPackageId.trim();
      modSuccess = `Module '${modPackageId.trim()}' successfully recorded!`;

      if (proceedNext) {
        studioStore.architectureModalTab = "feature";
      }
    } catch (e) {
      modError = e.message || "Failed to create module";
    }
  }

  async function handleCreateFeature(proceedNext = false) {
    featError = "";
    featSuccess = "";
    const err = validatePascalCase(featId);
    if (err) {
      featError = err;
      return;
    }
    if (!featModuleId) {
      featError = "Please select a target module.";
      return;
    }

    try {
      await studioStore.createFeature({
        id: featId.trim(),
        name: featName.trim() || featId.trim(),
        moduleId: featModuleId,
        description: featDescription.trim(),
        status: featStatus,
      });

      objModuleId = featModuleId;
      objFeatureId = featId.trim();
      featSuccess = `Feature '${featId.trim()}' added to module '${featModuleId}'!`;

      if (proceedNext) {
        studioStore.architectureModalTab = "object";
      }
    } catch (e) {
      featError = e.message || "Failed to create feature";
    }
  }

  async function handleCreateMetaObject() {
    objError = "";
    const err = validatePascalCase(objName);
    if (err) {
      objError = err;
      return;
    }
    if (!objModuleId) {
      objError = "Please select a target module.";
      return;
    }

    try {
      await studioStore.createMetaObject({
        objectType: objType,
        name: objName.trim(),
        moduleId: objModuleId,
        featureId: objFeatureId || null,
        caption: objCaption.trim() || objName.trim(),
        subType: objSubType,
        description: objDescription.trim(),
      });

      studioStore.closeArchitectureModal();
    } catch (e) {
      objError = e.message || "Failed to create meta-object";
    }
  }
</script>

<Dialog
  bind:open={studioStore.isArchitectureModalOpen}
  title="BOS Architecture Designer"
  description="Record and register Modules, Features, and Meta-Objects in strict enterprise alignment."
  size="xl"
>
  {#snippet children()}
    <div class="space-y-4">
      <!-- Step Navigation Header -->
      <div class="grid grid-cols-3 gap-2 p-1 rounded-xl bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800">
        <!-- 1. Module Tab -->
        <button
          type="button"
          class={
            "flex items-center justify-center gap-2 py-2 px-3 rounded-lg text-xs font-semibold transition-all " +
            (activeTab === "module"
              ? "bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-xs border border-slate-200/80 dark:border-slate-700"
              : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200")
          }
          onclick={() => (studioStore.architectureModalTab = "module")}
        >
          <div class="size-5 rounded-full flex items-center justify-center bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 text-[10px] font-bold">
            1
          </div>
          <Package class="size-3.5" />
          <span>Module (Package)</span>
        </button>

        <!-- 2. Feature Tab (Optional) -->
        <button
          type="button"
          class={
            "flex items-center justify-center gap-2 py-2 px-3 rounded-lg text-xs font-semibold transition-all " +
            (activeTab === "feature"
              ? "bg-white dark:bg-slate-800 text-orange-600 dark:text-orange-400 shadow-xs border border-slate-200/80 dark:border-slate-700"
              : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200")
          }
          onclick={() => (studioStore.architectureModalTab = "feature")}
        >
          <div class="size-5 rounded-full flex items-center justify-center bg-orange-50 dark:bg-orange-950/60 text-orange-600 text-[10px] font-bold">
            2
          </div>
          <Sparkles class="size-3.5" />
          <span>Feature <span class="text-[9px] font-normal opacity-70">(Optional)</span></span>
        </button>

        <!-- 3. Meta-Object Tab -->
        <button
          type="button"
          class={
            "flex items-center justify-center gap-2 py-2 px-3 rounded-lg text-xs font-semibold transition-all " +
            (activeTab === "object"
              ? "bg-white dark:bg-slate-800 text-blue-600 dark:text-blue-400 shadow-xs border border-slate-200/80 dark:border-slate-700"
              : "text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-200")
          }
          onclick={() => (studioStore.architectureModalTab = "object")}
        >
          <div class="size-5 rounded-full flex items-center justify-center bg-blue-50 dark:bg-blue-950/60 text-blue-600 text-[10px] font-bold">
            3
          </div>
          <Boxes class="size-3.5" />
          <span>Meta-Object</span>
        </button>
      </div>

      <!-- =================================================================== -->
      <!-- TAB 1: MODULE REGISTRATION CARD -->
      <!-- =================================================================== -->
      {#if activeTab === "module"}
        <div class="space-y-4 animate-in fade-in duration-150">
          <div class="p-3 rounded-lg border border-indigo-100 dark:border-indigo-900/50 bg-indigo-50/50 dark:bg-indigo-950/20 text-xs text-indigo-900 dark:text-indigo-200 flex items-start gap-2">
            <Info class="size-4 shrink-0 mt-0.5 text-indigo-600" />
            <div>
              <p class="font-semibold">Step 1: Define the Root Module / Package</p>
              <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                A module is a standalone, distributable enterprise package (`.bospkg`) providing a strict namespace. All features and meta-objects belong to a module.
              </p>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Module ID <span class="text-rose-500">*</span>
              </label>
              <input
                type="text"
                bind:value={modPackageId}
                placeholder="e.g. bos.sales or acme.crm"
                class="w-full text-xs font-mono px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
              <p class="text-[10px] text-slate-400">Lowercase, dot-separated namespace.</p>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Display Title
              </label>
              <input
                type="text"
                bind:value={modName}
                placeholder="e.g. Sales Management & Invoicing"
                class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
              <p class="text-[10px] text-slate-400">Human-readable module description.</p>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Publisher / Organization
              </label>
              <input
                type="text"
                bind:value={modPublisher}
                placeholder="e.g. Acme ERP or BOS Solutions"
                class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
              />
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div class="space-y-1">
                <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">Version</label>
                <input
                  type="text"
                  bind:value={modVersion}
                  placeholder="1.0.0"
                  class="w-full text-xs font-mono px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              </div>

              <div class="space-y-1">
                <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">Domain Category</label>
                <select
                  bind:value={modCategory}
                  class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                >
                  {#each DOMAIN_CATEGORIES as cat}
                    <option value={cat}>{cat}</option>
                  {/each}
                </select>
              </div>
            </div>
          </div>

          <div class="space-y-1">
            <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
              Module Description & Business Scope
            </label>
            <textarea
              bind:value={modDescription}
              rows="2"
              placeholder="Provide a concise description of business capabilities handled by this module..."
              class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
            ></textarea>
          </div>

          <!-- Live Card Preview -->
          <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40">
            <div class="flex items-center justify-between text-[11px] font-semibold text-slate-500 uppercase tracking-wider mb-2">
              <span>Studio Sidebar Card Preview</span>
              <span class="text-[10px] lowercase font-normal">as rendered in panel</span>
            </div>
            <div class="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 p-3 shadow-2xs flex items-center justify-between">
              <div class="flex items-center gap-2.5 min-w-0">
                <div class="size-8 rounded-lg bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center shrink-0 border border-indigo-200/50">
                  <Package class="size-4" />
                </div>
                <div class="min-w-0">
                  <div class="flex items-center gap-1.5">
                    <span class="text-xs font-bold text-slate-900 dark:text-slate-100 truncate">
                      {modPackageId || "module.id"}
                    </span>
                    <span class="text-[9px] font-bold px-1 py-0.2 rounded bg-indigo-50 dark:bg-indigo-950 text-indigo-600 border border-indigo-200/50">
                      v{modVersion || "1.0.0"}
                    </span>
                  </div>
                  <p class="text-[11px] text-slate-500 truncate">
                    {modName || "Module Display Name"} · <span class="font-medium text-slate-400">{modPublisher || "Publisher"}</span>
                  </p>
                </div>
              </div>
              <span class="text-[10px] font-bold px-2 py-0.5 rounded-full bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300">
                0 objects
              </span>
            </div>
          </div>

          {#if modError}
            <Banner tone="danger" title="Validation Error">{modError}</Banner>
          {/if}
          {#if modSuccess}
            <Banner tone="success" title="Module Created">{modSuccess}</Banner>
          {/if}

          <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-200 dark:border-slate-800">
            <Button variant="ghost" onclick={() => handleCreateModule(false)}>
              Save Module
            </Button>
            <Button variant="primary" class="gap-1.5 bg-indigo-600 hover:bg-indigo-700 text-white" onclick={() => handleCreateModule(true)}>
              Save & Proceed to Feature / Objects
              <ArrowRight class="size-3.5" />
            </Button>
          </div>
        </div>

      <!-- =================================================================== -->
      <!-- TAB 2: FEATURE REGISTRATION CARD (OPTIONAL) -->
      <!-- =================================================================== -->
      {:else if activeTab === "feature"}
        <div class="space-y-4 animate-in fade-in duration-150">
          <div class="p-3 rounded-lg border border-orange-100 dark:border-orange-900/50 bg-orange-50/50 dark:bg-orange-950/20 text-xs text-orange-900 dark:text-orange-200 flex items-start gap-2">
            <Sparkles class="size-4 shrink-0 mt-0.5 text-orange-600" />
            <div>
              <p class="font-semibold">Step 2: Define Functional Features (Completely Optional)</p>
              <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Features group related tables, pages, and workflows under a common business capability (e.g. `OrderProcessing`, `InvoicePosting`, `RBAC`).
              </p>
            </div>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Target Module <span class="text-rose-500">*</span>
              </label>
              <select
                bind:value={featModuleId}
                class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-orange-500 font-mono"
              >
                {#each registeredModules as mod}
                  <option value={mod.package_id}>{mod.package_id} ({mod.name || mod.package_id})</option>
                {/each}
              </select>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Feature Code / Identifier <span class="text-rose-500">*</span>
              </label>
              <input
                type="text"
                bind:value={featId}
                placeholder="e.g. OrderProcessing or LoginSecurity"
                class="w-full text-xs font-mono px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-orange-500"
              />
              <p class="text-[10px] text-slate-400">Singular PascalCase identifier.</p>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Feature Display Title
              </label>
              <input
                type="text"
                bind:value={featName}
                placeholder="e.g. Sales Quotes & Order Processing"
                class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-orange-500"
              />
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">Status</label>
              <select
                bind:value={featStatus}
                class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-orange-500"
              >
                <option value="active">Active (Production)</option>
                <option value="in_progress">In Progress</option>
                <option value="draft">Draft (Planning)</option>
                <option value="experimental">Experimental / Beta</option>
              </select>
            </div>
          </div>

          <div class="space-y-1">
            <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
              Feature Description & Capabilities
            </label>
            <textarea
              bind:value={featDescription}
              rows="2"
              placeholder="Describe what functionality this feature encapsulates..."
              class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-orange-500"
            ></textarea>
          </div>

          <!-- Feature Card Live Preview -->
          <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40">
            <div class="flex items-center justify-between text-[11px] font-semibold text-slate-500 uppercase tracking-wider mb-2">
              <span>Feature Card Preview</span>
              <span class="text-[10px] lowercase font-normal">sub-accordion in module</span>
            </div>
            <div class="rounded-lg border border-orange-200/80 dark:border-orange-800/80 bg-white dark:bg-slate-900 p-2.5 shadow-2xs flex items-center justify-between">
              <div class="flex items-center gap-2 min-w-0">
                <Sparkles class="size-4 text-orange-500 shrink-0" />
                <div class="min-w-0">
                  <div class="flex items-center gap-1.5">
                    <span class="text-xs font-bold text-slate-900 dark:text-slate-100">
                      {featId || "FeatureName"}
                    </span>
                    <span class="text-[9px] font-semibold px-1.5 py-0.2 rounded bg-orange-50 dark:bg-orange-950 text-orange-700 dark:text-orange-300 border border-orange-200">
                      {featStatus}
                    </span>
                  </div>
                  <p class="text-[10px] text-slate-400 truncate">
                    Module: <span class="font-mono text-slate-600 dark:text-slate-300">{featModuleId || "module.id"}</span>
                    {#if featName} · {featName}{/if}
                  </p>
                </div>
              </div>
              <span class="text-[10px] font-bold px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-slate-500">
                feature
              </span>
            </div>
          </div>

          {#if featError}
            <Banner tone="danger" title="Validation Error">{featError}</Banner>
          {/if}
          {#if featSuccess}
            <Banner tone="success" title="Feature Saved">{featSuccess}</Banner>
          {/if}

          <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-200 dark:border-slate-800">
            <Button variant="ghost" onclick={() => (studioStore.architectureModalTab = "object")}>
              Skip to Meta-Objects
            </Button>
            <Button variant="ghost" onclick={() => handleCreateFeature(false)}>
              Save Feature
            </Button>
            <Button variant="primary" class="gap-1.5 bg-orange-600 hover:bg-orange-700 text-white" onclick={() => handleCreateFeature(true)}>
              Save & Add Meta-Objects
              <ArrowRight class="size-3.5" />
            </Button>
          </div>
        </div>

      <!-- =================================================================== -->
      <!-- TAB 3: META-OBJECT REGISTRATION CARD -->
      <!-- =================================================================== -->
      {:else if activeTab === "object"}
        <div class="space-y-4 animate-in fade-in duration-150">
          <div class="p-3 rounded-lg border border-blue-100 dark:border-blue-900/50 bg-blue-50/50 dark:bg-blue-950/20 text-xs text-blue-900 dark:text-blue-200 flex items-start gap-2">
            <Boxes class="size-4 shrink-0 mt-0.5 text-blue-600" />
            <div>
              <p class="font-semibold">Step 3: Create Meta-Object under Selected Module & Feature</p>
              <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">
                Every meta-object is registered into the PostgreSQL catalog and opened in its specialized designer.
              </p>
            </div>
          </div>

          <!-- Target Module and Feature Selectors -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50">
            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Target Module <span class="text-rose-500">*</span></span>
                <button
                  type="button"
                  class="text-[10px] text-indigo-600 hover:underline flex items-center gap-0.5"
                  onclick={() => (studioStore.architectureModalTab = "module")}
                >
                  <Plus class="size-3" /> New Module
                </button>
              </label>
              <select
                bind:value={objModuleId}
                class="w-full text-xs font-mono px-2.5 py-1.5 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
              >
                {#each registeredModules as mod}
                  <option value={mod.package_id}>{mod.package_id} ({mod.name || mod.package_id})</option>
                {/each}
              </select>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300 flex items-center justify-between">
                <span>Target Feature <span class="text-slate-400 font-normal">(Optional)</span></span>
                <button
                  type="button"
                  class="text-[10px] text-orange-600 hover:underline flex items-center gap-0.5"
                  onclick={() => {
                    featModuleId = objModuleId;
                    studioStore.architectureModalTab = "feature";
                  }}
                >
                  <Plus class="size-3" /> New Feature
                </button>
              </label>
              <select
                bind:value={objFeatureId}
                class="w-full text-xs px-2.5 py-1.5 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
              >
                <option value="">None (Module Root / General)</option>
                {#each availableFeaturesForObj as feat}
                  <option value={feat.name}>{feat.caption || feat.name}</option>
                {/each}
              </select>
            </div>
          </div>

          <!-- Meta-Object Type Grid -->
          <div class="space-y-1.5">
            <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
              Select Meta-Object Type
            </label>
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
              {#each OBJECT_TYPES as type}
                {@const Icon = type.icon}
                {@const isSelected = objType === type.id}
                <button
                  type="button"
                  class={
                    "flex flex-col items-start p-2.5 rounded-lg border text-left transition-all " +
                    (isSelected
                      ? `${type.bg} ${type.border} ring-2 ring-indigo-500/30 shadow-xs font-semibold`
                      : "border-slate-200 dark:border-slate-800 hover:bg-slate-100/60 dark:hover:bg-slate-800/40")
                  }
                  onclick={() => (objType = type.id)}
                >
                  <div class="flex items-center justify-between w-full mb-1">
                    <Icon class={"size-4 " + type.color} />
                    {#if isSelected}
                      <Check class="size-3.5 text-indigo-600 dark:text-indigo-400" />
                    {/if}
                  </div>
                  <span class="text-xs font-bold text-slate-800 dark:text-slate-100">{type.name}</span>
                  <span class="text-[10px] text-slate-400 line-clamp-1 mt-0.5">{type.desc}</span>
                </button>
              {/each}
            </div>
          </div>

          <!-- Object Name and SubType -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Object Name <span class="text-rose-500">*</span>
              </label>
              <input
                type="text"
                bind:value={objName}
                placeholder="e.g. Customer or SalesOrder"
                class="w-full text-xs font-mono px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-blue-500"
              />
              <p class="text-[10px] text-slate-400">Singular PascalCase (e.g. User, InvoiceHeader).</p>
            </div>

            <div class="space-y-1">
              <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
                Subtype / Archetype
              </label>
              {#if objType === "page" || objType === "pageextension"}
                <select
                  bind:value={objSubType}
                  class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
                  <option value="Card">Card (Single Record Form)</option>
                  <option value="List">List (Data Table Grid)</option>
                  <option value="Document">Document (Header + Lines)</option>
                  <option value="StandardDialog">StandardDialog (Modal / Prompt)</option>
                  <option value="Workspace">Workspace (Hub / Dashboard)</option>
                </select>
              {:else if objType === "table" || objType === "tableextension"}
                <select
                  bind:value={objSubType}
                  class="w-full text-xs px-2.5 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
                >
                  <option value="Normal">Master / Standard Table</option>
                  <option value="Document">Document Header / Lines</option>
                  <option value="Ledger">Immutable Ledger Entry</option>
                  <option value="Setup">Setup / Single-Record Configuration</option>
                </select>
              {:else}
                <input
                  type="text"
                  bind:value={objSubType}
                  placeholder="Standard"
                  class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 focus:outline-none focus:ring-1 focus:ring-blue-500"
                />
              {/if}
            </div>
          </div>

          <div class="space-y-1">
            <label class="text-xs font-semibold text-slate-700 dark:text-slate-300">
              Caption / Display Title
            </label>
            <input
              type="text"
              bind:value={objCaption}
              placeholder="e.g. Customer Master Record"
              class="w-full text-xs px-3 py-2 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>

          <!-- Object Row Live Preview (showing uniform right-aligned badge!) -->
          <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40">
            <div class="flex items-center justify-between text-[11px] font-semibold text-slate-500 uppercase tracking-wider mb-2">
              <span>Navigation Tree Item Preview</span>
              <span class="text-[10px] font-normal text-emerald-600">✓ Guaranteed Uniform Right-Aligned Badge</span>
            </div>
            <div class="rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 px-3 py-2 shadow-2xs flex items-center justify-between">
              <div class="flex items-center gap-2 min-w-0">
                <PreviewIcon class={"size-3.5 shrink-0 " + selectedTypeObj.color} />
                <span class="text-xs font-medium text-slate-800 dark:text-slate-100 truncate">
                  {objName || "ObjectName"}
                </span>
                {#if objCaption}
                  <span class="text-[11px] text-slate-400 truncate">({objCaption})</span>
                {/if}
              </div>
              <div class="flex items-center gap-1.5 shrink-0 ml-auto">
                <span class={"text-[10px] font-semibold px-1.5 py-0.5 rounded border " + selectedTypeObj.badge}>
                  {objSubType || selectedTypeObj.name}
                </span>
              </div>
            </div>
          </div>

          {#if objError}
            <Banner tone="danger" title="Validation Error">{objError}</Banner>
          {/if}

          <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-200 dark:border-slate-800">
            <Button variant="ghost" onclick={() => studioStore.closeArchitectureModal()}>
              Cancel
            </Button>
            <Button variant="primary" class="gap-1.5 bg-indigo-600 hover:bg-indigo-700 text-white font-semibold" onclick={handleCreateMetaObject}>
              <Plus class="size-3.5" />
              Create & Open in Designer
            </Button>
          </div>
        </div>
      {/if}
    </div>
  {/snippet}
</Dialog>
