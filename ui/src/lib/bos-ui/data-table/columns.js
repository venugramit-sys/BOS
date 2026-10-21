const UTILITY = new Set(["selection", "expansion"]);

export function matchFilter(raw, filterValue) {
  if (filterValue == null) return true;
  const text = String(filterValue).trim();
  if (!text) return true;

  const range = text.match(/^(-?\d+(?:\.\d+)?)\.\.(-?\d+(?:\.\d+)?)$/);
  if (range) {
    const n = Number(raw);
    if (Number.isNaN(n)) return false;
    return n >= Number(range[1]) && n <= Number(range[2]);
  }

  const cmp = text.match(/^(>=|<=|>|<|=)\s*(-?\d+(?:\.\d+)?)$/);
  if (cmp) {
    const n = Number(raw);
    const target = Number(cmp[2]);
    if (Number.isNaN(n)) return false;
    if (cmp[1] === ">") return n > target;
    if (cmp[1] === "<") return n < target;
    if (cmp[1] === ">=") return n >= target;
    if (cmp[1] === "<=") return n <= target;
    return n === target;
  }

  return String(raw ?? "").toLowerCase().includes(text.toLowerCase());
}

function bosFilter(row, columnId, filterValue) {
  return matchFilter(row.getValue(columnId), filterValue);
}
bosFilter.autoRemove = (value) => value == null || String(value).trim() === "";

function equalsFilter(row, columnId, filterValue) {
  if (filterValue == null || filterValue === "" || filterValue === "*") return true;
  return String(row.getValue(columnId) ?? "") === String(filterValue);
}
equalsFilter.autoRemove = (value) => value == null || value === "" || value === "*";

export function formatValue(value, meta) {
  if (value == null || value === "") return "";
  if (typeof value === "boolean") return value ? "Yes" : "No";
  const numeric = meta?.kind === "number" || typeof value === "number";
  if (numeric) {
    const n = typeof value === "number" ? value : Number(value);
    if (!Number.isNaN(n)) {
      return new Intl.NumberFormat(undefined, {
        minimumFractionDigits: meta?.decimals ?? (Number.isInteger(n) ? 0 : 2),
        maximumFractionDigits: meta?.decimals ?? 2
      }).format(n);
    }
  }
  return String(value);
}

export function badgeTone(meta, value, row) {
  if (typeof meta?.badgeTone === "function") return meta.badgeTone(value, row) || "neutral";
  if (typeof meta?.badgeTone === "string") return meta.badgeTone;
  return "neutral";
}

function columnIdentity(spec, index) {
  return spec.id || spec.accessorKey || "";
}

function mapSpec(spec, index) {
  if (Array.isArray(spec.columns) && spec.columns.length) {
    const id = spec.id || String(spec.header || `group_${index}`);
    return {
      id,
      header: spec.header || id,
      columns: spec.columns.map((child, childIndex) => mapSpec(child, childIndex))
    };
  }

  const id = columnIdentity(spec, index);
  if (!id) {
    throw new Error("DataTable column requires an id or accessorKey");
  }

  const kind = spec.kind || "text";
  const interactive = kind === "checkbox" || kind === "switch";
  const meta = { ...spec, id, kind };

  const def = {
    id,
    header: spec.header ?? id,
    minSize: spec.minSize ?? (interactive ? 72 : 96),
    maxSize: spec.maxSize ?? 720,
    enableSorting: spec.enableSorting ?? !interactive,
    enableColumnFilter: spec.enableColumnFilter ?? !interactive,
    enableGlobalFilter: spec.enableGlobalFilter ?? !interactive,
    enableHiding: spec.enableHiding ?? true,
    enableResizing: spec.enableResizing ?? !interactive,
    enablePinning: spec.enablePinning ?? true,
    enableGrouping: spec.groupable === true,
    filterFn: spec.filter === "select" ? equalsFilter : bosFilter,
    meta
  };

  if (spec.size) def.size = spec.size;
  if (spec.aggregate) def.aggregationFn = spec.aggregate;
  if (typeof spec.accessorFn === "function") def.accessorFn = spec.accessorFn;
  else if (spec.accessorKey) def.accessorKey = spec.accessorKey;
  else def.accessorFn = () => undefined;

  return def;
}

function utilityColumn(id, kind, size) {
  return {
    id,
    header: "",
    size,
    minSize: size,
    maxSize: size + 8,
    enableSorting: false,
    enableColumnFilter: false,
    enableGlobalFilter: false,
    enableHiding: false,
    enableResizing: false,
    enablePinning: false,
    enableGrouping: false,
    meta: { id, kind, header: "" }
  };
}

export function buildColumnDefs(specs, options) {
  const defs = [];
  if (options.selectable) defs.push(utilityColumn("_select", "selection", 40));
  if (options.expandable) defs.push(utilityColumn("_expand", "expansion", 36));
  for (let index = 0; index < specs.length; index += 1) {
    defs.push(mapSpec(specs[index], index));
  }
  return defs;
}

export function buildInitialState(specs, options, saved) {
  const pinning = { start: [], end: [] };
  const visibility = {};
  const order = [];

  if (options.selectable) {
    pinning.start.push("_select");
    order.push("_select");
  }
  if (options.expandable) {
    pinning.start.push("_expand");
    order.push("_expand");
  }

  const walk = (list) => {
    for (const spec of list) {
      if (Array.isArray(spec.columns) && spec.columns.length) {
        walk(spec.columns);
        continue;
      }
      const id = spec.id || spec.accessorKey;
      if (!id) continue;
      order.push(id);
      if (spec.pin === "start") pinning.start.push(id);
      if (spec.pin === "end") pinning.end.push(id);
      if (spec.hidden) visibility[id] = false;
    }
  };
  walk(specs);

  const pageSize = saved?.pageSize || options.pageSize || 20;
  return {
    pagination: { pageIndex: 0, pageSize },
    columnPinning: saved?.columnPinning?.start ? saved.columnPinning : pinning,
    columnVisibility: { ...visibility, ...(saved?.columnVisibility || {}) },
    columnOrder: saved?.columnOrder?.length ? saved.columnOrder : order,
    columnSizing: saved?.columnSizing || {}
  };
}

function csvCell(value) {
  const text = value == null ? "" : String(value);
  if (/[",\n]/.test(text)) return `"${text.replaceAll('"', '""')}"`;
  return text;
}

function leafRows(rows, into) {
  for (const row of rows) {
    if (row.getIsGrouped?.() && row.subRows?.length) leafRows(row.subRows, into);
    else into.push(row);
  }
}

export function tableToCsv(table) {
  const columns = table.getVisibleLeafColumns().filter((column) => !UTILITY.has(column.columnDef.meta?.kind));
  const rows = [];
  leafRows(table.getFilteredRowModel().rows, rows);
  const header = columns.map((column) => csvCell(column.columnDef.header || column.id)).join(",");
  const body = rows.map((row) => columns.map((column) => csvCell(formatValue(row.getValue(column.id), column.columnDef.meta))).join(","));
  return [header, ...body].join("\n");
}

export function downloadCsv(filename, csv) {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

export function isUtilityKind(kind) {
  return UTILITY.has(kind);
}
