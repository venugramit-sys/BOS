<script>
  /**
   * NavigationPane.svelte
   * Object catalog rendered with the bos-ui tree, search, and context menu.
   */
  import {
    Table,
    Plus,
    GitBranch,
    FileCode,
    Layout,
    Cpu,
    Workflow,
    ListFilter,
    Folder,
    ChevronLeft,
    RefreshCw,
    Pencil,
    TableProperties,
  } from "lucide-svelte";
  import {
    Button,
    Input,
    Badge,
    Tree,
    ContextMenu,
    EmptyState,
    Spinner,
  } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  let selectedId = $state("");

  const groups = [
    { id: "table", label: "Tables & Extensions", icon: Table, types: ["table", "tableextension"] },
    { id: "query", label: "Queries & Views", icon: ListFilter, types: ["query", "view"] },
    { id: "page", label: "Pages & Forms", icon: Layout, types: ["page", "pageextension"] },
    { id: "codeunit", label: "Codeunits", icon: Cpu, types: ["codeunit"] },
    { id: "statemachine", label: "State Machines", icon: Workflow, types: ["statemachine"] },
    { id: "enum", label: "Enums", icon: GitBranch, types: ["enum", "enumextension"] },
    { id: "other", label: "Other", icon: FileCode, types: [] },
  ];

  function iconFor(type) {
    if (type === "tablelogic" || type === "tablelogicextension") return Cpu;
    return groups.find((group) => group.types.includes(type))?.icon || FileCode;
  }

  function findCompanionLogic(tableName) {
    return studioStore.catalog.find(
      (obj) =>
        (obj.object_type === "tablelogic" || obj.object_type === "tablelogicextension") &&
        (obj.target_table === tableName ||
          obj.definition?.target_table === tableName ||
          obj.name === `${tableName}Logic` ||
          obj.name.toLowerCase() === `${tableName.toLowerCase()}logic`)
    );
  }

  let nodes = $derived(
    groups
      .map((group) => {
        let children = [];

        if (group.id === "table") {
          // Tables and Table Extensions with nested TableLogic companions
          children = studioStore.catalog
            .filter((obj) => group.types.includes(obj.object_type))
            .map((obj) => {
              const logicObj = findCompanionLogic(obj.name);
              const subItems = logicObj
                ? [
                    {
                      id: logicObj.id || logicObj.name,
                      label: `${logicObj.name}`,
                      icon: Cpu,
                      badge: "tablelogic",
                      badgeTone: "success",
                      object: logicObj,
                    },
                  ]
                : [];

              return {
                id: obj.id || obj.name,
                label: obj.name,
                icon: iconFor(obj.object_type),
                badge: obj.object_type,
                badgeTone: "info",
                object: obj,
                children: subItems,
              };
            });
        } else if (group.id === "other") {
          // Exclude tablelogic since it is nested directly under its Table
          const knownTypes = [
            "table",
            "tableextension",
            "tablelogic",
            "tablelogicextension",
            "query",
            "view",
            "page",
            "pageextension",
            "codeunit",
            "statemachine",
            "enum",
            "enumextension",
          ];
          children = studioStore.catalog
            .filter((obj) => !knownTypes.includes(obj.object_type))
            .map((obj) => ({
              id: obj.id || obj.name,
              label: obj.name,
              icon: iconFor(obj.object_type),
              badge: obj.object_type,
              badgeTone: "info",
              object: obj,
            }));
        } else {
          children = studioStore.catalog
            .filter((obj) => group.types.includes(obj.object_type))
            .map((obj) => ({
              id: obj.id || obj.name,
              label: obj.name,
              icon: iconFor(obj.object_type),
              badge: obj.object_type,
              badgeTone: "info",
              object: obj,
            }));
        }

        return {
          id: group.id,
          label: group.label,
          icon: group.icon,
          badge: String(children.length),
          children,
        };
      })
      .filter((group) => group.children.length > 0 || !studioStore.searchQuery)
  );

  let expanded = $state(["table", "page", "codeunit"]);

  let selectedObject = $derived.by(() => {
    for (const group of nodes) {
      for (const child of group.children || []) {
        if (child.id === selectedId) return child.object;
        if (child.children?.length) {
          const nested = child.children.find((c) => c.id === selectedId);
          if (nested) return nested.object;
        }
      }
    }
    return null;
  });

  function openSelected(mode) {
    if (!selectedObject) return;
    studioStore.openObject(selectedObject.name, mode, selectedObject.object_type);
  }
</script>

{#if !studioStore.isNavCollapsed}
  <!-- Backdrop on mobile -->
  <button
    type="button"
    class="md:hidden fixed inset-0 z-40 bg-slate-900/50 backdrop-blur-sm cursor-default"
    aria-label="Close navigation"
    onclick={() => studioStore.collapseNav()}
  ></button>
  <aside
    class="h-full border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 md:bg-slate-50/70 md:dark:bg-slate-900/60 flex flex-col shrink-0 w-72 fixed inset-y-0 left-0 z-50 shadow-2xl md:static md:z-auto md:shadow-none animate-in slide-in-from-left duration-200"
  >
    <div class="p-3 border-b border-slate-200 dark:border-slate-800 space-y-2.5">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-1.5 text-xs font-bold uppercase tracking-wider">
          <Folder class="size-4 text-indigo-500" />
          <span>Objects</span>
          <Badge>{studioStore.objectCounts.total}</Badge>
        </div>
        <div class="flex items-center gap-1">
          <Button
            variant="ghost"
            size="xs"
            iconOnly
            icon={RefreshCw}
            loading={studioStore.isLoadingCatalog}
            tooltip="Refresh catalog"
            aria-label="Refresh catalog"
            onclick={() => studioStore.loadCatalog()}
          />
          <Button
            variant="ghost"
            size="xs"
            iconOnly
            icon={ChevronLeft}
            tooltip="Collapse navigation"
            shortcut="Ctrl+B"
            aria-label="Collapse navigation"
            onclick={() => studioStore.collapseNav()}
          />
        </div>
      </div>
      <Button class="w-full" size="sm" icon={Plus} onclick={() => (studioStore.isNewObjectModalOpen = true)}>
        New Meta-Object...
      </Button>
      <Input type="search" size="sm" bind:value={studioStore.searchQuery} placeholder="Search objects..." />
    </div>

    <div class="flex-1 min-h-0">
      {#if studioStore.isLoadingCatalog && studioStore.catalog.length === 0}
        <div class="py-8 flex flex-col items-center gap-2 text-xs text-slate-400">
          <Spinner />
          <p>Querying the meta-object catalog...</p>
        </div>
      {:else if studioStore.catalogError}
        <EmptyState title="Catalog unavailable" description={studioStore.catalogError} />
      {:else}
        <ContextMenu
          items={[
            { label: "Design view", icon: Pencil, action: () => openSelected("design") },
            {
              label: "Datasheet",
              icon: TableProperties,
              disabled: !selectedObject || (selectedObject.object_type !== "table" && selectedObject.object_type !== "tableextension"),
              action: () => openSelected("datasheet"),
            },
            { label: "Code", icon: FileCode, shortcut: "Ctrl+Enter", action: () => openSelected("code") },
          ]}
        >
          <Tree
            {nodes}
            bind:expanded
            bind:selectedId
            filter={studioStore.searchQuery}
            label="Meta-objects"
            onActivate={(node) => {
              if (node.object) studioStore.openObject(node.object.name, "design", node.object.object_type);
            }}
          />
        </ContextMenu>
      {/if}
    </div>
  </aside>
{:else}
  <aside
    class="hidden md:flex h-full border-r border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-900/60 flex-col shrink-0 w-12"
  >
    <div class="flex-1 flex flex-col items-center py-3 gap-3">
      <Button
        variant="ghost"
        icon={Folder}
        iconOnly
        tooltip="Expand navigation"
        shortcut="Ctrl+B"
        aria-label="Expand navigation"
        onclick={() => studioStore.expandNav()}
      />
      <Button icon={Plus} iconOnly aria-label="New meta-object" tooltip="New meta-object" onclick={() => (studioStore.isNewObjectModalOpen = true)} />
    </div>
  </aside>
{/if}
