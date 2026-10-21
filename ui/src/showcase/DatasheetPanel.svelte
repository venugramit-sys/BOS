<script>
  /**
   * Item journal datasheet. Amount is locked and follows quantity × price.
   * The work date is a company setting, not today's date.
   */
  import { Datasheet } from "$lib/bos-ui/index.js";

  let rows = $state([
    { id: "10000", itemId: "DRILL-10", location: "MAIN", quantity: 50, unitPrice: 12, amount: 600, postingDate: "2026-10-15" },
    { id: "20000", itemId: "SAW-02", location: "MAIN", quantity: 12, unitPrice: 18, amount: 216, postingDate: "2026-10-15" },
    { id: "30000", itemId: "BIT-04", location: "EAST", quantity: 100, unitPrice: 1.5, amount: 150, postingDate: "2026-10-16" }
  ]);
  let workDate = $state("2026-09-30");
  let active = $state(null);
  let opened = $state("");
  let lastEdit = $state("Click a cell and type. Enter moves to the next field you fill in.");

  const items = [
    { value: "DRILL-10", label: "Cordless drill" },
    { value: "SAW-02", label: "Circular saw" },
    { value: "BIT-04", label: "Driver bit" }
  ];
  const locations = [
    { value: "MAIN", label: "MAIN" },
    { value: "EAST", label: "EAST" },
    { value: "WEST", label: "WEST" }
  ];

  const columns = [
    { key: "itemId", label: "Item", editor: "lookup", width: 220, options: items, quickEntry: true, required: true },
    { key: "location", label: "Location", editor: "select", width: 120, options: locations, quickEntry: true },
    {
      key: "quantity",
      label: "Quantity",
      editor: "number",
      decimals: 0,
      total: "sum",
      width: 110,
      required: true,
      attention: (_value, row) => (Number(row?.quantity) > 80 ? "Quantity is above the usual order" : ""),
      validate: (value) => (value !== "" && value != null && Number(value) < 0 ? "Quantity cannot be negative" : "")
    },
    { key: "unitPrice", label: "Unit price", editor: "number", decimals: 2, width: 120 },
    { key: "amount", label: "Amount", editor: "number", decimals: 2, total: "sum", editable: false, hint: "Calculated", drillable: true, width: 120 },
    { key: "postingDate", label: "Posting date", editor: "date", width: 150, keepVisible: true, required: true }
  ];

  function syncAmount(row) {
    const quantity = Number(row.quantity) || 0;
    const price = Number(row.unitPrice) || 0;
    row.amount = Math.round(quantity * price * 100) / 100;
  }

  function onCommitCell(row, key, value) {
    row[key] = value;
    if (key === "quantity" || key === "unitPrice") syncAmount(row);
    lastEdit = `${row.id} · ${key} = ${value}`;
    if (active?.id === row.id) active = row;
  }
</script>

<div class="space-y-3">
  <div class="flex flex-wrap items-end justify-between gap-3">
    <div>
      <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Item journal datasheet</h2>
      <p class="mt-1 max-w-3xl text-sm text-slate-500 dark:text-slate-400">
        One row is one line. Enter skips Amount. Type <span class="font-mono">w</span> on Posting date to use the work date, or <span class="font-mono">t</span> for today. Alt+F3 keeps only the current value. Alt+Up reads a problem.
      </p>
    </div>
    <label class="text-xs text-slate-500">
      Work date
      <input type="date" class="ml-1 h-8 rounded border border-slate-200 px-2 text-sm text-slate-800 dark:border-slate-700 dark:bg-slate-950 dark:text-slate-100" bind:value={workDate} />
    </label>
  </div>
  <div class="grid h-[32rem] grid-cols-1 gap-3 lg:grid-cols-[minmax(0,1fr)_16rem]">
    <Datasheet
      label="Item journal"
      layoutKey="item-journal"
      bind:rows
      {columns}
      {workDate}
      freezeCount={1}
      {onCommitCell}
      onActiveRow={(row) => (active = row)}
      onOpenCard={(row) => (opened = row ? `${row.itemId} · ${row.location}` : "")}
      onDrillDown={(row) => (opened = `${row.itemId}: ${row.quantity} × ${row.unitPrice} = ${row.amount}`)}
      onInsertRow={(payload) => {
        const row = { id: String(40000 + rows.length * 10000), amount: 0, postingDate: workDate, ...payload };
        syncAmount(row);
        rows = [...rows, row];
        lastEdit = `Inserted ${row.id}`;
      }}
      onDeleteRow={(row) => {
        rows = rows.filter((item) => item.id !== row.id);
        lastEdit = `Deleted ${row.id}`;
      }}
    />
    <aside class="rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm dark:border-slate-800 dark:bg-slate-900">
      <p class="text-[10px] font-semibold uppercase tracking-wide text-slate-400">Line details</p>
      {#if active}
        <p class="mt-2 font-medium text-slate-800 dark:text-slate-100">{active.itemId || "No item"}</p>
        <dl class="mt-2 space-y-1 text-xs text-slate-600 dark:text-slate-300">
          <div class="flex justify-between gap-2"><dt>Location</dt><dd>{active.location || "—"}</dd></div>
          <div class="flex justify-between gap-2"><dt>Quantity</dt><dd>{active.quantity}</dd></div>
          <div class="flex justify-between gap-2"><dt>Amount</dt><dd>{active.amount}</dd></div>
          <div class="flex justify-between gap-2"><dt>Posting date</dt><dd>{active.postingDate || "—"}</dd></div>
        </dl>
      {:else}
        <p class="mt-2 text-xs text-slate-500">Move to a line. This panel follows that line.</p>
      {/if}
      {#if opened}
        <p class="mt-3 rounded border border-indigo-200 bg-white px-2 py-1 text-xs text-indigo-700 dark:border-indigo-900 dark:bg-slate-950 dark:text-indigo-200">{opened}</p>
      {/if}
    </aside>
  </div>
  <p class="text-xs text-slate-500" data-testid="datasheet-log">{lastEdit}</p>
</div>
