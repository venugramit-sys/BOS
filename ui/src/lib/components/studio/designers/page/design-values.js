/**
 * Pure helpers of the page designer (unit-tested with node:test).
 * - BOS literal encoding of property values (develop mode writes source text)
 * - node path arithmetic (`area:Content/group:General/field:no`)
 * - customization change merging (customize mode never writes source)
 */

const IDENT = /^[A-Za-z_][A-Za-z0-9_]*$/;

/** BOS source text of a property value of the given schema type. */
export function toBosValue(type, value) {
  if (value === null || value === undefined) return null;
  const t = String(type ?? "text");
  if (t === "bool") return value === true || value === "true" ? "true" : "false";
  if (t === "int" || t === "span") {
    const n = Number.parseInt(value, 10);
    if (!Number.isInteger(n)) throw new Error(`'${value}' is not a whole number`);
    return String(n);
  }
  if (t === "expr") {
    const s = String(value).trim();
    if (!s || /[;{}]/.test(s.replace(/"(?:[^"\\]|\\.)*"/g, ""))) throw new Error("an expression cannot contain ; { or }");
    return s;
  }
  if (t === "color") {
    const s = String(value).trim();
    if (/^#[0-9a-f]{6}$/i.test(s)) return JSON.stringify(s.toLowerCase());
    if (IDENT.test(s)) return s;
    throw new Error(`'${s}' is not a theme colour or #rrggbb`);
  }
  if (t.startsWith("enum") || t === "page") {
    const s = String(value).trim();
    if (!IDENT.test(s)) throw new Error(`'${s}' is not a valid name`);
    return s;
  }
  if (t === "fields") {
    const list = (Array.isArray(value) ? value : String(value).split(","))
      .map((x) => String(x).trim())
      .filter(Boolean);
    for (const x of list) if (!IDENT.test(x)) throw new Error(`'${x}' is not a field name`);
    return `[${list.join(", ")}]`;
  }
  // text, icon
  return JSON.stringify(String(value));
}

/** Reads an authored BOS value back into an editor value. */
export function fromBosValue(type, text) {
  if (text === null || text === undefined) return null;
  const s = String(text).trim();
  if (s.startsWith('"') && s.endsWith('"')) {
    try {
      return JSON.parse(s);
    } catch {
      return s.slice(1, -1);
    }
  }
  if (type === "bool") return s === "true";
  if (type === "fields" && s.startsWith("[")) return s.slice(1, -1).split(",").map((x) => x.trim()).filter(Boolean);
  return s;
}

export function parentPath(path) {
  const i = path.lastIndexOf("/");
  return i < 0 ? "layout" : path.slice(0, i);
}

export function lastSegment(path) {
  const seg = path.slice(path.lastIndexOf("/") + 1);
  const i = seg.indexOf(":");
  return { kind: i < 0 ? seg : seg.slice(0, i), name: i < 0 ? seg : seg.slice(i + 1) };
}

/** Kinds that can hold other controls (drop "into" targets). */
export const CONTAINER_KINDS = new Set(["area", "group", "grid", "fixed", "repeater", "cuegroup", "tabs", "tab", "flex", "split", "card", "wizardstep", "container"]);

/** Flat, document-ordered list of outline nodes. */
export function flatten(nodes, out = []) {
  for (const n of nodes ?? []) {
    out.push(n);
    flatten(n.children, out);
  }
  return out;
}

export function findNode(nodes, path) {
  return flatten(nodes).find((n) => n.path.toLowerCase() === String(path).toLowerCase()) ?? null;
}

/** Siblings of a node and its index among them. */
export function siblingsOf(nodes, path) {
  const parent = parentPath(path);
  const list = parent === "layout" ? nodes : (findNode(nodes, parent)?.children ?? []);
  return { list, index: list.findIndex((n) => n.path === path), parent };
}

/**
 * Turns a drop (target node + placement) into a move/insert location.
 * `placement`: "before" | "after" | "inside".
 */
export function dropLocation(nodes, target, placement) {
  if (placement === "inside") return { parent: target.path, before: null };
  const { list, index, parent } = siblingsOf(nodes, target.path);
  if (placement === "before") return { parent, before: target.path };
  return { parent, before: list[index + 1]?.path ?? null };
}

/** Whether `path` lies inside (or is) `ancestor`. */
export function isWithin(path, ancestor) {
  const p = path.toLowerCase();
  const a = ancestor.toLowerCase();
  return p === a || p.startsWith(a + "/");
}

// ----------------------------------------------------
// customization changes
// ----------------------------------------------------

export function emptyChanges() {
  return { page: {}, controls: [], moves: [], adds: [], actions: [] };
}

function clean(obj) {
  const out = {};
  for (const [k, v] of Object.entries(obj)) {
    if (v === undefined || v === null || v === "") continue;
    if (typeof v === "object" && !Array.isArray(v) && Object.keys(clean(v)).length === 0) continue;
    out[k] = typeof v === "object" && !Array.isArray(v) ? clean(v) : v;
  }
  return out;
}

/** Merges `patch` into the change of control `name` (null / "" clears a property). */
export function patchControl(changes, name, patch) {
  const next = structuredClone(changes);
  const i = next.controls.findIndex((c) => c.name === name);
  const current = i >= 0 ? next.controls[i] : { name };
  const merged = { ...current, ...patch };
  if (patch.style) merged.style = clean({ ...(current.style ?? {}), ...patch.style });
  if (patch.responsive) merged.responsive = clean({ ...(current.responsive ?? {}), ...patch.responsive, span: clean({ ...(current.responsive?.span ?? {}), ...(patch.responsive.span ?? {}) }) });
  const result = clean(merged);
  result.name = name;
  if (i >= 0) next.controls.splice(i, 1);
  if (Object.keys(result).length > 1) next.controls.push(result);
  return next;
}

/** Records a move of control `name` (a later move of the same control replaces the earlier one). */
export function moveControl(changes, name, anchor, position) {
  const next = structuredClone(changes);
  const added = next.adds.find((a) => a.field === name);
  if (added) {
    added.anchor = anchor;
    added.position = position;
    return next;
  }
  next.moves = next.moves.filter((m) => m.name !== name);
  next.moves.push({ name, anchor, position });
  return next;
}

export function addField(changes, field, anchor, position) {
  const next = structuredClone(changes);
  if (next.adds.some((a) => a.field === field)) return next;
  next.adds.push({ field, anchor, position });
  return next;
}

export function removeAddedField(changes, field) {
  const next = structuredClone(changes);
  next.adds = next.adds.filter((a) => a.field !== field);
  next.controls = next.controls.filter((c) => c.name !== field);
  return next;
}

export function patchAction(changes, name, patch) {
  const next = structuredClone(changes);
  const i = next.actions.findIndex((a) => a.name === name);
  const merged = clean({ ...(i >= 0 ? next.actions[i] : {}), ...patch, name });
  if (i >= 0) next.actions.splice(i, 1);
  if (Object.keys(merged).length > 1) next.actions.push(merged);
  return next;
}

/** Placement of a customization move/add from a drop location in the compiled tree. */
export function customizationPlacement(target, placement) {
  const { name } = lastSegment(target.path);
  if (placement === "inside") return { anchor: name, position: "last" };
  return { anchor: name, position: placement };
}

export function changeCount(changes) {
  const c = changes ?? emptyChanges();
  return (c.page?.caption ? 1 : 0) + (c.page?.style ? 1 : 0) + c.controls.length + c.moves.length + c.adds.length + c.actions.length;
}
