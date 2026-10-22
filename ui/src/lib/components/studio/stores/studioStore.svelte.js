/**
 * StudioState (Svelte 5 Runes)
 * Reactive multi-document workspace. One module-level $state object so every
 * read in a render sees the same tabs, catalog, and selection.
 */
import { StudioApi } from "./studioApi.js";
import { documentFromDetails, emitDocument } from "../designers/tableModel.js";
import { formatBosScriptSource } from "../code-editor/bosPrettier.js";

const listeners = new Set();

function notify() {
  for (const listener of listeners) listener();
}

export function onStudioChange(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

const state = $state({
  catalog: [],
  isLoadingCatalog: false,
  catalogError: null,
  activeCategory: "all",
  searchQuery: "",
  openTabs: [],
  activeTabId: "",
  selectedElement: null,
  isNewObjectModalOpen: false,
  isRelationshipsOpen: false,
  isNavCollapsed: false,
});

class StudioStore {
  get catalog() {
    return state.catalog;
  }
  set catalog(value) {
    state.catalog = value;
  }

  get isLoadingCatalog() {
    return state.isLoadingCatalog;
  }
  set isLoadingCatalog(value) {
    state.isLoadingCatalog = value;
  }

  get catalogError() {
    return state.catalogError;
  }
  set catalogError(value) {
    state.catalogError = value;
  }

  get activeCategory() {
    return state.activeCategory;
  }
  set activeCategory(value) {
    state.activeCategory = value;
  }

  get searchQuery() {
    return state.searchQuery;
  }
  set searchQuery(value) {
    state.searchQuery = value;
  }

  get openTabs() {
    return state.openTabs;
  }
  set openTabs(value) {
    state.openTabs = value;
  }

  get activeTabId() {
    return state.activeTabId;
  }
  set activeTabId(value) {
    state.activeTabId = value;
    notify();
  }

  get selectedElement() {
    return state.selectedElement;
  }
  set selectedElement(value) {
    state.selectedElement = value;
  }

  get isNewObjectModalOpen() {
    return state.isNewObjectModalOpen;
  }
  set isNewObjectModalOpen(value) {
    state.isNewObjectModalOpen = value;
  }

  get isRelationshipsOpen() {
    return state.isRelationshipsOpen;
  }
  set isRelationshipsOpen(value) {
    state.isRelationshipsOpen = value;
  }

  get isNavCollapsed() {
    return state.isNavCollapsed;
  }
  set isNavCollapsed(value) {
    state.isNavCollapsed = value;
  }

  toggleNav() {
    state.isNavCollapsed = !state.isNavCollapsed;
  }

  expandNav() {
    state.isNavCollapsed = false;
  }

  collapseNav() {
    state.isNavCollapsed = true;
  }

  get filteredCatalog() {
    return state.catalog.filter((obj) => {
      const matchesCategory =
        state.activeCategory === "all" ||
        obj.object_type === state.activeCategory ||
        (state.activeCategory === "table" && obj.object_type === "tableextension") ||
        (state.activeCategory === "page" && obj.object_type === "pageextension");

      const name = obj.name || obj.object_name || "";
      const matchesSearch =
        !state.searchQuery.trim() ||
        name.toLowerCase().includes(state.searchQuery.toLowerCase()) ||
        (obj.package_name && obj.package_name.toLowerCase().includes(state.searchQuery.toLowerCase()));

      return matchesCategory && matchesSearch;
    });
  }

  get objectCounts() {
    const counts = {
      table: 0,
      query: 0,
      page: 0,
      codeunit: 0,
      statemachine: 0,
      enum: 0,
      other: 0,
      total: 0,
    };

    for (const obj of state.catalog) {
      counts.total++;
      if (obj.object_type === "table" || obj.object_type === "tableextension") counts.table++;
      else if (obj.object_type === "tablelogic" || obj.object_type === "tablelogicextension") {
        // Companion of table
      }
      else if (obj.object_type === "query" || obj.object_type === "view") counts.query++;
      else if (obj.object_type === "page" || obj.object_type === "pageextension") counts.page++;
      else if (obj.object_type === "codeunit") counts.codeunit++;
      else if (obj.object_type === "statemachine") counts.statemachine++;
      else if (obj.object_type === "enum" || obj.object_type === "enumextension") counts.enum++;
      else counts.other++;
    }

    return counts;
  }

  /**
   * Refreshes the meta-objects catalog from the real Axum backend.
   */
  async loadCatalog() {
    state.isLoadingCatalog = true;
    state.catalogError = null;
    try {
      const objs = await StudioApi.listMetaObjects();
      const raw = Array.isArray(objs) ? objs : [];
      state.catalog = raw.map((o) => ({
        ...o,
        id: o.object_id || o.id || `${o.object_type}-${o.object_name || o.name}`,
        name: o.name || o.object_name || "",
        object_name: o.object_name || o.name || "",
      }));
    } catch (err) {
      state.catalogError = err.message || "Failed to load meta-objects catalog";
    } finally {
      state.isLoadingCatalog = false;
    }
  }

  /**
   * Opens an object in the workspace.
   * mode: 'design' | 'datasheet' | 'code'
   * State changes stay synchronous so they commit on the click, not in an async fork.
   */
  /**
   * `packageName` is only passed when the user chose one (New Object). Otherwise the saved object's own
   * package (from its details) is used, so applying an installed object never copies it into another package.
   */
  openObject(name, mode = "design", type = "table", packageName = null) {
    const tabId = `${type}-${name}`;
    const existingTab = state.openTabs.find((tab) => tab.id === tabId);

    state.isNavCollapsed = true;

    if (existingTab) {
      existingTab.mode = mode;
      if (packageName) existingTab.packageName = packageName;
      state.activeTabId = tabId;
      notify();
      return;
    }

    state.activeTabId = tabId;
    state.openTabs = [
      ...state.openTabs,
      {
        id: tabId,
        name,
        type,
        title: name,
        mode,
        packageName: packageName || null,
        isDirty: false,
        details: null,
        sourceCode: "",
      },
    ];
    notify();
    void this.loadObjectDetails(name, tabId, type);
  }

  async companionDefinitions(tableName) {
    const companions = [];
    for (const kind of ["tablelogic", "tablelogicextension"]) {
      let listed = [];
      try {
        const raw = await StudioApi.listMetaObjects(kind);
        listed = Array.isArray(raw) ? raw : [];
      } catch {
        continue;
      }
      for (const item of listed) {
        const objectName = item.object_name || item.name;
        if (!objectName) continue;
        try {
          const details = await StudioApi.getMetaObject(objectName);
          const target = details?.definition?.target_table;
          if (target && target.toLowerCase() === tableName.toLowerCase()) {
            companions.push({ kind, details });
          }
        } catch {
          // A catalog name can disappear between the list and the read.
        }
      }
    }
    return companions;
  }

  async loadObjectDetails(name, tabId, type) {
    const tab = () => state.openTabs.find((item) => item.id === tabId);
    try {
      const details = await StudioApi.getMetaObject(name);
      const current = tab();
      if (!current || !details) return;
      current.type = details.object_type || type;
      current.details = details;
      const savedPackage = details.summary?.package_name;
      if (savedPackage) current.packageName = savedPackage;
      const objectType = current.type;
      const savedScript = typeof details.source_script === "string" ? details.source_script.trim() : "";
      if (!current.isDirty && savedScript && (objectType === "table" || objectType === "tableextension")) {
        current.sourceCode = await formatBosScriptSource(savedScript);
      } else if (!current.isDirty && (objectType === "table" || objectType === "tableextension")) {
        const companions = await this.companionDefinitions(name);
        const model = documentFromDetails(name, details, companions);
        if (model.fields.length) {
          current.sourceCode = await formatBosScriptSource(emitDocument(model).combined);
        } else if (details.source_script) {
          current.sourceCode = await formatBosScriptSource(details.source_script);
        }
      } else if (!current.isDirty && details.source_script) {
        current.sourceCode = await formatBosScriptSource(details.source_script);
      }
    } catch {
      // A new draft has no saved definition yet.
    } finally {
      const current = tab();
      if (current) current.detailsLoaded = true;
      notify();
    }
  }

  closeTab(tabId) {
    const index = state.openTabs.findIndex((t) => t.id === tabId);
    if (index === -1) return;

    const nextId =
      state.activeTabId === tabId
        ? (state.openTabs.filter((t) => t.id !== tabId)[Math.max(0, index - 1)]?.id ?? "")
        : state.activeTabId;

    state.activeTabId = nextId;
    state.openTabs = state.openTabs.filter((t) => t.id !== tabId);
    notify();
  }

  setActiveTabMode(mode) {
    const tab = state.openTabs.find((item) => item.id === state.activeTabId);
    if (tab) tab.mode = mode;
  }

  selectElement(element) {
    state.selectedElement = element;
  }
}

export const studioStore = new StudioStore();
export const studioState = state;
