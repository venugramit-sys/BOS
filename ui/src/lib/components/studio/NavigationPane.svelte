<script>
  /**
   * NavigationPane.svelte
   * Enterprise Module, Feature & Meta-Object Explorer for BOS Studio.
   * - Module (e.g. bos.core, bos.sales) is the main top-level parent container.
   * - Feature (optional) is a structured domain group under the module to separate objects further.
   * - Inside each Feature (or direct under module if no feature), objects are categorized by type.
   * - Selecting "Features" category chip filters and highlights all feature groups with full fidelity.
   * - 100% uniform badges, high-contrast typography, and intuitive hover actions.
   */
  import {
    Package,
    Boxes,
    Folder,
    FolderOpen,
    Table,
    Layout,
    Cpu,
    Workflow,
    ListFilter,
    GitBranch,
    FileCode,
    Layers,
    ChevronDown,
    ChevronRight,
    ChevronLeft,
    ChevronsUpDown,
    ChevronsDownUp,
    Plus,
    Search,
    RefreshCw,
    Pencil,
    TableProperties,
    Check,
    X,
    Filter,
    Shield,
    Sparkles,
    Info,
    SlidersHorizontal,
  } from "lucide-svelte";
  import {
    Button,
    Badge,
    ContextMenu,
    EmptyState,
    Spinner,
  } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  // Category filter tabs for meta-objects inside modules
  const CATEGORIES = [
    { id: "all", label: "All", icon: Boxes },
    { id: "table", label: "Tables", icon: Table, types: ["table"] },
    { id: "page", label: "Pages", icon: Layout, types: ["page"] },
    { id: "codeunit", label: "Codeunits", icon: Cpu, types: ["codeunit"] },
    { id: "statemachine", label: "Workflows", icon: Workflow, types: ["statemachine"] },
    { id: "enum", label: "Enums", icon: GitBranch, types: ["enum"] },
    { id: "permissionset", label: "Security", icon: Shield, types: ["permissionset"] },
    { id: "menu", label: "Menus", icon: ListFilter, types: ["menu"] },
    { id: "query", label: "Queries", icon: ListFilter, types: ["query", "view"] },
    { id: "extension", label: "Extensions", icon: Layers, types: ["pageextension", "tableextension", "enumextension", "tablelogicextension"] },
  ];

  // Group definitions for meta-object types with enterprise color coding
  const TYPE_GROUPS = [
    {
      id: "table",
      label: "Tables",
      icon: Table,
      iconColor: "text-blue-600 dark:text-blue-400",
      badgeColor: "bg-blue-50 dark:bg-blue-950/60 text-blue-700 dark:text-blue-300 border-blue-200/60 dark:border-blue-800/60",
      types: ["table"],
    },
    {
      id: "page",
      label: "Pages",
      icon: Layout,
      iconColor: "text-indigo-600 dark:text-indigo-400",
      badgeColor: "bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border-indigo-200/60 dark:border-indigo-800/60",
      types: ["page"],
    },
    {
      id: "codeunit",
      label: "Codeunits",
      icon: Cpu,
      iconColor: "text-amber-600 dark:text-amber-400",
      badgeColor: "bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border-amber-200/60 dark:border-amber-800/60",
      types: ["codeunit"],
    },
    {
      id: "statemachine",
      label: "Workflows",
      icon: Workflow,
      iconColor: "text-rose-600 dark:text-rose-400",
      badgeColor: "bg-rose-50 dark:bg-rose-950/60 text-rose-700 dark:text-rose-300 border-rose-200/60 dark:border-rose-800/60",
      types: ["statemachine"],
    },
    {
      id: "enum",
      label: "Enums",
      icon: GitBranch,
      iconColor: "text-teal-600 dark:text-teal-400",
      badgeColor: "bg-teal-50 dark:bg-teal-950/60 text-teal-700 dark:text-teal-300 border-teal-200/60 dark:border-teal-800/60",
      types: ["enum"],
    },
    {
      id: "permissionset",
      label: "Permissions & Roles",
      icon: Shield,
      iconColor: "text-red-600 dark:text-red-400",
      badgeColor: "bg-red-50 dark:bg-red-950/60 text-red-700 dark:text-red-300 border-red-200/60 dark:border-red-800/60",
      types: ["permissionset"],
    },
    {
      id: "menu",
      label: "Navigation Menus",
      icon: ListFilter,
      iconColor: "text-emerald-600 dark:text-emerald-400",
      badgeColor: "bg-emerald-50 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 border-emerald-200/60 dark:border-emerald-800/60",
      types: ["menu"],
    },
    {
      id: "query",
      label: "Queries",
      icon: ListFilter,
      iconColor: "text-sky-600 dark:text-sky-400",
      badgeColor: "bg-sky-50 dark:bg-sky-950/60 text-sky-700 dark:text-sky-300 border-sky-200/60 dark:border-sky-800/60",
      types: ["query", "view"],
    },
    {
      id: "extension",
      label: "Extensions & Customizations",
      icon: Layers,
      iconColor: "text-purple-600 dark:text-purple-400",
      badgeColor: "bg-purple-50 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 border-purple-200/60 dark:border-purple-800/60",
      types: ["pageextension", "tableextension", "enumextension", "tablelogicextension"],
    },
  ];

  let selectedCategory = $state("all");
  let collapsedModules = $state({});
  let collapsedFeatures = $state({});
  let collapsedSubgroups = $state({});
  let selectedId = $state("");
  let selectedObject = $state(null);
  let selectedPkg = $state("");

  function toggleModule(pkgId) {
    collapsedModules = {
      ...collapsedModules,
      [pkgId]: !collapsedModules[pkgId],
    };
  }

  function toggleFeature(featKey) {
    collapsedFeatures = {
      ...collapsedFeatures,
      [featKey]: !collapsedFeatures[featKey],
    };
  }

  function toggleSubgroup(groupId) {
    collapsedSubgroups = {
      ...collapsedSubgroups,
      [groupId]: !collapsedSubgroups[groupId],
    };
  }

  let allCollapsed = $state(false);
  function toggleAll() {
    allCollapsed = !allCollapsed;
    const nextMod = {};
    const nextFeat = {};
    const nextSub = {};
    if (allCollapsed) {
      for (const m of modules) {
        nextMod[m.id] = true;
        if (m.features) {
          for (const f of m.features) {
            nextFeat[`${m.id}:${f.id}`] = true;
            for (const g of f.groups) nextSub[`${m.id}:${f.id}:${g.id}`] = true;
          }
        }
        if (m.unassigned) {
          nextFeat[`${m.id}:__unassigned__`] = true;
          for (const g of m.unassigned.groups) nextSub[`${m.id}:__unassigned__:${g.id}`] = true;
        }
        if (m.directGroups) {
          for (const g of m.directGroups) nextSub[`${m.id}:${g.id}`] = true;
        }
      }
    } else {
      for (const m of modules) {
        nextMod[m.id] = false;
        if (m.features) {
          for (const f of m.features) {
            nextFeat[`${m.id}:${f.id}`] = false;
            for (const g of f.groups) nextSub[`${m.id}:${f.id}:${g.id}`] = false;
          }
        }
        if (m.unassigned) {
          nextFeat[`${m.id}:__unassigned__`] = false;
          for (const g of m.unassigned.groups) nextSub[`${m.id}:__unassigned__:${g.id}`] = false;
        }
        if (m.directGroups) {
          for (const g of m.directGroups) nextSub[`${m.id}:${g.id}`] = false;
        }
      }
    }
    collapsedModules = nextMod;
    collapsedFeatures = nextFeat;
    collapsedSubgroups = nextSub;
  }

  // Module level: Check if all contents in module are collapsed
  function isModuleFeaturesCollapsed(mod) {
    if (collapsedModules[mod.id]) return true;
    if (mod.features && mod.features.length > 0) {
      return mod.features.every((f) => collapsedFeatures[`${mod.id}:${f.id}`]);
    }
    return false;
  }

  // Module level: Toggle all features and subgroups inside module
  function toggleModuleCollapseAll(mod, event) {
    if (event) event.stopPropagation();
    let hasExpanded = false;
    if (mod.features) {
      for (const f of mod.features) {
        const featKey = `${mod.id}:${f.id}`;
        if (!collapsedFeatures[featKey]) {
          hasExpanded = true;
          break;
        }
        for (const g of f.groups) {
          if (!collapsedSubgroups[`${mod.id}:${f.id}:${g.id}`]) {
            hasExpanded = true;
            break;
          }
        }
      }
    }
    if (mod.unassigned && mod.unassigned.groups?.length > 0) {
      const uKey = `${mod.id}:__unassigned__`;
      if (!collapsedFeatures[uKey]) {
        hasExpanded = true;
      }
      for (const g of mod.unassigned.groups) {
        if (!collapsedSubgroups[`${uKey}:${g.id}`]) {
          hasExpanded = true;
          break;
        }
      }
    }
    if (mod.directGroups) {
      for (const g of mod.directGroups) {
        if (!collapsedSubgroups[`${mod.id}:${g.id}`]) {
          hasExpanded = true;
          break;
        }
      }
    }

    const shouldCollapse = hasExpanded;
    const nextFeat = { ...collapsedFeatures };
    const nextSub = { ...collapsedSubgroups };

    if (mod.features) {
      for (const f of mod.features) {
        nextFeat[`${mod.id}:${f.id}`] = shouldCollapse;
        for (const g of f.groups) {
          nextSub[`${mod.id}:${f.id}:${g.id}`] = shouldCollapse;
        }
      }
    }
    if (mod.unassigned) {
      nextFeat[`${mod.id}:__unassigned__`] = shouldCollapse;
      for (const g of mod.unassigned.groups) {
        nextSub[`${mod.id}:__unassigned__:${g.id}`] = shouldCollapse;
      }
    }
    if (mod.directGroups) {
      for (const g of mod.directGroups) {
        nextSub[`${mod.id}:${g.id}`] = shouldCollapse;
      }
    }

    collapsedFeatures = nextFeat;
    collapsedSubgroups = nextSub;

    if (!shouldCollapse && collapsedModules[mod.id]) {
      collapsedModules = { ...collapsedModules, [mod.id]: false };
    }
  }

  // Feature level: Check if all subgroups in feature are collapsed
  function isFeatureGroupsCollapsed(modId, feat) {
    const featKey = `${modId}:${feat.id}`;
    if (collapsedFeatures[featKey]) return true;
    if (feat.groups && feat.groups.length > 0) {
      return feat.groups.every((g) => collapsedSubgroups[`${modId}:${feat.id}:${g.id}`]);
    }
    return false;
  }

  // Feature level: Toggle all subgroups inside feature
  function toggleFeatureCollapseAll(modId, feat, event) {
    if (event) event.stopPropagation();
    let hasExpanded = false;
    for (const g of feat.groups) {
      const subId = `${modId}:${feat.id}:${g.id}`;
      if (!collapsedSubgroups[subId]) {
        hasExpanded = true;
        break;
      }
    }
    const shouldCollapse = hasExpanded;
    const nextSub = { ...collapsedSubgroups };
    for (const g of feat.groups) {
      const subId = `${modId}:${feat.id}:${g.id}`;
      nextSub[subId] = shouldCollapse;
    }
    collapsedSubgroups = nextSub;

    const featKey = `${modId}:${feat.id}`;
    if (!shouldCollapse && collapsedFeatures[featKey]) {
      collapsedFeatures = { ...collapsedFeatures, [featKey]: false };
    }
  }

  // Unassigned level: Check if all unassigned groups are collapsed
  function isUnassignedGroupsCollapsed(modId, unassigned) {
    const uKey = `${modId}:__unassigned__`;
    if (collapsedFeatures[uKey]) return true;
    if (unassigned?.groups && unassigned.groups.length > 0) {
      return unassigned.groups.every((g) => collapsedSubgroups[`${uKey}:${g.id}`]);
    }
    return false;
  }

  // Unassigned level: Toggle all unassigned groups
  function toggleUnassignedCollapseAll(modId, unassigned, event) {
    if (event) event.stopPropagation();
    const uKey = `${modId}:__unassigned__`;
    let hasExpanded = false;
    for (const g of unassigned.groups) {
      const subId = `${uKey}:${g.id}`;
      if (!collapsedSubgroups[subId]) {
        hasExpanded = true;
        break;
      }
    }
    const shouldCollapse = hasExpanded;
    const nextSub = { ...collapsedSubgroups };
    for (const g of unassigned.groups) {
      const subId = `${uKey}:${g.id}`;
      nextSub[subId] = shouldCollapse;
    }
    collapsedSubgroups = nextSub;

    if (!shouldCollapse && collapsedFeatures[uKey]) {
      collapsedFeatures = { ...collapsedFeatures, [uKey]: false };
    }
  }

  /**
   * Deterministic visual styling for every meta-object item.
   */
  function getObjectVisuals(item) {
    const type = item.object_type || "table";
    if (type === "table") {
      return {
        icon: Table,
        color: "text-blue-500/70 dark:text-blue-400/70",
        badge: item.sub_type && item.sub_type !== "Normal" ? item.sub_type : "Table",
        badgeStyle: "bg-blue-50 dark:bg-blue-950/60 text-blue-700 dark:text-blue-300 border-blue-200/60 dark:border-blue-800/60",
      };
    }
    if (type === "tablelogic" || type === "tablelogicextension") {
      return {
        icon: Cpu,
        color: "text-emerald-500/70 dark:text-emerald-400/70",
        badge: "logic",
        badgeStyle: "bg-emerald-50 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 border-emerald-200/60 dark:border-emerald-800/60",
      };
    }
    if (type === "page") {
      return {
        icon: Layout,
        color: "text-indigo-500/70 dark:text-indigo-400/70",
        badge: item.sub_type || "Page",
        badgeStyle: "bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border-indigo-200/60 dark:border-indigo-800/60",
      };
    }
    if (type === "codeunit") {
      return {
        icon: Cpu,
        color: "text-amber-500/70 dark:text-amber-400/70",
        badge: item.sub_type || "Codeunit",
        badgeStyle: "bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border-amber-200/60 dark:border-amber-800/60",
      };
    }
    if (type === "statemachine") {
      return {
        icon: Workflow,
        color: "text-rose-500/70 dark:text-rose-400/70",
        badge: "Workflow",
        badgeStyle: "bg-rose-50 dark:bg-rose-950/60 text-rose-700 dark:text-rose-300 border-rose-200/60 dark:border-rose-800/60",
      };
    }
    if (type === "enum") {
      return {
        icon: GitBranch,
        color: "text-teal-500/70 dark:text-teal-400/70",
        badge: "Enum",
        badgeStyle: "bg-teal-50 dark:bg-teal-950/60 text-teal-700 dark:text-teal-300 border-teal-200/60 dark:border-teal-800/60",
      };
    }
    if (type === "query" || type === "view") {
      return {
        icon: ListFilter,
        color: "text-sky-500/70 dark:text-sky-400/70",
        badge: "Query",
        badgeStyle: "bg-sky-50 dark:bg-sky-950/60 text-sky-700 dark:text-sky-300 border-sky-200/60 dark:border-sky-800/60",
      };
    }
    if (type === "permissionset") {
      return {
        icon: Shield,
        color: "text-red-500/70 dark:text-red-400/70",
        badge: "PermissionSet",
        badgeStyle: "bg-red-50 dark:bg-red-950/60 text-red-700 dark:text-red-300 border-red-200/60 dark:border-red-800/60",
      };
    }
    if (type === "menu") {
      return {
        icon: ListFilter,
        color: "text-emerald-500/70 dark:text-emerald-400/70",
        badge: "Menu",
        badgeStyle: "bg-emerald-50 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 border-emerald-200/60 dark:border-emerald-800/60",
      };
    }
    if (type === "feature") {
      return {
        icon: Sparkles,
        color: "text-orange-500/70 dark:text-orange-400/70",
        badge: item.definition?.status || "feature",
        badgeStyle: "bg-orange-50 dark:bg-orange-950/60 text-orange-700 dark:text-orange-300 border-orange-200/60 dark:border-orange-800/60",
      };
    }
    if (type.includes("extension")) {
      return {
        icon: Layers,
        color: "text-purple-500/70 dark:text-purple-400/70",
        badge: item.targetName ? `extends ${item.targetName}` : "Extension",
        badgeStyle: "bg-purple-50 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 border-purple-200/60 dark:border-purple-800/60",
      };
    }
    return {
      icon: FileCode,
      color: "text-slate-400 dark:text-slate-500",
      badge: type,
      badgeStyle: "bg-slate-100 dark:bg-slate-800 text-slate-500 border-slate-200",
    };
  }

  function findCompanionLogic(tableName, catalog) {
    return catalog.find(
      (obj) =>
        (obj.object_type === "tablelogic" || obj.object_type === "tablelogicextension") &&
        (obj.target_table === tableName ||
          obj.definition?.target_table === tableName ||
          obj.name === `${tableName}Logic` ||
          obj.name.toLowerCase() === `${tableName.toLowerCase()}logic`)
    );
  }

  // Derived structured module tree with Module -> Feature (Optional) -> Meta-Objects
  let modules = $derived.by(() => {
    const query = (studioStore.searchQuery || "").trim().toLowerCase();
    const catFilter = selectedCategory;

    // 1. Collect all registered packages from backend
    const pkgMap = new Map();

    for (const pkg of studioStore.packages) {
      pkgMap.set(pkg.package_id, {
        id: pkg.package_id,
        package_id: pkg.package_id,
        name: pkg.name || pkg.package_id,
        version: pkg.version || "1.0.0",
        publisher: pkg.publisher || "BOS Core",
        status: pkg.status || "active",
        isCore: pkg.package_id === "bos.core" || pkg.package_id === "core",
        objects: [],
      });
    }

    // Always guarantee bos.core canonical platform package exists
    if (!pkgMap.has("bos.core")) {
      pkgMap.set("bos.core", {
        id: "bos.core",
        package_id: "bos.core",
        name: "BOS Unified Core Platform & Security Kernel",
        version: "1.0.0",
        publisher: "BOS Core Architecture Team",
        status: "active",
        isCore: true,
        objects: [],
      });
    }

    // Helper: Resolve which module an object belongs to
    function resolveTargetPackage(obj) {
      const rawPkg = obj.package_name || obj.package || "bos.core";
      if (rawPkg === "bos.customizations" || obj.object_type?.includes("extension")) {
        const targetName =
          obj.definition?.target_page ||
          obj.definition?.target_table ||
          obj.target_page ||
          obj.target_table ||
          (obj.object_name || obj.name || "").replace(/_Customization$/, "");

        const targetObj = studioStore.catalog.find(
          (o) =>
            (o.object_name === targetName || o.name === targetName) &&
            o.package_name !== "bos.customizations"
        );

        if (targetObj?.package_name && pkgMap.has(targetObj.package_name)) {
          return targetObj.package_name;
        }
      }
      return rawPkg;
    }

    // 2. Associate catalog objects with target module
    for (const obj of studioStore.catalog) {
      const targetPkgId = resolveTargetPackage(obj);
      if (!pkgMap.has(targetPkgId)) {
        pkgMap.set(targetPkgId, {
          id: targetPkgId,
          package_id: targetPkgId,
          name: targetPkgId,
          version: "1.0.0",
          publisher: targetPkgId.startsWith("bos.") ? "BOS Core" : "Workspace",
          status: "active",
          isCore: targetPkgId === "bos.core" || targetPkgId === "core",
          objects: [],
        });
      }

      const targetName =
        obj.definition?.target_page ||
        obj.definition?.target_table ||
        obj.target_page ||
        obj.target_table ||
        (obj.object_type?.includes("extension")
          ? (obj.object_name || obj.name || "").replace(/_Customization$/, "")
          : null);

      pkgMap.get(targetPkgId).objects.push({
        ...obj,
        targetName,
        feature: obj.feature || obj.definition?.feature || null,
        isCustomization: obj.package_name === "bos.customizations" || obj.object_type?.includes("extension"),
      });
    }

    // Helper: Group an array of meta-objects into TYPE_GROUPS
    function groupObjectsByType(rawObjs) {
      const groups = [];
      for (const g of TYPE_GROUPS) {
        if (catFilter !== "all" && catFilter !== g.id) continue;

        let items = [];
        if (g.id === "table") {
          const tableObjs = rawObjs.filter((o) => g.types.includes(o.object_type));
          for (const tbl of tableObjs) {
            const companion = findCompanionLogic(tbl.name, rawObjs);
            const tblMatches = !query || tbl.name.toLowerCase().includes(query) || (tbl.caption && tbl.caption.toLowerCase().includes(query));
            const compMatches = companion && (!query || companion.name.toLowerCase().includes(query));
            if (tblMatches || compMatches) {
              items.push({ ...tbl, companion });
            }
          }
        } else {
          items = rawObjs.filter((o) => {
            if (!g.types.includes(o.object_type)) return false;
            if (!query) return true;
            return (
              o.name.toLowerCase().includes(query) ||
              (o.caption && o.caption.toLowerCase().includes(query)) ||
              (o.targetName && o.targetName.toLowerCase().includes(query))
            );
          });
        }

        if (items.length > 0) {
          groups.push({
            id: g.id,
            label: g.label,
            icon: g.icon,
            iconColor: g.iconColor,
            badgeColor: g.badgeColor,
            count: items.length,
            items,
          });
        }
      }
      return groups;
    }

    const result = [];

    for (const [pkgId, pkgData] of pkgMap.entries()) {
      const allObjs = pkgData.objects;

      // 3. Extract distinct features defined in this module
      const featureDefsMap = new Map();

      // Registered feature meta-objects
      for (const o of allObjs) {
        if (o.object_type === "feature") {
          featureDefsMap.set(o.name, {
            id: o.name,
            name: o.name,
            caption: o.caption || o.name,
            description: o.definition?.description || "",
            status: o.definition?.status || "active",
            rawObject: o,
            objects: [],
          });
        }
      }

      // Discovered feature references from object definitions
      for (const o of allObjs) {
        if (o.object_type !== "feature" && o.feature && !featureDefsMap.has(o.feature)) {
          featureDefsMap.set(o.feature, {
            id: o.feature,
            name: o.feature,
            caption: o.feature,
            description: "",
            status: "active",
            rawObject: null,
            objects: [],
          });
        }
      }

      // Distribute non-feature meta-objects to feature groups or unassigned
      const unassignedObjs = [];
      for (const o of allObjs) {
        if (o.object_type === "feature") continue;
        if (o.feature && featureDefsMap.has(o.feature)) {
          featureDefsMap.get(o.feature).objects.push(o);
        } else {
          unassignedObjs.push(o);
        }
      }

      // Build structured feature groups
      const featureGroups = [];
      for (const [featId, featData] of featureDefsMap.entries()) {
        const featMatches = !query || featId.toLowerCase().includes(query) || featData.caption.toLowerCase().includes(query);
        const groups = groupObjectsByType(featData.objects);
        const totalObjCount = featData.objects.length;
        const visibleCount = groups.reduce((acc, g) => acc + g.count, 0);

        if (visibleCount > 0 || (featMatches && catFilter === "all")) {
          featureGroups.push({
            ...featData,
            totalCount: totalObjCount,
            visibleCount,
            groups,
          });
        }
      }

      featureGroups.sort((a, b) => a.caption.localeCompare(b.caption));

      // Build unassigned groups
      const unassignedGroups = groupObjectsByType(unassignedObjs);
      const unassignedVisibleCount = unassignedGroups.reduce((acc, g) => acc + g.count, 0);

      // Direct flat type groups (used if module has no features)
      const nonFeatureObjs = allObjs.filter((o) => o.object_type !== "feature");
      const directTypeGroups = groupObjectsByType(nonFeatureObjs);

      const hasFeatures = featureDefsMap.size > 0;
      const visibleCount = hasFeatures
        ? featureGroups.reduce((acc, f) => acc + f.visibleCount, 0) + unassignedVisibleCount
        : directTypeGroups.reduce((acc, g) => acc + g.count, 0);

      const pkgMatches = !query || pkgId.toLowerCase().includes(query) || pkgData.name.toLowerCase().includes(query);

      // Module is included if it has matching items or features
      if (visibleCount > 0 || (pkgMatches && catFilter === "all")) {
        result.push({
          ...pkgData,
          hasFeatures,
          features: featureGroups,
          unassigned: {
            totalCount: unassignedObjs.length,
            visibleCount: unassignedVisibleCount,
            groups: unassignedGroups,
          },
          directGroups: directTypeGroups,
          totalCount: nonFeatureObjs.length,
          visibleCount,
        });
      }
    }

    return result.sort((a, b) => {
      if (a.isCore && !b.isCore) return -1;
      if (!a.isCore && b.isCore) return 1;
      return a.id.localeCompare(b.id);
    });
  });

  function openObject(obj, mode = "design", pkgId = null) {
    if (!obj) return;
    selectedId = obj.id || obj.name;
    selectedObject = obj;
    selectedPkg = pkgId || obj.package_name || null;
    if (obj.object_type === "feature") {
      studioStore.openDetailCard("feature", obj);
      return;
    }
    studioStore.openObject(obj.name, mode, obj.object_type, selectedPkg);
  }

  function handleSelect(obj, pkgId) {
    selectedId = obj.id || obj.name;
    selectedObject = obj;
    selectedPkg = pkgId;
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
    class="fixed md:static inset-y-0 left-0 z-50 w-80 md:w-80 h-full border-r border-slate-200 dark:border-slate-800 bg-slate-50/90 dark:bg-slate-950/80 flex flex-col shrink-0 transition-transform duration-200 ease-in-out shadow-lg md:shadow-none"
  >
    <!-- Top Header: Architecture Title & Actions -->
    <div class="p-3 border-b border-slate-200/80 dark:border-slate-800/80 space-y-2.5 bg-white/60 dark:bg-slate-900/60 backdrop-blur-xs">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <div class="size-7 rounded-lg bg-indigo-600/10 dark:bg-indigo-400/10 text-indigo-600 dark:text-indigo-400 flex items-center justify-center">
            <Boxes class="size-4" />
          </div>
          <div>
            <div class="flex items-center gap-1.5">
              <span class="text-xs font-bold tracking-tight text-slate-900 dark:text-slate-100 uppercase">
                BOS MODULES
              </span>
              <span class="text-[10px] font-bold px-1.5 py-0.2 rounded-full bg-indigo-100 dark:bg-indigo-950 text-indigo-700 dark:text-indigo-300">
                {modules.length}
              </span>
            </div>
            <p class="text-[10px] text-slate-400 dark:text-slate-500">
              {studioStore.catalog.length} Meta-Objects registered
            </p>
          </div>
        </div>

        <div class="flex items-center gap-0.5">
          <button
            type="button"
            class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-200/60 dark:hover:bg-slate-800 transition-colors"
            title={allCollapsed ? "Expand All (Modules, Features & Groups)" : "Collapse All (Modules, Features & Groups)"}
            onclick={toggleAll}
            aria-label="Toggle collapse/expand all"
          >
            {#if allCollapsed}
              <ChevronsUpDown class="size-3.5" />
            {:else}
              <ChevronsDownUp class="size-3.5" />
            {/if}
          </button>
          <button
            type="button"
            class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-200/60 dark:hover:bg-slate-800"
            title="Refresh Catalog"
            onclick={() => studioStore.loadCatalog()}
            aria-label="Refresh catalog"
          >
            <RefreshCw class={"size-3.5 " + (studioStore.isLoadingCatalog ? "animate-spin" : "")} />
          </button>
          <button
            type="button"
            class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-200/60 dark:hover:bg-slate-800"
            onclick={() => studioStore.collapseNav()}
            aria-label="Collapse navigation"
          >
            <ChevronLeft class="size-4" />
          </button>
        </div>
      </div>

      <!-- Quick Action: Guided Architecture Addition -->
      <div class="space-y-1.5">
        <Button
          class="w-full justify-center text-xs h-8 shadow-xs font-semibold bg-indigo-600 hover:bg-indigo-700 text-white gap-1.5"
          size="sm"
          icon={Plus}
          onclick={() => studioStore.openArchitectureModal("module")}
        >
          Add Architecture Component...
        </Button>

        <div class="grid grid-cols-3 gap-1">
          <button
            type="button"
            class="flex items-center justify-center gap-1 py-1 px-1 rounded text-[10px] font-semibold bg-white hover:bg-slate-100 dark:bg-slate-900 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-200 transition-colors border border-slate-200 dark:border-slate-800 shadow-2xs"
            onclick={() => studioStore.openArchitectureModal("module")}
            title="Create new Module / Package"
          >
            <Package class="size-3 text-indigo-500 shrink-0" />
            <span class="truncate">+ Module</span>
          </button>
          <button
            type="button"
            class="flex items-center justify-center gap-1 py-1 px-1 rounded text-[10px] font-semibold bg-white hover:bg-slate-100 dark:bg-slate-900 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-200 transition-colors border border-slate-200 dark:border-slate-800 shadow-2xs"
            onclick={() => studioStore.openArchitectureModal("feature")}
            title="Create new Feature under module"
          >
            <Sparkles class="size-3 text-orange-500 shrink-0" />
            <span class="truncate">+ Feature</span>
          </button>
          <button
            type="button"
            class="flex items-center justify-center gap-1 py-1 px-1 rounded text-[10px] font-semibold bg-white hover:bg-slate-100 dark:bg-slate-900 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-200 transition-colors border border-slate-200 dark:border-slate-800 shadow-2xs"
            onclick={() => studioStore.openArchitectureModal("object")}
            title="Create new Meta-Object"
          >
            <Boxes class="size-3 text-blue-500 shrink-0" />
            <span class="truncate">+ Object</span>
          </button>
        </div>
      </div>

      <!-- Real-Time Search Bar -->
      <div class="relative">
        <Search class="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
        <input
          type="search"
          bind:value={studioStore.searchQuery}
          placeholder="Filter modules, features, or objects..."
          class="w-full text-xs pl-8 pr-7 py-1.5 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
        />
        {#if studioStore.searchQuery}
          <button
            type="button"
            class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
            onclick={() => (studioStore.searchQuery = "")}
            aria-label="Clear search"
          >
            <X class="size-3" />
          </button>
        {/if}
      </div>

      <!-- Category Filter Chips -->
      <div class="flex items-center gap-1 overflow-x-auto pb-0.5 no-scrollbar">
        {#each CATEGORIES as cat (cat.id)}
          <button
            type="button"
            class={
              "px-2 py-0.5 rounded text-[11px] font-medium whitespace-nowrap transition-colors " +
              (selectedCategory === cat.id
                ? "bg-indigo-600 text-white shadow-xs font-semibold"
                : "bg-slate-200/60 dark:bg-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700")
            }
            onclick={() => (selectedCategory = cat.id)}
          >
            {cat.label}
          </button>
        {/each}
      </div>
    </div>

    <!-- Modules, Features & Objects Tree List -->
    <div class="flex-1 min-h-0 overflow-y-auto px-2 py-2 space-y-1.5 focus:outline-none">
      {#if studioStore.isLoadingCatalog && studioStore.catalog.length === 0}
        <div class="py-12 flex flex-col items-center justify-center gap-2.5 text-xs text-slate-400">
          <Spinner size="md" />
          <p>Querying BOS meta-object catalog...</p>
        </div>
      {:else if studioStore.catalogError}
        <EmptyState title="Catalog unavailable" description={studioStore.catalogError} />
      {:else if modules.length === 0}
        <div class="py-10 text-center text-xs text-slate-400 px-4">
          <Boxes class="size-8 mx-auto mb-2 opacity-40 text-slate-400" />
          <p class="font-medium text-slate-600 dark:text-slate-300">No matching modules or features found</p>
          <p class="mt-1 text-[11px] text-slate-400">Try clearing your search query or switching to 'All'.</p>
        </div>
      {:else}
        <ContextMenu
          items={[
            { label: "Design View", icon: Pencil, action: () => openObject(selectedObject, "design", selectedPkg) },
            {
              label: "Datasheet",
              icon: TableProperties,
              disabled: !selectedObject || (selectedObject.object_type !== "table" && selectedObject.object_type !== "tableextension"),
              action: () => openObject(selectedObject, "datasheet", selectedPkg),
            },
            { label: "View Source Code", icon: FileCode, shortcut: "Ctrl+Enter", action: () => openObject(selectedObject, "code", selectedPkg) },
          ]}
        >
          <div class="space-y-1.5" role="tree" aria-label="BOS Modules, Features and Meta-Objects">
            {#each modules as mod (mod.id)}
              {@const isModExpanded = !collapsedModules[mod.id] || Boolean(studioStore.searchQuery)}
              <div class="rounded-lg border border-slate-200/90 dark:border-slate-800/90 bg-white dark:bg-slate-900/70 overflow-hidden shadow-2xs">
                <!-- Production-Grade Module Section Header -->
                <div class="group px-3 py-2 hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors">
                  <!-- Row 1: Module Identity, Badges & Elevated Action Dock -->
                  <div class="flex items-center justify-between gap-2">
                    <button
                      type="button"
                      data-module-id={mod.id}
                      class="flex items-center gap-2 min-w-0 flex-1 text-left cursor-pointer focus:outline-none"
                      onclick={() => toggleModule(mod.id)}
                      aria-expanded={isModExpanded}
                    >
                      <span class={"p-0.5 rounded text-slate-400 group-hover:text-slate-600 dark:group-hover:text-slate-200 transition-transform duration-150 " + (isModExpanded ? "rotate-90" : "")}>
                        <ChevronRight class="size-3.5" />
                      </span>
                      <Package class={"size-4 shrink-0 " + (mod.isCore ? "text-indigo-600 dark:text-indigo-400" : "text-emerald-600 dark:text-emerald-400")} />
                      <span class="text-xs font-bold font-mono text-slate-900 dark:text-slate-100 tracking-tight truncate">
                        {mod.id}
                      </span>
                    </button>

                    <!-- Right Controls: Elevated Floating Action Capsule & Counter -->
                    <div class="flex items-center gap-1.5 shrink-0">
                      <!-- Hover Actions Dock: Distinct floating pill with background, subtle border, shadow, and clear button targets -->
                      <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all duration-150 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-xs rounded-md px-1 py-0.5">
                        <button
                          type="button"
                          class="p-1 rounded hover:bg-orange-100/70 dark:hover:bg-orange-950/70 text-slate-500 hover:text-orange-600 dark:text-slate-400 dark:hover:text-orange-400 transition-colors"
                          title="Add Feature"
                          onclick={(e) => {
                            e.stopPropagation();
                            studioStore.openArchitectureModal("feature", mod.id);
                          }}
                          aria-label="Add feature to module"
                        >
                          <Sparkles class="size-3.5" />
                        </button>
                        <button
                          type="button"
                          class="p-1 rounded hover:bg-indigo-100/70 dark:hover:bg-indigo-950/70 text-slate-500 hover:text-indigo-600 dark:text-slate-400 dark:hover:text-indigo-400 transition-colors"
                          title="Add Meta-Object"
                          onclick={(e) => {
                            e.stopPropagation();
                            studioStore.openArchitectureModal("object", mod.id);
                          }}
                          aria-label="Add object to module"
                        >
                          <Plus class="size-3.5" />
                        </button>
                        <button
                          type="button"
                          class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200 transition-colors"
                          title="Module Details"
                          onclick={(e) => {
                            e.stopPropagation();
                            studioStore.openDetailCard("module", mod);
                          }}
                          aria-label="Inspect module"
                        >
                          <Info class="size-3.5" />
                        </button>
                        <div class="h-3 w-px bg-slate-200 dark:bg-slate-700 mx-0.5"></div>
                        <!-- Module Level Collapse All -->
                        <button
                          type="button"
                          class="p-1 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200 transition-colors"
                          title={isModuleFeaturesCollapsed(mod) ? `Expand all in ${mod.id}` : `Collapse all in ${mod.id}`}
                          onclick={(e) => toggleModuleCollapseAll(mod, e)}
                          aria-label="Toggle collapse all in module"
                        >
                          {#if isModuleFeaturesCollapsed(mod)}
                            <ChevronsUpDown class="size-3.5" />
                          {:else}
                            <ChevronsDownUp class="size-3.5" />
                          {/if}
                        </button>
                      </div>

                      <!-- Count Badge -->
                      <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 font-mono border border-slate-200/80 dark:border-slate-700/80 min-w-5 text-center">
                        {mod.visibleCount}
                      </span>
                    </div>
                  </div>

                  <!-- Row 2: Full-Length Description + Metadata Tags (CORE & Version) -->
                  <div class="mt-1 w-full pl-6 pr-1 flex items-center justify-between gap-2">
                    <button
                      type="button"
                      class="min-w-0 flex-1 text-left cursor-pointer focus:outline-none"
                      onclick={() => toggleModule(mod.id)}
                    >
                      <p class="text-[11px] font-medium text-slate-600 dark:text-slate-400 leading-snug line-clamp-2">
                        {mod.name && mod.name !== mod.id ? mod.name : 'Core system components and business logic'}
                      </p>
                    </button>
                    <div class="flex items-center gap-1.5 shrink-0">
                      {#if mod.isCore}
                        <span class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border border-indigo-200/60 dark:border-indigo-800/60 uppercase tracking-wider">
                          CORE
                        </span>
                      {/if}
                      <span class="text-[9px] font-mono text-slate-400 dark:text-slate-500 bg-slate-100 dark:bg-slate-800/80 px-1.5 py-0.5 rounded border border-slate-200/60 dark:border-slate-700/60">
                        v{mod.version}
                      </span>
                    </div>
                  </div>
                </div>

                <!-- Module Contents: Features & Objects -->
                {#if isModExpanded}
                  <div data-module-content={mod.id} class="border-t border-slate-100 dark:border-slate-800/50 bg-slate-50/50 dark:bg-slate-950/40 p-1 space-y-1.5">
                    {#if mod.hasFeatures}
                      <!-- 1. Render Registered Features -->
                      {#each mod.features as feat (`${mod.id}:${feat.id}`)}
                        {@const featKey = `${mod.id}:${feat.id}`}
                        {@const isFeatExpanded = !collapsedFeatures[featKey] || Boolean(studioStore.searchQuery)}
                        <div class="rounded-md border border-slate-200/90 dark:border-slate-800/90 border-l-[3px] border-l-amber-500 bg-white dark:bg-slate-900/70 overflow-hidden shadow-2xs">
                          <!-- Feature Header -->
                          <div class="group flex items-center justify-between px-2.5 py-1.5 hover:bg-amber-50/40 dark:hover:bg-amber-950/20 transition-colors">
                            <button
                              type="button"
                              class="flex items-center gap-1.5 min-w-0 flex-1 text-left cursor-pointer focus:outline-none"
                              onclick={() => toggleFeature(featKey)}
                              aria-expanded={isFeatExpanded}
                            >
                              <span class={"transition-transform duration-100 " + (isFeatExpanded ? "rotate-90" : "")}>
                                <ChevronRight class="size-3 text-amber-500" />
                              </span>
                              <Sparkles class="size-3.5 text-amber-500 shrink-0" />
                              <div class="min-w-0 flex-1 pr-1">
                                <span class="text-xs font-bold text-slate-800 dark:text-slate-100 tracking-tight leading-snug line-clamp-2" title={feat.caption}>
                                  {feat.caption || feat.name}
                                </span>
                              </div>
                            </button>

                            <div class="flex items-center gap-1 shrink-0 pl-1">
                              <!-- Feature Hover Action Capsule -->
                              <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all duration-150 bg-white dark:bg-slate-800 border border-amber-200 dark:border-amber-800/80 shadow-xs rounded-md px-1 py-0.5">
                                <button
                                  type="button"
                                  class="p-0.5 rounded hover:bg-amber-100 dark:hover:bg-amber-950/80 text-amber-700 dark:text-amber-300 transition-colors"
                                  title="Add Object to Feature"
                                  onclick={(e) => {
                                    e.stopPropagation();
                                    studioStore.openArchitectureModal("object", mod.id, feat.id);
                                  }}
                                  aria-label="Add object to feature"
                                >
                                  <Plus class="size-3.5" />
                                </button>
                                <button
                                  type="button"
                                  class="p-0.5 rounded hover:bg-amber-100 dark:hover:bg-amber-950/80 text-amber-700 dark:text-amber-300 transition-colors"
                                  title="Feature Details"
                                  onclick={(e) => {
                                    e.stopPropagation();
                                    studioStore.openDetailCard("feature", feat.rawObject || { ...feat, package_name: mod.id });
                                  }}
                                  aria-label="Inspect feature"
                                >
                                  <Info class="size-3.5" />
                                </button>
                                <div class="h-3 w-px bg-amber-200 dark:bg-amber-800/60 mx-0.5"></div>
                                <button
                                  type="button"
                                  class="p-0.5 rounded hover:bg-amber-100 dark:hover:bg-amber-950/80 text-amber-700 dark:text-amber-300 transition-colors"
                                  title={isFeatureGroupsCollapsed(mod.id, feat) ? `Expand all groups in ${feat.caption}` : `Collapse all groups in ${feat.caption}`}
                                  onclick={(e) => toggleFeatureCollapseAll(mod.id, feat, e)}
                                  aria-label="Toggle collapse all groups in feature"
                                >
                                  {#if isFeatureGroupsCollapsed(mod.id, feat)}
                                    <ChevronsUpDown class="size-3.5" />
                                  {:else}
                                    <ChevronsDownUp class="size-3.5" />
                                  {/if}
                                </button>
                              </div>

                              <span class="text-[10px] font-bold px-1.5 py-0.2 rounded border bg-amber-50 dark:bg-amber-950/60 text-amber-800 dark:text-amber-300 border-amber-200/80 dark:border-amber-800/80 min-w-5 text-center">
                                {feat.visibleCount}
                              </span>
                            </div>
                          </div>

                          <!-- Objects inside this Feature -->
                          {#if isFeatExpanded}
                            <div class="border-t border-orange-100/60 dark:border-orange-950/40 bg-slate-50/40 dark:bg-slate-950/30 p-1 space-y-1">
                              {#if feat.groups.length === 0}
                                <p class="text-[10px] text-slate-400 px-3 py-1.5 italic">
                                  No objects attached to this feature.
                                </p>
                              {:else}
                                {#each feat.groups as grp (`${mod.id}:${feat.id}:${grp.id}`)}
                                  {@const subId = `${mod.id}:${feat.id}:${grp.id}`}
                                  {@const isSubExpanded = !collapsedSubgroups[subId] || Boolean(studioStore.searchQuery)}
                                  <div class="space-y-0.5">
                                    <button
                                      type="button"
                                      class="w-full flex items-center justify-between px-2 py-1 rounded text-[11px] font-bold text-slate-700 dark:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-800/50 transition-colors"
                                      onclick={() => toggleSubgroup(subId)}
                                    >
                                      <div class="flex items-center gap-1.5">
                                        <span class={"transition-transform duration-100 " + (isSubExpanded ? "rotate-90" : "")}>
                                          <ChevronRight class="size-2.5 text-slate-400" />
                                        </span>
                                        <grp.icon class={"size-3 " + grp.iconColor} />
                                        <span class="font-semibold text-slate-700 dark:text-slate-200">{grp.label}</span>
                                      </div>
                                      <span class={"text-[9px] font-bold px-1.5 py-0.1 rounded border " + grp.badgeColor}>
                                        {grp.count}
                                      </span>
                                    </button>

                                    {#if isSubExpanded}
                                      <div class="pl-3 pr-1 space-y-0.5">
                                        {#each grp.items as item (item.id || item.name)}
                                          {@const isSelected = selectedId === (item.id || item.name) || studioStore.activeTabId === `${item.object_type}-${item.name}`}
                                          {@const vis = getObjectVisuals(item)}
                                          {@const ItemIcon = vis.icon}
                                          <div
                                            class={
                                              "group relative flex items-center justify-between px-2 py-1 rounded-md text-xs cursor-pointer transition-colors border " +
                                              (isSelected
                                                ? "bg-indigo-50/90 dark:bg-indigo-950/60 border-indigo-200 dark:border-indigo-800 text-indigo-900 dark:text-indigo-200 font-semibold shadow-2xs"
                                                : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")
                                            }
                                            role="treeitem"
                                            aria-selected={isSelected}
                                            tabindex="0"
                                            onclick={() => openObject(item, "design", mod.id)}
                                            oncontextmenu={() => handleSelect(item, mod.id)}
                                            onkeydown={(e) => {
                                              if (e.key === "Enter") openObject(item, "design", mod.id);
                                            }}
                                          >
                                            <div class="flex items-center gap-1.5 min-w-0 flex-1 pr-1">
                                              <ItemIcon class={"size-3.5 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity " + vis.color} />
                                              <span class="truncate font-medium text-xs text-slate-700 dark:text-slate-300 group-hover:text-slate-900 dark:group-hover:text-slate-100" title={item.caption ? `${item.name} (${item.caption})` : item.name}>
                                                {item.name}
                                              </span>
                                            </div>

                                            <div class="flex items-center gap-1 shrink-0 ml-auto">
                                              <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all duration-150 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-2xs rounded px-1 py-0.5 shrink-0">
                                                {#if item.object_type === "table" || item.object_type === "tableextension"}
                                                  <button
                                                    type="button"
                                                    class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                    title="Open Datasheet"
                                                    onclick={(e) => {
                                                      e.stopPropagation();
                                                      openObject(item, "datasheet", mod.id);
                                                    }}
                                                    aria-label="Open datasheet"
                                                  >
                                                    <TableProperties class="size-3" />
                                                  </button>
                                                {/if}
                                                <button
                                                  type="button"
                                                  class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                  title="Open Designer"
                                                  onclick={(e) => {
                                                    e.stopPropagation();
                                                    openObject(item, "design", mod.id);
                                                  }}
                                                  aria-label="Open designer"
                                                >
                                                  <Pencil class="size-3" />
                                                </button>
                                                <button
                                                  type="button"
                                                  class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                  title="View Code (Ctrl+Enter)"
                                                  onclick={(e) => {
                                                    e.stopPropagation();
                                                    openObject(item, "code", mod.id);
                                                  }}
                                                  aria-label="View code"
                                                >
                                                  <FileCode class="size-3" />
                                                </button>
                                              </div>
                                            </div>
                                          </div>

                                          <!-- Nested Companion TableLogic -->
                                          {#if item.companion}
                                            {@const comp = item.companion}
                                            {@const isCompSelected = selectedId === (comp.id || comp.name)}
                                            <div
                                              class={
                                                "group relative ml-3 flex items-center justify-between px-2 py-1 rounded text-[11px] cursor-pointer transition-colors border " +
                                                (isCompSelected
                                                  ? "bg-emerald-50/90 dark:bg-emerald-950/50 border-emerald-200 dark:border-emerald-800 text-emerald-900 dark:text-emerald-200 font-medium"
                                                  : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-600 dark:text-slate-400")
                                              }
                                              role="treeitem"
                                              aria-selected={isCompSelected}
                                              tabindex="0"
                                              onclick={() => openObject(comp, "code", mod.id)}
                                              oncontextmenu={() => handleSelect(comp, mod.id)}
                                              onkeydown={(e) => {
                                                if (e.key === "Enter") openObject(comp, "code", mod.id);
                                              }}
                                            >
                                              <div class="flex items-center gap-1.5 min-w-0 pr-1 flex-1">
                                                <Cpu class="size-3 text-emerald-500/70 dark:text-emerald-400/70 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity" />
                                                <span class="truncate font-mono text-[10px] text-slate-600 dark:text-slate-400 group-hover:text-emerald-800 dark:group-hover:text-emerald-200">
                                                  {comp.name}
                                                </span>
                                              </div>
                                              <div class="flex items-center gap-1 shrink-0 ml-auto">
                                                <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                                                  <button
                                                    type="button"
                                                    class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                                    title="View Code (Ctrl+Enter)"
                                                    onclick={(e) => {
                                                      e.stopPropagation();
                                                      openObject(comp, "code", mod.id);
                                                    }}
                                                    aria-label="View logic code"
                                                  >
                                                    <FileCode class="size-3" />
                                                  </button>
                                                </div>
                                              </div>
                                            </div>
                                          {/if}
                                        {/each}
                                      </div>
                                    {/if}
                                  </div>
                                {/each}
                              {/if}
                            </div>
                          {/if}
                        </div>
                      {/each}

                      <!-- 2. Render Unassigned Objects (if any exist) -->
                      {#if mod.unassigned.groups.length > 0}
                        {@const unassignedKey = `${mod.id}:__unassigned__`}
                        {@const isUnassignedExpanded = !collapsedFeatures[unassignedKey] || Boolean(studioStore.searchQuery)}
                        <div class="rounded-md border border-slate-200/90 dark:border-slate-800/90 border-l-[3px] border-l-slate-400 dark:border-l-slate-600 bg-white dark:bg-slate-900/70 overflow-hidden shadow-2xs">
                          <div class="group flex items-center justify-between px-2 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors">
                            <button
                              type="button"
                              class="flex items-center gap-1.5 min-w-0 flex-1 text-left cursor-pointer focus:outline-none"
                              onclick={() => toggleFeature(unassignedKey)}
                              aria-expanded={isUnassignedExpanded}
                            >
                              <span class={"transition-transform duration-100 " + (isUnassignedExpanded ? "rotate-90" : "")}>
                                <ChevronRight class="size-3 text-slate-400" />
                              </span>
                              <Folder class="size-3.5 text-slate-400 shrink-0" />
                              <span class="text-xs font-semibold text-slate-600 dark:text-slate-300 tracking-tight truncate">
                                General / Core Objects
                              </span>
                            </button>

                            <div class="flex items-center gap-1 shrink-0 pl-1.5">
                              <button
                                type="button"
                                class="p-0.5 rounded text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-200/60 dark:hover:bg-slate-800 transition-colors"
                                title={isUnassignedGroupsCollapsed(mod.id, mod.unassigned) ? "Expand all groups in General" : "Collapse all groups in General"}
                                onclick={(e) => toggleUnassignedCollapseAll(mod.id, mod.unassigned, e)}
                                aria-label="Toggle collapse all in general"
                              >
                                {#if isUnassignedGroupsCollapsed(mod.id, mod.unassigned)}
                                  <ChevronsUpDown class="size-3" />
                                {:else}
                                  <ChevronsDownUp class="size-3" />
                                {/if}
                              </button>
                              <span class="text-[10px] font-bold px-1.5 py-0.2 rounded border bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-700">
                                {mod.unassigned.visibleCount}
                              </span>
                            </div>
                          </div>

                          {#if isUnassignedExpanded}
                            <div class="border-t border-slate-100 dark:border-slate-800/50 bg-slate-50/40 dark:bg-slate-950/30 p-1 space-y-1">
                              {#each mod.unassigned.groups as grp (`${mod.id}:__unassigned__:${grp.id}`)}
                                {@const subId = `${mod.id}:__unassigned__:${grp.id}`}
                                {@const isSubExpanded = !collapsedSubgroups[subId] || Boolean(studioStore.searchQuery)}
                                <div class="space-y-0.5">
                                  <button
                                    type="button"
                                    class="w-full flex items-center justify-between px-2 py-1 rounded text-[11px] font-bold text-slate-700 dark:text-slate-200 hover:bg-slate-200/50 dark:hover:bg-slate-800/50 transition-colors"
                                    onclick={() => toggleSubgroup(subId)}
                                  >
                                    <div class="flex items-center gap-1.5">
                                      <span class={"transition-transform duration-100 " + (isSubExpanded ? "rotate-90" : "")}>
                                        <ChevronRight class="size-2.5 text-slate-400" />
                                      </span>
                                      <grp.icon class={"size-3 " + grp.iconColor} />
                                      <span class="font-semibold text-slate-700 dark:text-slate-200">{grp.label}</span>
                                    </div>
                                    <span class={"text-[9px] font-bold px-1.5 py-0.1 rounded border " + grp.badgeColor}>
                                      {grp.count}
                                    </span>
                                  </button>

                                  {#if isSubExpanded}
                                    <div class="pl-3 pr-1 space-y-0.5">
                                      {#each grp.items as item (item.id || item.name)}
                                        {@const isSelected = selectedId === (item.id || item.name) || studioStore.activeTabId === `${item.object_type}-${item.name}`}
                                        {@const vis = getObjectVisuals(item)}
                                        {@const ItemIcon = vis.icon}
                                        <div
                                          class={
                                            "group relative flex items-center justify-between px-2 py-1 rounded-md text-xs cursor-pointer transition-colors border " +
                                            (isSelected
                                              ? "bg-indigo-50/90 dark:bg-indigo-950/60 border-indigo-200 dark:border-indigo-800 text-indigo-900 dark:text-indigo-200 font-semibold shadow-2xs"
                                              : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")
                                          }
                                          role="treeitem"
                                          aria-selected={isSelected}
                                          tabindex="0"
                                          onclick={() => openObject(item, "design", mod.id)}
                                          oncontextmenu={() => handleSelect(item, mod.id)}
                                          onkeydown={(e) => {
                                            if (e.key === "Enter") openObject(item, "design", mod.id);
                                          }}
                                        >
                                          <div class="flex items-center gap-1.5 min-w-0 flex-1 pr-1">
                                            <ItemIcon class={"size-3.5 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity " + vis.color} />
                                            <span class="truncate font-medium text-xs text-slate-700 dark:text-slate-300 group-hover:text-slate-900 dark:group-hover:text-slate-100" title={item.caption ? `${item.name} (${item.caption})` : item.name}>
                                              {item.name}
                                            </span>
                                          </div>

                                          <div class="flex items-center gap-1 shrink-0 ml-auto">
                                            <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-all duration-150 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-2xs rounded px-1 py-0.5 shrink-0">
                                              {#if item.object_type === "table" || item.object_type === "tableextension"}
                                                <button
                                                  type="button"
                                                  class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                  title="Open Datasheet"
                                                  onclick={(e) => {
                                                    e.stopPropagation();
                                                    openObject(item, "datasheet", mod.id);
                                                  }}
                                                  aria-label="Open datasheet"
                                                >
                                                  <TableProperties class="size-3" />
                                                </button>
                                              {/if}
                                              <button
                                                type="button"
                                                class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                title="Open Designer"
                                                onclick={(e) => {
                                                  e.stopPropagation();
                                                  openObject(item, "design", mod.id);
                                                }}
                                                aria-label="Open designer"
                                              >
                                                <Pencil class="size-3" />
                                              </button>
                                              <button
                                                type="button"
                                                class="p-0.5 rounded hover:bg-slate-100 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
                                                title="View Code (Ctrl+Enter)"
                                                onclick={(e) => {
                                                  e.stopPropagation();
                                                  openObject(item, "code", mod.id);
                                                }}
                                                aria-label="View code"
                                              >
                                                <FileCode class="size-3" />
                                              </button>
                                            </div>
                                          </div>
                                        </div>

                                        <!-- Nested Companion TableLogic -->
                                        {#if item.companion}
                                          {@const comp = item.companion}
                                          {@const isCompSelected = selectedId === (comp.id || comp.name)}
                                          <div
                                            class={
                                              "group relative ml-3 flex items-center justify-between px-2 py-1 rounded text-[11px] cursor-pointer transition-colors border " +
                                              (isCompSelected
                                                ? "bg-emerald-50/90 dark:bg-emerald-950/50 border-emerald-200 dark:border-emerald-800 text-emerald-900 dark:text-emerald-200 font-medium"
                                                : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-600 dark:text-slate-400")
                                            }
                                            role="treeitem"
                                            aria-selected={isCompSelected}
                                            tabindex="0"
                                            onclick={() => openObject(comp, "code", mod.id)}
                                            oncontextmenu={() => handleSelect(comp, mod.id)}
                                            onkeydown={(e) => {
                                              if (e.key === "Enter") openObject(comp, "code", mod.id);
                                            }}
                                          >
                                            <div class="flex items-center gap-1.5 min-w-0 pr-1 flex-1">
                                              <Cpu class="size-3 text-emerald-500/70 dark:text-emerald-400/70 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity" />
                                              <span class="truncate font-mono text-[10px] text-slate-600 dark:text-slate-400 group-hover:text-emerald-800 dark:group-hover:text-emerald-200">
                                                {comp.name}
                                              </span>
                                            </div>
                                            <div class="flex items-center gap-1 shrink-0 ml-auto">
                                              <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                                                <button
                                                  type="button"
                                                  class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                                  title="View Code (Ctrl+Enter)"
                                                  onclick={(e) => {
                                                    e.stopPropagation();
                                                    openObject(comp, "code", mod.id);
                                                  }}
                                                  aria-label="View logic code"
                                                >
                                                  <FileCode class="size-3" />
                                                </button>
                                              </div>
                                            </div>
                                          </div>
                                        {/if}
                                      {/each}
                                    </div>
                                  {/if}
                                </div>
                              {/each}
                            </div>
                          {/if}
                        </div>
                      {/if}
                    {:else}
                      <!-- Module has NO features defined: Render Category Groups Directly -->
                      {#if mod.directGroups.length === 0}
                        <p class="text-[11px] text-slate-400 px-3 py-2 italic">
                          No meta-objects match the active category filter.
                        </p>
                      {:else}
                        {#each mod.directGroups as grp (`${mod.id}:${grp.id}`)}
                          {@const subId = `${mod.id}:${grp.id}`}
                          {@const isSubExpanded = !collapsedSubgroups[subId] || Boolean(studioStore.searchQuery)}
                          <div class="space-y-0.5">
                            <button
                              type="button"
                              class="w-full flex items-center justify-between px-2 py-1.5 rounded text-xs font-bold text-slate-800 dark:text-slate-100 hover:text-slate-900 dark:hover:text-white hover:bg-slate-200/50 dark:hover:bg-slate-800/50 transition-colors"
                              onclick={() => toggleSubgroup(subId)}
                            >
                              <div class="flex items-center gap-1.5">
                                <span class={"transition-transform duration-100 " + (isSubExpanded ? "rotate-90" : "")}>
                                  <ChevronRight class="size-3 text-slate-400" />
                                </span>
                                <grp.icon class={"size-3.5 " + grp.iconColor} />
                                <span class="font-bold text-slate-800 dark:text-slate-100 tracking-tight">{grp.label}</span>
                              </div>
                              <span class={"text-[10px] font-bold px-1.5 py-0.2 rounded border " + grp.badgeColor}>
                                {grp.count}
                              </span>
                            </button>

                            {#if isSubExpanded}
                              <div class="pl-4 pr-1 space-y-0.5">
                                {#each grp.items as item (item.id || item.name)}
                                  {@const isSelected = selectedId === (item.id || item.name) || studioStore.activeTabId === `${item.object_type}-${item.name}`}
                                  {@const vis = getObjectVisuals(item)}
                                  {@const ItemIcon = vis.icon}
                                  <div
                                    class={
                                      "group relative flex items-center justify-between px-2 py-1.5 rounded-md text-xs cursor-pointer transition-colors border " +
                                      (isSelected
                                        ? "bg-indigo-50/90 dark:bg-indigo-950/60 border-indigo-200 dark:border-indigo-800 text-indigo-900 dark:text-indigo-200 font-semibold shadow-2xs"
                                        : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-700 dark:text-slate-300")
                                    }
                                    role="treeitem"
                                    aria-selected={isSelected}
                                    tabindex="0"
                                    onclick={() => openObject(item, "design", mod.id)}
                                    oncontextmenu={() => handleSelect(item, mod.id)}
                                    onkeydown={(e) => {
                                      if (e.key === "Enter") openObject(item, "design", mod.id);
                                    }}
                                  >
                                    <div class="flex items-center gap-2 min-w-0 flex-1 pr-1">
                                      <ItemIcon class={"size-3.5 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity " + vis.color} />
                                      <span class="truncate font-medium text-xs text-slate-700 dark:text-slate-300 group-hover:text-slate-900 dark:group-hover:text-slate-100" title={item.caption ? `${item.name} (${item.caption})` : item.name}>
                                        {item.name}
                                      </span>
                                    </div>

                                    <div class="flex items-center gap-1 shrink-0 ml-auto">
                                      <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                                        {#if item.object_type === "table" || item.object_type === "tableextension"}
                                          <button
                                            type="button"
                                            class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                            title="Open Datasheet"
                                            onclick={(e) => {
                                              e.stopPropagation();
                                              openObject(item, "datasheet", mod.id);
                                            }}
                                            aria-label="Open datasheet"
                                          >
                                            <TableProperties class="size-3" />
                                          </button>
                                        {/if}
                                        <button
                                          type="button"
                                          class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                          title="Open Designer"
                                          onclick={(e) => {
                                            e.stopPropagation();
                                            openObject(item, "design", mod.id);
                                          }}
                                          aria-label="Open designer"
                                        >
                                          <Pencil class="size-3" />
                                        </button>
                                        <button
                                          type="button"
                                          class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                          title="View Code (Ctrl+Enter)"
                                          onclick={(e) => {
                                            e.stopPropagation();
                                            openObject(item, "code", mod.id);
                                          }}
                                          aria-label="View code"
                                        >
                                          <FileCode class="size-3" />
                                        </button>
                                      </div>
                                    </div>
                                  </div>

                                  <!-- Nested Companion TableLogic -->
                                  {#if item.companion}
                                    {@const comp = item.companion}
                                    {@const isCompSelected = selectedId === (comp.id || comp.name)}
                                    <div
                                      class={
                                        "group relative ml-3 flex items-center justify-between px-2 py-1 rounded text-[11px] cursor-pointer transition-colors border " +
                                        (isCompSelected
                                          ? "bg-emerald-50/90 dark:bg-emerald-950/50 border-emerald-200 dark:border-emerald-800 text-emerald-900 dark:text-emerald-200 font-medium"
                                          : "border-transparent hover:bg-slate-200/60 dark:hover:bg-slate-800/60 text-slate-600 dark:text-slate-400")
                                      }
                                      role="treeitem"
                                      aria-selected={isCompSelected}
                                      tabindex="0"
                                      onclick={() => openObject(comp, "code", mod.id)}
                                      oncontextmenu={() => handleSelect(comp, mod.id)}
                                      onkeydown={(e) => {
                                        if (e.key === "Enter") openObject(comp, "code", mod.id);
                                      }}
                                    >
                                      <div class="flex items-center gap-1.5 min-w-0 pr-1 flex-1">
                                        <Cpu class="size-3 text-emerald-500/70 dark:text-emerald-400/70 shrink-0 opacity-65 group-hover:opacity-100 transition-opacity" />
                                        <span class="truncate font-mono text-[10px] text-slate-600 dark:text-slate-400 group-hover:text-emerald-800 dark:group-hover:text-emerald-200">
                                          {comp.name}
                                        </span>
                                      </div>
                                      <div class="flex items-center gap-1 shrink-0 ml-auto">
                                        <div class="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
                                          <button
                                            type="button"
                                            class="p-0.5 rounded hover:bg-slate-300 dark:hover:bg-slate-700 text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
                                            title="View Code (Ctrl+Enter)"
                                            onclick={(e) => {
                                              e.stopPropagation();
                                              openObject(comp, "code", mod.id);
                                            }}
                                            aria-label="View logic code"
                                          >
                                            <FileCode class="size-3" />
                                          </button>
                                        </div>
                                      </div>
                                    </div>
                                  {/if}
                                {/each}
                              </div>
                            {/if}
                          </div>
                        {/each}
                      {/if}
                    {/if}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </ContextMenu>
      {/if}
    </div>
  </aside>
{:else}
  <!-- Collapsed Rail View -->
  <aside
    class="hidden md:flex h-full border-r border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-900/70 flex-col shrink-0 w-12"
  >
    <div class="flex-1 flex flex-col items-center py-3 gap-3">
      <Button
        variant="ghost"
        icon={Boxes}
        iconOnly
        tooltip="Expand Modules Navigation (Ctrl+B)"
        shortcut="Ctrl+B"
        aria-label="Expand navigation"
        onclick={() => studioStore.expandNav()}
      />
      <Button
        icon={Plus}
        iconOnly
        aria-label="Add Architecture Component"
        tooltip="Add Component"
        onclick={() => studioStore.openArchitectureModal("module")}
      />
    </div>
  </aside>
{/if}

<style>
  /* Hide scrollbar for category filter bar */
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
