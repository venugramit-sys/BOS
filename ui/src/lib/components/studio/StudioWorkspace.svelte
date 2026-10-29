<script>
  /**
   * StudioWorkspace.svelte (Svelte 5 Runes)
   * Root Developer Studio. Chrome comes from bos-ui.
   */
  import { onMount } from "svelte";
  import {
    Table,
    Cpu,
    Workflow,
    Layout,
    ListFilter,
    GitBranch,
    FileCode,
    Plus,
    Network,
    Folder,
    Play,
    Database,
    Menu,
    Zap,
  } from "lucide-svelte";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";
  import { shell } from "$lib/components/shell/index.js";
  import {
    Menubar,
    MenuHelpers,
    Button,
    Tabs,
    EmptyState,
    CommandPalette,
    StatusBar,
    ToastViewport,
    ConfirmDialog,
  } from "$lib/bos-ui/index.js";
  import NavigationPane from "./NavigationPane.svelte";
  import NewObjectModal from "./NewObjectModal.svelte";
  import ArchitectureModal from "./ArchitectureModal.svelte";
  import ArchitectureDetailCard from "./ArchitectureDetailCard.svelte";
  import TableDesigner from "./designers/TableDesigner.svelte";
  import QueryDesigner from "./designers/QueryDesigner.svelte";
  import PageDesigner from "./designers/PageDesigner.svelte";
  import StateMachineDesigner from "./designers/StateMachineDesigner.svelte";
  import PermissionSetDesigner from "./designers/PermissionSetDesigner.svelte";
  import RelationshipsViewer from "./designers/RelationshipsViewer.svelte";
  import MonacoCodeStudio from "./code-editor/MonacoCodeStudio.svelte";
  import { studioStore, studioState, onStudioChange } from "./stores/studioStore.svelte.js";

  let { theme = "dark" } = $props();

  let paletteOpen = $state(false);
  let pendingCloseId = $state("");
  let openTabs = $state([]);
  let activeTabId = $state("");

  onMount(() => {
    studioStore.loadCatalog();
    return onStudioChange(() => {
      openTabs = studioState.openTabs.map((tab) => tab);
      activeTabId = studioState.activeTabId;
    });
  });

  function getTabIcon(type) {
    switch (type) {
      case "table":
      case "tableextension":
        return Table;
      case "query":
      case "view":
        return ListFilter;
      case "page":
      case "pageextension":
        return Layout;
      case "codeunit":
        return Cpu;
      case "statemachine":
        return Workflow;
      case "enum":
      case "enumextension":
        return GitBranch;
      default:
        return FileCode;
    }
  }

  let currentTab = $derived(openTabs.find((item) => item.id === activeTabId) ?? openTabs[0] ?? null);

  let documentTabs = $derived(
    openTabs.map((tab) => ({
      id: tab.id,
      label: tab.title,
      icon: getTabIcon(tab.type),
      dirty: tab.isDirty,
      closable: true,
    }))
  );

  let activePageActions = $derived.by(() => {
    if (currentTab?.type !== "page") return [];
    const pMeta = pageManifest.manifest?.pages?.[currentTab.title];
    return pMeta?.actions ?? [];
  });

  let commands = $derived([
    ...(activePageActions.length > 0 ? [{
      heading: `Page Actions (${currentTab.title})`,
      items: activePageActions.map((a) => ({
        id: `page-action-${a.name}`,
        label: `${a.caption || a.name} · ${a.area || "Processing"}`,
        keywords: [a.name, a.area || "", a.group || "", "action", "run", "page"],
        icon: Zap,
        action: () => {
          window.dispatchEvent(new CustomEvent("bos:execute-page-action", { detail: { name: a.name } }));
        }
      }))
    }] : []),
    {
      heading: "Commands",
      items: [
        {
          id: "new-object",
          label: "Add Architecture Component (Module, Feature, Object)...",
          shortcut: "Ctrl+N",
          icon: Plus,
          action: () => studioStore.openArchitectureModal("module"),
        },
        {
          id: "relationships",
          label: "View relationships",
          icon: Network,
          action: () => (studioStore.isRelationshipsOpen = true),
        },
        {
          id: "toggle-nav",
          label: studioStore.isNavCollapsed ? "Expand navigation" : "Collapse navigation",
          shortcut: "Ctrl+B",
          action: () => studioStore.toggleNav(),
        },
      ],
    },
    {
      heading: "Objects",
      items: studioStore.catalog.slice(0, 80).map((obj) => ({
        id: `open-${obj.id || obj.name}`,
        label: obj.name,
        keywords: [obj.object_type, obj.package_name || ""],
        icon: getTabIcon(obj.object_type),
        action: () => studioStore.openObject(obj.name, "design", obj.object_type),
      })),
    },
  ]);

  let menus = $derived([
    {
      id: "file",
      label: "File",
      items: [
        MenuHelpers.item("New Architecture Component...", () => studioStore.openArchitectureModal("module"), {
          icon: Plus,
          shortcut: "Ctrl+N",
        }),
        MenuHelpers.item("Close Tab", () => requestClose(studioStore.activeTabId), {
          shortcut: "Ctrl+W",
          disabled: !studioStore.activeTabId,
        }),
      ],
    },
    {
      id: "view",
      label: "View",
      items: [
        MenuHelpers.item(
          studioStore.isNavCollapsed ? "Expand Navigation" : "Collapse Navigation",
          () => studioStore.toggleNav(),
          { shortcut: "Ctrl+B" }
        ),
        MenuHelpers.item("Command Palette...", () => (paletteOpen = true), { shortcut: "Ctrl+K" }),
        MenuHelpers.separator(),
        MenuHelpers.item("Relationships", () => (studioStore.isRelationshipsOpen = true), {
          icon: Network,
        }),
      ],
    },
    {
      id: "database",
      label: "Database",
      items: [
        MenuHelpers.item("Database Manager...", () => shell.openDbManager(), {
          icon: Database,
        }),
        MenuHelpers.item("Environment & Context...", () => shell.openEnvironmentDialog(), {}),
        MenuHelpers.separator(),
        MenuHelpers.item("Refresh System Health", () => shell.refreshSystem(), {}),
      ],
    },
    {
      id: "build",
      label: "Build",
      items: [
        MenuHelpers.item("Apply is available in the active designer", () => {}, {
          icon: Play,
          shortcut: "F5",
          disabled: true,
        }),
      ],
    },
  ]);

  function requestClose(tabId) {
    if (!tabId) return;
    const tab = studioStore.openTabs.find((item) => item.id === tabId);
    if (tab?.isDirty) {
      pendingCloseId = tabId;
      return;
    }
    studioStore.closeTab(tabId);
  }

  $effect(() => {
    const onKey = (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b") {
        event.preventDefault();
        studioStore.toggleNav();
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "n") {
        event.preventDefault();
        studioStore.isNewObjectModalOpen = true;
      }
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "w") {
        event.preventDefault();
        requestClose(studioStore.activeTabId);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<div class="w-full h-full flex flex-col bg-slate-100/70 dark:bg-slate-950 text-slate-900 dark:text-slate-100 overflow-hidden font-sans">
  <div class="flex items-center border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900">
    <Button
      variant="ghost"
      size="xs"
      iconOnly
      icon={Menu}
      class="md:hidden shrink-0 ml-1.5"
      aria-label="Toggle objects navigation"
      onclick={() => studioStore.toggleNav()}
    />
    <div class="flex-1 min-w-0">
      <Menubar menus={menus} />
    </div>
  </div>

  <div class="flex-1 flex min-h-0">
    <NavigationPane />
    <main class="flex-1 flex flex-col min-w-0 min-h-0 bg-white dark:bg-slate-950">
      {#if openTabs.length > 0}
        {@const tab = openTabs.find((item) => item.id === activeTabId) ?? openTabs[0]}
        <div class="border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900 px-1 sm:px-2 pt-1 overflow-x-auto no-scrollbar">
          <Tabs
            value={tab.id}
            variant="document"
            items={documentTabs}
            onValueChange={(id) => {
              if (id) studioStore.activeTabId = id;
            }}
            onClose={requestClose}
          />
        </div>

        <div class="flex-1 min-h-0 overflow-hidden bg-slate-50/50 dark:bg-slate-900/30 p-1 sm:p-2 md:p-3">
          {#each openTabs as tab (tab.id)}
            {#if tab.id === activeTabId}
                {#if tab.type === "table" || tab.type === "tableextension"}
                  {#if tab.mode === "code"}
                    <MonacoCodeStudio
                      objectName={tab.name}
                      objectType={tab.type}
                      packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                      bind:content={tab.sourceCode}
                      focusLine={tab.focusLine || 0}
                      {theme}
                      onDesign={() => (tab.mode = "design")}
                      onChange={() => (tab.isDirty = true)}
                      onApplySuccess={() => (tab.isDirty = false)}
                    />
                  {:else}
                    <TableDesigner
                      tableName={tab.name}
                      sourceCode={tab.sourceCode}
                      objectDetails={tab.details}
                      detailsLoaded={Boolean(tab.detailsLoaded)}
                      packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                      bind:mode={tab.mode}
                      onSourceChange={(script) => {
                        tab.sourceCode = script;
                      }}
                      onJumpToCode={(_script, line) => {
                        tab.focusLine = line || 0;
                        tab.mode = "code";
                      }}
                    />
                  {/if}
              {:else if tab.type === "query" || tab.type === "view"}
                {#if tab.mode === "code"}
                  <MonacoCodeStudio
                    objectName={tab.name}
                    objectType={tab.type}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:content={tab.sourceCode}
                    focusLine={tab.focusLine || 0}
                    {theme}
                    onDesign={() => (tab.mode = "design")}
                    onChange={() => (tab.isDirty = true)}
                    onApplySuccess={() => (tab.isDirty = false)}
                  />
                {:else}
                  <QueryDesigner
                    queryName={tab.name}
                    sourceCode={tab.sourceCode}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:isDirty={tab.isDirty}
                    bind:mode={tab.mode}
                    onSourceChange={(script) => {
                      tab.sourceCode = script;
                    }}
                    onJumpToCode={(_script, line) => {
                      tab.focusLine = line || 0;
                      tab.mode = "code";
                    }}
                  />
                {/if}
              {:else if tab.type === "page" || tab.type === "pageextension"}
                {#if tab.mode === "code"}
                  <MonacoCodeStudio
                    objectName={tab.name}
                    objectType={tab.type}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:content={tab.sourceCode}
                    focusLine={tab.focusLine || 0}
                    {theme}
                    onDesign={() => (tab.mode = "design")}
                    onChange={() => (tab.isDirty = true)}
                    onApplySuccess={() => (tab.isDirty = false)}
                  />
                {:else}
                  <PageDesigner
                    pageName={tab.name}
                    sourceCode={tab.sourceCode}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:mode={tab.mode}
                    bind:isDirty={tab.isDirty}
                    onSourceChange={(script) => {
                      tab.sourceCode = script;
                    }}
                    onJumpToCode={(_script, line) => {
                      tab.focusLine = line || 0;
                      tab.mode = "code";
                    }}
                  />
                {/if}
              {:else if tab.type === "statemachine" || tab.type === "statemachineextension"}
                {#if tab.mode === "code"}
                  <MonacoCodeStudio
                    objectName={tab.name}
                    objectType={tab.type}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:content={tab.sourceCode}
                    focusLine={tab.focusLine || 0}
                    {theme}
                    onDesign={() => (tab.mode = "design")}
                    onChange={() => (tab.isDirty = true)}
                    onApplySuccess={() => (tab.isDirty = false)}
                  />
                {:else}
                  <StateMachineDesigner
                    stateMachineName={tab.name}
                    sourceCode={tab.sourceCode}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:mode={tab.mode}
                    onSourceChange={(script) => {
                      tab.sourceCode = script;
                    }}
                    onJumpToCode={(_script, line) => {
                      tab.focusLine = line || 0;
                      tab.mode = "code";
                    }}
                  />
                {/if}
              {:else if tab.type === "permissionset"}
                {#if tab.mode === "code"}
                  <MonacoCodeStudio
                    objectName={tab.name}
                    objectType={tab.type}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:content={tab.sourceCode}
                    focusLine={tab.focusLine || 0}
                    {theme}
                    onDesign={() => (tab.mode = "design")}
                    onChange={() => (tab.isDirty = true)}
                    onApplySuccess={() => (tab.isDirty = false)}
                  />
                {:else}
                  <PermissionSetDesigner
                    permissionSetName={tab.name}
                    sourceCode={tab.sourceCode}
                    packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                    bind:mode={tab.mode}
                    onSourceChange={(script) => {
                      tab.sourceCode = script;
                    }}
                    onJumpToCode={(_script, line) => {
                      tab.focusLine = line || 0;
                      tab.mode = "code";
                    }}
                  />
                {/if}
              {:else}
                <MonacoCodeStudio
                  objectName={tab.name}
                  objectType={tab.type}
                  packageName={tab.packageName || tab.details?.summary?.package_name || "bos.app"}
                  bind:content={tab.sourceCode}
                  focusLine={tab.focusLine || 0}
                  {theme}
                  onChange={() => (tab.isDirty = true)}
                  onApplySuccess={() => (tab.isDirty = false)}
                />
              {/if}
            {/if}
          {/each}
        </div>
      {:else}
        <div class="flex-1 min-h-0 overflow-hidden bg-slate-50/50 dark:bg-slate-900/30 p-3">
          <EmptyState
            class="h-full"
            icon={Folder}
            title="BOS Developer Studio"
            description="Pick an object from the navigation tree, or create one. Ctrl+K jumps to any object."
          >
            <Button icon={Plus} onclick={() => (studioStore.isNewObjectModalOpen = true)}>
              Create Meta-Object
            </Button>
            <Button variant="secondary" icon={Network} onclick={() => (studioStore.isRelationshipsOpen = true)}>
              View Relationships
            </Button>
          </EmptyState>
        </div>
      {/if}

      <StatusBar
        leading={[
          { id: "objects", label: `${studioStore.objectCounts.total} objects`, tone: studioStore.catalogError ? "danger" : "success" },
          { id: "error", label: studioStore.catalogError || "Catalog synced" },
        ]}
        trailing={[
          {
            id: "tab",
            label: currentTab ? `${currentTab.type} · ${currentTab.mode || "design"}` : "No document",
          },
          { id: "palette", label: "Ctrl+K", onClick: () => (paletteOpen = true) },
        ]}
      />
    </main>
  </div>
</div>

<ToastViewport />
<CommandPalette bind:open={paletteOpen} groups={commands} recentKey="bos.studio.recent" />
<ArchitectureModal />
<ArchitectureDetailCard />
<NewObjectModal />
<RelationshipsViewer bind:isOpen={studioStore.isRelationshipsOpen} />

<ConfirmDialog
  bind:open={() => Boolean(pendingCloseId), (next) => { if (!next) pendingCloseId = ""; }}
  tone="danger"
  title="Close without saving?"
  description="This document has unsaved changes. Closing it drops the draft."
  confirmLabel="Close"
  onConfirm={() => {
    studioStore.closeTab(pendingCloseId);
    pendingCloseId = "";
  }}
  onCancel={() => (pendingCloseId = "")}
/>
