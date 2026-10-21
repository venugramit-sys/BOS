/**
 * Exact decimal arithmetic on strings (BigInt scaled integers).
 * BOS exchanges amounts as decimal strings ("1250.50"); page expressions must never round through floats.
 */

const DECIMAL_RE = /^([+-])?(\d+)(?:\.(\d+))?$/;

/** Parses a decimal string/number into `{ int: BigInt, scale }` (value = int / 10^scale), or null. */
export function parseDecimal(value) {
  if (value === null || value === undefined || value === "") return null;
  if (typeof value === "bigint") return { int: value, scale: 0 };
  let text;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) return null;
    text = numberToPlain(value);
  } else if (typeof value === "string") {
    text = value.trim();
  } else {
    return null;
  }
  const m = DECIMAL_RE.exec(text);
  if (!m) return null;
  const frac = m[3] ?? "";
  const int = BigInt(m[2] + frac) * (m[1] === "-" ? -1n : 1n);
  return { int, scale: frac.length };
}

/** Number to a plain (non-exponential) decimal string. */
function numberToPlain(n) {
  const s = String(n);
  if (!/e/i.test(s)) return s;
  return n.toLocaleString("en-US", { useGrouping: false, maximumFractionDigits: 20 });
}

export function isDecimal(value) {
  return parseDecimal(value) !== null;
}

function align(a, b) {
  const scale = Math.max(a.scale, b.scale);
  return [a.int * 10n ** BigInt(scale - a.scale), b.int * 10n ** BigInt(scale - b.scale), scale];
}

/** Canonical string: no trailing fractional zeros, no "-0". */
export function toDecimalString({ int, scale }) {
  const neg = int < 0n;
  let digits = (neg ? -int : int).toString().padStart(scale + 1, "0");
  let whole = scale ? digits.slice(0, digits.length - scale) : digits;
  let frac = scale ? digits.slice(digits.length - scale).replace(/0+$/, "") : "";
  const body = frac ? `${whole}.${frac}` : whole;
  return neg && body !== "0" ? `-${body}` : body;
}

export function compareDecimal(a, b) {
  const [x, y] = align(a, b);
  return x === y ? 0 : x < y ? -1 : 1;
}

export function addDecimal(a, b) {
  const [x, y, scale] = align(a, b);
  return { int: x + y, scale };
}

export function subDecimal(a, b) {
  const [x, y, scale] = align(a, b);
  return { int: x - y, scale };
}

export function mulDecimal(a, b) {
  return { int: a.int * b.int, scale: a.scale + b.scale };
}

/** Division rounded half away from zero to `scale` fractional digits. Returns null on division by zero. */
export function divDecimal(a, b, scale = 10) {
  if (b.int === 0n) return null;
  // a/b = (a.int / 10^a.scale) / (b.int / 10^b.scale)
  const num = a.int * 10n ** BigInt(b.scale + scale + 1);
  const den = b.int * 10n ** BigInt(a.scale);
  let q = num / den;
  q = q >= 0n ? (q + 5n) / 10n : (q - 5n) / 10n;
  return { int: q, scale };
}

/** Rounds half away from zero to `places` fractional digits. */
export function roundDecimal(a, places = 0) {
  if (a.scale <= places) return { int: a.int * 10n ** BigInt(places - a.scale), scale: places };
  const factor = 10n ** BigInt(a.scale - places);
  const half = factor / 2n;
  const q = a.int >= 0n ? (a.int + half) / factor : (a.int - half) / factor;
  return { int: q, scale: places };
}

/**
 * Fixed-point display string with `min`..`max` fractional digits (NAV DecimalPlaces), rounding half away from zero.
 * Thousands are grouped with `group` (default ",").
 */
export function formatDecimal(value, min = 0, max = min, group = ",") {
  const d = parseDecimal(value);
  if (!d) return value === null || value === undefined ? "" : String(value);
  const r = roundDecimal(d, max);
  const neg = r.int < 0n;
  const digits = (neg ? -r.int : r.int).toString().padStart(max + 1, "0");
  const whole = max ? digits.slice(0, digits.length - max) : digits;
  let frac = max ? digits.slice(digits.length - max) : "";
  while (frac.length > min && frac.endsWith("0")) frac = frac.slice(0, -1);
  const grouped = group ? whole.replace(/\B(?=(\d{3})+(?!\d))/g, group) : whole;
  const body = frac ? `${grouped}.${frac}` : grouped;
  const isZero = r.int === 0n;
  return neg && !isZero ? `-${body}` : body;
}
