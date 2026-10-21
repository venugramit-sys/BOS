<script>
  /**
   * PageToolbox.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Control Toolbox
   * Palette of draggable enterprise layout containers, input fields, and action buttons.
   */
  import {
    LayoutGrid,
    Type,
    Hash,
    Calendar,
    CheckSquare,
    Layers,
    Search,
    Play,
    ChevronLeft,
    BarChart3,
    ListOrdered,
    PanelRight,
  } from "lucide-svelte";
  import { Button } from "$lib/bos-ui/index.js";

  let { onAddElement = null, onClose = null } = $props();

  const toolSections = [
    {
      title: "Containers & Layout",
      items: [
        { type: "fasttab", label: "FastTab Group", icon: Layers, desc: "Collapsible summary section" },
        { type: "grid", label: "2-Column Layout", icon: LayoutGrid, desc: "Responsive 2-col input matrix" },
        { type: "subform", label: "Subform (Grid)", icon: Layers, desc: "High-density child lines grid" },
        { type: "factbox", label: "FactBox Side Part", icon: PanelRight, desc: "Side panel of related facts" },
        { type: "step", label: "Wizard Step", icon: ListOrdered, desc: "Ordered step for Wizard layout" },
      ],
    },
    {
      title: "Data Controls",
      items: [
        { type: "text", label: "Text Box", icon: Type, desc: "Single-line string field" },
        { type: "decimal", label: "Decimal / Currency", icon: Hash, desc: "Formatted numeric input" },
        { type: "date", label: "Date Picker", icon: Calendar, desc: "Calendar selector" },
        { type: "lookup", label: "Lookup Dropdown", icon: Search, desc: "Foreign table relation lookup" },
        { type: "toggle", label: "Boolean Toggle", icon: CheckSquare, desc: "Yes/No switch" },
        { type: "tile", label: "KPI Cue Tile", icon: BarChart3, desc: "Count / metric tile for Workspace" },
      ],
    },
    {
      title: "Actions & Workflow",
      items: [
        { type: "action", label: "Action Button", icon: Play, desc: "Trigger procedural codeunit" },
      ],
    },
  ];
</script>

<aside class="w-60 border-r border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-900/60 p-3 flex flex-col shrink-0 select-none overflow-y-auto">
  <div class="text-xs font-bold text-slate-800 dark:text-slate-200 uppercase tracking-wider mb-3 flex items-center justify-between">
    <div class="flex items-center gap-1.5">
      <LayoutGrid class="size-3.5 text-indigo-500" />
      <span>Control Toolbox</span>
    </div>
    {#if onClose}
      <Button variant="ghost" size="xs" iconOnly icon={ChevronLeft} aria-label="Collapse toolbox" onclick={onClose} />
    {/if}
  </div>

  <div class="space-y-4">
    {#each toolSections as section}
      <div>
        <span class="text-[10px] font-bold text-slate-400 uppercase tracking-wider block mb-1.5">
          {section.title}
        </span>
        <div class="space-y-1">
          {#each section.items as item}
            <Button
              variant="secondary"
              class="w-full h-auto whitespace-normal justify-start py-2 text-left"
              icon={item.icon}
              onclick={() => onAddElement?.(item.type, item.label)}
            >
              <span class="min-w-0 text-left">
                <span class="block truncate">{item.label}</span>
                <span class="block truncate text-[10px] font-normal text-slate-400">{item.desc}</span>
              </span>
            </Button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</aside>
