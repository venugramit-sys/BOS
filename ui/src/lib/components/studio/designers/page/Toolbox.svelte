<script>
  /**
   * Toolbox.svelte
   * Mature Page Designer Toolbox powered by bos-ui primitives.
   * - Full support for Page Variables (declare, inspect, drag, place).
   * - Table Fields (source table columns & FlowFields).
   * - Rich enterprise controls (Inputs, Dates, Comboboxes, Switches, Grids, FastTabs, Actions).
   */
  import {
    Search,
    Plus,
    Variable,
    Table,
    Type,
    KeyRound,
    Binary,
    Calendar,
    ChevronDown,
    ToggleLeft,
    AlignLeft,
    FolderPlus,
    LayoutGrid,
    CreditCard,
    Rows,
    FolderKanban,
    Columns,
    Layers,
    Activity,
    MousePointerClick,
    Tag,
    Heading,
    Info,
    Image,
    Minus,
    Maximize2,
    Check
  } from "lucide-svelte";
  import { Button, Input, Select, Badge, Dialog } from "$lib/bos-ui/index.js";
  import { CONTAINER_KINDS } from "./design-values.js";

  /** @type {{ model: import('./designer-model.svelte.js').PageDesignerModel }} */
  let { model } = $props();

  let query = $state("");
  let activeTab = $state("all"); // 'all' | 'variables' | 'fields' | 'inputs' | 'containers' | 'actions' | 'extensions'
  let isAddVarDialogOpen = $state(false);

  // New variable form state
  let newVarName = $state("");
  let newVarType = $state("String");
  let newVarCaption = $state("");
  let newVarDefault = $state("");
  let varError = $state("");

  const VAR_TYPES = [
    { value: "String", label: "String" },
    { value: "Integer", label: "Integer" },
    { value: "Decimal", label: "Decimal" },
    { value: "Boolean", label: "Boolean" },
    { value: "Date", label: "Date" },
    { value: "DateTime", label: "DateTime" },
    { value: "UUID", label: "UUID" },
    { value: "Blob", label: "Blob" }
  ];

  // Enterprise control templates with rich metadata & icons
  const ENTERPRISE_CONTROLS = [
    // --- Extension Directives (for Page Extensions) ---
    {
      category: "extensions",
      kind: "ext_addlast",
      label: "addLast (Append Group)",
      desc: "Append group/fields to end of target container",
      icon: Plus,
      source: 'addLast(Content) {\n    group(CustomGroup) {\n        caption = "Custom Group";\n        field(custom_field) { caption = "Custom Field"; }\n    }\n}'
    },
    {
      category: "extensions",
      kind: "ext_addafter",
      label: "addAfter (Insert After)",
      desc: "Insert group/fields immediately after target",
      icon: Plus,
      source: 'addAfter(General) {\n    group(AuditTrail) {\n        caption = "Audit Trail";\n        field(audit_note) { caption = "Audit Note"; }\n    }\n}'
    },
    {
      category: "extensions",
      kind: "ext_modify",
      label: "modify (Override Control)",
      desc: "Non-invasively modify base page control properties",
      icon: Layers,
      source: 'modify(name) {\n    caption = "Legal Name";\n    editable = false;\n}'
    },
    // --- Data & Input Controls ---
    {
      category: "inputs",
      kind: "field_text",
      label: "Text Input",
      desc: "Standard alphanumeric input",
      icon: Type,
      source: 'field(NewTextField) { caption = "Text Field"; }'
    },
    {
      category: "inputs",
      kind: "field_password",
      label: "Password Field",
      desc: "Masked secure text entry",
      icon: KeyRound,
      source: 'field(PasswordField) { caption = "Password"; extendedDatatype = Masked; }'
    },
    {
      category: "inputs",
      kind: "field_number",
      label: "Number / Decimal",
      desc: "Numeric input with decimal precision",
      icon: Binary,
      source: 'field(AmountField) { caption = "Amount"; decimalPlaces = 2; }'
    },
    {
      category: "inputs",
      kind: "field_date",
      label: "Date Picker",
      desc: "Calendar popover selector",
      icon: Calendar,
      source: 'field(DueDateField) { caption = "Due Date"; control = DatePicker; }'
    },
    {
      category: "inputs",
      kind: "field_select",
      label: "Combobox / Dropdown",
      desc: "Searchable option dropdown",
      icon: ChevronDown,
      source: 'field(StatusField) { caption = "Status"; control = DropDown; }'
    },
    {
      category: "inputs",
      kind: "field_switch",
      label: "Switch / Toggle",
      desc: "Boolean toggle control",
      icon: ToggleLeft,
      source: 'field(IsEnabledField) { caption = "Active"; control = Switch; }'
    },
    {
      category: "inputs",
      kind: "field_textarea",
      label: "Multiline Textarea",
      desc: "Multi-row text block",
      icon: AlignLeft,
      source: 'field(NotesField) { caption = "Notes"; multiline = true; }'
    },

    // --- Containers & Layout ---
    {
      category: "containers",
      kind: "group",
      label: "Group (FastTab)",
      desc: "Collapsible enterprise FastTab section",
      icon: FolderPlus,
      source: 'group(NewGroup) { caption = "General Details"; }'
    },
    {
      category: "containers",
      kind: "card",
      label: "Card Container",
      desc: "Elevated bordered card panel",
      icon: CreditCard,
      source: 'card(NewCard) { caption = "Summary Card"; }'
    },
    {
      category: "containers",
      kind: "grid",
      label: "Grid (Columns)",
      desc: "Multi-column responsive grid layout",
      icon: LayoutGrid,
      source: "grid(NewGrid) { cols = 2; }"
    },
    {
      category: "containers",
      kind: "flex",
      label: "Row (Flex)",
      desc: "Horizontal flexbox row container",
      icon: Rows,
      source: "flex(NewRow) { }"
    },
    {
      category: "containers",
      kind: "tabs",
      label: "Tabs Container",
      desc: "Tabbed views with sub-tabs",
      icon: FolderKanban,
      source: 'tabs(NewTabs) {\n    tab(GeneralTab) { caption = "General"; }\n    tab(LinesTab) { caption = "Lines"; }\n}'
    },
    {
      category: "containers",
      kind: "split",
      label: "Split Panes",
      desc: "Dual master-detail resizable split layout",
      icon: Columns,
      source: 'split(NewSplit) {\n    group(LeftPane) { caption = "Master"; }\n    group(RightPane) { caption = "Detail"; }\n}'
    },
    {
      category: "containers",
      kind: "part",
      label: "Part (Subpage)",
      desc: "Embedded subpage or lines grid",
      icon: Layers,
      source: "part(LinesSubpage; LinesPageName) { }"
    },
    {
      category: "containers",
      kind: "cuegroup",
      label: "Cue Group (KPIs)",
      desc: "Tiles with live KPI counters",
      icon: Activity,
      source: 'cuegroup(ActivitiesGroup) { caption = "Activities"; }'
    },
    {
      category: "containers",
      kind: "section",
      label: "Section Container",
      desc: "Semantic <section> wrapper for landing/web blocks",
      icon: LayoutGrid,
      source: 'section(HeroSection) { caption = "Hero Section"; }'
    },
    {
      category: "containers",
      kind: "nav",
      label: "Navigation Bar",
      desc: "Semantic <nav> container for menus and breadcrumbs",
      icon: Rows,
      source: 'nav(NavBar) { caption = "Top Navigation"; }'
    },
    {
      category: "containers",
      kind: "footer",
      label: "Footer Region",
      desc: "Semantic <footer> container for page bottom",
      icon: Rows,
      source: 'footer(PageFooter) { caption = "Page Footer"; }'
    },
    {
      category: "containers",
      kind: "aside",
      label: "Sidebar (Aside)",
      desc: "Semantic <aside> container for complementary sidebars",
      icon: Columns,
      source: 'aside(Sidebar) { caption = "Sidebar"; }'
    },
    {
      category: "containers",
      kind: "usercontrol",
      label: "User Control",
      desc: "Custom registered Svelte 5 component / widget",
      icon: Layers,
      source: 'usercontrol(CustomWidget; "CustomWidget") { }'
    },

    // --- Actions & Visuals ---
    {
      category: "actions",
      kind: "action",
      label: "Action Button",
      desc: "Interactive trigger button",
      icon: MousePointerClick,
      source: 'action(NewAction) { caption = "Submit"; primary = true; }'
    },
    {
      category: "actions",
      kind: "label",
      label: "Label / Text",
      desc: "Static descriptive label",
      icon: Heading,
      source: 'label(NewLabel) { caption = "Informational Notice"; }'
    },
    {
      category: "actions",
      kind: "headline",
      label: "Headline Banner",
      desc: "Promoted hero greeting headline",
      icon: Heading,
      source: 'headline(NewHeadline) { caption = "Welcome to Workspace"; }'
    },
    {
      category: "actions",
      kind: "divider",
      label: "Divider",
      desc: "Horizontal rule separator",
      icon: Minus,
      source: "divider(NewDivider);"
    },
    {
      category: "actions",
      kind: "spacer",
      label: "Spacer",
      desc: "Vertical breathing space",
      icon: Maximize2,
      source: "spacer(NewSpacer);"
    },
    {
      category: "actions",
      kind: "image",
      label: "Image / Media",
      desc: "Branding graphic or photo",
      icon: Image,
      source: 'image(NewImage) { src = "/assets/logo.png"; }'
    }
  ];

  // Placed fields/variables index
  const placedSet = $derived.by(() => {
    const set = new Set();
    for (const c of model.compiledControls) {
      if (c.field) {
        if (c.field.source) set.add(c.field.source.toLowerCase());
        if (c.field.key) set.add(c.field.key.toLowerCase());
        if (c.name) set.add(c.name.toLowerCase());
      }
    }
    return set;
  });

  // Filtered Page Variables
  const filteredVariables = $derived(
    model.variables
      .map((v) => ({ ...v, placed: placedSet.has(v.name.toLowerCase()) }))
      .filter(
        (v) =>
          !query ||
          v.name.toLowerCase().includes(query.toLowerCase()) ||
          (v.caption ?? "").toLowerCase().includes(query.toLowerCase()) ||
          v.data_type.toLowerCase().includes(query.toLowerCase())
      )
      .sort((a, b) => Number(a.placed) - Number(b.placed))
  );

  // Filtered Table Fields
  const filteredFields = $derived(
    model.fields
      .map((f) => ({ ...f, placed: placedSet.has(f.name.toLowerCase()) }))
      .filter(
        (f) =>
          !query ||
          f.name.toLowerCase().includes(query.toLowerCase()) ||
          (f.caption ?? "").toLowerCase().includes(query.toLowerCase()) ||
          (f.type ?? "").toLowerCase().includes(query.toLowerCase())
      )
      .sort((a, b) => Number(a.placed) - Number(b.placed))
  );

  // Filtered Controls Palette
  const filteredControls = $derived(
    ENTERPRISE_CONTROLS.filter((c) => {
      const matchCat = activeTab === "all" || activeTab === c.category;
      const matchQ =
        !query ||
        c.label.toLowerCase().includes(query.toLowerCase()) ||
        c.desc.toLowerCase().includes(query.toLowerCase());
      return matchCat && matchQ;
    })
  );

  function fieldItem(f, isVariable = false) {
    return {
      kind: "field",
      field: f.name,
      label: f.caption || f.name,
      source: `field(${f.name});`,
      isVariable
    };
  }

  function startDrag(e, item) {
    if (e.button !== 0) return;
    e.preventDefault();
    model.dragItem = item;
  }

  function add(item) {
    const sel = model.selectedCompiled;
    if (sel && CONTAINER_KINDS.has(sel.kind)) return model.insertAt(item, { path: sel.path }, "inside");
    if (sel) return model.insertAt(item, { path: sel.path }, "after");
    return model.insertAt(item, null, "inside");
  }

  async function handleCreateVariable() {
    varError = "";
    if (!newVarName.trim()) {
      varError = "Variable name is required.";
      return;
    }
    const safeName = newVarName.trim().replace(/[^a-zA-Z0-9_]/g, "");
    if (!safeName) {
      varError = "Invalid variable name. Use alphanumeric characters.";
      return;
    }
    try {
      await model.addVariable({
        name: safeName,
        data_type: newVarType,
        caption: newVarCaption.trim() || safeName,
        default_value: newVarDefault.trim() || null
      });
      newVarName = "";
      newVarCaption = "";
      newVarDefault = "";
      isAddVarDialogOpen = false;
    } catch (e) {
      varError = e.message || "Failed to add variable";
    }
  }
</script>

<div class="flex flex-col gap-3 text-xs">
  <!-- Search Filter -->
  <div class="relative">
    <Input
      size="sm"
      placeholder="Filter fields, variables, controls..."
      bind:value={query}
      class="pl-8"
    />
    <Search class="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 pointer-events-none" />
  </div>

  <!-- Category Filter Pills -->
  <div class="flex items-center gap-1 overflow-x-auto pb-1 no-scrollbar text-[11px]">
    {#each [
      ["all", "All"],
      ["variables", `Vars (${model.variables.length})`],
      ["fields", `Table (${model.fields.length})`],
      ["inputs", "Inputs"],
      ["containers", "Layout"],
      ["actions", "Actions"],
      ...(model.objectKind === "pageextension" ? [["extensions", "Directives"]] : [])
    ] as [id, label] (id)}
      <button
        type="button"
        class={
          "px-2 py-0.5 rounded-full font-medium transition-colors shrink-0 " +
          (activeTab === id
            ? "bg-indigo-600 text-white font-semibold shadow-2xs"
            : "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-200")
        }
        onclick={() => (activeTab = id)}
      >
        {label}
      </button>
    {/each}
  </div>

  <!-- ===================================================================== -->
  <!-- 1. PAGE VARIABLES SECTION -->
  <!-- ===================================================================== -->
  {#if activeTab === "all" || activeTab === "variables"}
    <section class="space-y-1.5 p-2 rounded-lg border border-purple-200/70 dark:border-purple-900/40 bg-purple-50/30 dark:bg-purple-950/20">
      <div class="flex items-center justify-between">
        <h4 class="text-[10px] font-bold uppercase tracking-wider text-purple-700 dark:text-purple-300 flex items-center gap-1.5">
          <Variable class="size-3 text-purple-600" />
          Page Variables ({filteredVariables.length})
        </h4>
        <Button
          size="xs"
          variant="ghost"
          class="h-5 px-1.5 text-[10px] gap-1 text-purple-700 dark:text-purple-300 hover:bg-purple-100 dark:hover:bg-purple-900/50"
          onclick={() => (isAddVarDialogOpen = true)}
        >
          <Plus class="size-2.5" /> New Var
        </Button>
      </div>

      {#if filteredVariables.length === 0}
        <p class="text-[11px] text-slate-400 italic px-1">
          {model.variables.length ? "No matching variables." : "No page variables declared. Click 'New Var' to declare."}
        </p>
      {:else}
        <ul class="space-y-1">
          {#each filteredVariables as v (v.name)}
            <li>
              <div
                class={
                  "group flex touch-none select-none items-center justify-between rounded-md px-2 py-1.5 border transition-all " +
                  (v.placed
                    ? "border-slate-200/60 dark:border-slate-800 bg-white/50 dark:bg-slate-900/50 opacity-60 text-slate-400"
                    : "border-purple-200/60 dark:border-purple-900/60 bg-white dark:bg-slate-900 hover:border-purple-400 cursor-grab shadow-2xs")
                }
                onpointerdown={(e) => !v.placed && startDrag(e, fieldItem(v, true))}
                role="button"
                tabindex={v.placed ? -1 : 0}
                aria-disabled={v.placed}
                aria-label={`Place variable ${v.name}`}
                onkeydown={(e) => {
                  if (!v.placed && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    add(fieldItem(v, true));
                  }
                }}
              >
                <div class="flex items-center gap-1.5 min-w-0 pr-1 flex-1">
                  <span class="size-1.5 rounded-full bg-purple-500 shrink-0"></span>
                  <div class="truncate">
                    <span class="font-mono font-semibold text-slate-800 dark:text-slate-100 text-[11px] block truncate">
                      {v.name}
                    </span>
                    {#if v.caption && v.caption !== v.name}
                      <span class="text-[10px] text-slate-400 block truncate">{v.caption}</span>
                    {/if}
                  </div>
                </div>

                <div class="flex items-center gap-1 shrink-0">
                  <span class="font-mono text-[9px] px-1 py-0.2 rounded bg-purple-100/70 dark:bg-purple-900/50 text-purple-800 dark:text-purple-300">
                    {v.data_type}
                  </span>
                  {#if v.placed}
                    <Badge tone="success" size="sm" class="text-[9px] px-1 py-0.2">
                      <Check class="size-2.5 mr-0.5" /> placed
                    </Badge>
                  {:else}
                    <button
                      type="button"
                      class="hidden rounded p-0.5 text-purple-600 hover:bg-purple-100 group-hover:inline-flex dark:hover:bg-purple-900"
                      title="Add variable to layout"
                      onpointerdown={(e) => e.stopPropagation()}
                      onclick={() => add(fieldItem(v, true))}
                    >
                      <Plus class="size-3" />
                    </button>
                  {/if}
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  <!-- ===================================================================== -->
  <!-- 2. TABLE FIELDS SECTION -->
  <!-- ===================================================================== -->
  {#if (activeTab === "all" || activeTab === "fields") && (model.fields.length > 0 || !model.compiled?.source_table)}
    <section class="space-y-1.5 p-2 rounded-lg border border-slate-200/80 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/40">
      <div class="flex items-center justify-between">
        <h4 class="text-[10px] font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 flex items-center gap-1.5">
          <Table class="size-3 text-blue-500" />
          Table Fields ({filteredFields.length})
        </h4>
        {#if model.compiled?.source_table}
          <span class="text-[10px] font-mono text-slate-400">
            {model.compiled.source_table}
          </span>
        {/if}
      </div>

      {#if filteredFields.length === 0}
        <p class="text-[11px] text-slate-400 italic px-1">
          {model.fields.length ? "No matching table fields." : "This page has no bound source table."}
        </p>
      {:else}
        <ul class="space-y-1">
          {#each filteredFields as f (f.name)}
            <li>
              <div
                class={
                  "group flex touch-none select-none items-center justify-between rounded-md px-2 py-1.5 border transition-all " +
                  (f.placed
                    ? "border-slate-200/60 dark:border-slate-800 bg-white/40 dark:bg-slate-900/40 opacity-60 text-slate-400"
                    : "border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 hover:border-blue-400 cursor-grab shadow-2xs")
                }
                onpointerdown={(e) => !f.placed && startDrag(e, fieldItem(f, false))}
                role="button"
                tabindex={f.placed ? -1 : 0}
                aria-disabled={f.placed}
                aria-label={`Place field ${f.name}`}
                onkeydown={(e) => {
                  if (!f.placed && (e.key === "Enter" || e.key === " ")) {
                    e.preventDefault();
                    add(fieldItem(f, false));
                  }
                }}
              >
                <div class="flex items-center gap-1.5 min-w-0 pr-1 flex-1">
                  <span class={"size-1.5 rounded-full shrink-0 " + (f.flow ? "bg-amber-500" : "bg-blue-500")}></span>
                  <div class="truncate">
                    <span class="font-medium text-slate-800 dark:text-slate-100 text-[11px] block truncate">
                      {f.caption || f.name}
                    </span>
                    {#if f.caption && f.caption !== f.name}
                      <span class="font-mono text-[9px] text-slate-400 block truncate">{f.name}</span>
                    {/if}
                  </div>
                </div>

                <div class="flex items-center gap-1 shrink-0">
                  <span class="font-mono text-[9px] px-1 py-0.2 rounded bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400">
                    {f.flow ? "FlowField" : f.type}
                  </span>
                  {#if f.placed}
                    <Badge tone="neutral" size="sm" class="text-[9px] px-1 py-0.2">
                      placed
                    </Badge>
                  {:else}
                    <button
                      type="button"
                      class="hidden rounded p-0.5 text-blue-600 hover:bg-blue-100 group-hover:inline-flex dark:hover:bg-blue-900"
                      title="Add field to layout"
                      onpointerdown={(e) => e.stopPropagation()}
                      onclick={() => add(fieldItem(f, false))}
                    >
                      <Plus class="size-3" />
                    </button>
                  {/if}
                </div>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}

  <!-- ===================================================================== -->
  <!-- 3. MATURE ENTERPRISE CONTROLS PALETTE -->
  <!-- ===================================================================== -->
  {#if activeTab !== "variables" && activeTab !== "fields"}
    <section class="space-y-1.5">
      <h4 class="text-[10px] font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 flex items-center justify-between">
        <span>Controls Palette ({filteredControls.length})</span>
      </h4>

      <ul class="grid grid-cols-1 sm:grid-cols-2 gap-1.5">
        {#each filteredControls as c (c.kind)}
          {@const Icon = c.icon}
          <li>
            <div
              class="group flex items-start gap-2 p-2 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 hover:border-indigo-400 dark:hover:border-indigo-500 transition-all touch-none select-none shadow-2xs"
              class:cursor-grab={model.mode === "develop"}
              role="button"
              tabindex={model.mode === "develop" ? 0 : -1}
              aria-label={`Add ${c.label}`}
              onpointerdown={(e) => model.mode === "develop" && startDrag(e, c)}
              onkeydown={(e) => {
                if (model.mode === "develop" && (e.key === "Enter" || e.key === " ")) {
                  e.preventDefault();
                  add(c);
                }
              }}
            >
              <div class="size-6 rounded-md bg-slate-100 dark:bg-slate-800 flex items-center justify-center shrink-0 text-slate-600 dark:text-slate-400 group-hover:text-indigo-600 group-hover:bg-indigo-50 dark:group-hover:bg-indigo-950 transition-colors">
                <Icon class="size-3.5" />
              </div>
              <div class="min-w-0 flex-1">
                <span class="font-semibold text-slate-800 dark:text-slate-100 text-[11px] block truncate">
                  {c.label}
                </span>
                <span class="text-[10px] text-slate-400 block truncate leading-tight">
                  {c.desc}
                </span>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<!-- ===================================================================== -->
<!-- ADD PAGE VARIABLE DIALOG (bos-ui) -->
<!-- ===================================================================== -->
<Dialog
  bind:open={isAddVarDialogOpen}
  title="Declare Page Variable"
  description="Add a runtime state variable to this page. Page variables hold temporary form state without touching table schema."
  size="sm"
>
  <div class="space-y-3 py-2 text-xs">
    {#if varError}
      <p class="p-2 rounded bg-rose-50 text-rose-700 text-xs font-medium border border-rose-200">
        {varError}
      </p>
    {/if}

    <div class="space-y-1">
      <label class="font-semibold text-slate-700 dark:text-slate-300">
        Variable Identifier <span class="text-rose-500">*</span>
      </label>
      <Input
        size="sm"
        placeholder="e.g. username, rememberMe, filterStatus"
        bind:value={newVarName}
      />
    </div>

    <div class="space-y-1">
      <label class="font-semibold text-slate-700 dark:text-slate-300">
        BOS Data Type <span class="text-rose-500">*</span>
      </label>
      <Select
        bind:value={newVarType}
        options={VAR_TYPES}
      />
    </div>

    <div class="space-y-1">
      <label class="font-semibold text-slate-700 dark:text-slate-300">
        Display Caption (Optional)
      </label>
      <Input
        size="sm"
        placeholder="Label shown on bound fields"
        bind:value={newVarCaption}
      />
    </div>

    <div class="space-y-1">
      <label class="font-semibold text-slate-700 dark:text-slate-300">
        Default Initial Value (Optional)
      </label>
      <Input
        size="sm"
        placeholder="e.g. false, 0, 'ACTIVE'"
        bind:value={newVarDefault}
      />
    </div>
  </div>

  {#snippet footer()}
    <div class="flex items-center justify-end gap-2 w-full">
      <Button variant="ghost" size="sm" onclick={() => (isAddVarDialogOpen = false)}>
        Cancel
      </Button>
      <Button size="sm" onclick={handleCreateVariable}>
        Add Variable
      </Button>
    </div>
  {/snippet}
</Dialog>

<style>
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
