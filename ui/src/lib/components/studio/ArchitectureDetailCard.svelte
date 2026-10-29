<script>
  /**
   * ArchitectureDetailCard.svelte (Svelte 5 Runes)
   * 100% Neat, Clean, Production-Quality Mobile-First Responsive Record Card.
   * Features searchable, filterable data table with clean type badges, responsive layout,
   * and zero text clipping or overlapping.
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
    Plus,
    X,
    ExternalLink,
    Search,
    Building2,
    Calendar,
    ArrowUpRight,
    TableProperties,
    Pencil,
    FileCode,
  } from "lucide-svelte";
  import { Dialog, Button } from "$lib/bos-ui/index.js";
  import { studioStore } from "./stores/studioStore.svelte.js";

  let detail = $derived(studioStore.selectedDetailCard);
  let isOpen = $derived(Boolean(detail));

  // Filters for objects inside the detail card
  let objectSearchQuery = $state("");
  let objectCategoryFilter = $state("all");

  function close() {
    studioStore.closeDetailCard();
    objectSearchQuery = "";
    objectCategoryFilter = "all";
  }

  // Derived objects in module/feature
  let moduleObjects = $derived.by(() => {
    if (!detail || detail.type !== "module") return [];
    const pkgId = detail.data.package_id || detail.data.id;
    return studioStore.catalog.filter((o) => o.package_name === pkgId);
  });

  let moduleFeatures = $derived.by(() => {
    if (!detail || detail.type !== "module") return [];
    const pkgId = detail.data.package_id || detail.data.id;
    const feats = [];
    const seen = new Set();
    for (const o of studioStore.catalog) {
      if (o.package_name === pkgId) {
        if (o.object_type === "feature" && !seen.has(o.name)) {
          seen.add(o.name);
          feats.push(o);
        } else if (o.feature && !seen.has(o.feature)) {
          seen.add(o.feature);
          feats.push({
            id: o.feature,
            name: o.feature,
            caption: o.feature,
            package_name: pkgId,
            object_type: "feature",
            definition: { status: "active", description: `Feature domain under ${pkgId}` }
          });
        }
      }
    }
    return feats;
  });

  let featureObjects = $derived.by(() => {
    if (!detail || detail.type !== "feature") return [];
    const featName = detail.data.name || detail.data.id;
    const pkgId = detail.data.package_name;
    return studioStore.catalog.filter(
      (o) => o.package_name === pkgId && o.object_type !== "feature" && (o.definition?.feature === featName || o.feature === featName)
    );
  });

  // Filtered module objects
  let filteredModuleObjects = $derived.by(() => {
    let list = moduleObjects;
    const q = objectSearchQuery.trim().toLowerCase();
    if (q) {
      list = list.filter((o) =>
        (o.name || "").toLowerCase().includes(q) ||
        (o.caption && o.caption.toLowerCase().includes(q)) ||
        (o.object_type || "").toLowerCase().includes(q) ||
        (o.sub_type && o.sub_type.toLowerCase().includes(q))
      );
    }
    if (objectCategoryFilter !== "all") {
      if (objectCategoryFilter === "table") {
        list = list.filter((o) => o.object_type === "table" || o.object_type === "tablelogic");
      } else if (objectCategoryFilter === "extension") {
        list = list.filter((o) => (o.object_type || "").includes("extension"));
      } else {
        list = list.filter((o) => o.object_type === objectCategoryFilter);
      }
    }
    return list;
  });

  // Category counts
  let countsByCategory = $derived.by(() => {
    const counts = { all: moduleObjects.length, table: 0, page: 0, codeunit: 0, statemachine: 0, enum: 0, extension: 0 };
    for (const o of moduleObjects) {
      const t = o.object_type || "table";
      if (t === "table" || t === "tablelogic") counts.table++;
      else if (t === "page") counts.page++;
      else if (t === "codeunit") counts.codeunit++;
      else if (t === "statemachine") counts.statemachine++;
      else if (t === "enum") counts.enum++;
      else if (t.includes("extension")) counts.extension++;
    }
    return counts;
  });

  function getBadgeConfig(type) {
    switch (type) {
      case "table":
        return {
          icon: Table,
          label: "Table",
          badgeStyle: "bg-blue-50 dark:bg-blue-950/60 text-blue-700 dark:text-blue-300 border-blue-200/60 dark:border-blue-800/60",
          iconColor: "text-blue-600 dark:text-blue-400",
        };
      case "tablelogic":
      case "tablelogicextension":
        return {
          icon: Cpu,
          label: "TableLogic",
          badgeStyle: "bg-emerald-50 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-300 border-emerald-200/60 dark:border-emerald-800/60",
          iconColor: "text-emerald-600 dark:text-emerald-400",
        };
      case "page":
        return {
          icon: Layout,
          label: "Page",
          badgeStyle: "bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border-indigo-200/60 dark:border-indigo-800/60",
          iconColor: "text-indigo-600 dark:text-indigo-400",
        };
      case "codeunit":
        return {
          icon: Cpu,
          label: "Codeunit",
          badgeStyle: "bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border-amber-200/60 dark:border-amber-800/60",
          iconColor: "text-amber-600 dark:text-amber-400",
        };
      case "statemachine":
        return {
          icon: Workflow,
          label: "Workflow",
          badgeStyle: "bg-rose-50 dark:bg-rose-950/60 text-rose-700 dark:text-rose-300 border-rose-200/60 dark:border-rose-800/60",
          iconColor: "text-rose-600 dark:text-rose-400",
        };
      case "enum":
      case "enumextension":
        return {
          icon: GitBranch,
          label: "Enum",
          badgeStyle: "bg-teal-50 dark:bg-teal-950/60 text-teal-700 dark:text-teal-300 border-teal-200/60 dark:border-teal-800/60",
          iconColor: "text-teal-600 dark:text-teal-400",
        };
      case "query":
      case "view":
        return {
          icon: ListFilter,
          label: "Query",
          badgeStyle: "bg-sky-50 dark:bg-sky-950/60 text-sky-700 dark:text-sky-300 border-sky-200/60 dark:border-sky-800/60",
          iconColor: "text-sky-600 dark:text-sky-400",
        };
      default:
        if (type && type.includes("extension")) {
          return {
            icon: Layers,
            label: "Extension",
            badgeStyle: "bg-purple-50 dark:bg-purple-950/60 text-purple-700 dark:text-purple-300 border-purple-200/60 dark:border-purple-800/60",
            iconColor: "text-purple-600 dark:text-purple-400",
          };
        }
        return {
          icon: Boxes,
          label: type,
          badgeStyle: "bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 border-slate-200 dark:border-slate-700",
          iconColor: "text-slate-500",
        };
    }
  }
</script>

{#if isOpen && detail}
  <Dialog
    open={isOpen}
    onclose={close}
    title={detail.type === "module" ? "Module Architecture Card" : "Feature Architecture Card"}
    description={detail.type === "module" ? "Detailed package manifest, registered features, and meta-objects." : "Feature capabilities, status, and attached meta-objects."}
    size="xl"
  >
    {#snippet children()}
      <div class="space-y-4 max-h-[75vh] overflow-y-auto pr-1">
        <!-- ================================================================= -->
        <!-- MODULE DETAIL CARD -->
        <!-- ================================================================= -->
        {#if detail.type === "module"}
          {@const mod = detail.data}
          
          <!-- Module Hero Banner -->
          <div class="p-4 rounded-xl border border-indigo-200/80 dark:border-indigo-900/60 bg-gradient-to-br from-indigo-50/70 via-white to-slate-50/50 dark:from-indigo-950/40 dark:via-slate-900 dark:to-slate-950/50 shadow-xs space-y-3">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div class="flex items-center gap-3">
                <div class="size-11 rounded-xl bg-indigo-600 text-white flex items-center justify-center font-bold shadow-md shrink-0">
                  <Package class="size-6" />
                </div>
                <div class="min-w-0">
                  <div class="flex items-center flex-wrap gap-2">
                    <h3 class="text-sm sm:text-base font-bold text-slate-900 dark:text-slate-100 font-mono tracking-tight truncate">
                      {mod.package_id || mod.id}
                    </h3>
                    {#if mod.isCore || mod.package_id === "bos.core"}
                      <span class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300 border border-indigo-200/50">
                        CORE
                      </span>
                    {/if}
                    <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-emerald-50 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-300 border border-emerald-200/60 flex items-center gap-1">
                      <span class="size-1.5 rounded-full bg-emerald-500"></span>
                      {mod.status || "active"}
                    </span>
                  </div>
                  <p class="text-xs text-slate-600 dark:text-slate-400 mt-0.5 font-medium">
                    {mod.name && mod.name !== mod.package_id ? mod.name : "Enterprise Business Module"}
                  </p>
                </div>
              </div>

              <div class="flex items-center gap-2 self-start sm:self-center">
                <span class="text-xs font-mono font-bold px-2.5 py-1 rounded-md bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-200 shadow-2xs">
                  v{mod.version || "1.0.0"}
                </span>
              </div>
            </div>

            <!-- Meta details grid -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-2 pt-3 border-t border-indigo-100/80 dark:border-indigo-900/40 text-xs">
              <div class="flex items-center gap-2 text-slate-500">
                <Building2 class="size-3.5 text-slate-400 shrink-0" />
                <span class="truncate">Publisher: <strong class="text-slate-700 dark:text-slate-300 font-semibold">{mod.publisher || "BOS Enterprise"}</strong></span>
              </div>
              <div class="flex items-center gap-2 text-slate-500">
                <Boxes class="size-3.5 text-slate-400 shrink-0" />
                <span>Objects: <strong class="text-slate-700 dark:text-slate-300 font-semibold">{moduleObjects.length} registered</strong></span>
              </div>
              <div class="flex items-center gap-2 text-slate-500">
                <Sparkles class="size-3.5 text-slate-400 shrink-0" />
                <span>Features: <strong class="text-slate-700 dark:text-slate-300 font-semibold">{moduleFeatures.length} capabilities</strong></span>
              </div>
            </div>
          </div>

          <!-- Features Section inside Module -->
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <h4 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <Sparkles class="size-3.5 text-orange-500" />
                Registered Features ({moduleFeatures.length})
              </h4>
              <Button
                variant="ghost"
                class="h-7 text-xs gap-1 text-orange-600 dark:text-orange-400 hover:bg-orange-50 dark:hover:bg-orange-950/50"
                onclick={() => {
                  studioStore.openArchitectureModal("feature", mod.package_id || mod.id);
                  close();
                }}
              >
                <Plus class="size-3.5" /> Add Feature
              </Button>
            </div>

            {#if moduleFeatures.length === 0}
              <div class="p-3.5 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-center text-xs text-slate-400 bg-slate-50/40 dark:bg-slate-950/20">
                No features registered yet under this module. Features are optional sub-capabilities.
              </div>
            {:else}
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
                {#each moduleFeatures as feat}
                  <div class="p-3 rounded-xl border border-slate-200/80 dark:border-slate-800 bg-white dark:bg-slate-900 flex items-start justify-between gap-3 shadow-2xs hover:border-orange-300 transition-colors">
                    <div class="min-w-0">
                      <div class="flex items-center gap-1.5">
                        <Sparkles class="size-3.5 text-orange-500 shrink-0" />
                        <span class="text-xs font-bold text-slate-800 dark:text-slate-200 truncate">
                          {feat.caption || feat.name}
                        </span>
                      </div>
                      <p class="text-[11px] font-mono text-slate-400 mt-0.5">{feat.name}</p>
                      {#if feat.definition?.description}
                        <p class="text-xs text-slate-500 line-clamp-1 mt-1">{feat.definition.description}</p>
                      {/if}
                    </div>
                    <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-orange-50 dark:bg-orange-950 text-orange-700 dark:text-orange-300 border border-orange-200/60 shrink-0">
                      {feat.definition?.status || "active"}
                    </span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <!-- ============================================================= -->
          <!-- 100% NEAT, CLEAN, RESPONSIVE DATA TABLE FOR META-OBJECTS -->
          <!-- ============================================================= -->
          <div class="space-y-2.5 pt-1">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
              <h4 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <Boxes class="size-3.5 text-indigo-500" />
                Meta-Objects in this Module ({filteredModuleObjects.length} / {moduleObjects.length})
              </h4>
              <Button
                variant="ghost"
                class="h-7 text-xs gap-1 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/50 self-start sm:self-auto"
                onclick={() => {
                  studioStore.openArchitectureModal("object", mod.package_id || mod.id);
                  close();
                }}
              >
                <Plus class="size-3.5" /> Add Meta-Object
              </Button>
            </div>

            <!-- Filter Controls: Search Bar & Type Chips -->
            <div class="space-y-2 p-2.5 rounded-xl border border-slate-200/80 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-900/50">
              <div class="flex flex-col sm:flex-row items-stretch sm:items-center gap-2">
                <!-- Search Input -->
                <div class="relative flex-1">
                  <Search class="size-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
                  <input
                    type="search"
                    bind:value={objectSearchQuery}
                    placeholder="Search objects by name, caption, or type..."
                    class="w-full text-xs pl-8 pr-7 py-1.5 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500"
                  />
                  {#if objectSearchQuery}
                    <button
                      type="button"
                      class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                      onclick={() => (objectSearchQuery = "")}
                    >
                      <X class="size-3" />
                    </button>
                  {/if}
                </div>

                <!-- Category Pills -->
                <div class="flex items-center gap-1 overflow-x-auto pb-0.5 no-scrollbar shrink-0">
                  <button
                    type="button"
                    class={
                      "px-2 py-1 rounded-md text-[11px] font-medium transition-colors " +
                      (objectCategoryFilter === "all"
                        ? "bg-indigo-600 text-white font-semibold shadow-2xs"
                        : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700 hover:bg-slate-100")
                    }
                    onclick={() => (objectCategoryFilter = "all")}
                  >
                    All ({countsByCategory.all})
                  </button>
                  <button
                    type="button"
                    class={
                      "px-2 py-1 rounded-md text-[11px] font-medium transition-colors " +
                      (objectCategoryFilter === "table"
                        ? "bg-blue-600 text-white font-semibold shadow-2xs"
                        : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700 hover:bg-slate-100")
                    }
                    onclick={() => (objectCategoryFilter = "table")}
                  >
                    Tables ({countsByCategory.table})
                  </button>
                  <button
                    type="button"
                    class={
                      "px-2 py-1 rounded-md text-[11px] font-medium transition-colors " +
                      (objectCategoryFilter === "page"
                        ? "bg-indigo-600 text-white font-semibold shadow-2xs"
                        : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700 hover:bg-slate-100")
                    }
                    onclick={() => (objectCategoryFilter = "page")}
                  >
                    Pages ({countsByCategory.page})
                  </button>
                  <button
                    type="button"
                    class={
                      "px-2 py-1 rounded-md text-[11px] font-medium transition-colors " +
                      (objectCategoryFilter === "codeunit"
                        ? "bg-amber-600 text-white font-semibold shadow-2xs"
                        : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700 hover:bg-slate-100")
                    }
                    onclick={() => (objectCategoryFilter = "codeunit")}
                  >
                    Codeunits ({countsByCategory.codeunit})
                  </button>
                  <button
                    type="button"
                    class={
                      "px-2 py-1 rounded-md text-[11px] font-medium transition-colors " +
                      (objectCategoryFilter === "enum"
                        ? "bg-teal-600 text-white font-semibold shadow-2xs"
                        : "bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700 hover:bg-slate-100")
                    }
                    onclick={() => (objectCategoryFilter = "enum")}
                  >
                    Enums ({countsByCategory.enum})
                  </button>
                </div>
              </div>
            </div>

            <!-- Clean Table Container -->
            <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-2xs">
              {#if filteredModuleObjects.length === 0}
                <div class="p-8 text-center text-xs text-slate-400 space-y-1">
                  <Boxes class="size-8 mx-auto text-slate-300 dark:text-slate-600" />
                  <p class="font-medium text-slate-600 dark:text-slate-300">No matching meta-objects found</p>
                  <p class="text-[11px]">Try adjusting your search query or category filter chip.</p>
                </div>
              {:else}
                <!-- Desktop / Tablet Table View -->
                <div class="hidden sm:block overflow-x-auto max-h-72 overflow-y-auto">
                  <table class="w-full text-left border-collapse text-xs">
                    <thead class="sticky top-0 bg-slate-100/90 dark:bg-slate-800/90 backdrop-blur-xs text-slate-600 dark:text-slate-300 border-b border-slate-200 dark:border-slate-700 text-[11px] font-bold uppercase tracking-wider select-none z-10">
                      <tr>
                        <th class="py-2.5 px-3 w-36">Meta Type</th>
                        <th class="py-2.5 px-3">Object Name & Caption</th>
                        <th class="py-2.5 px-3 w-40">Archetype / Feature</th>
                        <th class="py-2.5 px-3 text-right w-24">Action</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
                      {#each filteredModuleObjects as obj}
                        {@const badge = getBadgeConfig(obj.object_type)}
                        {@const Icon = badge.icon}
                        {@const hasDistinctCaption = obj.caption && obj.caption.trim() !== obj.name.trim()}
                        <tr class="hover:bg-slate-50/80 dark:hover:bg-slate-800/40 transition-colors group">
                          <!-- Type Column with Clean Colored Pill -->
                          <td class="py-2.5 px-3 align-middle">
                            <span class={"inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-semibold border " + badge.badgeStyle}>
                              <Icon class={"size-3.5 shrink-0 " + badge.iconColor} />
                              <span>{badge.label}</span>
                            </span>
                          </td>

                          <!-- Name & Caption Column -->
                          <td class="py-2.5 px-3 align-middle min-w-0">
                            <div class="min-w-0">
                              <span class="font-bold text-slate-900 dark:text-slate-100 font-mono text-xs truncate block" title={obj.name}>
                                {obj.name}
                              </span>
                              {#if hasDistinctCaption}
                                <span class="text-[11px] text-slate-500 dark:text-slate-400 truncate block mt-0.5">
                                  {obj.caption}
                                </span>
                              {/if}
                            </div>
                          </td>

                          <!-- Archetype / Feature Column -->
                          <td class="py-2.5 px-3 align-middle text-slate-500">
                            <div class="flex items-center gap-1.5 flex-wrap">
                              {#if obj.sub_type && obj.sub_type !== "Normal"}
                                <span class="text-[10px] font-mono px-1.5 py-0.2 rounded bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300 border border-slate-200/60 dark:border-slate-700/60">
                                  {obj.sub_type}
                                </span>
                              {/if}
                              {#if obj.definition?.feature || obj.feature}
                                <span class="text-[10px] font-medium px-1.5 py-0.2 rounded bg-orange-50 dark:bg-orange-950 text-orange-700 dark:text-orange-300 border border-orange-200/60 flex items-center gap-1">
                                  <Sparkles class="size-2.5" />
                                  <span>{obj.definition?.feature || obj.feature}</span>
                                </span>
                              {/if}
                            </div>
                          </td>

                          <!-- Actions Column -->
                          <td class="py-2.5 px-3 align-middle text-right">
                            <Button
                              variant="ghost"
                              class="h-7 text-xs px-2.5 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950 font-semibold gap-1"
                              onclick={() => {
                                studioStore.openObject(obj.name, "design", obj.object_type, mod.package_id || mod.id);
                                close();
                              }}
                            >
                              <span>Open</span>
                              <ArrowUpRight class="size-3" />
                            </Button>
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>

                <!-- Mobile-First Stacked Card View (< 640px) -->
                <div class="sm:hidden divide-y divide-slate-100 dark:divide-slate-800/60 max-h-72 overflow-y-auto">
                  {#each filteredModuleObjects as obj}
                    {@const badge = getBadgeConfig(obj.object_type)}
                    {@const Icon = badge.icon}
                    {@const hasDistinctCaption = obj.caption && obj.caption.trim() !== obj.name.trim()}
                    <div class="p-3 space-y-2 hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors">
                      <div class="flex items-center justify-between gap-2">
                        <span class={"inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-semibold border " + badge.badgeStyle}>
                          <Icon class={"size-3.5 " + badge.iconColor} />
                          <span>{badge.label}</span>
                        </span>

                        <Button
                          variant="ghost"
                          class="h-6 text-xs px-2 text-indigo-600 dark:text-indigo-400 font-semibold gap-1"
                          onclick={() => {
                            studioStore.openObject(obj.name, "design", obj.object_type, mod.package_id || mod.id);
                            close();
                          }}
                        >
                          <span>Open</span>
                          <ArrowUpRight class="size-3" />
                        </Button>
                      </div>

                      <div>
                        <span class="font-bold text-slate-900 dark:text-slate-100 font-mono text-xs block">
                          {obj.name}
                        </span>
                        {#if hasDistinctCaption}
                          <p class="text-[11px] text-slate-500 mt-0.5">{obj.caption}</p>
                        {/if}
                      </div>

                      {#if (obj.sub_type && obj.sub_type !== "Normal") || obj.definition?.feature || obj.feature}
                        <div class="flex items-center gap-1.5 flex-wrap pt-0.5">
                          {#if obj.sub_type && obj.sub_type !== "Normal"}
                            <span class="text-[10px] font-mono px-1.5 py-0.2 rounded bg-slate-100 dark:bg-slate-800 text-slate-600">
                              {obj.sub_type}
                            </span>
                          {/if}
                          {#if obj.definition?.feature || obj.feature}
                            <span class="text-[10px] font-medium px-1.5 py-0.2 rounded bg-orange-50 text-orange-700 flex items-center gap-1">
                              <Sparkles class="size-2.5" />
                              <span>{obj.definition?.feature || obj.feature}</span>
                            </span>
                          {/if}
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>

        <!-- ================================================================= -->
        <!-- FEATURE DETAIL CARD -->
        <!-- ================================================================= -->
        {:else if detail.type === "feature"}
          {@const feat = detail.data}
          <div class="p-4 rounded-xl border border-orange-200/80 dark:border-orange-900/60 bg-gradient-to-br from-orange-50/70 via-white to-slate-50/50 dark:from-orange-950/40 dark:via-slate-900 dark:to-slate-950/50 shadow-xs space-y-3">
            <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
              <div class="flex items-center gap-3">
                <div class="size-11 rounded-xl bg-orange-600 text-white flex items-center justify-center font-bold shadow-md shrink-0">
                  <Sparkles class="size-6" />
                </div>
                <div>
                  <div class="flex items-center gap-2">
                    <h3 class="text-sm sm:text-base font-bold text-slate-900 dark:text-slate-100">
                      {feat.caption || feat.name}
                    </h3>
                    <span class="text-[10px] font-semibold px-2 py-0.5 rounded-full bg-orange-100 dark:bg-orange-950 text-orange-800 dark:text-orange-300 border border-orange-200">
                      {feat.definition?.status || "active"}
                    </span>
                  </div>
                  <p class="text-xs font-mono text-slate-500 mt-0.5">
                    Identifier: {feat.name} · Parent Module: {feat.package_name}
                  </p>
                </div>
              </div>
            </div>

            {#if feat.definition?.description}
              <p class="text-xs text-slate-600 dark:text-slate-300 pt-2 border-t border-orange-100 dark:border-orange-900/40">
                {feat.definition.description}
              </p>
            {/if}
          </div>

          <!-- Objects attached to this Feature -->
          <div class="space-y-2.5 pt-1">
            <div class="flex items-center justify-between">
              <h4 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <Boxes class="size-3.5 text-blue-500" />
                Meta-Objects in this Feature ({featureObjects.length})
              </h4>
              <Button
                variant="ghost"
                class="h-7 text-xs gap-1 text-indigo-600 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950"
                onclick={() => {
                  studioStore.openArchitectureModal("object", feat.package_name, feat.name);
                  close();
                }}
              >
                <Plus class="size-3.5" /> Add Object to Feature
              </Button>
            </div>

            <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-2xs">
              {#if featureObjects.length === 0}
                <div class="p-8 text-center text-xs text-slate-400 space-y-1">
                  <Boxes class="size-8 mx-auto text-slate-300 dark:text-slate-600" />
                  <p class="font-medium text-slate-600 dark:text-slate-300">No objects assigned to this feature</p>
                  <p class="text-[11px]">When creating a table, page, or codeunit, select '{feat.name}' as the target feature.</p>
                </div>
              {:else}
                <div class="overflow-x-auto max-h-72 overflow-y-auto">
                  <table class="w-full text-left border-collapse text-xs">
                    <thead class="sticky top-0 bg-slate-100/90 dark:bg-slate-800/90 backdrop-blur-xs text-slate-600 dark:text-slate-300 border-b border-slate-200 dark:border-slate-700 text-[11px] font-bold uppercase tracking-wider">
                      <tr>
                        <th class="py-2.5 px-3 w-36">Meta Type</th>
                        <th class="py-2.5 px-3">Object Name</th>
                        <th class="py-2.5 px-3 text-right w-24">Action</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
                      {#each featureObjects as obj}
                        {@const badge = getBadgeConfig(obj.object_type)}
                        {@const Icon = badge.icon}
                        <tr class="hover:bg-slate-50 dark:hover:bg-slate-800/40 transition-colors">
                          <td class="py-2.5 px-3">
                            <span class={"inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-[11px] font-semibold border " + badge.badgeStyle}>
                              <Icon class={"size-3.5 " + badge.iconColor} />
                              <span>{badge.label}</span>
                            </span>
                          </td>
                          <td class="py-2.5 px-3 font-mono font-bold text-slate-800 dark:text-slate-200">
                            {obj.name}
                          </td>
                          <td class="py-2.5 px-3 text-right">
                            <Button
                              variant="ghost"
                              class="h-7 text-xs px-2.5 text-indigo-600 font-semibold gap-1"
                              onclick={() => {
                                studioStore.openObject(obj.name, "design", obj.object_type, feat.package_name);
                                close();
                              }}
                            >
                              <span>Open</span>
                              <ArrowUpRight class="size-3" />
                            </Button>
                          </td>
                        </tr>
                      {/each}
                    </tbody>
                  </table>
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {/snippet}

    {#snippet footer()}
      <Button variant="ghost" onclick={close}>Close</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .no-scrollbar::-webkit-scrollbar {
    display: none;
  }
  .no-scrollbar {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
</style>
