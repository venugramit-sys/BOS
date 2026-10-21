<script>
  import { Badge, Button, DataTable, Kbd, toastStore } from "$lib/bos-ui/index.js";

  let rows = $state([
    { id: "C-1001", name: "Northwind Traders", city: "Seattle", status: "Open", salesperson: "Ann Berg", balance: 12840.5, creditLimit: 25000, blocked: false, shipComplete: true, posted: "2026-10-02" },
    { id: "C-1002", name: "Contoso Retail", city: "Chicago", status: "Released", salesperson: "Ben Cho", balance: 8420, creditLimit: 15000, blocked: false, shipComplete: false, posted: "2026-09-18" },
    { id: "C-1003", name: "Fabrikam Foods", city: "Seattle", status: "Posted", salesperson: "Ann Berg", balance: 3100.25, creditLimit: 8000, blocked: false, shipComplete: true, posted: "2026-08-30" },
    { id: "C-1004", name: "Wide World Importers", city: "Austin", status: "Open", salesperson: "Cara Diaz", balance: 22150, creditLimit: 40000, blocked: false, shipComplete: true, posted: "2026-10-11" },
    { id: "C-1005", name: "Adventure Works", city: "Denver", status: "Blocked", salesperson: "Ben Cho", balance: 0, creditLimit: 5000, blocked: true, shipComplete: false, posted: "2026-07-14" },
    { id: "C-1006", name: "Litware Hotels", city: "Chicago", status: "Released", salesperson: "Cara Diaz", balance: 6400, creditLimit: 12000, blocked: false, shipComplete: true, posted: "2026-10-01" },
    { id: "C-1007", name: "Proseware Market", city: "Austin", status: "Open", salesperson: "Ann Berg", balance: 980.75, creditLimit: 6000, blocked: false, shipComplete: false, posted: "2026-09-22" },
    { id: "C-1008", name: "Alpine Ski House", city: "Denver", status: "Posted", salesperson: "Ben Cho", balance: 15440, creditLimit: 20000, blocked: false, shipComplete: true, posted: "2026-06-19" },
    { id: "C-1009", name: "Coho Winery", city: "Seattle", status: "Open", salesperson: "Cara Diaz", balance: 4320, creditLimit: 9000, blocked: false, shipComplete: true, posted: "2026-10-08" },
    { id: "C-1010", name: "Graphic Design Institute", city: "Chicago", status: "Blocked", salesperson: "Ann Berg", balance: 760, creditLimit: 2000, blocked: true, shipComplete: false, posted: "2026-05-03" },
    { id: "C-1011", name: "Humongous Insurance", city: "Austin", status: "Released", salesperson: "Ben Cho", balance: 18990, creditLimit: 30000, blocked: false, shipComplete: true, posted: "2026-09-29" },
    { id: "C-1012", name: "Lucerne Publishing", city: "Denver", status: "Open", salesperson: "Cara Diaz", balance: 2750.4, creditLimit: 7000, blocked: false, shipComplete: false, posted: "2026-10-12" },
    { id: "C-1013", name: "Margie's Travel", city: "Seattle", status: "Posted", salesperson: "Ann Berg", balance: 5100, creditLimit: 10000, blocked: false, shipComplete: true, posted: "2026-08-02" },
    { id: "C-1014", name: "School of Fine Art", city: "Chicago", status: "Open", salesperson: "Ben Cho", balance: 640, creditLimit: 3500, blocked: false, shipComplete: false, posted: "2026-10-09" },
    { id: "C-1015", name: "Southridge Video", city: "Austin", status: "Released", salesperson: "Cara Diaz", balance: 12110, creditLimit: 18000, blocked: false, shipComplete: true, posted: "2026-09-15" },
    { id: "C-1016", name: "Tailspin Toys", city: "Denver", status: "Open", salesperson: "Ann Berg", balance: 8900, creditLimit: 16000, blocked: false, shipComplete: true, posted: "2026-10-06" }
  ]);

  let tableApi = $state(null);
  let editLog = $state("Click a credit limit, status, salesperson, date, blocked, or ship-complete cell to edit it.");

  const statusOptions = [
    { value: "Open", label: "Open" },
    { value: "Released", label: "Released" },
    { value: "Posted", label: "Posted" },
    { value: "Blocked", label: "Blocked" }
  ];

  const salespeople = [
    { value: "Ann Berg", label: "Ann Berg" },
    { value: "Ben Cho", label: "Ben Cho" },
    { value: "Cara Diaz", label: "Cara Diaz" }
  ];

  const columns = [
    { id: "id", accessorKey: "id", header: "No.", pin: "start", size: 110 },
    { id: "name", accessorKey: "name", header: "Customer", kind: "custom", size: 220 },
    { id: "city", accessorKey: "city", header: "City", groupable: true, filter: "select", size: 140 },
    {
      id: "status",
      accessorKey: "status",
      header: "Status",
      kind: "badge",
      filter: "select",
      groupable: true,
      editable: true,
      editor: "select",
      options: statusOptions,
      badgeTone: (value) => (value === "Posted" ? "success" : value === "Released" ? "info" : value === "Blocked" ? "danger" : "warning"),
      size: 140
    },
    {
      id: "salesperson",
      accessorKey: "salesperson",
      header: "Salesperson",
      editable: true,
      editor: "lookup",
      filter: "select",
      options: salespeople,
      size: 160
    },
    {
      id: "amounts",
      header: "Amounts",
      columns: [
        { id: "balance", accessorKey: "balance", header: "Balance", kind: "number", aggregate: "sum", decimals: 2, size: 140 },
        { id: "creditLimit", accessorKey: "creditLimit", header: "Credit limit", kind: "number", aggregate: "sum", editable: true, editor: "number", decimals: 2, size: 150 }
      ]
    },
    { id: "blocked", accessorKey: "blocked", header: "Blocked", kind: "checkbox", editable: true, size: 100 },
    { id: "shipComplete", accessorKey: "shipComplete", header: "Ship complete", kind: "switch", editable: true, size: 130 },
    { id: "posted", accessorKey: "posted", header: "Posted", kind: "date", editable: true, editor: "date", size: 150 }
  ];

  const previewColumns = [
    { id: "id", accessorKey: "id", header: "Order", pin: "start", size: 110 },
    { id: "customer", accessorKey: "customer", header: "Customer", size: 200 },
    { id: "amount", accessorKey: "amount", header: "Amount", kind: "number", aggregate: "sum", decimals: 2, size: 140 }
  ];

  const previewSeed = [
    { id: "SO-4401", customer: "Northwind Traders", amount: 1280 },
    { id: "SO-4402", customer: "Contoso Retail", amount: 640 }
  ];

  let listState = $state("ready");

  let previewRows = $derived(listState === "empty" || listState === "loading" ? [] : previewSeed);
  let previewLoading = $derived(listState === "loading");
  let previewError = $derived(listState === "error" ? "The sales service did not answer. Try the list again." : "");

  function statusTone(value) {
    if (value === "Posted") return "success";
    if (value === "Released") return "info";
    if (value === "Blocked") return "danger";
    return "warning";
  }
</script>

<div class="space-y-8">
  <div class="space-y-2">
    <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Data table</h2>
    <p class="text-xs text-slate-500 dark:text-slate-400">
      Source: <code class="rounded bg-slate-200 px-1.5 py-0.5 font-mono text-[11px] dark:bg-slate-800">$lib/bos-ui/data-table</code>
    </p>
    <p class="max-w-3xl text-sm text-slate-500 dark:text-slate-400">
      TanStack Table keeps the rows sorted, filtered, grouped, and paged. bos-ui draws the toolbar, badges, editors, and the column dialog. Row height follows the density setting on the Theme page. Column order, width, and pins for this list are saved in this browser.
    </p>
  </div>

  <div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
    <div class="rounded-lg border border-slate-200 bg-white p-3 text-xs text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
      <div class="mb-1 font-semibold text-slate-800 dark:text-slate-100">Sort, filter, group</div>
      Click a header to sort. Shift-click adds another sort. Open Filters and type <span class="font-mono">&gt;10000</span> or <span class="font-mono">1000..9000</span> on Balance. Group by City or Status, then Expand or Collapse.
    </div>
    <div class="rounded-lg border border-slate-200 bg-white p-3 text-xs text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
      <div class="mb-1 font-semibold text-slate-800 dark:text-slate-100">Edit a cell</div>
      Click Credit limit, Status, Salesperson, Posted, Blocked, or Ship complete. <Kbd keys="Enter" /> commits. <Kbd keys="Escape" /> cancels. <Kbd keys="F8" /> copies the cell above. Arrow keys move the focus.
    </div>
    <div class="rounded-lg border border-slate-200 bg-white p-3 text-xs text-slate-600 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-300">
      <div class="mb-1 font-semibold text-slate-800 dark:text-slate-100">Pin, resize, export</div>
      Drag a header edge to resize. Right-click a header to pin, group, or hide it. Columns saves the layout. Check rows, then Release. Double-click a row to open it. Export downloads the filtered rows.
    </div>
  </div>

  <div class="h-[42rem]">
    <DataTable
      label="Customers"
      {columns}
      bind:rows
      bind:api={tableApi}
      stateKey="showcase.customers.v2"
      pageSize={8}
      onCellChange={(row, key, value) => (editLog = `${row.id}.${key} = ${value}`)}
      onRowActivate={(row) => toastStore.push({ tone: "info", title: row.name, description: `${row.id} · ${row.city} · ${row.salesperson}` })}
      contextItems={(row) => [
        { label: "Copy number", action: () => navigator.clipboard?.writeText(row.id) }
      ]}
    >
      {#snippet cell({ row })}
        <span class="block min-w-0 px-1">
          <span class="block truncate font-medium text-slate-800 dark:text-slate-100">{row.name}</span>
          <span class="mt-0.5 block truncate text-[10px] text-slate-400">{row.city}</span>
        </span>
      {/snippet}
      {#snippet expanded({ row })}
        <div class="flex flex-wrap items-center gap-2 text-xs text-slate-600 dark:text-slate-300">
          <span class="font-medium">{row.name}</span>
          <Badge tone={statusTone(row.status)} dot>{row.status}</Badge>
          <span>{row.salesperson}</span>
          <span>Balance {row.balance}</span>
          <span>Limit {row.creditLimit}</span>
        </div>
      {/snippet}
      {#snippet toolbar()}
        <Button
          size="xs"
          variant="secondary"
          onclick={() => {
            tableApi?.getColumn("status")?.setFilterValue("Open");
            toastStore.push({ tone: "info", title: "Status is Open", description: "Other statuses stay hidden until that filter is cleared." });
          }}
        >
          Open only
        </Button>
      {/snippet}
      {#snippet batch({ rows: selected })}
        <Button
          size="xs"
          onclick={() => {
            const ids = new Set(selected.map((row) => row.id));
            rows = rows.map((row) => (ids.has(row.id) ? { ...row, status: "Released" } : row));
            toastStore.push({ tone: "success", title: "Released", description: `${selected.length} customers moved to Released.` });
          }}
        >
          Release
        </Button>
      {/snippet}
    </DataTable>
  </div>
  <p class="text-xs text-slate-500">{editLog}</p>

  <section class="space-y-3">
    <div>
      <h3 class="text-sm font-semibold text-slate-900 dark:text-slate-100">Empty, loading, and error</h3>
      <p class="mt-1 text-xs text-slate-500 dark:text-slate-400">
        The same table shows a spinner while the first page is loading, an empty state when the filter matches nothing, and a banner when the service fails.
      </p>
    </div>
    <div class="flex flex-wrap gap-2">
      {#each [
        { id: "ready", label: "Ready" },
        { id: "empty", label: "Empty" },
        { id: "loading", label: "Loading" },
        { id: "error", label: "Error" }
      ] as mode}
        <Button size="xs" variant={listState === mode.id ? "primary" : "secondary"} onclick={() => (listState = mode.id)}>
          {mode.label}
        </Button>
      {/each}
    </div>
    <div class="h-72">
      <DataTable
        label="Open orders"
        columns={previewColumns}
        rows={previewRows}
        loading={previewLoading}
        error={previewError}
        searchable={false}
        showColumnControls={false}
        exportable={false}
        pageSize={10}
        emptyTitle="No open orders"
        emptyDescription="Post a sales order or clear the filter to see rows here."
      />
    </div>
  </section>
</div>
