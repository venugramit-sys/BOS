/**
 * State of the WYSIWYG page designer.
 *
 * Develop mode edits the `.bos` source of the page (or page extension) through server-side span edits
 * (`POST /api/v1/pages/edit`), so comments and formatting survive; saving applies the source like a
 * hand-written change. Customize mode edits the machine-owned customization layer
 * (`PUT /api/v1/pages/{page}/customization`), limited to what super users may change.
 * Both modes preview the real compiled page (design build) with real records after every change.
 */
import { designerApi } from "./designer-api.js";
import { loadPreview, pageManifest } from "$lib/page-kit/loader.svelte.js";
import { StudioApi } from "../../stores/studioApi.js";
import {
  emptyChanges,
  patchControl,
  moveControl,
  addField,
  removeAddedField,
  patchAction,
  customizationPlacement,
  changeCount,
  dropLocation,
  findNode,
  flatten,
  lastSegment,
  isWithin,
  parentPath as parentOf
} from "./design-values.js";

const MAX_UNDO = 100;

function flatCompiled(nodes, out = []) {
  for (const n of nodes ?? []) {
    out.push(n);
    flatCompiled(n.children, out);
  }
  return out;
}

export class PageDesignerModel {
  mode = $state("develop");
  source = $state("");
  savedSource = $state("");
  outline = $state.raw(null);
  /** Lowered IR of the object being designed (page or extension). */
  ir = $state.raw(null);
  objectKind = $state("page");
  targetPage = $state("");
  schema = $state.raw(null);
  compiled = $state.raw(null);
  diagnostics = $state.raw([]);
  Component = $state.raw(null);
  previewVersion = $state(0);
  fields = $state.raw([]);
  selected = $state(null);
  /** Toolbox item being dragged onto the canvas (`{ kind, source, field?, label }`). */
  dragItem = $state.raw(null);
  viewport = $state("full");
  busy = $state(false);
  saving = $state(false);
  error = $state("");
  notice = $state("");
  changes = $state(emptyChanges());
  savedChanges = $state(emptyChanges());
  customizable = $state(false);
  undoStack = $state.raw([]);
  redoStack = $state.raw([]);
  /** Package the object belongs to (Apply stores it there). Follows the Studio tab once its details load. */
  packageName = $state("bos.app");
  /** The source does not parse: `{ message, line }`. The canvas then shows the published page read-only. */
  parseError = $state.raw(null);
  /** The canvas shows the published page instead of this source (the source does not parse yet). */
  showingPublished = $state(false);
  /** An API page (no user interface): the designer offers code editing only. */
  apiPage = $state(false);

  #previewSeq = 0;
  #previewTimer = null;
  #previewWaiters = [];
  #base = null;
  #baseExtensions = [];

  /**
   * @param {{ name: string, packageName: string, onSourceChange?: (src: string) => void }} opts
   */
  constructor({ name, packageName, onSourceChange }) {
    this.name = name;
    this.packageName = packageName || "bos.app";
    this.onSourceChange = onSourceChange;
  }

  get object() {
    return this.outline?.objects?.find((o) => o.name.toLowerCase() === this.name.toLowerCase()) ?? null;
  }
  get nodes() {
    return this.object?.layout?.nodes ?? [];
  }
  get actionNodes() {
    return this.object?.actions?.nodes ?? [];
  }
  get dirty() {
    return this.mode === "develop" ? this.source !== this.savedSource : JSON.stringify(this.changes) !== JSON.stringify(this.savedChanges);
  }
  get canUndo() {
    return this.undoStack.length > 0;
  }
  get canRedo() {
    return this.redoStack.length > 0;
  }
  get changeCount() {
    return changeCount(this.changes);
  }
  get compiledControls() {
    return flatCompiled(this.compiled?.controls);
  }
  /** Compiled control at the selected path (auto-filled values, data binding). */
  get selectedCompiled() {
    return this.selected ? (this.compiledControls.find((c) => c.path === this.selected) ?? null) : null;
  }
  /** Authored outline node at the selected path (develop mode). */
  get selectedNode() {
    return this.selected && this.selected !== "page" ? findNode([...this.nodes, ...this.actionNodes], this.selected) : null;
  }
  get selectedAction() {
    if (!this.selected?.startsWith("action:")) return null;
    const name = this.selected.slice("action:".length);
    return this.compiled?.actions?.find((a) => a.name === name) ?? null;
  }
  get errors() {
    return this.diagnostics.filter((d) => d.severity === "error");
  }
  /** Table fields not placed on the page yet (toolbox). */
  get unplacedFields() {
    const placed = new Set(this.compiledControls.filter((c) => c.field).map((c) => (c.field.source ?? c.name).toLowerCase()));
    return this.fields.filter((f) => !placed.has(f.name.toLowerCase()));
  }

  // ----------------------------------------------------
  // LOADING
  // ----------------------------------------------------

  /** Source loaded (or reloaded) by the Studio while nothing is edited here: start over from it. */
  async sourceLoaded(source) {
    if ((source ?? "") === this.source || this.dirty) return;
    await this.init(source);
  }

  async init(source) {
    this.busy = true;
    this.error = "";
    try {
      this.schema = await designerApi.propertySchema();
      this.source = source ?? "";
      this.savedSource = this.source;
      await this.#parse();
      this.objectKind = this.object?.kind ?? "page";
      this.targetPage = this.objectKind === "pageextension" ? (this.object?.target ?? "") : this.name;
      // API pages have no user interface: there is nothing to draw, they are edited in code.
      this.apiPage = String(this.ir?.page_type ?? "").toLowerCase() === "api";
      // Until the Studio has loaded the source, only a page the manifest knows can be looked up.
      await pageManifest.start();
      const knownPage = Boolean(this.object) || Boolean(pageManifest.entry(this.targetPage));
      const compiled = this.apiPage || !knownPage ? null : await designerApi.compiled(this.targetPage).catch(() => null);
      this.#base = compiled?.base ?? null;
      this.#baseExtensions = compiled?.extensions ?? [];
      this.customizable = Boolean(compiled);
      this.mode = this.object || !this.customizable ? "develop" : "customize";
      if (this.customizable) {
        const current = await designerApi.customization(this.targetPage);
        this.changes = { ...emptyChanges(), ...current.changes };
        this.savedChanges = $state.snapshot(this.changes);
      }
      const table = compiled?.page?.source_table ?? this.ir?.source_table ?? this.#base?.source_table;
      if (table) this.fields = await designerApi.tableFields(table).catch(() => []);
      await this.refreshPreview(true);
    } catch (e) {
      this.error = e.message ?? String(e);
    } finally {
      this.busy = false;
    }
  }

  async #parse() {
    if (!this.source.trim()) {
      this.outline = { objects: [] };
      this.ir = null;
      return;
    }
    const res = await designerApi.parse(this.source, this.packageName);
    this.#setModel(res);
  }

  #setModel(res) {
    this.outline = res.outline;
    const model = res.model;
    const lname = this.name.toLowerCase();
    this.ir = model ? ([...(model.pages ?? []), ...(model.page_extensions ?? [])].find((p) => p.name.toLowerCase() === lname) ?? null) : null;
    if (res.error) {
      const line = /line (\d+)/i.exec(res.error);
      this.parseError = { message: res.error, line: line ? Number(line[1]) : 0 };
    } else {
      this.parseError = null;
    }
  }

  // ----------------------------------------------------
  // PREVIEW
  // ----------------------------------------------------

  /**
   * Recompiles the design preview (debounced; `now` skips the debounce). Every caller's promise settles
   * when the preview that covers its change has rendered, including calls superseded by a later one.
   */
  refreshPreview(now = false) {
    clearTimeout(this.#previewTimer);
    return new Promise((resolve) => {
      this.#previewWaiters.push(resolve);
      this.#previewTimer = setTimeout(() => {
        const waiters = this.#previewWaiters.splice(0);
        this.#preview().finally(() => waiters.forEach((w) => w()));
      }, now ? 0 : 200);
    });
  }

  async #preview() {
    if (this.apiPage) return;
    const seq = ++this.#previewSeq;
    try {
      let result;
      let published = false;
      if (this.mode === "customize") {
        result = await designerApi.previewCustomization(this.targetPage, this.changes);
      } else if (this.objectKind === "page" && this.ir) {
        result = await designerApi.preview(this.ir, this.#baseExtensions);
      } else if (this.objectKind === "pageextension" && this.ir && this.#base) {
        const others = this.#baseExtensions.filter((e) => e.name.toLowerCase() !== this.name.toLowerCase());
        result = await designerApi.preview(this.#base, [...others, this.ir]);
      } else if (this.#base) {
        // The source does not parse (yet): show what is published so the page is never blank.
        result = await designerApi.preview(this.#base, this.#baseExtensions);
        published = true;
      } else {
        return;
      }
      const component = await loadPreview(result);
      if (seq !== this.#previewSeq) return;
      this.compiled = result.compiled;
      this.diagnostics = result.diagnostics ?? [];
      this.Component = component;
      this.showingPublished = published;
      this.previewVersion += 1;
      this.error = "";
    } catch (e) {
      if (seq === this.#previewSeq) this.error = e.message ?? String(e);
    }
  }

  // ----------------------------------------------------
  // HISTORY
  // ----------------------------------------------------

  #snapshot() {
    return this.mode === "develop" ? { mode: "develop", source: this.source } : { mode: "customize", changes: $state.snapshot(this.changes) };
  }

  #push() {
    this.undoStack = [...this.undoStack.slice(-MAX_UNDO + 1), this.#snapshot()];
    this.redoStack = [];
  }

  async #restore(snap) {
    if (snap.mode === "develop") {
      this.source = snap.source;
      await this.#parse();
      this.onSourceChange?.(this.source);
    } else {
      this.changes = snap.changes;
    }
    await this.refreshPreview(true);
  }

  async undo() {
    const snap = this.undoStack.at(-1);
    if (!snap) return;
    this.undoStack = this.undoStack.slice(0, -1);
    this.redoStack = [...this.redoStack, this.#snapshot()];
    await this.#restore(snap);
  }

  async redo() {
    const snap = this.redoStack.at(-1);
    if (!snap) return;
    this.redoStack = this.redoStack.slice(0, -1);
    this.undoStack = [...this.undoStack, this.#snapshot()];
    await this.#restore(snap);
  }

  setMode(mode) {
    if (mode === this.mode) return;
    if (mode === "develop" && !this.object) {
      this.notice = "This page's source is not open here; open the page object in the Studio to develop it.";
      return;
    }
    if (mode === "customize" && this.source !== this.savedSource) {
      this.notice = "Customize mode works on the published page; your unapplied Develop changes are kept and show again in Develop.";
    }
    this.mode = mode;
    this.undoStack = [];
    this.redoStack = [];
    this.refreshPreview(true);
  }

  // ----------------------------------------------------
  // DEVELOP MODE (source edits)
  // ----------------------------------------------------

  async applyOps(ops) {
    if (!ops.length) return;
    if (this.parseError) {
      this.notice = `Fix the syntax error first (line ${this.parseError.line || "?"}); visual edits need source that parses.`;
      return;
    }
    this.busy = true;
    try {
      const res = await designerApi.edit(this.source, this.name, ops, this.packageName);
      this.#push();
      this.source = res.source;
      this.#setModel(res);
      if (res.selected) this.selected = res.selected;
      this.onSourceChange?.(this.source);
      this.error = "";
      await this.refreshPreview();
    } catch (e) {
      this.error = e.message ?? String(e);
    } finally {
      this.busy = false;
    }
  }

  /** Whether the selected node is defined in this source (not by an extension or the base page). */
  isAuthored(path) {
    return path === "page" || Boolean(findNode([...this.nodes, ...this.actionNodes], path));
  }

  /** Develop mode can only move or insert relative to controls this source defines. */
  #editableHere(path) {
    if (path === "layout" || this.isAuthored(path)) return true;
    this.notice = "That spot belongs to a control defined by another object (a page extension or the base page). Edit that object, or use Customize mode.";
    return false;
  }

  // ----------------------------------------------------
  // SHARED OPERATIONS (dispatch on mode)
  // ----------------------------------------------------

  /** Sets (or clears with `null`) a property of the selected control. `block`: `style` | `responsive`. */
  async setProperty(path, name, bosValue, { block = null, raw = null } = {}) {
    if (this.mode === "develop") {
      const op = bosValue === null ? { op: "removeProperty", path, name, block } : { op: "setProperty", path, name, value: bosValue, block };
      return this.applyOps([op]);
    }
    this.#push();
    if (path === "page") {
      const page = { ...(this.changes.page ?? {}) };
      if (block === "style") page.style = { ...(page.style ?? {}), [name]: raw ?? undefined };
      else page[name] = raw ?? undefined;
      this.changes = { ...this.changes, page };
    } else if (path.startsWith("action:")) {
      this.changes = patchAction($state.snapshot(this.changes), path.slice(7), { [name]: raw });
    } else {
      const control = lastSegment(path).name;
      if (block === "style") this.changes = patchControl($state.snapshot(this.changes), control, { style: { [name]: raw } });
      else if (block === "responsive")
        this.changes = patchControl($state.snapshot(this.changes), control, name === "hideBelow" ? { responsive: { hideBelow: raw } } : { responsive: { span: { [name]: raw } } });
      else this.changes = patchControl($state.snapshot(this.changes), control, { [name]: raw });
    }
    return this.refreshPreview();
  }

  /** Moves a control next to / into `target` (`placement`: before | after | inside). */
  async moveTo(path, target, placement) {
    if (isWithin(target.path, path)) return;
    if (this.mode === "develop") {
      if (!this.#editableHere(path) || !this.#editableHere(placement === "inside" ? target.path : parentOf(target.path))) return;
      const loc = dropLocation(this.nodes, target, placement);
      if (loc.before === path) return;
      return this.applyOps([{ op: "move", path, parent: loc.parent, before: loc.before }]);
    }
    this.#push();
    const { anchor, position } = customizationPlacement(target, placement);
    this.changes = moveControl($state.snapshot(this.changes), lastSegment(path).name, anchor, position);
    return this.refreshPreview();
  }

  /** Inserts a new control (develop) or adds a table field (customize) at `target`/`placement`. */
  async insertAt(item, target, placement) {
    if (this.mode === "develop") {
      if (target && !this.#editableHere(placement === "inside" ? target.path : parentOf(target.path))) return;
      if (!target && this.nodes.length === 0) {
        // Empty page: controls live in an area (NAV `area(Content)`).
        const body = item.source.split("\n").map((l) => `    ${l}`).join("\n");
        return this.applyOps([{ op: "insert", parent: "layout", before: null, source: `area(Content) {\n${body}\n}` }]);
      }
      const loc = target ? dropLocation(this.nodes, target, placement) : { parent: this.nodes[0].path, before: null };
      return this.applyOps([{ op: "insert", parent: loc.parent, before: loc.before, source: item.source }]);
    }
    if (item.kind !== "field") {
      this.notice = "Customize mode can add table fields only; switch to Develop to add other controls.";
      return;
    }
    const anchorTarget = target ?? { path: this.compiledControls.find((c) => c.kind === "group" || c.kind === "area")?.path ?? "" };
    if (!anchorTarget.path) return;
    this.#push();
    const { anchor, position } = customizationPlacement(anchorTarget, target ? placement : "inside");
    this.changes = addField($state.snapshot(this.changes), item.field, anchor, position);
    return this.refreshPreview();
  }

  /** Delete (develop) or hide (customize; an added field is simply removed again). */
  async remove(path) {
    if (this.mode === "develop") {
      if (!this.isAuthored(path)) {
        this.notice = "This control is defined by another object; hide it in Customize mode or edit that object.";
        return;
      }
      if (this.selected === path) this.selected = null;
      return this.applyOps([{ op: "delete", path }]);
    }
    const { name } = lastSegment(path);
    this.#push();
    if (path.startsWith("action:")) this.changes = patchAction($state.snapshot(this.changes), path.slice(7), { visible: false });
    else if (this.changes.adds.some((a) => a.field === name)) this.changes = removeAddedField($state.snapshot(this.changes), name);
    else this.changes = patchControl($state.snapshot(this.changes), name, { visible: false });
    return this.refreshPreview();
  }

  /** Moves the selected node one sibling up / down (keyboard alternative to dragging). */
  async nudge(path, delta) {
    const flat = this.mode === "develop" ? flatten(this.nodes) : this.compiledControls;
    const node = flat.find((n) => n.path === path);
    if (!node) return;
    const parent = path.slice(0, path.lastIndexOf("/"));
    const siblings = flat.filter((n) => n.path.slice(0, n.path.lastIndexOf("/")) === parent);
    const i = siblings.findIndex((n) => n.path === path);
    const other = siblings[i + delta];
    if (!other) return;
    return this.moveTo(path, other, delta < 0 ? "before" : "after");
  }

  // ----------------------------------------------------
  // SAVE
  // ----------------------------------------------------

  async save() {
    this.saving = true;
    this.error = "";
    try {
      if (this.mode === "develop") {
        if (this.parseError) throw new Error(`The source has a syntax error: ${this.parseError.message}`);
        const res = await StudioApi.applyScript(this.source, this.packageName);
        // `/api/script/apply` answers 200 with `applied: false` when the script has errors.
        if (res?.applied === false || res?.success === false) {
          const errors = (res.diagnostics ?? []).filter((d) => String(d.severity).toLowerCase() === "error");
          throw new Error(errors.length ? errors.map((d) => `${d.code ? d.code + ": " : ""}${d.message}`).join("\n") : res.message || "The page was not applied.");
        }
        this.savedSource = this.source;
        this.notice = "Page applied and published.";
      } else {
        const res = await designerApi.saveCustomization(this.targetPage, this.changes);
        this.savedChanges = $state.snapshot(this.changes);
        this.diagnostics = res.diagnostics ?? this.diagnostics;
        this.notice = "Customization published for everyone using this database.";
      }
      return true;
    } catch (e) {
      this.error = e.message ?? String(e);
      if (e.diagnostics?.length) this.diagnostics = e.diagnostics;
      return false;
    } finally {
      this.saving = false;
    }
  }

  /** Discards every customization (publishes an empty layer). */
  async resetCustomization() {
    this.#push();
    this.changes = emptyChanges();
    await this.refreshPreview(true);
  }
}
