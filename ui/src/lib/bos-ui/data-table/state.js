const PREFIX = "bos.table.";

export function loadTableState(stateKey) {
  if (!stateKey || typeof localStorage === "undefined") return null;
  try {
    const raw = localStorage.getItem(PREFIX + stateKey);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return null;
    return parsed;
  } catch {
    return null;
  }
}

export function saveTableState(stateKey, state) {
  if (!stateKey || typeof localStorage === "undefined" || !state) return;
  const payload = {
    columnVisibility: state.columnVisibility || {},
    columnSizing: state.columnSizing || {},
    columnOrder: state.columnOrder || [],
    columnPinning: state.columnPinning || { start: [], end: [] },
    pageSize: state.pagination?.pageSize
  };
  localStorage.setItem(PREFIX + stateKey, JSON.stringify(payload));
}
