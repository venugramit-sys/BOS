/**
 * Read-only display formatting of record values according to the resolved field metadata.
 */
import { formatDecimal, parseDecimal } from "./decimal.js";

function pad(n) {
  return String(n).padStart(2, "0");
}

/** `YYYY-MM-DD` (or ISO date-time) → locale date. Invalid input is returned unchanged. */
export function formatDate(value, locale) {
  if (!value) return "";
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(String(value));
  if (!m) return String(value);
  const d = new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3]));
  return d.toLocaleDateString(locale, { year: "numeric", month: "short", day: "2-digit" });
}

export function formatDateTime(value, locale) {
  if (!value) return "";
  const d = new Date(String(value));
  if (Number.isNaN(d.getTime())) return String(value);
  return d.toLocaleString(locale, { year: "numeric", month: "short", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

/** ISO date-time → value for `<input type="datetime-local">` (local time, minutes precision). */
export function toLocalDateTimeInput(value) {
  if (!value) return "";
  const d = new Date(String(value));
  if (Number.isNaN(d.getTime())) return "";
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** `<input type="datetime-local">` value → ISO date-time (UTC). */
export function fromLocalDateTimeInput(value) {
  if (!value) return null;
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

export function optionCaption(field, value) {
  const opt = field?.options?.find((o) => o.value === value);
  return opt ? opt.caption : value ?? "";
}

/** Display text of a value for grids, FactBoxes and read-only fields. */
export function formatValue(field, value) {
  if (value === null || value === undefined || value === "") return "";
  if (!field) return String(value);
  switch (field.control) {
    case "number": {
      if (!parseDecimal(value)) return String(value);
      if (field.blank_zero && parseDecimal(value).int === 0n) return "";
      const [min, max] = field.decimals ?? [0, 5];
      return formatDecimal(value, min, max);
    }
    case "date":
      return formatDate(value);
    case "dateTime":
      return formatDateTime(value);
    case "select":
      return optionCaption(field, value);
    case "masked":
      return "••••••";
    case "checkbox":
    case "switch":
      return value === true || value === "true" ? "Yes" : "No";
    case "json":
      return typeof value === "string" ? value : JSON.stringify(value);
    default:
      return String(value);
  }
}
