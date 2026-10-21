/**
 * `fx`: runtime helpers for compiled page expressions (see docs/17 §5).
 * Generated pages only ever call these functions; values arrive as record JSON (decimals as strings).
 */
import {
  parseDecimal,
  toDecimalString,
  compareDecimal,
  addDecimal,
  subDecimal,
  mulDecimal,
  divDecimal,
  roundDecimal
} from "./decimal.js";

let workDateProvider = () => todayIso();

/** Lets the shell provide the user's work date (NAV WorkDate). */
export function setWorkDateProvider(fn) {
  workDateProvider = typeof fn === "function" ? fn : () => todayIso();
}

function todayIso() {
  const d = new Date();
  const p = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

function isBlank(v) {
  return v === null || v === undefined || v === "" || (Array.isArray(v) && v.length === 0);
}

function asBoolean(v) {
  if (typeof v === "boolean") return v;
  if (typeof v === "string") {
    const s = v.trim().toLowerCase();
    if (s === "true" || s === "yes") return true;
    if (s === "false" || s === "no" || s === "") return false;
  }
  return null;
}

function decimalResult(d) {
  return d === null ? null : toDecimalString(d);
}

function binary(a, b, op) {
  const x = parseDecimal(a);
  const y = parseDecimal(b);
  if (!x || !y) return null;
  return decimalResult(op(x, y));
}

export const fx = Object.freeze({
  /** Decimal literal from compiled source (kept as an exact string). */
  num(text) {
    return String(text);
  },

  truthy(v) {
    if (isBlank(v)) return false;
    const b = asBoolean(v);
    if (b !== null) return b;
    const d = parseDecimal(v);
    if (d) return d.int !== 0n;
    return Boolean(v);
  },

  eq(a, b) {
    if (isBlank(a) && isBlank(b)) return true;
    if (isBlank(a) || isBlank(b)) return false;
    const x = parseDecimal(a);
    const y = parseDecimal(b);
    if (x && y) return compareDecimal(x, y) === 0;
    const ba = asBoolean(a);
    const bb = asBoolean(b);
    if (ba !== null && bb !== null && (typeof a === "boolean" || typeof b === "boolean")) return ba === bb;
    return String(a) === String(b);
  },

  /** Three-way comparison: numbers exactly, everything else as strings (ISO dates order correctly). */
  cmp(a, b) {
    if (isBlank(a) && isBlank(b)) return 0;
    if (isBlank(a)) return -1;
    if (isBlank(b)) return 1;
    const x = parseDecimal(a);
    const y = parseDecimal(b);
    if (x && y) return compareDecimal(x, y);
    const sa = String(a);
    const sb = String(b);
    return sa === sb ? 0 : sa < sb ? -1 : 1;
  },

  in(a, list) {
    return Array.isArray(list) && list.some((item) => fx.eq(a, item));
  },

  neg(a) {
    const x = parseDecimal(a);
    return x ? toDecimalString({ int: -x.int, scale: x.scale }) : null;
  },
  add: (a, b) => binary(a, b, addDecimal),
  sub: (a, b) => binary(a, b, subDecimal),
  mul: (a, b) => binary(a, b, mulDecimal),
  div: (a, b) => binary(a, b, (x, y) => divDecimal(x, y)),

  isEmpty: (v) => isBlank(v),
  isNotEmpty: (v) => !isBlank(v),
  today: () => todayIso(),
  now: () => new Date().toISOString(),
  workDate: () => workDateProvider(),
  len: (v) => (isBlank(v) ? 0 : String(v).length),
  lower: (v) => (isBlank(v) ? "" : String(v).toLowerCase()),
  upper: (v) => (isBlank(v) ? "" : String(v).toUpperCase()),
  trim: (v) => (isBlank(v) ? "" : String(v).trim()),
  contains: (a, b) => !isBlank(a) && String(a).toLowerCase().includes(String(b ?? "").toLowerCase()),
  startsWith: (a, b) => !isBlank(a) && String(a).toLowerCase().startsWith(String(b ?? "").toLowerCase()),
  endsWith: (a, b) => !isBlank(a) && String(a).toLowerCase().endsWith(String(b ?? "").toLowerCase()),
  round(a, places = 0) {
    const x = parseDecimal(a);
    const p = Number(places);
    if (!x || !Number.isInteger(p) || p < 0) return null;
    return toDecimalString(roundDecimal(x, p));
  },
  abs(a) {
    const x = parseDecimal(a);
    return x ? toDecimalString({ int: x.int < 0n ? -x.int : x.int, scale: x.scale }) : null;
  },
  min: (...values) => values.reduce((m, v) => (m === undefined || fx.cmp(v, m) < 0 ? v : m), undefined),
  max: (...values) => values.reduce((m, v) => (m === undefined || fx.cmp(v, m) > 0 ? v : m), undefined),
  coalesce: (...values) => values.find((v) => !isBlank(v)) ?? null
});
