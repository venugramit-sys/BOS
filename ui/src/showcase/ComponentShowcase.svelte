<script>
  import { Menubar, MenuHelpers } from "$lib/bos-ui/menu/index.js";
  import { Input } from "$lib/bos-ui/input/index.js";
  import { Combobox } from "$lib/bos-ui/combobox/index.js";
  import { DatePicker } from "$lib/bos-ui/date-picker/index.js";
  import { themeStore } from "$lib/bos-ui/theme/index.js";
  import ControlKitPanel from "./ControlKitPanel.svelte";
  import DataTablePanel from "./DataTablePanel.svelte";
  import DatasheetPanel from "./DatasheetPanel.svelte";
  import PageSurfacePanel from "./PageSurfacePanel.svelte";
  import StudioDesignersPanel from "./StudioDesignersPanel.svelte";
  import { AppShell, shell } from "$lib/components/shell/index.js";
  import { cn } from "$lib/utils.js";
  import {
    File,
    FolderOpen,
    Save,
    Share2,
    FileSpreadsheet,
    FileCode,
    FileText,
    Trash2,
    Undo2,
    Redo2,
    Scissors,
    Copy,
    ClipboardPaste,
    Sliders,
    Layers,
    Terminal,
    Sparkles,
    Sun,
    Moon,
    Laptop,
    HelpCircle,
    BookOpen,
    Info,
    CheckCircle2,
    Palette,
    Compass,
    Code2,
    Eye,
    Search,
    Lock,
    DollarSign,
    Hash,
    Calculator,
    Shield,
    Calendar,
    CalendarDays,
    Clock,
    Phone
  } from "lucide-svelte";

  // Active showcase category
  let activeCategory = $state("input");
  let previewCode = $state("C-10042");
  let previewCustomer = $state("C-10042");
  let previewDate = $state("2026-10-14");
  const surfacePreviewMenus = [
    {
      id: "file",
      label: "File",
      items: [
        { type: "item", label: "Save", shortcut: "Ctrl+S" },
        { type: "separator" },
        { type: "item", label: "Close" }
      ]
    }
  ];
  let darkMode = $state(false);
  let previewViewport = $state("desktop"); // 'desktop' | 'tablet' | 'mobile'

  // Dynamic Menubar Interactive State
  let showLineNumbers = $state(true);
  let wordWrap = $state(false);
  let showMinimap = $state(true);
  let themeMode = $state("system");
  let actionLogs = $state([]);

  // Dynamic Input Showcase Interactive State
  let testSearchVal = $state("Cronus North America");
  let testPasswordVal = $state("P@ssw0rd99!");
  let testQuantityVal = $state("25");
  let testCurrencyVal = $state("4,850.00");
  let testErpCodeVal = $state("CUST-1002");
  let testLookupVal = $state("NET-30");
  let testMathVal = $state("120 * 1.15");
  let testComplexMathVal = $state("(1500 - 250) / 2");
  let testCreditCardVal = $state("4532 8920 1823 4242");
  let testSsnVal = $state("123-45-6789");
  let testIbanVal = $state("US33CHAS01234567890123");
  let testInputSize = $state("md");
  let testInputStatus = $state("default");
  let testInputLoading = $state(false);
  let testInputClearable = $state(true);
  let testInputCopyable = $state(true);
  let testPhonePatternVal = $state("3125550143");
  let testEinPatternVal = $state("123456789");
  let testF8SourceVal = $state("GL-ACC-40100");
  let testF8TargetVal = $state("");

  // Combobox / Multi-Column Lookup Interactive State
  let selectedCustomer = $state("CUST-002");
  let multiSelectedCustomers = $state(["CUST-001", "CUST-003"]);
  let selectedItemCode = $state("ITEM-1001");
  let testComboboxDensity = $state("md"); // 'sm' | 'md' | 'lg'
  let testComboboxVariant = $state("outline"); // 'outline' | 'filled' | 'underline' | 'ghost'
  let testComboboxRounded = $state("md"); // 'none' | 'sm' | 'md' | 'pill'
  let testComboboxStatus = $state("default");
  let testComboboxLoading = $state(false);
  let testComboboxFactBox = $state(true);

  // Date / Time Picker Interactive State
  let testDateValue = $state("");
  let testDateRange = $state({ start: "", end: "" });
  let testDateWithTime = $state("");
  let testTimeValue = $state("09:00");
  let testDateDensity = $state("md");
  let testDateVariant = $state("outline");
  let testDateRounded = $state("md");
  const closedFiscalPeriods = [
    { start: "2025-01-01", end: "2025-03-31" }, // Q1 2025 locked
    { start: "2025-04-01", end: "2025-06-30" }  // Q2 2025 locked
  ];
  const demoCustomers = [
    { id: "CUST-001", name: "Progressive Home Furnishings", balance: 4250.0, city: "Chicago", phone: "+1 312 555-0143", creditLimit: 15000 },
    { id: "CUST-002", name: "First Up Consultants", balance: 12890.0, city: "London", phone: "+44 20 7946 0912", creditLimit: 25000 },
    { id: "CUST-003", name: "Graphic Design Institute", balance: 0.0, city: "Seattle", phone: "+1 206 555-0198", creditLimit: 5000 },
    { id: "CUST-004", name: "Wide World Importers", balance: 38500.0, city: "New York", phone: "+1 212 555-0177", creditLimit: 50000 },
    { id: "CUST-005", name: "Alpine Ski House", balance: 1120.0, city: "Zurich", phone: "+41 22 555-0188", creditLimit: 10000 },
    { id: "CUST-006", name: "Woodgrove Bank", balance: 0.0, city: "Frankfurt", phone: "+49 69 555-0123", creditLimit: 100000 }
  ];

  const customerColumns = [
    { key: "id", label: "No.", width: "100px", mono: true, primary: true },
    { key: "name", label: "Customer Name", minWidth: "180px" },
    { key: "balance", label: "Balance LCY", width: "110px", align: "right", format: "currency" },
    { key: "city", label: "City", width: "100px" }
  ];

  const demoItems = [
    { id: "ITEM-1001", name: 'Touring Bicycle Blue 28"', uom: "PCS", price: 1250.0, inventory: 42 },
    { id: "ITEM-1002", name: 'Mountain Bike Red 26"', uom: "PCS", price: 1400.0, inventory: 18 },
    { id: "ITEM-1003", name: "Road Cycling Helmet", uom: "PCS", price: 85.0, inventory: 120 },
    { id: "ITEM-1004", name: "Ergonomic Saddle Gel", uom: "PCS", price: 45.0, inventory: 8 },
    { id: "ITEM-1005", name: "Bicycle Floor Pump Alloy", uom: "PCS", price: 65.0, inventory: 55 }
  ];

  const itemColumns = [
    { key: "id", label: "Item No.", width: "100px", mono: true, primary: true },
    { key: "name", label: "Description", minWidth: "180px" },
    { key: "uom", label: "UOM", width: "70px", align: "center", mono: true },
    { key: "price", label: "Unit Price", width: "100px", align: "right", format: "currency" },
    { key: "inventory", label: "Stock", width: "80px", align: "right", format: "number" }
  ];

  function logAction(title, details = "") {
    const time = new Date().toLocaleTimeString();
    actionLogs = [{ id: Date.now(), time, title, details }, ...actionLogs.slice(0, 19)];
  }

  // Showcase Menubar configuration
  let demoMenus = $derived([
    {
      id: "file",
      label: "File",
      items: [
        {
          id: "new-file",
          label: "New File",
          icon: File,
          shortcut: "Ctrl+N",
          action: () => logAction("New File triggered")
        },
        {
          id: "open-file",
          label: "Open Project...",
          icon: FolderOpen,
          shortcut: "Ctrl+O",
          action: () => logAction("Open Project modal triggered")
        },
        MenuHelpers.separator(),
        {
          id: "save",
          label: "Save Document",
          icon: Save,
          shortcut: "Ctrl+S",
          action: () => logAction("Save Document triggered")
        },
        {
          type: "submenu",
          id: "export",
          label: "Export As",
          icon: Share2,
          badge: "New",
          badgeVariant: "accent",
          items: [
            {
              id: "export-pdf",
              label: "Adobe PDF (.pdf)",
              icon: FileText,
              action: () => logAction("Exported as PDF")
            },
            {
              id: "export-excel",
              label: "Excel Workbook (.xlsx)",
              icon: FileSpreadsheet,
              action: () => logAction("Exported as Excel Workbook")
            },
            {
              type: "submenu",
              id: "export-data",
              label: "Structured Data",
              icon: FileCode,
              items: [
                {
                  id: "export-json",
                  label: "JSON Schema (.json)",
                  action: () => logAction("Exported as JSON")
                },
                {
                  id: "export-csv",
                  label: "Comma Separated (.csv)",
                  action: () => logAction("Exported as CSV")
                }
              ]
            }
          ]
        },
        MenuHelpers.separator(),
        {
          id: "delete",
          label: "Delete Workspace",
          icon: Trash2,
          destructive: true,
          shortcut: "Del",
          action: () => logAction("Delete Workspace action fired")
        }
      ]
    },
    {
      id: "edit",
      label: "Edit",
      items: [
        {
          id: "undo",
          label: "Undo",
          icon: Undo2,
          shortcut: "Ctrl+Z",
          action: () => logAction("Undo executed")
        },
        {
          id: "redo",
          label: "Redo",
          icon: Redo2,
          shortcut: "Ctrl+Y",
          action: () => logAction("Redo executed")
        },
        MenuHelpers.separator(),
        {
          id: "cut",
          label: "Cut",
          icon: Scissors,
          shortcut: "Ctrl+X",
          action: () => logAction("Cut to clipboard")
        },
        {
          id: "copy",
          label: "Copy",
          icon: Copy,
          shortcut: "Ctrl+C",
          action: () => logAction("Copy to clipboard")
        },
        {
          id: "paste",
          label: "Paste",
          icon: ClipboardPaste,
          shortcut: "Ctrl+V",
          action: () => logAction("Pasted from clipboard")
        }
      ]
    },
    {
      id: "view",
      label: "View",
      items: [
        MenuHelpers.group("Editor Layout", [
          {
            type: "checkbox",
            id: "line-numbers",
            label: "Line Numbers",
            checked: showLineNumbers,
            shortcut: "Alt+L",
            onCheckedChange: (val) => {
              showLineNumbers = val;
              logAction("Toggled Line Numbers", `Value: ${val}`);
            }
          },
          {
            type: "checkbox",
            id: "word-wrap",
            label: "Word Wrap",
            checked: wordWrap,
            shortcut: "Alt+Z",
            onCheckedChange: (val) => {
              wordWrap = val;
              logAction("Toggled Word Wrap", `Value: ${val}`);
            }
          },
          {
            type: "checkbox",
            id: "minimap",
            label: "Code Minimap",
            checked: showMinimap,
            onCheckedChange: (val) => {
              showMinimap = val;
              logAction("Toggled Minimap", `Value: ${val}`);
            }
          }
        ]),
        MenuHelpers.separator(),
        MenuHelpers.group("Theme Mode", [
          {
            type: "radio-group",
            value: themeMode,
            options: [
              { value: "system", label: "Follow System" },
              { value: "light", label: "Light Theme" },
              { value: "dark", label: "Dark Theme" }
            ],
            onValueChange: (val) => {
              themeMode = val;
              logAction("Selected Theme Mode", `Theme: ${val}`);
            }
          }
        ])
      ]
    },
    {
      id: "tools",
      label: "Tools",
      items: [
        {
          id: "schema-inspector",
          label: "Schema Inspector",
          icon: Layers,
          badge: "BOS Meta",
          badgeVariant: "accent",
          action: () => logAction("Schema Inspector opened")
        },
        {
          id: "terminal",
          label: "CLI Sandbox",
          icon: Terminal,
          shortcut: "Ctrl+`",
          action: () => logAction("Sandbox Terminal toggled")
        },
        {
          id: "perf",
          label: "QuickJS Fuel Monitor",
          icon: Sparkles,
          action: () => logAction("Fuel Monitor launched")
        }
      ]
    },
    {
      id: "help",
      label: "Help",
      items: [
        {
          id: "docs",
          label: "Documentation",
          icon: BookOpen,
          href: "https://www.shadcn-svelte.com/docs/components/menubar",
          target: "_blank"
        },
        {
          id: "about",
          label: "About BOS Studio",
          icon: Info,
          action: () => logAction("About dialog opened")
        }
      ]
    }
  ]);
</script>

<svelte:head>
  <title>BOS Component Showcase & Design Guide</title>
</svelte:head>

<div class="min-h-screen flex flex-col font-sans transition-colors {darkMode ? 'dark bg-slate-950 text-slate-100' : 'bg-slate-50 text-slate-900'}">
  <!-- Showcase Top Bar -->
  <header class="border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-6 py-3 flex items-center justify-between shadow-xs sticky top-0 z-40">
    <div class="flex items-center gap-3">
      <div class="size-8 rounded-lg bg-indigo-600 text-white flex items-center justify-center font-bold text-sm shadow-xs">
        <Compass class="size-4" />
      </div>
      <div>
        <div class="flex items-center gap-2">
          <h1 class="font-bold text-sm tracking-tight">BOS Design Guide & Component Showcase</h1>
          <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-amber-100 dark:bg-amber-950/60 text-amber-800 dark:text-amber-300">
            Internal Dev Sandbox
          </span>
        </div>
        <p class="text-[11px] text-slate-400">Design testing, edge cases, and component refinement canvas (isolated from live app).</p>
      </div>
    </div>

    <div class="flex items-center gap-3">
      <button
        onclick={() => (darkMode = !darkMode)}
        class="px-2.5 py-1.5 rounded-lg border border-slate-200 dark:border-slate-800 hover:bg-slate-100 dark:hover:bg-slate-800 text-xs font-medium flex items-center gap-1.5 cursor-pointer transition-colors"
      >
        {#if darkMode}
          <Sun class="size-3.5 text-amber-400" />
          <span>Light Mode</span>
        {:else}
          <Moon class="size-3.5 text-slate-600" />
          <span>Dark Mode</span>
        {/if}
      </button>

      <a
        href="/"
        class="px-3 py-1.5 rounded-lg bg-slate-900 dark:bg-slate-100 text-white dark:text-slate-900 text-xs font-medium hover:opacity-90 transition-opacity"
      >
        Go to Live App
      </a>
    </div>
  </header>

  <!-- Showcase Workspace Layout -->
  <div class="flex-1 flex overflow-hidden">
    <!-- Sidebar: Component Directory -->
    <aside class="w-64 border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 p-4 space-y-6 shrink-0 hidden md:block">
      <div>
        <div class="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-2">Available Components</div>
        <nav class="space-y-1">
          <button
            onclick={() => (activeCategory = "app-shell")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'app-shell' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>App Shell Layout</span>
            <span class="size-2 rounded-full bg-indigo-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "menubar")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'menubar' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Menubar</span>
            <span class="size-2 rounded-full bg-emerald-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "input")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'input' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Input Field</span>
            <span class="size-2 rounded-full bg-violet-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "combobox")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'combobox' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Combobox / Lookup</span>
            <span class="size-2 rounded-full bg-amber-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "date-picker")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'date-picker' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Date / Time Picker</span>
            <span class="size-2 rounded-full bg-orange-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "datasheet")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'datasheet' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Datasheet</span>
            <span class="size-2 rounded-full bg-sky-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "data-table")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'data-table' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Data table</span>
            <span class="size-2 rounded-full bg-teal-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "control-kit")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'control-kit' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Studio control kit</span>
            <span class="size-2 rounded-full bg-indigo-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "page-surface")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'page-surface' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Page Surface (8 Layouts)</span>
            <span class="size-2 rounded-full bg-cyan-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "studio-designers")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'studio-designers' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Studio Visual Designers</span>
            <span class="size-2 rounded-full bg-fuchsia-500"></span>
          </button>
          <button
            onclick={() => (activeCategory = "theme")}
            class="w-full text-left px-3 py-2 rounded-lg text-xs font-medium flex items-center justify-between transition-colors cursor-pointer {activeCategory === 'theme' ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-700 dark:text-indigo-300 font-semibold' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
          >
            <span>Theme & Personalization</span>
            <span class="size-2 rounded-full bg-rose-500"></span>
          </button>
        </nav>
      </div>

      <div>
        <div class="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-2">Phase 1-4 Architecture</div>
        <div class="space-y-1 text-xs text-slate-400">
          <div class="px-3 py-1.5 rounded flex items-center justify-between text-emerald-600 dark:text-emerald-400 font-medium">
            <span>FactBox Side Panel</span>
            <span class="text-[10px] font-mono">docs/28</span>
          </div>
          <div class="px-3 py-1.5 rounded flex items-center justify-between text-emerald-600 dark:text-emerald-400 font-medium">
            <span>FastTab Card View</span>
            <span class="text-[10px] font-mono">docs/28</span>
          </div>
          <div class="px-3 py-1.5 rounded flex items-center justify-between text-emerald-600 dark:text-emerald-400 font-medium">
            <span>Argon2id & Storno</span>
            <span class="text-[10px] font-mono">v19 PHC</span>
          </div>
        </div>
      </div>
    </aside>

    <!-- Main Content Area -->
    <main class="flex-1 p-6 overflow-y-auto space-y-6 max-w-7xl mx-auto">
      <label class="md:hidden block text-xs font-medium text-slate-500">
        <span class="mb-1 block">Component</span>
        <select
          bind:value={activeCategory}
          class="h-9 w-full rounded-md border border-slate-200 bg-white px-2 text-sm text-slate-800 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
        >
          <option value="app-shell">App Shell Layout</option>
          <option value="menubar">Menubar</option>
          <option value="input">Input Field</option>
          <option value="combobox">Combobox / Lookup</option>
          <option value="date-picker">Date / Time Picker</option>
          <option value="datasheet">Datasheet</option>
          <option value="data-table">Data table</option>
          <option value="control-kit">Studio control kit</option>
          <option value="page-surface">Page Surface (8 Layouts)</option>
          <option value="studio-designers">Studio Visual Designers</option>
          <option value="theme">Theme & Personalization</option>
        </select>
      </label>
      {#if activeCategory === "app-shell"}
        <!-- Header for App Shell -->
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h2 class="text-xl font-bold tracking-tight">BOS App Shell & Responsive Layout</h2>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              Source: <code class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[11px] font-mono">$lib/components/shell</code>
            </p>
          </div>

          <!-- Viewport Mode Simulation Controls -->
          <div class="flex items-center gap-1.5 p-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 text-xs">
            <span class="text-[10px] font-semibold uppercase text-slate-400 px-2">Viewport:</span>
            <button
              onclick={() => (previewViewport = "desktop")}
              class="px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors {previewViewport === 'desktop' ? 'bg-indigo-600 text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
            >
              Desktop
            </button>
            <button
              onclick={() => (previewViewport = "tablet")}
              class="px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors {previewViewport === 'tablet' ? 'bg-indigo-600 text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
            >
              Tablet (768px)
            </button>
            <button
              onclick={() => (previewViewport = "mobile")}
              class="px-2.5 py-1 rounded text-xs font-medium cursor-pointer transition-colors {previewViewport === 'mobile' ? 'bg-indigo-600 text-white' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'}"
            >
              Mobile (375px)
            </button>
          </div>
        </div>

        <!-- Simulated Responsive Shell Frame -->
        <div class="p-2 sm:p-4 rounded-2xl bg-slate-200/70 dark:bg-slate-900/50 border border-slate-300 dark:border-slate-800 overflow-hidden flex justify-center">
          <div
            class={cn(
              "h-[640px] rounded-xl overflow-hidden shadow-2xl border border-slate-300 dark:border-slate-800 bg-white dark:bg-slate-950 transition-all duration-300 flex flex-col",
              previewViewport === "desktop" && "w-full",
              previewViewport === "tablet" && "w-[768px]",
              previewViewport === "mobile" && "w-[375px]"
            )}
          >
            <!-- Render Live AppShell -->
            <AppShell forceMobile={previewViewport === "mobile"}>
              {#snippet menubarSnippet()}
                <Menubar menus={demoMenus} />
              {/snippet}

              <!-- Document Surface Content -->
              <div class="p-4 space-y-4">
                {#if shell.activeTabId === "dashboard"}
                  <!-- Role Center View -->
                  <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
                    <div>
                      <h3 class="font-bold text-base">Sales Order Processor — Role Center</h3>
                      <p class="text-xs text-slate-500">Welcome back, John Doe. Here is your operational pulse.</p>
                    </div>
                  </div>

                  <!-- KPI Cue Tiles -->
                  <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
                    <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1 shadow-2xs">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Open Orders</span>
                      <div class="text-xl font-bold text-indigo-600 dark:text-indigo-400">18</div>
                      <span class="text-[10px] text-emerald-600 font-medium">↑ 4 new today</span>
                    </div>
                    <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1 shadow-2xs">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Pending Release</span>
                      <div class="text-xl font-bold text-amber-600 dark:text-amber-400">5</div>
                      <span class="text-[10px] text-slate-400 font-medium">Needs approval</span>
                    </div>
                    <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1 shadow-2xs">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Ready to Ship</span>
                      <div class="text-xl font-bold text-emerald-600 dark:text-emerald-400">12</div>
                      <span class="text-[10px] text-slate-400 font-medium">Warehouse queue</span>
                    </div>
                    <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1 shadow-2xs">
                      <span class="text-[10px] font-semibold uppercase tracking-wider text-slate-400">Delayed</span>
                      <div class="text-xl font-bold text-red-600 dark:text-red-400">1</div>
                      <span class="text-[10px] text-red-500 font-medium">Backordered</span>
                    </div>
                  </div>

                  <!-- Recent Orders Table -->
                  <div class="rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-2xs">
                    <div class="px-3 py-2 border-b border-slate-100 dark:border-slate-800 font-semibold text-xs flex items-center justify-between">
                      <span>Active Documents</span>
                      <span class="text-[10px] text-slate-400">Sorted by Date</span>
                    </div>
                    <table class="w-full text-left text-xs">
                      <thead class="bg-slate-50 dark:bg-slate-950/50 text-[10px] uppercase text-slate-400 font-semibold border-b border-slate-100 dark:border-slate-800">
                        <tr>
                          <th class="p-2.5">No.</th>
                          <th class="p-2.5">Customer Name</th>
                          <th class="p-2.5 hidden sm:table-cell">Status</th>
                          <th class="p-2.5 text-right">Amount</th>
                        </tr>
                      </thead>
                      <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
                        <tr class="hover:bg-slate-50 dark:hover:bg-slate-800/40 cursor-pointer">
                          <td class="p-2.5 font-medium text-indigo-600 dark:text-indigo-400">SO-1001</td>
                          <td class="p-2.5 font-medium">Tesla Inc.</td>
                          <td class="p-2.5 hidden sm:table-cell"><span class="px-2 py-0.5 rounded text-[10px] font-medium bg-emerald-100 text-emerald-800 dark:bg-emerald-950/60 dark:text-emerald-400">Released</span></td>
                          <td class="p-2.5 text-right font-mono font-medium">$42,300.00</td>
                        </tr>
                        <tr class="hover:bg-slate-50 dark:hover:bg-slate-800/40 cursor-pointer">
                          <td class="p-2.5 font-medium text-indigo-600 dark:text-indigo-400">SO-1002</td>
                          <td class="p-2.5 font-medium">Acme Corp</td>
                          <td class="p-2.5 hidden sm:table-cell"><span class="px-2 py-0.5 rounded text-[10px] font-medium bg-amber-100 text-amber-800 dark:bg-amber-950/60 dark:text-amber-400">Open</span></td>
                          <td class="p-2.5 text-right font-mono font-medium">$12,150.00</td>
                        </tr>
                        <tr class="hover:bg-slate-50 dark:hover:bg-slate-800/40 cursor-pointer">
                          <td class="p-2.5 font-medium text-indigo-600 dark:text-indigo-400">SO-1003</td>
                          <td class="p-2.5 font-medium">Contoso Ltd</td>
                          <td class="p-2.5 hidden sm:table-cell"><span class="px-2 py-0.5 rounded text-[10px] font-medium bg-blue-100 text-blue-800 dark:bg-blue-950/60 dark:text-blue-400">In Review</span></td>
                          <td class="p-2.5 text-right font-mono font-medium">$8,400.00</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>

                {:else if shell.activeTabId === "cust-1000"}
                  <!-- Customer Card View -->
                  <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
                    <div>
                      <h3 class="font-bold text-base">Customer Card: 10000 · Tesla Inc.</h3>
                      <p class="text-xs text-slate-500">Tier 1 Strategic Commercial Account · Austin, TX</p>
                    </div>
                  </div>

                  <!-- FastTab Section: General -->
                  <div class="rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 space-y-3 shadow-2xs">
                    <div class="font-bold text-xs uppercase tracking-wider text-slate-500">General Information</div>
                    <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
                      <div>
                        <label class="block text-[11px] text-slate-400 font-medium mb-1">
                          <span>Customer No.</span>
                          <input class="w-full mt-1 px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 font-normal" value="10000" readonly />
                        </label>
                      </div>
                      <div>
                        <label class="block text-[11px] text-slate-400 font-medium mb-1">
                          <span>Customer Name</span>
                          <input class="w-full mt-1 px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 font-normal" value="Tesla Inc." />
                        </label>
                      </div>
                      <div>
                        <label class="block text-[11px] text-slate-400 font-medium mb-1">
                          <span>Posting Group</span>
                          <input class="w-full mt-1 px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 font-normal" value="DOMESTIC" />
                        </label>
                      </div>
                      <div>
                        <label class="block text-[11px] text-slate-400 font-medium mb-1">
                          <span>Currency Code</span>
                          <input class="w-full mt-1 px-2.5 py-1.5 rounded border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950 font-normal" value="USD" />
                        </label>
                      </div>
                    </div>
                  </div>

                {:else}
                  <!-- Sales Orders List -->
                  <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
                    <div>
                      <h3 class="font-bold text-base">Sales Orders (Worksheet)</h3>
                      <p class="text-xs text-slate-500">High-density spreadsheet view with inline editing.</p>
                    </div>
                  </div>
                  <div class="p-6 rounded-lg border border-dashed border-slate-200 dark:border-slate-800 text-center text-xs text-slate-400">
                    High-density editable data grid ready for records.
                  </div>
                {/if}
              </div>
            </AppShell>
          </div>
        </div>

        <!-- Documentation & Architectural Notes -->
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2">
          <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1.5 shadow-2xs">
            <span class="font-bold text-xs uppercase tracking-wider text-indigo-600 dark:text-indigo-400">1. Mobile-First Ergonomics</span>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              On mobile viewports (&lt; 768px), the top sidebar collapses into an off-canvas drawer and primary actions shift down to the thumb-accessible bottom navigation bar and centered Floating Action Button (+).
            </p>
          </div>

          <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1.5 shadow-2xs">
            <span class="font-bold text-xs uppercase tracking-wider text-indigo-600 dark:text-indigo-400">2. Multi-Document Tabs</span>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              Users can keep multiple cards, lists, and reports open simultaneously with unsaved change badges (<span class="text-amber-500">•</span>) and keyboard shortcuts without losing drafted form data.
            </p>
          </div>

          <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 space-y-1.5 shadow-2xs">
            <span class="font-bold text-xs uppercase tracking-wider text-indigo-600 dark:text-indigo-400">3. Contextual FactBox Sidecar</span>
            <p class="text-xs text-slate-500 dark:text-slate-400">
              Right-hand analytical sidebar displays real-time ledger balances, credit gauges, and linked attachment counts, collapsable with a single click.
            </p>
          </div>
        </div>

      {:else if activeCategory === "menubar"}
        <!-- Header for Component -->
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-xl font-bold tracking-tight">Menubar Component</h2>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              Source: <code class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[11px] font-mono">$lib/bos-ui/menu</code>
            </p>
          </div>
          <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 flex items-center gap-1">
            <CheckCircle2 class="size-3.5" />
            Zero-Mock Verified
          </span>
        </div>

        <!-- Live Interactive Component Demonstration Container -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Live Component Preview</span>
            <span class="text-[11px] text-slate-400">Click any top-level menu or hover submenus below</span>
          </div>

          <!-- THE COMPONENT UNDER TEST -->
          <div class="p-4 rounded-xl bg-slate-100/70 dark:bg-slate-950/70 border border-slate-200 dark:border-slate-800 flex items-center justify-start">
            <Menubar
              menus={demoMenus}
              onSelect={(item) => logAction(`Selected: ${item.label || item.id}`)}
            />
          </div>

          <!-- Interactive Reactive State Indicators -->
          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
            <div class="p-3 rounded-lg border border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50">
              <span class="text-[10px] text-slate-400 font-medium uppercase tracking-wider">Line Numbers</span>
              <div class="text-sm font-semibold mt-1 {showLineNumbers ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400'}">
                {showLineNumbers ? 'Enabled' : 'Disabled'}
              </div>
            </div>

            <div class="p-3 rounded-lg border border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50">
              <span class="text-[10px] text-slate-400 font-medium uppercase tracking-wider">Word Wrap</span>
              <div class="text-sm font-semibold mt-1 {wordWrap ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400'}">
                {wordWrap ? 'Enabled' : 'Disabled'}
              </div>
            </div>

            <div class="p-3 rounded-lg border border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50">
              <span class="text-[10px] text-slate-400 font-medium uppercase tracking-wider">Minimap</span>
              <div class="text-sm font-semibold mt-1 {showMinimap ? 'text-indigo-600 dark:text-indigo-400' : 'text-slate-400'}">
                {showMinimap ? 'Visible' : 'Hidden'}
              </div>
            </div>

            <div class="p-3 rounded-lg border border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/50">
              <span class="text-[10px] text-slate-400 font-medium uppercase tracking-wider">Theme Mode</span>
              <div class="text-sm font-semibold mt-1 capitalize text-indigo-600 dark:text-indigo-400">
                {themeMode}
              </div>
            </div>
          </div>
        </div>

        <!-- 2 Column: Specs & Event Stream -->
        <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <!-- Code Usage & API Guide -->
          <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-5 shadow-xs space-y-3">
            <div class="flex items-center gap-2">
              <Code2 class="size-4 text-indigo-500" />
              <h3 class="font-bold text-xs uppercase tracking-wider">Schema Composition Guide</h3>
            </div>
            <pre class="p-3 rounded-lg bg-slate-950 text-slate-200 text-[11px] font-mono overflow-x-auto leading-relaxed border border-slate-800">
&lt;script&gt;
  import &#123; Menubar, MenuHelpers &#125; from "$lib/bos-ui/menu/index.js";

  const menus = [
    &#123;
      label: "File",
      items: [
        &#123; label: "New", shortcut: "Ctrl+N", action: () =&gt; ... &#125;,
        MenuHelpers.separator(),
        &#123;
          type: "submenu",
          label: "Export",
          items: [
            &#123; label: "PDF", action: () =&gt; ... &#125;
          ]
        &#125;
      ]
    &#125;
  ];
&lt;/script&gt;

&lt;Menubar &#123;menus&#125; /&gt;</pre>
          </div>

          <!-- Real-Time Event & Action Stream -->
          <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-5 shadow-xs flex flex-col h-[340px]">
            <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
              <div class="flex items-center gap-2">
                <Terminal class="size-4 text-indigo-500" />
                <h3 class="font-bold text-xs uppercase tracking-wider">Live Action Stream</h3>
              </div>
              <button
                onclick={() => (actionLogs = [])}
                class="text-[10px] text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 cursor-pointer"
              >
                Clear
              </button>
            </div>

            <div class="flex-1 overflow-y-auto py-2 space-y-2">
              {#if actionLogs.length === 0}
                <div class="h-full flex flex-col items-center justify-center text-center p-6 text-slate-400">
                  <Sliders class="size-7 stroke-1 opacity-40 mb-2" />
                  <p class="text-xs">Click any menu item above to inspect dispatches.</p>
                </div>
              {:else}
                {#each actionLogs as log (log.id)}
                  <div class="p-2 rounded-lg border border-slate-100 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-xs">
                    <div class="flex items-center justify-between text-[10px] text-slate-400 font-mono">
                      <span>{log.time}</span>
                    </div>
                    <div class="font-medium text-slate-800 dark:text-slate-200 mt-0.5">{log.title}</div>
                    {#if log.details}
                      <div class="text-[11px] text-indigo-600 dark:text-indigo-400 font-mono mt-0.5">{log.details}</div>
                    {/if}
                  </div>
                {/each}
              {/if}
            </div>
          </div>
        </div>

      {:else if activeCategory === "input"}
        <!-- Header for Input Component -->
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-xl font-bold tracking-tight">Input Field Component</h2>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              Source: <code class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[11px] font-mono">$lib/bos-ui/input</code>
            </p>
          </div>
          <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 flex items-center gap-1">
            <CheckCircle2 class="size-3.5" />
            Zero-Mock Verified
          </span>
        </div>

        <!-- Interactive Playground Controls Bar -->
        <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-xs flex flex-wrap items-center justify-between gap-4">
          <div class="flex flex-wrap items-center gap-4 text-xs">
            <!-- Size Switcher -->
            <div class="flex items-center gap-1.5">
              <span class="text-slate-400 font-medium">Size:</span>
              <div class="flex items-center p-0.5 rounded-lg bg-slate-100 dark:bg-slate-800">
                {#each ["sm", "md", "lg"] as sz}
                  <button
                    type="button"
                    onclick={() => (testInputSize = sz)}
                    class="px-2 py-1 rounded-md text-[11px] font-medium transition-colors cursor-pointer {testInputSize === sz ? 'bg-white dark:bg-slate-900 text-indigo-600 dark:text-indigo-400 shadow-2xs font-semibold' : 'text-slate-600 dark:text-slate-400'}"
                  >
                    {sz.toUpperCase()}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Status Switcher -->
            <div class="flex items-center gap-1.5">
              <span class="text-slate-400 font-medium">Status:</span>
              <div class="flex items-center p-0.5 rounded-lg bg-slate-100 dark:bg-slate-800">
                {#each ["default", "error", "warning", "success", "info"] as st}
                  <button
                    type="button"
                    onclick={() => (testInputStatus = st)}
                    class="px-2 py-1 rounded-md text-[11px] font-medium transition-colors cursor-pointer capitalize {testInputStatus === st ? 'bg-white dark:bg-slate-900 text-indigo-600 dark:text-indigo-400 shadow-2xs font-semibold' : 'text-slate-600 dark:text-slate-400'}"
                  >
                    {st}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Feature Toggles -->
            <label class="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" bind:checked={testInputLoading} class="rounded text-indigo-600" />
              <span>Loading Spinner</span>
            </label>

            <label class="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" bind:checked={testInputClearable} class="rounded text-indigo-600" />
              <span>Clearable Button</span>
            </label>

            <label class="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" bind:checked={testInputCopyable} class="rounded text-indigo-600" />
              <span>Copyable Button</span>
            </label>
          </div>

          <div class="text-[11px] text-slate-400 font-mono">
            Svelte 5 Runes · Extended shadcn-svelte/input
          </div>
        </div>

        <!-- 1. Polymorphic Field Types Grid -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="border-b border-slate-100 dark:border-slate-800 pb-2">
            <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100">
              1. Polymorphic Input Types & Extended Adornments
            </h3>
            <p class="text-xs text-slate-400 mt-0.5">
              The component dynamically shifts layout, icons, keyboard interaction, and formatting based on field type.
            </p>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <!-- Search Type -->
            <Input
              type="search"
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              clearable={testInputClearable}
              copyable={testInputCopyable}
              label="Global Search (Debounced)"
              placeholder="Search customers or ledger accounts..."
              bind:value={testSearchVal}
              onDebounce={(val) => logAction("Search debounced", `Term: "${val}"`)}
              onClear={() => logAction("Search cleared")}
              helperText="Auto-inferred Search icon with 1-click clear."
            />

            <!-- Password Type with Eye Toggle -->
            <Input
              type="password"
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              copyable={testInputCopyable}
              label="API Secret Key (Password Toggle)"
              placeholder="Enter secure API token..."
              bind:value={testPasswordVal}
              helperText="Eye toggle dynamically changes text visibility."
            />

            <!-- Number Stepper -->
            <Input
              type="number"
              stepper={true}
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              copyable={testInputCopyable}
              min={0}
              max={100}
              step={5}
              label="Order Quantity (Numeric Stepper)"
              placeholder="0"
              bind:value={testQuantityVal}
              suffixText="units"
              helperText="Includes (+/-) buttons and ArrowUp/Down hotkeys."
            />

            <!-- Currency Amount -->
            <Input
              type="currency"
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              clearable={testInputClearable}
              copyable={testInputCopyable}
              prefixText="$"
              suffixText="USD"
              label="Unit Price (Currency)"
              placeholder="0.00"
              bind:value={testCurrencyVal}
              helperText="Static prefix and suffix badges."
            />

            <!-- ERP Code (Auto Uppercase) -->
            <Input
              type="code"
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              clearable={testInputClearable}
              copyable={testInputCopyable}
              autoUppercase={true}
              label="ERP Customer Code (Auto-Uppercase)"
              placeholder="e.g. CUST-1002"
              bind:value={testErpCodeVal}
              helperText="Forces uppercase formatting for master data codes."
            />

            <!-- Lookup / AssistEdit -->
            <Input
              type="lookup"
              size={testInputSize}
              status={testInputStatus}
              loading={testInputLoading}
              clearable={testInputClearable}
              copyable={testInputCopyable}
              label="Payment Terms (AssistEdit Lookup)"
              placeholder="Select terms..."
              bind:value={testLookupVal}
              onLookup={() => logAction("AssistEdit Lookup Triggered", "Opening Payment Terms table")}
              helperText="Dynamics BC AssistEdit (...) button with callback."
            />
          </div>
        </div>

        <!-- 2. Validation States & Inline Messages -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="border-b border-slate-100 dark:border-slate-800 pb-2">
            <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100">
              2. Validation States, Descriptions & Character Counters
            </h3>
            <p class="text-xs text-slate-400 mt-0.5">
              Standardized state tokens (error, warning, success, info) with semantic icons, inline descriptions, and boundary counters.
            </p>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <Input
              label="Posting Date"
              value="2026-03-31"
              status="error"
              errorMessage="Date is in a closed accounting period."
            />

            <Input
              label="Credit Limit"
              value="150,000.00"
              prefixText="$"
              status="warning"
              warningMessage="Customer limit exceeded by $12,450.00."
            />

            <Input
              label="Tax Registration No."
              value="EU987654321B"
              status="success"
              successMessage="Verified active via VIES tax registry."
            />

            <Input
              label="Document Remark"
              value="Urgent shipment"
              maxlength={25}
              showCount={true}
              helperText="Character counter updates live."
            />
          </div>
        </div>

        <!-- 3. In-Field Mathematical Calculations & Sensitive Field Masking -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="border-b border-slate-100 dark:border-slate-800 pb-2">
            <h3 class="text-sm font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
              <Calculator class="size-4 text-indigo-500" />
              <span>3. In-Field Mathematical Calculations & Sensitive Field Masking</span>
            </h3>
            <p class="text-xs text-slate-400 mt-0.5">
              Enterprise features: Safe Shunting-Yard tokenized parser for inline math calculations (BC/SAP pattern), and automatic masking for sensitive financial and identity fields (credit cards, SSN, IBAN) with unmask toggle and raw clipboard copying.
            </p>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <!-- Math: Unit Price Calculation -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Calculator class="size-3.5 text-indigo-500" />
                  <span>Inline Math (Price Markup)</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">evalMath={true}</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Type an expression like <code class="px-1 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[10px] font-mono">120 * 1.15</code> or <code class="px-1 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[10px] font-mono">500 + 75</code> and press <kbd class="px-1 py-0.5 rounded border border-slate-300 dark:border-slate-700 text-[10px]">Enter</kbd> or Blur.
              </p>
              <Input
                type="currency"
                evalMath={true}
                prefixText="$"
                suffixText="USD"
                size={testInputSize}
                status={testInputStatus}
                loading={testInputLoading}
                clearable={testInputClearable}
                copyable={testInputCopyable}
                label="Unit Price Formula"
                placeholder="e.g. 120 * 1.15"
                bind:value={testMathVal}
                onCalculate={(res, expr) => logAction("Math Evaluated", `${expr} = ${res}`)}
                helperText="Safe parser evaluates arithmetic and formats currency."
              />
            </div>

            <!-- Math: Discount / Rebate Formula -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Calculator class="size-3.5 text-emerald-500" />
                  <span>Parentheses & Complex Arithmetic</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">evalMath={true}</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Supports parentheses and precedence: <code class="px-1 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[10px] font-mono">(1500 - 250) / 2</code>.
              </p>
              <Input
                type="number"
                evalMath={true}
                stepper={true}
                size={testInputSize}
                status={testInputStatus}
                loading={testInputLoading}
                clearable={testInputClearable}
                copyable={testInputCopyable}
                label="Net Allocation Formula"
                placeholder="(A - B) / C"
                bind:value={testComplexMathVal}
                suffixText="units"
                onCalculate={(res, expr) => logAction("Complex Math Evaluated", `${expr} = ${res}`)}
                helperText="Evaluates formula into clean numeric quantity."
              />
            </div>

            <!-- Masking: Credit Card Number -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Shield class="size-3.5 text-amber-500" />
                  <span>Sensitive Credit Card</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">maskType="credit-card"</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Masked on blur to <code class="px-1 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[10px] font-mono">•••• 4242</code>. Shield button reveals; copy button copies true raw digits!
              </p>
              <Input
                type="masked"
                masked={true}
                maskType="credit-card"
                unmaskable={true}
                copyable={true}
                size={testInputSize}
                status={testInputStatus}
                label="Corporate Card Number"
                placeholder="Enter 16-digit card number"
                bind:value={testCreditCardVal}
                helperText="Focus to edit raw digits. Copy button copies real card number."
              />
            </div>

            <!-- Masking: SSN / Tax ID -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Shield class="size-3.5 text-rose-500" />
                  <span>Tax ID / SSN Masking</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">maskType="ssn"</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Formats government IDs with standard obfuscation: <code class="px-1 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[10px] font-mono">•••-••-6789</code>.
              </p>
              <Input
                type="text"
                masked={true}
                maskType="ssn"
                unmaskable={true}
                copyable={true}
                size={testInputSize}
                status={testInputStatus}
                label="Social Security / National Tax ID"
                placeholder="123-45-6789"
                bind:value={testSsnVal}
                helperText="Click Shield icon to toggle mask or copy raw digits."
              />
            </div>

            <!-- Masking: Bank IBAN -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Shield class="size-3.5 text-blue-500" />
                  <span>Bank Account / IBAN</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">maskType="iban"</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Masks intermediate bank routing digits while exposing country code & last 4 digits.
              </p>
              <Input
                type="code"
                masked={true}
                maskType="iban"
                unmaskable={true}
                copyable={true}
                size={testInputSize}
                status={testInputStatus}
                label="Direct Debit Bank IBAN"
                placeholder="US33CHAS01234567890123"
                bind:value={testIbanVal}
                helperText="Auto-uppercased and masked for banking safety."
              />
            </div>

            <!-- General Custom Masking with Copy -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Shield class="size-3.5 text-violet-500" />
                  <span>Custom Token Masking</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">visibleCount=4</span>
              </div>
              <p class="text-[11px] text-slate-500">
                General-purpose token masking keeps last 4 characters visible and obscures secret tokens.
              </p>
              <Input
                type="text"
                masked={true}
                maskType="custom"
                unmaskable={true}
                copyable={true}
                size={testInputSize}
                status={testInputStatus}
                label="Production Secret API Key"
                value="sk_live_9981248912389172348"
                helperText="Copy button safely extracts the full underlying API token."
              />
            </div>
          </div>
        </div>

        <!-- Section 4: Enterprise Typing Ergonomics (Live Pattern Masking & F8 Copy-Above) -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Section 4: Enterprise Typing Ergonomics & Shorthands</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Live interactive pattern masking (formats as you type) and Dynamics NAV/BC <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">F8</code> shortcut to duplicate the cell directly above.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-indigo-50 dark:bg-indigo-950 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800">
              Hotkey: F8 Copy Above
            </span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Pattern 1: Phone Number -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Phone class="size-3.5 text-indigo-500" />
                  <span>Phone Number Live Format</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">pattern="(###) ###-####"</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Type digits only: automatically formats with parentheses, spacing, and hyphen in real time.
              </p>
              <Input
                type="text"
                pattern="(###) ###-####"
                size={testInputSize}
                status={testInputStatus}
                label="Direct Work Phone"
                placeholder="(312) 555-0143"
                bind:value={testPhonePatternVal}
                helperText="Formatted as you type: try deleting digits or typing 10 numbers."
              />
              <div class="text-[11px] font-mono text-slate-400">
                Raw value: <span class="text-indigo-600 dark:text-indigo-400 font-semibold">{testPhonePatternVal}</span>
              </div>
            </div>

            <!-- Pattern 2: Federal Tax EIN -->
            <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-2">
              <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
                <span class="flex items-center gap-1.5">
                  <Hash class="size-3.5 text-emerald-500" />
                  <span>Federal Employer ID (EIN)</span>
                </span>
                <span class="text-[10px] text-slate-400 font-mono">pattern="##-#######"</span>
              </div>
              <p class="text-[11px] text-slate-500">
                US IRS 9-digit format: automatically adds hyphen after the 2-digit district prefix.
              </p>
              <Input
                type="text"
                pattern="##-#######"
                size={testInputSize}
                status={testInputStatus}
                label="Employer Identification Number"
                placeholder="12-3456789"
                bind:value={testEinPatternVal}
                helperText="Auto-inserts hyphen after first two digits."
              />
              <div class="text-[11px] font-mono text-slate-400">
                Raw value: <span class="text-emerald-600 dark:text-emerald-400 font-semibold">{testEinPatternVal}</span>
              </div>
            </div>
          </div>

          <!-- Pattern 3: F8 Copy-Above Demonstration -->
          <div class="p-4 rounded-lg border border-slate-100 dark:border-slate-800/60 bg-slate-50/50 dark:bg-slate-950/40 space-y-3">
            <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
              <span class="flex items-center gap-1.5">
                <Copy class="size-3.5 text-violet-500" />
                <span>Dynamics NAV / BC F8 Copy-Above Pattern</span>
              </span>
              <span class="px-2 py-0.5 rounded text-[10px] font-mono bg-violet-50 dark:bg-violet-950 text-violet-600 dark:text-violet-400 border border-violet-200 dark:border-violet-800">
                Press F8 in Row 2
              </span>
            </div>
            <p class="text-xs text-slate-500">
              In high-speed journal voucher data entry, accountants press <kbd class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 font-mono text-[11px] font-semibold text-slate-800 dark:text-slate-200 border border-slate-300 dark:border-slate-700">F8</kbd> to duplicate the field value from the line immediately preceding. Focus the Line 2 input and press F8 to instantly copy Line 1's Account No.
            </p>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 pt-1 max-w-xl">
              <div>
                <Input
                  label="Journal Line 1 — G/L Account No."
                  bind:value={testF8SourceVal}
                  size={testInputSize}
                  status={testInputStatus}
                  helperText="Preceding row account code"
                />
              </div>
              <div>
                <Input
                  label="Journal Line 2 — G/L Account No."
                  bind:value={testF8TargetVal}
                  size={testInputSize}
                  status={testInputStatus}
                  placeholder="Focus here & press F8"
                  helperText="Press F8 to duplicate Line 1 value"
                  onCopyAbove={() => {
                    testF8TargetVal = testF8SourceVal;
                    logAction("F8 Copied Above", `${testF8SourceVal} -> Line 2`);
                  }}
                />
              </div>
            </div>
          </div>
        </div>

      {:else if activeCategory === "combobox"}
        <!-- Header for Combobox Component -->
        <div class="flex items-center justify-between">
          <div>
            <h2 class="text-xl font-bold tracking-tight">Combobox / Table Relation Lookup</h2>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
              Source: <code class="px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-[11px] font-mono">$lib/bos-ui/combobox</code>
            </p>
          </div>
          <span class="px-2.5 py-1 rounded-full text-xs font-medium bg-emerald-100 dark:bg-emerald-950/60 text-emerald-700 dark:text-emerald-400 flex items-center gap-1">
            <CheckCircle2 class="size-3.5" />
            Zero-Mock Verified
          </span>
        </div>

        <!-- Personalization & Look-and-Feel Customizer Bar -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-4 shadow-xs space-y-3">
          <div class="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-slate-800">
            <span class="text-xs font-bold uppercase tracking-wider text-slate-500 flex items-center gap-1.5">
              <Sliders class="size-3.5 text-indigo-500" />
              <span>Personalization & Customization Engine</span>
            </span>
            <span class="text-[11px] text-slate-400">Live Look & Feel Calibration</span>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-3 text-xs">
            <!-- Density Axis -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">Density</label>
              <select
                bind:value={testComboboxDensity}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-xs font-medium cursor-pointer"
              >
                <option value="sm">Compact (28px ERP)</option>
                <option value="md">Standard (36px)</option>
                <option value="lg">Spacious (44px Touch)</option>
              </select>
            </div>

            <!-- Surface & Variant Axis -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">Surface Variant</label>
              <select
                bind:value={testComboboxVariant}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-xs font-medium cursor-pointer"
              >
                <option value="outline">Outline (Classic)</option>
                <option value="filled">Filled (Modern Surface)</option>
                <option value="underline">Underline (Flushed)</option>
                <option value="ghost">Ghost (In-Grid)</option>
              </select>
            </div>

            <!-- Corner Radius Axis -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">Corner Radius</label>
              <select
                bind:value={testComboboxRounded}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-xs font-medium cursor-pointer"
              >
                <option value="none">Sharp (0px Enterprise)</option>
                <option value="sm">Subtle (3px Fluent)</option>
                <option value="md">Smooth (6px SaaS)</option>
                <option value="pill">Pill (Full Round)</option>
              </select>
            </div>

            <!-- Status Axis -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">Validation State</label>
              <select
                bind:value={testComboboxStatus}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-xs font-medium cursor-pointer"
              >
                <option value="default">Default</option>
                <option value="error">Error</option>
                <option value="warning">Warning</option>
                <option value="success">Success</option>
              </select>
            </div>

            <!-- Loading Spinner Toggle -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">Async Query</label>
              <button
                type="button"
                onclick={() => (testComboboxLoading = !testComboboxLoading)}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 text-xs font-medium cursor-pointer transition-colors {testComboboxLoading ? 'bg-indigo-50 dark:bg-indigo-950 text-indigo-600 border-indigo-400' : 'bg-white dark:bg-slate-800'}"
              >
                {testComboboxLoading ? "Spinner: ON" : "Spinner: OFF"}
              </button>
            </div>

            <!-- FactBox Preview Toggle -->
            <div class="space-y-1">
              <label class="text-[11px] font-medium text-slate-500">FactBox Preview</label>
              <button
                type="button"
                onclick={() => (testComboboxFactBox = !testComboboxFactBox)}
                class="w-full px-2 py-1 rounded border border-slate-200 dark:border-slate-700 text-xs font-medium cursor-pointer transition-colors {testComboboxFactBox ? 'bg-indigo-50 dark:bg-indigo-950 text-indigo-600 border-indigo-400' : 'bg-white dark:bg-slate-800'}"
              >
                {testComboboxFactBox ? "Preview: ON" : "Preview: OFF"}
              </button>
            </div>
          </div>
        </div>

        <!-- Section 1: Customer Relation Multi-Column Lookup (Dynamics BC Benchmark) -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 1: Multi-Column Table Relation (Dynamics BC)</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Displays tabular columns (No., Customer Name, Balance LCY, City), filters across all fields as you type, and supports keyboard <kbd class="px-1 py-0.5 rounded bg-slate-100 dark:bg-slate-800 font-mono text-[10px]">F4</kbd> / <kbd class="px-1 py-0.5 rounded bg-slate-100 dark:bg-slate-800 font-mono text-[10px]">Alt+Down</kbd>.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-indigo-50 dark:bg-indigo-950 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800">
              Value: {selectedCustomer || "null"}
            </span>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6 items-start">
            <!-- Left: Interactive Combobox Component -->
            <div class="space-y-4">
              <Combobox
                label="Bill-to Customer No."
                placeholder="Select or search customer (F4)..."
                bind:value={selectedCustomer}
                items={demoCustomers}
                columns={customerColumns}
                valueKey="id"
                labelKey="name"
                density={testComboboxDensity}
                variant={testComboboxVariant}
                rounded={testComboboxRounded}
                status={testComboboxStatus}
                loading={testComboboxLoading}
                showFactBox={testComboboxFactBox}
                sortable={true}
                persistRecentKey="customers"
                newRecordLabel="+ New Customer"
                fullListLabel="Select from full list (F4)"
                helperText="Type any text (name, city, or code) to search across all columns."
                errorMessage={testComboboxStatus === "error" ? "Customer account is blocked for new sales orders." : ""}
                warningMessage={testComboboxStatus === "warning" ? "Customer balance exceeds recommended credit limit." : ""}
                onCreateNew={() => logAction("Create New Customer (+ New) triggered")}
                onFullList={() => logAction("Select from full list (PAGE::Customer List) triggered")}
                onSelect={(cust) => logAction("Customer Selected", `${cust.id} - ${cust.name}`)}
                onClear={() => logAction("Customer Selection Cleared")}
              />

              <div class="p-3 rounded-lg bg-slate-50 dark:bg-slate-950/40 border border-slate-100 dark:border-slate-800 text-xs space-y-1">
                <span class="font-semibold text-slate-700 dark:text-slate-300">Keyboard Shortcuts:</span>
                <div class="grid grid-cols-2 gap-1 text-[11px] text-slate-500 font-mono">
                  <div><kbd class="font-bold">F4 / Alt+Down</kbd>: Toggle Lookup</div>
                  <div><kbd class="font-bold">↑ / ↓</kbd>: Navigate Rows</div>
                  <div><kbd class="font-bold">Enter</kbd>: Confirm Selection</div>
                  <div><kbd class="font-bold">Esc</kbd>: Close / Clear</div>
                </div>
              </div>
            </div>

            <!-- Right: Live Customer Card FactBox -->
            {#if demoCustomers.find(c => c.id === selectedCustomer)}
              {@const activeCust = demoCustomers.find(c => c.id === selectedCustomer)}
              <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 space-y-3">
                <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
                  <span class="text-xs font-bold uppercase tracking-wider text-slate-500">Customer Details FactBox</span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-semibold bg-emerald-100 dark:bg-emerald-950 text-emerald-700 dark:text-emerald-400">
                    Active Account
                  </span>
                </div>

                <div class="space-y-2 text-xs">
                  <div class="flex items-center justify-between">
                    <span class="text-slate-500">Customer No.:</span>
                    <span class="font-mono font-bold text-indigo-600 dark:text-indigo-400">{activeCust.id}</span>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="text-slate-500">Company Name:</span>
                    <span class="font-semibold text-slate-800 dark:text-slate-200">{activeCust.name}</span>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="text-slate-500">City / Location:</span>
                    <span class="text-slate-700 dark:text-slate-300">{activeCust.city}</span>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="text-slate-500">Phone Contact:</span>
                    <span class="font-mono text-slate-600 dark:text-slate-400">{activeCust.phone}</span>
                  </div>
                  <div class="flex items-center justify-between pt-1 border-t border-slate-200 dark:border-slate-800">
                    <span class="text-slate-500">Outstanding Balance:</span>
                    <span class="font-mono font-bold {activeCust.balance > 0 ? 'text-amber-600 dark:text-amber-400' : 'text-emerald-600 dark:text-emerald-400'}">
                      ${activeCust.balance.toLocaleString('en-US', { minimumFractionDigits: 2 })}
                    </span>
                  </div>
                  <div class="flex items-center justify-between">
                    <span class="text-slate-500">Credit Limit LCY:</span>
                    <span class="font-mono text-slate-600 dark:text-slate-400">
                      ${activeCust.creditLimit.toLocaleString('en-US', { minimumFractionDigits: 2 })}
                    </span>
                  </div>
                </div>
              </div>
            {:else}
              <div class="p-6 rounded-xl border border-dashed border-slate-200 dark:border-slate-800 text-center text-xs text-slate-400">
                Select a customer from the lookup dropdown to inspect the live FactBox.
              </div>
            {/if}
          </div>
        </div>

        <!-- Section 2: Inventory Items & Pricing Relation Lookup -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 2: Inventory & SKU Table Relation</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Displays 5 columns with custom numeric and currency formatting for order line items.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-violet-50 dark:bg-violet-950 text-violet-600 dark:text-violet-400 border border-violet-200 dark:border-violet-800">
              Selected Item: {selectedItemCode}
            </span>
          </div>

          <div class="max-w-md">
            <Combobox
              label="Line Item No."
              placeholder="Select inventory item..."
              bind:value={selectedItemCode}
              items={demoItems}
              columns={itemColumns}
              valueKey="id"
              labelKey="name"
              density={testComboboxDensity}
              variant={testComboboxVariant}
              rounded={testComboboxRounded}
              newRecordLabel="+ New Item Card"
              fullListLabel="Select from item list (F4)"
              onCreateNew={() => logAction("Create New Item Card triggered")}
              onFullList={() => logAction("Open Full Item List triggered")}
              onSelect={(item) => logAction("Item Selected", `${item.id} - ${item.name} ($${item.price})`)}
            />
          </div>
        </div>

        <!-- Section 3: Multi-Select Token Chips & Category Grouping -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 3: Multi-Select Token Chips & Location Grouping</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Pick multiple records that collapse into removable tag pills with checkmarks, plus sticky grouping by city/region.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-emerald-50 dark:bg-emerald-950 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800">
              Selected: {multiSelectedCustomers.length} accounts
            </span>
          </div>

          <div class="max-w-2xl">
            <Combobox
              label="Assigned Customer Accounts (Multi-Select)"
              placeholder="Select one or more customer accounts..."
              bind:value={multiSelectedCustomers}
              items={demoCustomers}
              columns={customerColumns}
              valueKey="id"
              labelKey="name"
              multiple={true}
              groupBy="city"
              density={testComboboxDensity}
              variant={testComboboxVariant}
              rounded={testComboboxRounded}
              helperText="Click items to toggle selection. Each item renders as a removable chip badge."
              onSelect={(cust) => logAction("Added Customer Tag", `${cust.id} - ${cust.name}`)}
              onRemove={(cust) => logAction("Removed Customer Tag", `${cust.id} - ${cust.name}`)}
              onClear={() => logAction("Cleared all selected tags")}
            />
          </div>
        </div>
      {:else if activeCategory === "date-picker"}
        <!-- ==== DATE PICKER SHOWCASE ==== -->
        <div class="space-y-2 mb-6">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <Calendar class="size-6 text-orange-500" />
            <span>Date, Date Range & Time Picker</span>
          </h2>
          <p class="text-sm text-slate-500 dark:text-slate-400">
            Dynamics NAV / Business Central inspired enterprise date field with built-in BC formula engine (<code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">cm+10d</code>, <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">t</code>, <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">cy</code>), visual calendar, accounting presets sidebar, closing dates, period locking, and time selection.
          </p>
        </div>

        <!-- Live Personalization Controls -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-5 shadow-xs">
          <div class="text-[10px] font-bold uppercase tracking-wider text-slate-400 mb-3 flex items-center gap-1.5">
            <Sliders class="size-3.5 text-indigo-500" />
            <span>Live Personalization & Style Axes</span>
          </div>
          <div class="flex flex-wrap gap-6">
            <div class="flex flex-col gap-1.5">
              <span class="text-[11px] font-medium text-slate-500">Density / ERP Sizing</span>
              <div class="flex gap-1">
                {#each ["sm", "md", "lg"] as d}
                  <button
                    type="button"
                    onclick={() => (testDateDensity = d)}
                    class="px-2.5 py-1 text-[11px] font-medium rounded-md border cursor-pointer transition-colors {testDateDensity === d ? 'bg-indigo-600 text-white border-indigo-600' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {d}
                  </button>
                {/each}
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <span class="text-[11px] font-medium text-slate-500">Surface Variant</span>
              <div class="flex gap-1">
                {#each ["outline", "filled", "underline", "ghost"] as v}
                  <button
                    type="button"
                    onclick={() => (testDateVariant = v)}
                    class="px-2.5 py-1 text-[11px] font-medium rounded-md border cursor-pointer transition-colors {testDateVariant === v ? 'bg-indigo-600 text-white border-indigo-600' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {v}
                  </button>
                {/each}
              </div>
            </div>
            <div class="flex flex-col gap-1.5">
              <span class="text-[11px] font-medium text-slate-500">Corner Radius</span>
              <div class="flex gap-1">
                {#each ["none", "sm", "md", "pill"] as r}
                  <button
                    type="button"
                    onclick={() => (testDateRounded = r)}
                    class="px-2.5 py-1 text-[11px] font-medium rounded-md border cursor-pointer transition-colors {testDateRounded === r ? 'bg-indigo-600 text-white border-indigo-600' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {r}
                  </button>
                {/each}
              </div>
            </div>
          </div>
        </div>

        <!-- Pattern 1: Single Date with BC Formula Engine -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 1: High-Speed Keyboard Formula Engine</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Type accounting shorthands: <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">t</code> (today), <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">cm</code> (current month end), <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">cm+10d</code>, <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">cq</code>, <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">cy</code> then press Enter or Tab.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-indigo-50 dark:bg-indigo-950 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800">
              Hotkey: F4 or Alt+↓
            </span>
          </div>

          <div class="max-w-md">
            <DatePicker
              label="Posting Date"
              placeholder="e.g. cm+10d, t, -cm"
              bind:value={testDateValue}
              density={testDateDensity}
              variant={testDateVariant}
              rounded={testDateRounded}
              required={true}
              allowClosingDates={true}
              helperText="Press Enter to evaluate formula or click calendar (F4)"
              onFormulaCalculated={(r) => logAction("Formula evaluated", `${r?.formula ?? ''} → ${r?.date ? r.date.toISOString().slice(0, 10) : ''}`)}
              onChange={(v) => logAction("Posting Date changed", String(v))}
            />
          </div>

          {#if testDateValue}
            <div class="text-xs font-mono bg-slate-50 dark:bg-slate-800/60 px-3 py-2 rounded-lg border border-slate-100 dark:border-slate-700 flex items-center gap-2">
              <span class="text-slate-400">Bound Value:</span>
              <span class="text-indigo-600 dark:text-indigo-400 font-semibold">{JSON.stringify(testDateValue)}</span>
            </div>
          {/if}
        </div>

        <!-- Pattern 2: Date Range with Accounting Presets -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 2: Date Range with Accounting Presets Sidebar</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Click the calendar to open dual selection with MTD, QTD, YTD, Full Fiscal Year one-click presets, or type range syntax like <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-[11px]">-cm .. cm</code>.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-amber-50 dark:bg-amber-950 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-800">
              Range Mode
            </span>
          </div>

          <div class="max-w-lg">
            <DatePicker
              label="Financial Report Period"
              bind:value={testDateRange}
              range={true}
              density={testDateDensity}
              variant={testDateVariant}
              rounded={testDateRounded}
              helperText="Select start and end dates or choose an accounting preset from the sidebar"
              onChange={(v) => logAction("Date range changed", `${v?.start ?? ''} .. ${v?.end ?? ''}`)}
            />
          </div>

          {#if testDateRange?.start || testDateRange?.end}
            <div class="text-xs font-mono bg-slate-50 dark:bg-slate-800/60 px-3 py-2 rounded-lg border border-slate-100 dark:border-slate-700 flex items-center gap-2">
              <span class="text-slate-400">Selected Interval:</span>
              <span class="text-emerald-600 dark:text-emerald-400 font-semibold">{testDateRange.start || "..."}</span>
              <span class="text-slate-400">to</span>
              <span class="text-emerald-600 dark:text-emerald-400 font-semibold">{testDateRange.end || "..."}</span>
            </div>
          {/if}
        </div>

        <!-- Pattern 3: DateTime with Embedded Time Picker -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 3: DateTime with Embedded Posting Time</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Integrates 15-minute interval time selection for warehouse dispatch tickets, journal timestamping, and audit logging.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-violet-50 dark:bg-violet-950 text-violet-600 dark:text-violet-400 border border-violet-200 dark:border-violet-800">
              showTime=true
            </span>
          </div>

          <div class="max-w-md">
            <DatePicker
              label="Warehouse Dispatch Timestamp"
              bind:value={testDateWithTime}
              bind:timeValue={testTimeValue}
              showTime={true}
              timeStep={15}
              density={testDateDensity}
              variant={testDateVariant}
              rounded={testDateRounded}
              helperText="Date combined with posting clock time"
              onChange={(v) => logAction("Dispatch timestamp changed", `${v} @ ${testTimeValue}`)}
            />
          </div>

          {#if testDateWithTime}
            <div class="text-xs font-mono bg-slate-50 dark:bg-slate-800/60 px-3 py-2 rounded-lg border border-slate-100 dark:border-slate-700 flex items-center gap-2">
              <span class="text-slate-400">Recorded:</span>
              <span class="text-indigo-600 dark:text-indigo-400 font-semibold">{testDateWithTime}</span>
              <span class="text-slate-400">at</span>
              <span class="text-indigo-600 dark:text-indigo-400 font-semibold">{testTimeValue}</span>
            </div>
          {/if}
        </div>

        <!-- Pattern 4: Closed Fiscal Period Lock -->
        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-4">
          <div class="flex items-center justify-between pb-3 border-b border-slate-100 dark:border-slate-800">
            <div>
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Pattern 4: Closed Fiscal Period Lock & Audit Protection</span>
              <p class="text-xs text-slate-400 mt-0.5">
                Q1 & Q2 2025 periods are legally closed. Closed dates are crossed out with lock icons and cannot be picked.
              </p>
            </div>
            <span class="px-2 py-0.5 rounded text-[11px] font-mono bg-rose-50 dark:bg-rose-950 text-rose-600 dark:text-rose-400 border border-rose-200 dark:border-rose-800">
              Period Audit Lock
            </span>
          </div>

          <div class="max-w-md">
            <DatePicker
              label="Restricted Ledger Entry Date"
              bind:value={testDateValue}
              closedPeriods={closedFiscalPeriods}
              density={testDateDensity}
              variant={testDateVariant}
              rounded={testDateRounded}
              warningMessage="Q1 and Q2 2025 are closed fiscal periods. Dates in those periods cannot be selected."
              onChange={(v) => logAction("Ledger date selected", String(v))}
            />
          </div>
        </div>

      {:else if activeCategory === "datasheet"}
        <DatasheetPanel />
      {:else if activeCategory === "data-table"}
        <DataTablePanel />
      {:else if activeCategory === "control-kit"}
        <ControlKitPanel />
      {:else if activeCategory === "page-surface"}
        <PageSurfacePanel />
      {:else if activeCategory === "studio-designers"}
        <StudioDesignersPanel />
      {:else if activeCategory === "theme"}
        <!-- ==== THEME & PERSONALIZATION STORE SHOWCASE ==== -->
        <div class="space-y-2 mb-6">
          <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100 flex items-center gap-2">
            <Palette class="size-6 text-rose-500" />
            <span>Theme & Personalization Store</span>
          </h2>
          <p class="text-sm text-slate-500 dark:text-slate-400">
            One saved setting, <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">bos_user_preference</code>, drives every bos-ui control through <code class="font-mono bg-slate-100 dark:bg-slate-800 px-1 py-0.5 rounded text-xs">surface.js</code>. Density changes Data table row height. Surface variant and corner radius update Input, Combobox, DatePicker, and Menubar, including the editors inside a Data table.
          </p>
        </div>

        <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-xs space-y-6">
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            <!-- Density Preference -->
            <div class="space-y-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Interface Density</span>
              <p class="text-xs text-slate-400">Controls vertical padding and font size across ERP forms.</p>
              <div class="flex gap-1.5 pt-1">
                {#each [
                  { id: "sm", label: "Compact (28px)" },
                  { id: "md", label: "Standard (36px)" },
                  { id: "lg", label: "Touch (44px)" }
                ] as opt}
                  <button
                    type="button"
                    onclick={() => themeStore.setDensity(opt.id)}
                    class="px-2.5 py-1.5 text-xs font-medium rounded-md border cursor-pointer transition-colors {themeStore.density === opt.id ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {opt.id}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Variant Preference -->
            <div class="space-y-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Surface Variant</span>
              <p class="text-xs text-slate-400">Form field frame styling and contrast boundaries.</p>
              <div class="flex flex-wrap gap-1.5 pt-1">
                {#each ["outline", "filled", "underline", "ghost"] as v}
                  <button
                    type="button"
                    onclick={() => themeStore.setVariant(v)}
                    class="px-2.5 py-1.5 text-xs font-medium rounded-md border cursor-pointer transition-colors {themeStore.variant === v ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {v}
                  </button>
                {/each}
              </div>
            </div>

            <!-- Rounded Corner Preference -->
            <div class="space-y-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Corner Radius</span>
              <p class="text-xs text-slate-400">Geometric styling from sharp ERP to modern soft curves.</p>
              <div class="flex gap-1.5 pt-1">
                {#each [
                  { id: "none", label: "Sharp 0px" },
                  { id: "sm", label: "Subtle 2px" },
                  { id: "md", label: "Standard 6px" },
                  { id: "pill", label: "Pill Full" }
                ] as r}
                  <button
                    type="button"
                    onclick={() => themeStore.setRounded(r.id)}
                    class="px-2.5 py-1.5 text-xs font-medium rounded-md border cursor-pointer transition-colors {themeStore.rounded === r.id ? 'bg-indigo-600 text-white border-indigo-600 shadow-2xs' : 'border-slate-200 dark:border-slate-700 text-slate-600 dark:text-slate-400 hover:border-indigo-400'}"
                  >
                    {r.id}
                  </button>
                {/each}
              </div>
            </div>

            <!-- ERP Work Date -->
            <div class="space-y-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Simulated Work Date</span>
              <p class="text-xs text-slate-400">Dynamics NAV style working date for simulated journal posting.</p>
              <div class="pt-1">
                <input
                  type="date"
                  value={themeStore.workDate}
                  oninput={(e) => themeStore.setWorkDate(e.currentTarget.value)}
                  class="h-8 px-2.5 text-xs rounded-md border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-900 dark:text-slate-100 font-mono outline-hidden focus:border-indigo-500"
                />
              </div>
            </div>
          </div>

          <div class="pt-4 border-t border-slate-100 dark:border-slate-800 space-y-4">
            <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Live preview from this setting</span>
            <Menubar menus={surfacePreviewMenus} />
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
              <Input label="Customer No." bind:value={previewCode} />
              <Combobox
                label="Customer"
                items={[
                  { id: "C-10042", name: "Tesla" },
                  { id: "C-10088", name: "Acme" }
                ]}
                bind:value={previewCustomer}
                valueKey="id"
                labelKey="name"
              />
              <DatePicker label="Posting Date" bind:value={previewDate} />
            </div>
          </div>

          <!-- Persisted State JSON Viewer -->
          <div class="pt-4 border-t border-slate-100 dark:border-slate-800">
            <div class="flex items-center justify-between mb-2">
              <span class="text-xs font-semibold uppercase tracking-wider text-slate-500">Live Reactive Store State</span>
              <button
                type="button"
                onclick={() => themeStore.resetDefaults()}
                class="px-2.5 py-1 text-xs rounded-md border border-rose-200 dark:border-rose-900/50 text-rose-600 dark:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/40 cursor-pointer transition-colors"
              >
                Reset Defaults
              </button>
            </div>
            <pre class="text-xs font-mono bg-slate-50 dark:bg-slate-800/60 p-4 rounded-lg border border-slate-100 dark:border-slate-700 text-indigo-600 dark:text-indigo-400 overflow-auto">{JSON.stringify(
              {
                density: themeStore.density,
                variant: themeStore.variant,
                rounded: themeStore.rounded,
                theme: themeStore.theme,
                workDate: themeStore.workDate,
                dateFormat: themeStore.dateFormat
              },
              null,
              2
            )}</pre>
          </div>
        </div>
      {/if}
    </main>
  </div>
</div>
