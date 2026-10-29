/**
 * Pure record logic of compiled pages (no Svelte runes, unit-tested with node:test).
 */

/** Page types that show many records at once. */
export const LIST_TYPES = new Set(["List", "ListPart", "Worksheet", "Board", "Timeline"]);

/** Every field control of the control table `C`, in document order. */
export function fieldControls(C) {
  return C.filter((c) => c.kind === "field" || c.kind === "cue").filter((c) => c.field);
}

/** Keys the user may write to the table: bound columns that are editable or primary keys for creation. */
export function writableKeys(C) {
  return [...new Set(fieldControls(C).filter((c) => c.field.binding === "column" && (c.field.editable || c.field.is_primary_key)).map((c) => c.field.key))];
}

/** Typed default value of a field for a new record. */
export function typedDefault(field) {
  const raw = field.default_value;
  if (raw === undefined || raw === null) return field.control === "checkbox" || field.control === "switch" ? false : null;
  if (field.control === "checkbox" || field.control === "switch") return raw === true || String(raw).toLowerCase() === "true";
  return raw;
}

/** Initial record of a new entry: table defaults, then the page filters (links) on top. */
export function newRecordValues(C, filters = {}) {
  const rec = {};
  for (const c of fieldControls(C)) {
    if (c.field.binding !== "column") continue;
    if (!(c.field.key in rec)) rec[c.field.key] = typedDefault(c.field);
  }
  for (const [k, v] of Object.entries(filters ?? {})) rec[k] = v;
  return rec;
}

/** Initial page variables (declared defaults). */
export function initialVariables(META) {
  const vars = {};
  for (const v of META.variables ?? []) {
    let value = v.default_value ?? null;
    if (/^Boolean/i.test(v.data_type)) value = value === null ? false : String(value).toLowerCase() === "true";
    vars[v.name] = value;
  }
  return vars;
}

function same(a, b) {
  if (a === b) return true;
  if ((a === null || a === undefined || a === "") && (b === null || b === undefined || b === "")) return true;
  if (typeof a === "object" || typeof b === "object") return JSON.stringify(a) === JSON.stringify(b);
  return String(a) === String(b);
}

/** Keys whose value differs between the loaded record and the edited one. */
export function changedKeys(original, current, keys) {
  return keys.filter((k) => !same(original?.[k], current?.[k]));
}

/** PATCH body: changed writable fields plus the optimistic-concurrency version. */
export function buildPatch(original, current, keys) {
  const body = {};
  for (const k of changedKeys(original, current, keys)) body[k] = current[k] === "" ? null : current[k];
  if (original && original._system_version !== undefined) body._system_version = original._system_version;
  return body;
}

/** POST body: every writable field that has a value. */
export function buildInsert(current, keys) {
  const body = {};
  for (const k of keys) {
    const v = current?.[k];
    if (v !== undefined && v !== null && v !== "") body[k] = v;
  }
  return body;
}

/** Exact-match entity filter from link values (`{ key: { $eq: value } }`), skipping unset values. */
export function exactFilter(values) {
  const filter = {};
  for (const [k, v] of Object.entries(values ?? {})) {
    if (v === undefined) continue;
    filter[k] = v === null ? null : { $eq: v };
  }
  return filter;
}

/** Evaluates `[{ field, value: (rec, page) => any }]` link functions. */
export function evaluateLinks(links, rec, vars) {
  const out = {};
  for (const l of links ?? []) {
    try {
      out[l.field] = l.value(rec ?? {}, vars ?? {});
    } catch {
      out[l.field] = undefined;
    }
  }
  return out;
}

/** True when every link value is known (a part shows nothing until its parent record exists). */
export function linksComplete(values) {
  return Object.values(values).every((v) => v !== undefined && v !== null && v !== "");
}

/** Question of an unanswered `ctx.ui.confirm`, from a structured 409 or a codeunit error message. */
export function pendingConfirm(error) {
  if (!error) return null;
  if (error.code === "CONFIRM_REQUIRED" && error.confirm) return error.confirm;
  const m = /ConfirmRequired: (.+?)(?: \[stack:|$)/.exec(String(error.message ?? ""));
  return m ? m[1].trim() : null;
}

/** Maps a server toast (`ctx.ui`) to a bos-ui toast tone. */
export function toastTone(type) {
  switch (String(type ?? "").toLowerCase()) {
    case "success":
      return "success";
    case "warning":
      return "warning";
    case "error":
    case "danger":
      return "danger";
    default:
      return "info";
  }
}

/** Normalizes `ctx.ui` directives returned by a trigger. */
export function normalizeDirectives(list) {
  return (Array.isArray(list) ? list : []).filter((d) => d && typeof d.type === "string" && !d.pending);
}

function toSnake(s) {
  return String(s).replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}
function toCamel(s) {
  return String(s).replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
}

/** Classifies a save error for the page frame. */
export function classifySaveError(error) {
  const msg = String(error?.message ?? error ?? "");
  if (error?.status === 409) return { kind: "conflict", message: msg };
  if (error?.status === 428) return { kind: "version", message: msg };

  const fields = {};
  if (error?.fieldErrors && typeof error.fieldErrors === "object") {
    for (const [k, v] of Object.entries(error.fieldErrors)) {
      fields[k] = v;
      fields[toSnake(k)] = v;
      fields[toCamel(k)] = v;
    }
  }

  // Extract field mentions from business logic / tablelogic errors
  const m =
    /Field ['"]?(\w+)['"]? is required/i.exec(msg) ||
    /Validation Failed:\s*Field ['"]?(\w+)['"]?/i.exec(msg) ||
    /['"]?(\w+)['"]? cannot be blank/i.exec(msg) ||
    /column ['"]?(\w+)['"]? of relation/i.exec(msg);
  if (m) {
    const rawKey = m[1];
    fields[rawKey] = msg;
    fields[toSnake(rawKey)] = msg;
    fields[toCamel(rawKey)] = msg;
  }

  if (Object.keys(fields).length) {
    return { kind: "fields", message: msg, fields };
  }
  return { kind: "error", message: msg };
}

/** Values of `data_caption_fields`, joined for the page header ("C1000 · Adatum"). */
export function dataCaption(META, rec) {
  return (META.data_caption_fields ?? [])
    .map((k) => rec?.[k])
    .filter((v) => v !== null && v !== undefined && v !== "")
    .join(" · ");
}

/** Required fields of `controls` that are still empty (wizard step / save validation). */
export function missingRequired(controls, rec, vars) {
  return controls
    .filter((c) => c.field?.required)
    .filter((c) => {
      const v = c.field.binding === "variable" ? vars?.[c.field.key] : rec?.[c.field.key];
      return v === null || v === undefined || v === "";
    })
    .map((c) => c.caption || c.name);
}
