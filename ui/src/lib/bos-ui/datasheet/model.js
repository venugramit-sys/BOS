/**
 * Pure datasheet rules shared by the grid and by tests.
 * A datasheet is a record grid: one row is one record, one column is one field.
 */

import { evaluateMath } from "../input/evaluator.js";
import { evaluateDateFormula, formatISODate, parseISODate } from "../date-picker/evaluator.js";

export const NEW_ROW_KEY = "__bos_new__";

/**
 * @param {string} dataType
 */
export function editorForType(dataType) {
  const base = String(dataType || "").replace(/\(.*\)/, "").trim().toLowerCase();
  if (base === "boolean" || base === "bool") return "checkbox";
  if (base === "date" || base === "datetime") return "date";
  if (base === "integer" || base === "bigint" || base === "decimal" || base === "number") return "number";
  return "text";
}

/**
 * @param {string} dataType
 */
export function decimalsOf(dataType) {
  const match = /decimal\s*\(\s*\d+\s*,\s*(\d+)\s*\)/i.exec(String(dataType || ""));
  if (match) return Number(match[1]);
  const base = String(dataType || "").replace(/\(.*\)/, "").trim().toLowerCase();
  if (base === "integer" || base === "bigint") return 0;
  return undefined;
}

/**
 * Build sheet columns from a BOS table field list.
 * Primary keys, FlowFields, and computed fields stay locked.
 * @param {Array<Record<string, any>>} fields
 */
export function columnsFromFields(fields) {
  return (fields || [])
    .filter((field) => field && field.visible !== false)
    .map((field) => {
      const editor = editorForType(field.dataType);
      const locked = Boolean(field.isPrimaryKey || field.flowField || field.computed || field.editable === false);
      return {
        key: field.name,
        label: field.caption || field.name,
        editor,
        editable: !locked,
        quickEntry: !locked,
        total: editor === "number" ? "sum" : undefined,
        decimals: editor === "number" ? decimalsOf(field.dataType) : undefined,
        width: editor === "checkbox" ? 96 : 160,
        hint: field.flowField || field.computed ? "Calculated" : ""
      };
    });
}

/**
 * @param {Record<string, any>} column
 * @param {unknown} raw
 */
export function displayCell(column, raw) {
  if (raw == null || raw === "") return "";
  if (column?.editor === "checkbox") return raw ? "Yes" : "No";
  if (column?.editor === "select" || column?.editor === "lookup") {
    const match = (column.options || []).find((option) => String(option.value) === String(raw));
    if (!match?.label) return String(raw);
    if (column.editor === "lookup" && match.label !== String(raw)) return `${raw} · ${match.label}`;
    return match.label;
  }
  if (column?.editor === "number") return formatNumber(raw, column.decimals);
  return String(raw);
}

/**
 * @param {unknown} value
 * @param {number | undefined} decimals
 */
export function formatNumber(value, decimals) {
  const number = typeof value === "number" ? value : Number(String(value).replace(/,/g, ""));
  if (!Number.isFinite(number)) return "";
  if (decimals == null) {
    return Number(number.toFixed(6)).toString();
  }
  return number.toLocaleString("en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals
  });
}

/**
 * Pocket-calculator math stays here. Cross-field business rules stay in tablelogic.
 * @param {Record<string, any>} column
 * @param {unknown} raw
 * @returns {{ value: unknown, error: string }}
 */
export function coerceCell(column, raw, workDate = "") {
  if (column?.editor === "checkbox") {
    if (typeof raw === "boolean") return { value: raw, error: "" };
    const text = String(raw ?? "").trim().toLowerCase();
    if (["1", "true", "yes", "y"].includes(text)) return { value: true, error: "" };
    if (["0", "false", "no", "n", ""].includes(text)) return { value: false, error: "" };
    return { value: raw, error: "Enter yes or no" };
  }
  if (column?.editor === "date") {
    const text = raw == null ? "" : String(raw).trim();
    if (!text) return { value: "", error: "" };
    const result = evaluateDateFormula(text, parseISODate(workDate) || new Date());
    const iso = result ? formatISODate(result.date) : "";
    if (!iso) return { value: text, error: "Enter a date" };
    return { value: result.isClosing ? `C${iso}` : iso, error: "" };
  }
  if (column?.editor === "number") {
    if (raw === "" || raw == null) return { value: "", error: "" };
    const evaluated = evaluateMath(String(raw).replace(/,/g, ""));
    if (evaluated == null || Number.isNaN(evaluated)) return { value: raw, error: "Enter a number" };
    return { value: evaluated, error: "" };
  }
  if (column?.editor === "select" || column?.editor === "lookup") {
    const text = raw == null ? "" : String(raw).trim();
    if (!text) return { value: "", error: "" };
    const options = column.options || [];
    const match = options.find((option) => String(option.value) === text || String(option.label).toLowerCase() === text.toLowerCase());
    if (options.length && !match) return { value: text, error: "Choose a value from the list" };
    return { value: match ? match.value : text, error: "" };
  }
  return { value: raw == null ? "" : String(raw), error: "" };
}

/**
 * @param {Record<string, any>} column
 * @param {unknown} value
 * @param {Record<string, any> | null} row
 */
export function columnLocked(column, row) {
  if (!column) return true;
  const flag = column.editable;
  if (typeof flag === "function") return flag(row) === false;
  return flag === false;
}

/**
 * @param {Record<string, any>} column
 * @param {unknown} value
 * @param {Record<string, any> | null} row
 */
export function checkCell(column, value, row) {
  if (typeof column?.validate === "function") {
    const message = column.validate(value, row);
    if (message) return String(message);
  }
  return "";
}

/**
 * @param {Array<Record<string, any>>} columns
 * @param {number} index
 * @param {number} step
 * @param {"any" | "entry"} mode
 */
/**
 * Letters typed quickly belong to one search. A pause starts a new search.
 * @param {string} previous
 * @param {string} char
 * @param {number} elapsedMs
 * @param {number} [windowMs]
 */
export function nextTypeQuery(previous, char, elapsedMs, windowMs = 800) {
  if (!previous || elapsedMs > windowMs) return char;
  return `${previous}${char}`;
}

/**
 * First option whose label or value starts with the typed text.
 * @param {Array<{ value?: unknown, label?: unknown }> | undefined} options
 * @param {string} query
 */
export function matchOption(options, query) {
  const needle = String(query || "").trim().toLowerCase();
  if (!needle) return "";
  const list = options || [];
  const hit = list.find((option) => String(option.label ?? option.value ?? "").toLowerCase().startsWith(needle))
    || list.find((option) => String(option.value ?? "").toLowerCase().startsWith(needle));
  return hit ? String(hit.value) : "";
}

export function stepColumn(columns, index, step, mode, open) {
  let cursor = index;
  for (let guard = 0; guard < columns.length; guard += 1) {
    cursor += step;
    if (cursor < 0 || cursor >= columns.length) return null;
    const column = columns[cursor];
    if (mode === "any") return cursor;
    const allowed = typeof open === "function" ? open(column) : column.editable !== false && column.quickEntry !== false;
    if (allowed) return cursor;
  }
  return null;
}

/**
 * Enter walks Quick Entry fields. Tab walks every column.
 * @param {Array<Record<string, any>>} columns
 * @param {number} row
 * @param {number} col
 * @param {number} rowCount
 * @param {number} step
 * @param {"any" | "entry"} mode
 */
export function moveCell(columns, row, col, rowCount, step, mode, open) {
  const nextCol = stepColumn(columns, col, step, mode, open);
  if (nextCol != null) return { row, col: nextCol };
  const eligible = columns
    .map((column, index) => ({ column, index }))
    .filter(({ column }) => mode === "any" || (typeof open === "function" ? open(column) : column.editable !== false && column.quickEntry !== false));
  if (!eligible.length || rowCount <= 0) return { row, col };
  const nextRow = row + (step > 0 ? 1 : -1);
  if (nextRow < 0 || nextRow >= rowCount) return { row, col };
  const wrapped = step > 0 ? eligible[0].index : eligible[eligible.length - 1].index;
  return { row: nextRow, col: wrapped };
}

/**
 * @param {{ row: number, col: number }} a
 * @param {{ row: number, col: number }} b
 */
export function normalizeRange(a, b) {
  return {
    r1: Math.min(a.row, b.row),
    r2: Math.max(a.row, b.row),
    c1: Math.min(a.col, b.col),
    c2: Math.max(a.col, b.col)
  };
}

/**
 * @param {{ r1: number, r2: number, c1: number, c2: number }} range
 * @param {number} row
 * @param {number} col
 */
export function inRange(range, row, col) {
  return row >= range.r1 && row <= range.r2 && col >= range.c1 && col <= range.c2;
}

/**
 * @param {unknown[][]} matrix
 */
export function selectionStats(matrix) {
  const numbers = [];
  let count = 0;
  for (const line of matrix) {
    for (const value of line) {
      count += 1;
      if (value === "" || value == null || typeof value === "boolean") continue;
      const number = typeof value === "number" ? value : Number(String(value).replace(/,/g, ""));
      if (Number.isFinite(number) && String(value).trim() !== "") numbers.push(number);
    }
  }
  const sum = numbers.reduce((total, value) => total + value, 0);
  return {
    count,
    numeric: numbers.length,
    sum,
    average: numbers.length ? sum / numbers.length : null
  };
}

/**
 * @param {unknown[][]} matrix
 */
export function toTsv(matrix) {
  return matrix
    .map((line) => line.map((value) => (value == null ? "" : String(value))).join("\t"))
    .join("\n");
}

/**
 * @param {string} text
 */
export function parseTsv(text) {
  const source = String(text || "").replace(/\r\n/g, "\n").replace(/\n$/, "");
  if (!source) return [];
  return source.split("\n").map((line) => line.split("\t"));
}

/**
 * @param {number} rowCount
 * @param {Array<Record<string, any>>} columns
 * @param {{ row: number, col: number }} anchor
 * @param {unknown[][]} matrix
 */
export function pastePlan(rowCount, columns, anchor, matrix) {
  const plans = [];
  for (let r = 0; r < matrix.length; r += 1) {
    const rowIndex = anchor.row + r;
    if (rowIndex < 0 || rowIndex >= rowCount) break;
    for (let c = 0; c < matrix[r].length; c += 1) {
      const colIndex = anchor.col + c;
      if (colIndex < 0 || colIndex >= columns.length) break;
      const column = columns[colIndex];
      if (column.editable === false) continue;
      plans.push({ rowIndex, colIndex, key: column.key, raw: matrix[r][c] });
    }
  }
  return plans;
}

/**
 * One number copies. Two or more numbers continue the last step. Text copies.
 * @param {unknown[]} columnValues
 * @param {number} extra
 */
export function extendFill(columnValues, extra) {
  if (extra <= 0 || !columnValues.length) return [];
  const numbers = columnValues.map((value) => (typeof value === "number" ? value : Number(value)));
  const numeric = columnValues.length >= 2 && numbers.every((value) => Number.isFinite(value));
  if (numeric) {
    const delta = numbers[numbers.length - 1] - numbers[numbers.length - 2];
    let last = numbers[numbers.length - 1];
    return Array.from({ length: extra }, () => {
      last += delta;
      return last;
    });
  }
  const last = columnValues[columnValues.length - 1];
  return Array.from({ length: extra }, () => last);
}

/**
 * @param {Record<string, any>} row
 * @param {Array<Record<string, any>>} columns
 * @param {Record<string, string>} filters
 * @param {string} query
 */
export function rowMatches(row, columns, filters, query) {
  const needle = String(query || "").trim().toLowerCase();
  if (needle) {
    const hit = columns.some((column) => displayCell(column, row?.[column.key]).toLowerCase().includes(needle));
    if (!hit) return false;
  }
  for (const column of columns) {
    const filter = String(filters?.[column.key] || "").trim();
    if (!filter) continue;
    if (!cellMatchesFilter(displayCell(column, row?.[column.key]), row?.[column.key], filter)) return false;
  }
  return true;
}

/**
 * Business Central filter words: =exact, <>not, 10..20, >10, A*, and A|B.
 * A plain word still means "contains".
 * @param {unknown} shown
 * @param {unknown} raw
 * @param {string} expression
 */
export function cellMatchesFilter(shown, raw, expression) {
  const expr = String(expression || "").trim();
  if (!expr) return true;
  if (expr.includes("|")) return expr.split("|").some((part) => cellMatchesFilter(shown, raw, part.trim()));
  const text = shown == null ? "" : String(shown);
  const lower = text.toLowerCase();
  if (expr.startsWith("<>") || expr.startsWith("!=")) return lower !== expr.slice(2).trim().toLowerCase();
  if (expr.startsWith("=")) return lower === expr.slice(1).trim().toLowerCase();
  if (expr.includes("..")) {
    const [start, end] = expr.split("..");
    return inFilterRange(raw, text, start.trim(), end.trim());
  }
  if (expr.includes("*") || expr.includes("?")) {
    const pattern = `^${expr.toLowerCase().replace(/[.+^${}()|[\]\\*?]/g, "\\$&").replace(/\\\*/g, ".*").replace(/\\\?/g, ".")}$`;
    return new RegExp(pattern, "i").test(text);
  }
  const operator = /^(>=|<=|>|<)/.exec(expr);
  if (operator) return compareFilterOp(raw, text, operator[1], expr.slice(operator[1].length).trim());
  return lower.includes(expr.toLowerCase());
}

/**
 * @param {unknown} raw
 * @param {string} shown
 */
function filterComparable(raw, shown) {
  if (typeof raw === "number" && Number.isFinite(raw)) return raw;
  const trimmed = String(shown ?? "").trim().replace(/,/g, "");
  if (trimmed !== "" && /^-?\d+(\.\d+)?$/.test(trimmed)) return Number(trimmed);
  return String(shown ?? "").toLowerCase();
}

/**
 * @param {unknown} raw
 * @param {string} shown
 * @param {string} start
 * @param {string} end
 */
function inFilterRange(raw, shown, start, end) {
  const value = filterComparable(raw, shown);
  if (start !== "" && value < filterComparable(start, start)) return false;
  if (end !== "" && value > filterComparable(end, end)) return false;
  return true;
}

/**
 * @param {unknown} raw
 * @param {string} shown
 * @param {string} operator
 * @param {string} bound
 */
function compareFilterOp(raw, shown, operator, bound) {
  const value = filterComparable(raw, shown);
  const other = filterComparable(bound, bound);
  if (operator === ">") return value > other;
  if (operator === ">=") return value >= other;
  if (operator === "<") return value < other;
  return value <= other;
}

/**
 * A required field is a reminder. It does not trap the clerk in the cell.
 * @param {Record<string, any>} column
 * @param {unknown} value
 * @param {Record<string, any> | null} row
 */
export function fieldProblem(column, value, row) {
  if (!column) return "";
  if (column.required && (value === "" || value == null)) return `${column.label || "This field"} is required`;
  if (typeof column.attention === "function") {
    const message = column.attention(value, row);
    if (message) return String(message);
  }
  if (typeof column.validate === "function") {
    const message = column.validate(value, row);
    if (message) return String(message);
  }
  return "";
}

/**
 * @param {Record<string, any>} row
 * @param {Array<Record<string, any>>} columns
 */
export function fieldProblems(row, columns) {
  const problems = [];
  for (const column of columns || []) {
    const message = fieldProblem(column, row?.[column.key], row);
    if (message) problems.push({ key: column.key, label: column.label || column.key, message });
  }
  return problems;
}

/**
 * Hide, reorder, and Quick Entry choices for one clerk. A column marked keepVisible stays on screen.
 * @param {Array<Record<string, any>>} columns
 * @param {{ order?: string[], hidden?: string[], quickEntry?: Record<string, boolean> } | null} layout
 */
export function applyColumnLayout(columns, layout) {
  const source = columns || [];
  const hidden = new Set(layout?.hidden || []);
  const order = Array.isArray(layout?.order) ? layout.order : [];
  const quick = layout?.quickEntry || {};
  const byKey = new Map(source.map((column) => [column.key, column]));
  const ordered = [];
  for (const key of order) {
    const column = byKey.get(key);
    if (!column) continue;
    ordered.push(column);
    byKey.delete(key);
  }
  for (const column of byKey.values()) ordered.push(column);
  return ordered
    .filter((column) => column.keepVisible || !hidden.has(column.key))
    .map((column) => (quick[column.key] == null ? column : { ...column, quickEntry: Boolean(quick[column.key]) }));
}

/**
 * @param {Array<Record<string, any>>} columns
 * @param {{ hidden?: string[] } | null} layout
 */
export function hiddenSharedNote(columns, layout) {
  const hidden = new Set(layout?.hidden || []);
  const shared = (columns || []).filter((column) => column.sharedAcrossRows && !column.keepVisible && hidden.has(column.key));
  if (!shared.length) return "";
  const names = shared.map((column) => column.label || column.key).join(", ");
  return `${names} is hidden. Every line uses the same value.`;
}

/**
 * @param {number} viewportHeight
 * @param {number} rowHeight
 */
export function pageStep(viewportHeight, rowHeight) {
  const row = Number(rowHeight) || 32;
  return Math.max(1, Math.floor((Number(viewportHeight) || 0) / row) - 1);
}

/**
 * Scroll offset that puts a column fully in view, to the right of frozen columns.
 * `columnLeft` and `columnWidth` are in the scroll content, including the row gutter.
 * `frozenWidth` is the sticky gutter plus pinned columns, measured in the viewport.
 * @param {number} scrollLeft
 * @param {number} clientWidth
 * @param {number} columnLeft
 * @param {number} columnWidth
 * @param {number} frozenWidth
 * @param {number} [pad]
 */
export function revealColumnScroll(scrollLeft, clientWidth, columnLeft, columnWidth, frozenWidth, pad = 8) {
  const view = Math.max(0, Number(clientWidth) || 0);
  const frozen = Math.max(0, Math.min(view, Number(frozenWidth) || 0));
  const left = Number(columnLeft) || 0;
  const width = Math.max(0, Number(columnWidth) || 0);
  const gap = Math.max(0, Number(pad) || 0);
  const right = left + width;
  const room = view - frozen;
  let next = Number(scrollLeft) || 0;
  if (width + gap <= room) {
    if (right + gap > next + view) next = right + gap - view;
    if (left < next + frozen) next = left - frozen;
  } else if (left < next + frozen || right > next + view) {
    next = left - frozen;
  }
  return Math.max(0, next);
}

/**
 * @param {Record<string, any>} left
 * @param {Record<string, any>} right
 * @param {Record<string, any>} column
 * @param {"asc" | "desc"} direction
 */
export function compareCells(left, right, column, direction) {
  const sign = direction === "desc" ? -1 : 1;
  const a = left?.[column.key];
  const b = right?.[column.key];
  if (column.editor === "number") {
    const an = a === "" || a == null ? Number.NEGATIVE_INFINITY : Number(a);
    const bn = b === "" || b == null ? Number.NEGATIVE_INFINITY : Number(b);
    return (an - bn) * sign;
  }
  return String(a ?? "").localeCompare(String(b ?? ""), undefined, { numeric: true, sensitivity: "base" }) * sign;
}

/**
 * @param {Record<string, unknown>} draft
 */
export function draftPayload(draft) {
  /** @type {Record<string, unknown>} */
  const payload = {};
  for (const [key, value] of Object.entries(draft || {})) {
    if (value === "" || value == null) continue;
    payload[key] = value;
  }
  return payload;
}

/**
 * @param {Array<Record<string, any>>} rows
 * @param {Array<Record<string, any>>} columns
 */
export function columnTotal(rows, column) {
  if (column?.total !== "sum" || column.editor !== "number") return null;
  return rows.reduce((total, row) => {
    const value = Number(row?.[column.key]);
    return Number.isFinite(value) ? total + value : total;
  }, 0);
}
