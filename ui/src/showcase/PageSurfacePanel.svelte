<script>
  /**
   * PageSurfacePanel.svelte (Svelte 5 Runes)
   * Showcase panel demonstrating the 8 compact ERP Page Layouts defined in docs/28:
   * List, Record, Document, Board, Workspace, Split, Timeline, Wizard.
   * Includes interactive layout switching, live search filtering, FactBox side panel,
   * CRUD modal creation/editing, and live <PageSurface /> catalog integration.
   */
  import {
    List,
    FileText,
    Receipt,
    Columns,
    LayoutDashboard,
    SplitSquareVertical,
    Clock,
    Sparkles,
    Plus,
    Search,
    Edit3,
    Trash2,
    Play,
    PanelRight,
    Check,
    X,
    Filter,
    ChevronRight,
    ArrowRight,
    RefreshCw,
    Building2,
    ShieldCheck,
    AlertCircle,
    SlidersHorizontal,
    ExternalLink
  } from "lucide-svelte";
  import { Button, Input, Badge, Banner, Dialog, Select, EmptyState, toastStore } from "$lib/bos-ui/index.js";
  import PageSurface from "$lib/components/shell/PageSurface.svelte";

  // Mode: 'simulator' (Interactive 8 Layouts Sandbox) or 'engine' (Live PageSurface meta catalog component)
  let showcaseMode = $state("simulator"); // 'simulator' | 'engine'
  let liveEnginePage = $state("CustomerList");

  // Layout switcher state
  let currentLayout = $state("List");
  let searchFilter = $state("");
  let isFactBoxOpen = $state(true);

  // CRUD Modal state
  let isCrudModalOpen = $state(false);
  let crudMode = $state("create"); // 'create' | 'edit'
  let recordDraft = $state({
    id: "",
    name: "",
    city: "",
    balance: 0,
    creditLimit: 10000,
    status: "Active"
  });

  // Wizard state
  let wizardStep = $state(1);

  // Sample data for 8 Layouts simulation
  let demoCustomers = $state([
    {
      id: "CUST-1001",
      name: "CRONUS USA Logistics Corp",
      city: "Chicago, IL",
      balance: 14250.0,
      creditLimit: 50000,
      status: "Active",
      phone: "+1 (312) 555-0143",
      email: "orders@cronus-usa.com",
      paymentTerms: "NET-30",
      companyId: "COMP-001"
    },
    {
      id: "CUST-1002",
      name: "Progressive Home Furnishings",
      city: "Seattle, WA",
      balance: 4890.5,
      creditLimit: 25000,
      status: "Active",
      phone: "+1 (206) 555-0198",
      email: "accounts@progressive.com",
      paymentTerms: "NET-14",
      companyId: "COMP-001"
    },
    {
      id: "CUST-1003",
      name: "First Up Global Consultants",
      city: "London, UK",
      balance: 38200.0,
      creditLimit: 40000,
      status: "Blocked",
      phone: "+44 20 7946 0912",
      email: "finance@firstup.co.uk",
      paymentTerms: "COD",
      companyId: "COMP-002"
    },
    {
      id: "CUST-1004",
      name: "Alpine Ski & Resort Systems",
      city: "Zurich, CH",
      balance: 1120.0,
      creditLimit: 15000,
      status: "Active",
      phone: "+41 22 555-0188",
      email: "info@alpineski.ch",
      paymentTerms: "NET-30",
      companyId: "COMP-003"
    },
    {
      id: "CUST-1005",
      name: "Wide World Importers",
      city: "New York, NY",
      balance: 0.0,
      creditLimit: 80000,
      status: "Pending",
      phone: "+1 (212) 555-0177",
      email: "billing@wideworld.com",
      paymentTerms: "NET-60",
      companyId: "COMP-001"
    }
  ]);

  let selectedCustomer = $state(demoCustomers[0]);

  // Document Layout demo lines
  let documentHeader = $state({
    docNo: "SO-10042",
    customer: "CRONUS USA Logistics Corp",
    orderDate: "2026-10-14",
    postingDate: "2026-10-17",
    currency: "USD",
    status: "Released"
  });

  let documentLines = $state([
    { lineNo: 10000, item: "ITEM-1001", description: 'Touring Bicycle Blue 28"', qty: 5, unitPrice: 1250.0, lineAmount: 6250.0 },
    { lineNo: 20000, item: "ITEM-1003", description: "Road Cycling Helmet Carbon", qty: 10, unitPrice: 85.0, lineAmount: 850.0 },
    { lineNo: 30000, item: "ITEM-1005", description: "Bicycle Floor Pump Alloy Gauge", qty: 4, unitPrice: 65.0, lineAmount: 260.0 }
  ]);

  // Board layout state (Kanban)
  let boardOrders = $state([
    { id: "ORD-201", title: "CRONUS USA - Batch A", amount: 6250.0, status: "Draft", priority: "Normal" },
    { id: "ORD-202", title: "Progressive - Fixtures", amount: 3400.0, status: "Open", priority: "High" },
    { id: "ORD-203", title: "First Up - Advisory", amount: 8900.0, status: "Approved", priority: "Urgent" },
    { id: "ORD-204", title: "Alpine - Gear Order", amount: 1120.0, status: "Shipped", priority: "Normal" },
    { id: "ORD-205", title: "Wide World - Containers", amount: 14500.0, status: "Approved", priority: "Normal" },
    { id: "ORD-206", title: "Nordic Dist - Spare Parts", amount: 2100.0, status: "Draft", priority: "Low" }
  ]);

  // Timeline events
  let timelineEvents = $state([
    {
      id: 1,
      title: "G/L Register #12 Posted (SAP Storno Reversal)",
      detail: "Reversed Register #11 via Document REV-INV-10042. Debits and credits swapped cleanly.",
      time: "10 mins ago",
      type: "reversal",
      author: "SUPER Administrator"
    },
    {
      id: 2,
      title: "Sales Order SO-10042 Released",
      detail: "State transition applied: Draft -> Released. Validation passed: amount > 0.",
      time: "45 mins ago",
      type: "state",
      author: "SalesManager"
    },
    {
      id: 3,
      title: "Argon2id Security Re-hash Completed",
      detail: "User passwords verified and migrated to PHC format $argon2id$v=19$m=65536,t=3,p=4.",
      time: "2 hours ago",
      type: "security",
      author: "SecurityAdmin"
    },
    {
      id: 4,
      title: "Multi-Company Partition Sync",
      detail: "Company COMP-001 (CRONUS USA) synchronized 5 new ledger entries.",
      time: "5 hours ago",
      type: "system",
      author: "Engine Scheduler"
    }
  ]);

  // Filtered rows for List & Split
  let filteredCustomers = $derived.by(() => {
    const q = searchFilter.trim().toLowerCase();
    if (!q) return demoCustomers;
    return demoCustomers.filter(
      (c) =>
        c.id.toLowerCase().includes(q) ||
        c.name.toLowerCase().includes(q) ||
        c.city.toLowerCase().includes(q) ||
        c.status.toLowerCase().includes(q)
    );
  });

  // Document total
  let docTotal = $derived(
    documentLines.reduce((acc, row) => acc + (row.lineAmount || 0), 0)
  );

  function openCreateModal() {
    crudMode = "create";
    recordDraft = {
      id: `CUST-${1000 + demoCustomers.length + 1}`,
      name: "",
      city: "New York, NY",
      balance: 0,
      creditLimit: 15000,
      status: "Active"
    };
    isCrudModalOpen = true;
  }

  function openEditModal(c) {
    if (!c) return;
    crudMode = "edit";
    recordDraft = { ...c };
    isCrudModalOpen = true;
  }

  function saveRecord() {
    if (!recordDraft.name.trim()) {
      toastStore.push({ tone: "danger", title: "Validation Error", description: "Customer name is required" });
      return;
    }
    if (crudMode === "create") {
      const newRec = {
        ...recordDraft,
        phone: "+1 555-0100",
        email: "contact@" + recordDraft.name.toLowerCase().replace(/[^a-z0-9]/g, "") + ".com",
        paymentTerms: "NET-30",
        companyId: "COMP-001"
      };
      demoCustomers = [...demoCustomers, newRec];
      selectedCustomer = newRec;
      toastStore.push({ tone: "success", title: "Customer Created", description: `Added ${newRec.name} (${newRec.id})` });
    } else {
      demoCustomers = demoCustomers.map((c) => (c.id === recordDraft.id ? { ...c, ...recordDraft } : c));
      if (selectedCustomer?.id === recordDraft.id) {
        selectedCustomer = { ...selectedCustomer, ...recordDraft };
      }
      toastStore.push({ tone: "success", title: "Customer Updated", description: `Updated ${recordDraft.id}` });
    }
    isCrudModalOpen = false;
  }

  function deleteCustomer(id) {
    demoCustomers = demoCustomers.filter((c) => c.id !== id);
    if (selectedCustomer?.id === id) {
      selectedCustomer = demoCustomers[0] || null;
    }
    toastStore.push({ tone: "warning", title: "Record Deleted", description: `Deleted customer ${id}` });
  }

  function moveBoardOrder(orderId, targetStatus) {
    boardOrders = boardOrders.map((o) => (o.id === orderId ? { ...o, status: targetStatus } : o));
    toastStore.push({ tone: "info", title: "Status Transition", description: `Order ${orderId} moved to ${targetStatus}` });
  }
</script>

<div class="space-y-6">
  <!-- Top Showcase Header -->
  <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 pb-4 border-b border-slate-200 dark:border-slate-800">
    <div>
      <div class="flex items-center gap-2">
        <h2 class="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
          Page Surface (8 Layouts)
        </h2>
        <Badge tone="info">docs/28 Architecture</Badge>
      </div>
      <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
        Demonstrating all 8 canonical BOS compact page layout types: List, Record, Document, Board, Workspace, Split, Timeline, and Wizard with live search filtering, FactBox toggling, and CRUD modal simulation.
      </p>
    </div>

    <!-- Mode Selector: Interactive Sandbox vs Live Engine -->
    <div class="flex items-center gap-2">
      <div class="p-1 rounded-lg border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex text-xs">
        <button
          type="button"
          onclick={() => (showcaseMode = "simulator")}
          class="px-3 py-1.5 rounded-md font-medium transition-colors cursor-pointer {showcaseMode === 'simulator' ? 'bg-indigo-600 text-white shadow-2xs' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'}"
        >
          8 Layouts Sandbox
        </button>
        <button
          type="button"
          onclick={() => (showcaseMode = "engine")}
          class="px-3 py-1.5 rounded-md font-medium transition-colors cursor-pointer {showcaseMode === 'engine' ? 'bg-indigo-600 text-white shadow-2xs' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'}"
        >
          Live Engine &lt;PageSurface /&gt;
        </button>
      </div>
    </div>
  </div>

  {#if showcaseMode === "simulator"}
    <!-- 8 Layouts Simulator Navigation Bar -->
    <div class="flex flex-wrap items-center justify-between gap-3 bg-slate-50 dark:bg-slate-900/50 p-2.5 rounded-xl border border-slate-200 dark:border-slate-800">
      <div class="flex flex-wrap items-center gap-1.5">
        {#each [
          { id: "List", label: "List", icon: List },
          { id: "Record", label: "Record", icon: FileText },
          { id: "Document", label: "Document", icon: Receipt },
          { id: "Board", label: "Board", icon: Columns },
          { id: "Workspace", label: "Workspace", icon: LayoutDashboard },
          { id: "Split", label: "Split", icon: SplitSquareVertical },
          { id: "Timeline", label: "Timeline", icon: Clock },
          { id: "Wizard", label: "Wizard", icon: Sparkles }
        ] as layout}
          <button
            type="button"
            onclick={() => (currentLayout = layout.id)}
            class="px-2.5 py-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition-all cursor-pointer {currentLayout === layout.id ? 'bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-xs border border-slate-200 dark:border-slate-700' : 'text-slate-600 dark:text-slate-400 hover:bg-slate-200/50 dark:hover:bg-slate-800/50'}"
          >
            <layout.icon class="size-3.5" />
            <span>{layout.label}</span>
          </button>
        {/each}
      </div>

      <!-- Action buttons & FactBox toggle -->
      <div class="flex items-center gap-2">
        <Button size="xs" icon={Plus} onclick={openCreateModal}>New Record</Button>
        <Button
          size="xs"
          variant={isFactBoxOpen ? "secondary" : "ghost"}
          icon={PanelRight}
          onclick={() => (isFactBoxOpen = !isFactBoxOpen)}
        >
          FactBox
        </Button>
      </div>
    </div>

    <!-- Layout Container with Optional FactBox Split -->
    <div class="flex gap-4 items-start">
      <!-- Main Layout Body -->
      <div class="flex-1 min-w-0 bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl p-4 shadow-2xs space-y-4">
        <!-- Sub-header & Live Search Filter -->
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pb-3 border-b border-slate-100 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <Badge tone="info">{currentLayout} Layout</Badge>
            <span class="text-xs text-slate-400 font-mono">Table: Customer (Singular)</span>
          </div>

          <div class="relative w-full sm:w-64">
            <Search class="size-3.5 absolute left-2.5 top-2.5 text-slate-400 pointer-events-none" />
            <input
              type="text"
              bind:value={searchFilter}
              placeholder="Search records..."
              class="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900 focus:outline-hidden focus:ring-1 focus:ring-indigo-500"
            />
          </div>
        </div>

        <!-- 1. LIST LAYOUT -->
        {#if currentLayout === "List"}
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse">
              <thead>
                <tr class="border-b border-slate-200 dark:border-slate-800 text-slate-500 font-medium">
                  <th class="py-2 px-3">No.</th>
                  <th class="py-2 px-3">Customer Name</th>
                  <th class="py-2 px-3">City</th>
                  <th class="py-2 px-3 text-right">Balance (USD)</th>
                  <th class="py-2 px-3 text-right">Credit Limit</th>
                  <th class="py-2 px-3">Status</th>
                  <th class="py-2 px-3 text-right">Actions</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 dark:divide-slate-800/60">
                {#each filteredCustomers as customer (customer.id)}
                  <tr
                    tabindex="0"
                    role="row"
                    onclick={() => (selectedCustomer = customer)}
                    onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") selectedCustomer = customer; }}
                    class="hover:bg-slate-50 dark:hover:bg-slate-900/60 cursor-pointer transition-colors {selectedCustomer?.id === customer.id ? 'bg-indigo-50/60 dark:bg-indigo-950/30' : ''}"
                  >
                    <td class="py-2.5 px-3 font-mono font-medium text-indigo-600 dark:text-indigo-400">{customer.id}</td>
                    <td class="py-2.5 px-3 font-medium text-slate-900 dark:text-slate-100">{customer.name}</td>
                    <td class="py-2.5 px-3 text-slate-600 dark:text-slate-400">{customer.city}</td>
                    <td class="py-2.5 px-3 text-right font-mono font-medium">${customer.balance.toLocaleString(undefined, { minimumFractionDigits: 2 })}</td>
                    <td class="py-2.5 px-3 text-right font-mono text-slate-500">${customer.creditLimit.toLocaleString()}</td>
                    <td class="py-2.5 px-3">
                      <Badge tone={customer.status === 'Active' ? 'success' : customer.status === 'Blocked' ? 'danger' : 'warning'}>
                        {customer.status}
                      </Badge>
                    </td>
                    <td class="py-2.5 px-3 text-right space-x-1" onclick={(e) => e.stopPropagation()}>
                      <button
                        type="button"
                        onclick={() => openEditModal(customer)}
                        class="p-1 rounded text-slate-400 hover:text-indigo-600 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer"
                        title="Edit"
                      >
                        <Edit3 class="size-3.5" />
                      </button>
                      <button
                        type="button"
                        onclick={() => deleteCustomer(customer.id)}
                        class="p-1 rounded text-slate-400 hover:text-rose-600 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer"
                        title="Delete"
                      >
                        <Trash2 class="size-3.5" />
                      </button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

        <!-- 2. RECORD / CARD LAYOUT -->
        {:else if currentLayout === "Record"}
          {#if selectedCustomer}
            <div class="space-y-4">
              <div class="flex items-center justify-between bg-slate-50 dark:bg-slate-900 p-3 rounded-lg border border-slate-200 dark:border-slate-800">
                <div class="flex items-center gap-3">
                  <div class="size-10 rounded-full bg-indigo-100 dark:bg-indigo-950/70 flex items-center justify-center text-indigo-600 dark:text-indigo-400 font-bold text-sm">
                    {selectedCustomer.name.slice(0, 2).toUpperCase()}
                  </div>
                  <div>
                    <h3 class="font-bold text-sm text-slate-900 dark:text-slate-100">{selectedCustomer.name}</h3>
                    <p class="text-xs text-slate-500 font-mono">{selectedCustomer.id} • {selectedCustomer.city}</p>
                  </div>
                </div>
                <div class="flex items-center gap-2">
                  <Button size="xs" variant="secondary" icon={Edit3} onclick={() => openEditModal(selectedCustomer)}>Edit</Button>
                  <Badge tone={selectedCustomer.status === 'Active' ? 'success' : 'danger'}>{selectedCustomer.status}</Badge>
                </div>
              </div>

              <!-- FastTab: General Information -->
              <div class="border border-slate-200 dark:border-slate-800 rounded-lg p-4 space-y-3">
                <div class="font-semibold text-xs uppercase tracking-wider text-slate-500 pb-2 border-b border-slate-100 dark:border-slate-800">
                  General FastTab
                </div>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-xs">
                  <div>
                    <span class="text-slate-400 block text-[11px]">Customer No.</span>
                    <span class="font-mono font-medium">{selectedCustomer.id}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 block text-[11px]">Primary Phone</span>
                    <span>{selectedCustomer.phone}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 block text-[11px]">Billing Email</span>
                    <span>{selectedCustomer.email}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 block text-[11px]">Assigned Company Partition</span>
                    <span class="font-mono text-indigo-600 dark:text-indigo-400">{selectedCustomer.companyId}</span>
                  </div>
                </div>
              </div>

              <!-- FastTab: Invoicing & Financial Terms -->
              <div class="border border-slate-200 dark:border-slate-800 rounded-lg p-4 space-y-3">
                <div class="font-semibold text-xs uppercase tracking-wider text-slate-500 pb-2 border-b border-slate-100 dark:border-slate-800">
                  Invoicing & Financial Terms
                </div>
                <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs">
                  <div>
                    <span class="text-slate-400 block text-[11px]">Current Balance LCY</span>
                    <span class="font-mono font-bold text-slate-900 dark:text-slate-100 text-sm">
                      ${selectedCustomer.balance.toLocaleString(undefined, { minimumFractionDigits: 2 })}
                    </span>
                  </div>
                  <div>
                    <span class="text-slate-400 block text-[11px]">Credit Limit LCY</span>
                    <span class="font-mono font-medium">${selectedCustomer.creditLimit.toLocaleString()}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 block text-[11px]">Payment Terms Code</span>
                    <Badge tone="neutral">{selectedCustomer.paymentTerms}</Badge>
                  </div>
                </div>
              </div>
            </div>
          {:else}
            <EmptyState icon={FileText} title="No record selected" description="Select a customer to view FastTab details." />
          {/if}

        <!-- 3. DOCUMENT LAYOUT -->
        {:else if currentLayout === "Document"}
          <div class="space-y-4">
            <!-- Header Section -->
            <div class="bg-slate-50 dark:bg-slate-900/60 p-4 rounded-lg border border-slate-200 dark:border-slate-800 space-y-3">
              <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-sm text-slate-900 dark:text-slate-100">{documentHeader.docNo}</h3>
                  <Badge tone="info">{documentHeader.status}</Badge>
                </div>
                <Button size="xs" icon={Play} onclick={() => toastStore.push({ tone: "success", title: "Document Posted", description: `Posted ${documentHeader.docNo} to Ledger.` })}>
                  Post Document
                </Button>
              </div>
              <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs">
                <div>
                  <span class="text-slate-400 text-[11px] block">Customer</span>
                  <span class="font-medium">{documentHeader.customer}</span>
                </div>
                <div>
                  <span class="text-slate-400 text-[11px] block">Order Date</span>
                  <span class="font-mono">{documentHeader.orderDate}</span>
                </div>
                <div>
                  <span class="text-slate-400 text-[11px] block">Posting Date</span>
                  <span class="font-mono">{documentHeader.postingDate}</span>
                </div>
                <div>
                  <span class="text-slate-400 text-[11px] block">Currency</span>
                  <span class="font-mono font-semibold">{documentHeader.currency}</span>
                </div>
              </div>
            </div>

            <!-- Child Subpart: Document Lines Grid -->
            <div>
              <div class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-2">Order Lines</div>
              <div class="overflow-x-auto border border-slate-200 dark:border-slate-800 rounded-lg">
                <table class="w-full text-left text-xs border-collapse">
                  <thead class="bg-slate-50 dark:bg-slate-900 text-slate-500">
                    <tr class="border-b border-slate-200 dark:border-slate-800">
                      <th class="py-2 px-3">Line</th>
                      <th class="py-2 px-3">Item No.</th>
                      <th class="py-2 px-3">Description</th>
                      <th class="py-2 px-3 text-right">Quantity</th>
                      <th class="py-2 px-3 text-right">Unit Price</th>
                      <th class="py-2 px-3 text-right">Total Amount</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100 dark:divide-slate-800">
                    {#each documentLines as line}
                      <tr>
                        <td class="py-2 px-3 font-mono text-slate-400">{line.lineNo}</td>
                        <td class="py-2 px-3 font-mono font-medium text-indigo-600 dark:text-indigo-400">{line.item}</td>
                        <td class="py-2 px-3">{line.description}</td>
                        <td class="py-2 px-3 text-right font-mono">{line.qty}</td>
                        <td class="py-2 px-3 text-right font-mono">${line.unitPrice.toFixed(2)}</td>
                        <td class="py-2 px-3 text-right font-mono font-semibold">${line.lineAmount.toFixed(2)}</td>
                      </tr>
                    {/each}
                  </tbody>
                  <tfoot>
                    <tr class="bg-slate-50 dark:bg-slate-900 font-bold border-t border-slate-200 dark:border-slate-800">
                      <td colspan="5" class="py-2.5 px-3 text-right">Total Invoice LCY:</td>
                      <td class="py-2.5 px-3 text-right font-mono text-sm text-indigo-600 dark:text-indigo-400">
                        ${docTotal.toLocaleString(undefined, { minimumFractionDigits: 2 })}
                      </td>
                    </tr>
                  </tfoot>
                </table>
              </div>
            </div>
          </div>

        <!-- 4. BOARD / KANBAN LAYOUT -->
        {:else if currentLayout === "Board"}
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
            {#each ["Draft", "Open", "Approved", "Shipped"] as colStatus}
              {@const colOrders = boardOrders.filter((o) => o.status === colStatus)}
              <div class="bg-slate-50 dark:bg-slate-900/60 p-3 rounded-lg border border-slate-200 dark:border-slate-800 space-y-2">
                <div class="flex items-center justify-between pb-1.5 border-b border-slate-200 dark:border-slate-800">
                  <span class="text-xs font-semibold text-slate-700 dark:text-slate-300">{colStatus}</span>
                  <Badge tone="neutral">{colOrders.length}</Badge>
                </div>
                <div class="space-y-2 min-h-48">
                  {#each colOrders as order (order.id)}
                    <div class="p-2.5 rounded-md bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 shadow-2xs space-y-1.5 text-xs">
                      <div class="flex items-center justify-between">
                        <span class="font-mono font-semibold text-indigo-600 dark:text-indigo-400">{order.id}</span>
                        <Badge tone={order.priority === 'Urgent' ? 'danger' : order.priority === 'High' ? 'warning' : 'neutral'}>
                          {order.priority}
                        </Badge>
                      </div>
                      <div class="text-slate-800 dark:text-slate-200 font-medium">{order.title}</div>
                      <div class="flex items-center justify-between pt-1 border-t border-slate-100 dark:border-slate-800/80">
                        <span class="font-mono font-medium">${order.amount.toLocaleString()}</span>
                        {#if colStatus !== "Shipped"}
                          <button
                            type="button"
                            onclick={() => {
                              const next = colStatus === 'Draft' ? 'Open' : colStatus === 'Open' ? 'Approved' : 'Shipped';
                              moveBoardOrder(order.id, next);
                            }}
                            class="text-[11px] text-indigo-600 hover:underline flex items-center gap-0.5 cursor-pointer"
                          >
                            Advance <ArrowRight class="size-3" />
                          </button>
                        {/if}
                      </div>
                    </div>
                  {/each}
                </div>
              </div>
            {/each}
          </div>

        <!-- 5. WORKSPACE / ROLECENTER LAYOUT -->
        {:else if currentLayout === "Workspace"}
          <div class="space-y-4">
            <!-- Cue Cards KPI Strip -->
            <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
              <div class="p-3 bg-indigo-50/50 dark:bg-indigo-950/20 border border-indigo-200 dark:border-indigo-900/50 rounded-lg">
                <span class="text-[11px] font-medium text-slate-500 uppercase tracking-wider block">YTD Posted Sales</span>
                <span class="text-xl font-bold font-mono text-indigo-600 dark:text-indigo-400">$1,482,900</span>
                <span class="text-[10px] text-emerald-600 font-medium block mt-1">+14.2% vs budget</span>
              </div>
              <div class="p-3 bg-emerald-50/50 dark:bg-emerald-950/20 border border-emerald-200 dark:border-emerald-900/50 rounded-lg">
                <span class="text-[11px] font-medium text-slate-500 uppercase tracking-wider block">Open Invoices</span>
                <span class="text-xl font-bold font-mono text-emerald-600 dark:text-emerald-400">42</span>
                <span class="text-[10px] text-slate-500 block mt-1">Ready for posting</span>
              </div>
              <div class="p-3 bg-amber-50/50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-900/50 rounded-lg">
                <span class="text-[11px] font-medium text-slate-500 uppercase tracking-wider block">Overdue Receivables</span>
                <span class="text-xl font-bold font-mono text-amber-600 dark:text-amber-400">$38,200</span>
                <span class="text-[10px] text-amber-600 font-medium block mt-1">1 Account Blocked</span>
              </div>
              <div class="p-3 bg-rose-50/50 dark:bg-rose-950/20 border border-rose-200 dark:border-rose-900/50 rounded-lg">
                <span class="text-[11px] font-medium text-slate-500 uppercase tracking-wider block">Storno Reversals</span>
                <span class="text-xl font-bold font-mono text-rose-600 dark:text-rose-400">2 Registers</span>
                <span class="text-[10px] text-slate-500 block mt-1">SAP S/4HANA audited</span>
              </div>
            </div>

            <!-- Quick Action Tiles -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
              <button
                type="button"
                onclick={openCreateModal}
                class="p-4 rounded-lg border border-slate-200 dark:border-slate-800 hover:border-indigo-400 dark:hover:border-indigo-600 text-left transition-colors cursor-pointer group"
              >
                <Plus class="size-5 text-indigo-500 group-hover:scale-110 transition-transform mb-2" />
                <div class="font-semibold text-xs text-slate-900 dark:text-slate-100">Create New Customer</div>
                <div class="text-[11px] text-slate-400">Provision record with singular PascalCase naming</div>
              </button>
              <button
                type="button"
                onclick={() => toastStore.push({ tone: "info", title: "Batch Posting Queued" })}
                class="p-4 rounded-lg border border-slate-200 dark:border-slate-800 hover:border-indigo-400 dark:hover:border-indigo-600 text-left transition-colors cursor-pointer group"
              >
                <Play class="size-5 text-emerald-500 group-hover:scale-110 transition-transform mb-2" />
                <div class="font-semibold text-xs text-slate-900 dark:text-slate-100">Post Batch Journals</div>
                <div class="text-[11px] text-slate-400">Run General Journal Codeunit 12 posting routine</div>
              </button>
              <button
                type="button"
                onclick={() => toastStore.push({ tone: "success", title: "Audit Log Generated" })}
                class="p-4 rounded-lg border border-slate-200 dark:border-slate-800 hover:border-indigo-400 dark:hover:border-indigo-600 text-left transition-colors cursor-pointer group"
              >
                <ShieldCheck class="size-5 text-violet-500 group-hover:scale-110 transition-transform mb-2" />
                <div class="font-semibold text-xs text-slate-900 dark:text-slate-100">Argon2id Integrity Audit</div>
                <div class="text-[11px] text-slate-400">Verify user hash parameters and RBAC role boundaries</div>
              </button>
            </div>
          </div>

        <!-- 6. SPLIT LAYOUT (Master-Detail) -->
        {:else if currentLayout === "Split"}
          <div class="grid grid-cols-1 md:grid-cols-12 gap-4 h-96">
            <!-- Left Master List -->
            <div class="md:col-span-5 border border-slate-200 dark:border-slate-800 rounded-lg overflow-y-auto divide-y divide-slate-100 dark:divide-slate-800">
              {#each filteredCustomers as c (c.id)}
                <button
                  type="button"
                  onclick={() => (selectedCustomer = c)}
                  class="w-full text-left p-3 hover:bg-slate-50 dark:hover:bg-slate-900/60 cursor-pointer transition-colors block {selectedCustomer?.id === c.id ? 'bg-indigo-50/80 dark:bg-indigo-950/40 border-l-3 border-indigo-600' : ''}"
                >
                  <div class="flex items-center justify-between">
                    <span class="font-mono text-xs font-semibold text-indigo-600 dark:text-indigo-400">{c.id}</span>
                    <Badge tone={c.status === 'Active' ? 'success' : 'danger'}>{c.status}</Badge>
                  </div>
                  <div class="text-xs font-medium text-slate-900 dark:text-slate-100 mt-1">{c.name}</div>
                  <div class="flex items-center justify-between text-[11px] text-slate-400 mt-1">
                    <span>{c.city}</span>
                    <span class="font-mono font-medium">${c.balance.toLocaleString()}</span>
                  </div>
                </button>
              {/each}
            </div>

            <!-- Right Detail Pane -->
            <div class="md:col-span-7 border border-slate-200 dark:border-slate-800 rounded-lg p-4 space-y-3 bg-slate-50/50 dark:bg-slate-900/30 overflow-y-auto">
              {#if selectedCustomer}
                <div class="flex items-center justify-between pb-3 border-b border-slate-200 dark:border-slate-800">
                  <div>
                    <h4 class="font-bold text-sm text-slate-900 dark:text-slate-100">{selectedCustomer.name}</h4>
                    <p class="text-xs text-slate-500 font-mono">{selectedCustomer.id} • {selectedCustomer.city}</p>
                  </div>
                  <Button size="xs" variant="secondary" icon={Edit3} onclick={() => openEditModal(selectedCustomer)}>Edit</Button>
                </div>
                <div class="grid grid-cols-2 gap-3 text-xs">
                  <div>
                    <span class="text-slate-400 text-[11px] block">Current Balance</span>
                    <span class="font-mono font-bold text-slate-900 dark:text-slate-100">${selectedCustomer.balance.toLocaleString(undefined, { minimumFractionDigits: 2 })}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 text-[11px] block">Credit Limit</span>
                    <span class="font-mono">${selectedCustomer.creditLimit.toLocaleString()}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 text-[11px] block">Contact Email</span>
                    <span>{selectedCustomer.email}</span>
                  </div>
                  <div>
                    <span class="text-slate-400 text-[11px] block">Terms</span>
                    <Badge tone="neutral">{selectedCustomer.paymentTerms}</Badge>
                  </div>
                </div>
              {:else}
                <EmptyState icon={SplitSquareVertical} title="No record selected" description="Click on a row on the left to inspect its details." />
              {/if}
            </div>
          </div>

        <!-- 7. TIMELINE LAYOUT -->
        {:else if currentLayout === "Timeline"}
          <div class="space-y-4">
            <div class="relative pl-6 space-y-6 before:absolute before:left-2 before:top-2 before:bottom-2 before:w-0.5 before:bg-slate-200 dark:before:bg-slate-800">
              {#each timelineEvents as ev (ev.id)}
                <div class="relative group">
                  <div class="absolute -left-6 top-1 size-4 rounded-full border-2 border-white dark:border-slate-950 {ev.type === 'reversal' ? 'bg-rose-500' : ev.type === 'state' ? 'bg-indigo-500' : ev.type === 'security' ? 'bg-emerald-500' : 'bg-slate-400'}"></div>
                  <div class="bg-slate-50 dark:bg-slate-900 p-3 rounded-lg border border-slate-200 dark:border-slate-800 space-y-1">
                    <div class="flex items-center justify-between text-xs">
                      <span class="font-bold text-slate-900 dark:text-slate-100">{ev.title}</span>
                      <span class="text-slate-400 font-mono text-[11px]">{ev.time}</span>
                    </div>
                    <p class="text-xs text-slate-600 dark:text-slate-400">{ev.detail}</p>
                    <div class="flex items-center gap-2 pt-1 text-[11px] text-slate-400">
                      <span>Actor:</span>
                      <Badge tone="neutral">{ev.author}</Badge>
                    </div>
                  </div>
                </div>
              {/each}
            </div>
          </div>

        <!-- 8. WIZARD LAYOUT -->
        {:else if currentLayout === "Wizard"}
          <div class="space-y-6 max-w-xl mx-auto py-2">
            <!-- Step Tracker -->
            <div class="flex items-center justify-between">
              {#each [
                { step: 1, label: "Entity Profile" },
                { step: 2, label: "Posting & Dimensions" },
                { step: 3, label: "Confirmation" }
              ] as s}
                <div class="flex items-center gap-2">
                  <div class="size-7 rounded-full flex items-center justify-center font-bold text-xs {wizardStep >= s.step ? 'bg-indigo-600 text-white' : 'bg-slate-100 dark:bg-slate-800 text-slate-400'}">
                    {s.step}
                  </div>
                  <span class="text-xs font-medium {wizardStep >= s.step ? 'text-slate-900 dark:text-slate-100' : 'text-slate-400'} hidden sm:inline">
                    {s.label}
                  </span>
                </div>
              {/each}
            </div>

            <!-- Wizard Step Body -->
            <div class="p-4 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/40 space-y-4">
              {#if wizardStep === 1}
                <h4 class="font-bold text-sm">Step 1: Customer Profile Setup</h4>
                <div class="space-y-3">
                  <Input label="Customer Full Legal Name" placeholder="e.g. Apex Global Logistics Inc" />
                  <Input label="Primary City / Jurisdiction" placeholder="e.g. Dallas, TX" />
                </div>
              {:else if wizardStep === 2}
                <h4 class="font-bold text-sm">Step 2: General Ledger & Dimensions</h4>
                <div class="space-y-3">
                  <Select
                    label="Customer Posting Group"
                    options={[
                      { value: "DOMESTIC", label: "DOMESTIC - US Standard" },
                      { value: "EU", label: "EU - Intra-Community VAT" },
                      { value: "EXPORT", label: "EXPORT - Zero Rated Foreign" }
                    ]}
                  />
                  <Input label="Dimension Set ID" value="DIM-US-CORP-40" />
                </div>
              {:else}
                <h4 class="font-bold text-sm text-emerald-600 flex items-center gap-1.5">
                  <Check class="size-4" /> Ready to Provision Customer
                </h4>
                <p class="text-xs text-slate-500">
                  Customer will be initialized with immutable system columns (_id, _tenant_id, _company_id, _system_version).
                </p>
              {/if}
            </div>

            <!-- Wizard Step Controls -->
            <div class="flex items-center justify-between pt-2">
              <Button
                size="xs"
                variant="secondary"
                disabled={wizardStep === 1}
                onclick={() => (wizardStep -= 1)}
              >
                Previous
              </Button>
              {#if wizardStep < 3}
                <Button size="xs" onclick={() => (wizardStep += 1)}>Next Step</Button>
              {:else}
                <Button size="xs" onclick={() => { wizardStep = 1; toastStore.push({ tone: "success", title: "Wizard Completed", description: "Customer onboarded successfully." }); }}>
                  Finish & Provision
                </Button>
              {/if}
            </div>
          </div>
        {/if}
      </div>

      <!-- Optional FactBox Side Panel (docs/28) -->
      {#if isFactBoxOpen}
        <aside class="w-72 shrink-0 bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl p-4 shadow-2xs space-y-4 text-xs">
          <div class="flex items-center justify-between pb-2 border-b border-slate-200 dark:border-slate-800">
            <span class="font-bold uppercase tracking-wider text-[11px] text-slate-500">FactBox Statistics</span>
            <button
              type="button"
              onclick={() => (isFactBoxOpen = false)}
              class="text-slate-400 hover:text-slate-600"
            >
              <X class="size-3.5" />
            </button>
          </div>

          {#if selectedCustomer}
            <div class="space-y-3">
              <div>
                <span class="text-slate-400 text-[10px] uppercase font-semibold block">Customer Name</span>
                <span class="font-bold text-slate-900 dark:text-slate-100">{selectedCustomer.name}</span>
              </div>

              <div class="p-2.5 rounded-lg bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-100 dark:border-indigo-900/40 space-y-1">
                <span class="text-[10px] font-semibold text-indigo-700 dark:text-indigo-300 uppercase block">Credit Utilization</span>
                <div class="flex justify-between items-center font-mono">
                  <span>${selectedCustomer.balance.toLocaleString()}</span>
                  <span class="text-slate-400">/ ${selectedCustomer.creditLimit.toLocaleString()}</span>
                </div>
                <!-- Progress bar -->
                <div class="w-full bg-slate-200 dark:bg-slate-800 h-1.5 rounded-full overflow-hidden mt-1">
                  <div
                    class="bg-indigo-600 h-full rounded-full"
                    style="width: {Math.min(100, Math.round((selectedCustomer.balance / selectedCustomer.creditLimit) * 100))}%"
                  ></div>
                </div>
              </div>

              <div class="space-y-1.5 pt-1">
                <div class="flex justify-between text-slate-600 dark:text-slate-400">
                  <span>Partition ID:</span>
                  <span class="font-mono text-indigo-600">{selectedCustomer.companyId}</span>
                </div>
                <div class="flex justify-between text-slate-600 dark:text-slate-400">
                  <span>Payment Terms:</span>
                  <span class="font-mono">{selectedCustomer.paymentTerms}</span>
                </div>
                <div class="flex justify-between text-slate-600 dark:text-slate-400">
                  <span>RLS Status:</span>
                  <Badge tone="success">Isolated</Badge>
                </div>
              </div>
            </div>
          {:else}
            <div class="text-slate-400 py-4 text-center">Select a customer record to inspect FactBox metrics.</div>
          {/if}
        </aside>
      {/if}
    </div>

  {:else}
    <!-- Live Catalog Engine Mode: Embeds <PageSurface pageName="..." /> -->
    <div class="space-y-4">
      <div class="p-3 bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-100 dark:border-indigo-900/40 rounded-xl flex items-center justify-between text-xs">
        <div class="flex items-center gap-2">
          <Building2 class="size-4 text-indigo-600" />
          <span>Live Axum Runtime Page Surface Component: <code>&lt;PageSurface pageName="{liveEnginePage}" /&gt;</code></span>
        </div>
        <div class="w-48">
          <Select
            bind:value={liveEnginePage}
            options={[
              { value: "CustomerList", label: "CustomerList" },
              { value: "SalesOrderDoc", label: "SalesOrderDoc" },
              { value: "ItemBoard", label: "ItemBoard" },
              { value: "AccountantWorkspace", label: "AccountantWorkspace" }
            ]}
          />
        </div>
      </div>

      <div class="bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl p-4 shadow-2xs">
        <PageSurface pageName={liveEnginePage} />
      </div>
    </div>
  {/if}
</div>

<!-- CRUD Creation / Editing Modal -->
<Dialog bind:open={isCrudModalOpen} title="{crudMode === 'create' ? 'Create New' : 'Edit'} Customer Record" description="Populate singular table fields with non-invasive validation.">
  {#snippet children()}
    <div class="space-y-3 py-2 text-xs">
      <Input label="Customer No. (ID)" bind:value={recordDraft.id} disabled={crudMode === 'edit'} placeholder="CUST-1006" />
      <Input label="Customer Full Name" bind:value={recordDraft.name} placeholder="e.g. Cronus Global Services" />
      <Input label="City / Region" bind:value={recordDraft.city} placeholder="e.g. San Francisco, CA" />
      <div class="grid grid-cols-2 gap-3">
        <Input label="Balance LCY" type="number" bind:value={recordDraft.balance} />
        <Input label="Credit Limit" type="number" bind:value={recordDraft.creditLimit} />
      </div>
      <Select
        label="Account Status"
        bind:value={recordDraft.status}
        options={[
          { value: "Active", label: "Active" },
          { value: "Blocked", label: "Blocked" },
          { value: "Pending", label: "Pending" }
        ]}
      />
    </div>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" size="sm" onclick={() => (isCrudModalOpen = false)}>Cancel</Button>
    <Button size="sm" onclick={saveRecord}>Save Record</Button>
  {/snippet}
</Dialog>
