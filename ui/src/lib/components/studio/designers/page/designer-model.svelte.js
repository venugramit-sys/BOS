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
    if (!this.selected) return null;
    if (this.selected === "header:title") {
      return {
        path: "header:title",
        kind: "headline",
        name: "Title",
        caption: this.compiled?.caption ?? this.object?.caption ?? this.name,
        style: this.compiled?.style ?? this.object?.style ?? {},
        visible: true
      };
    }
    if (this.selected === "header:instruction") {
      return {
        path: "header:instruction",
        kind: "label",
        name: "Instruction",
        caption: this.compiled?.instructional_text ?? this.object?.instructional_text ?? "",
        visible: Boolean(this.compiled?.instructional_text ?? this.object?.instructional_text)
      };
    }
    if (this.selected === "header") {
      return {
        path: "header",
        kind: "header",
        name: "Header",
        caption: "Page Header",
        style: this.compiled?.style ?? this.object?.style ?? {},
        visible: true
      };
    }
    return this.compiledControls.find((c) => c.path === this.selected) ?? null;
  }
  /** Authored outline node at the selected path (develop mode). */
  get selectedNode() {
    if (!this.selected || this.selected === "page") return null;
    if (this.selected === "header:title") {
      return {
        path: "header:title",
        kind: "headline",
        name: "Title",
        caption: this.object?.caption ?? this.name,
        style: this.object?.style ?? {},
        blocks: this.object?.blocks ?? [],
        props: this.object?.props ?? []
      };
    }
    if (this.selected === "header:instruction") {
      return {
        path: "header:instruction",
        kind: "label",
        name: "Instruction",
        caption: this.object?.instructional_text ?? "",
        blocks: this.object?.blocks ?? [],
        props: this.object?.props ?? []
      };
    }
    if (this.selected === "header") {
      return {
        path: "header",
        kind: "header",
        name: "Header",
        caption: "Page Header",
        style: this.object?.style ?? {},
        blocks: this.object?.blocks ?? [],
        props: this.object?.props ?? []
      };
    }
    return findNode([...this.nodes, ...this.actionNodes], this.selected);
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
  /** Page variables declared on the page. */
  get variables() {
    if (this.compiled?.variables?.length) return this.compiled.variables;
    if (this.ir?.variables?.length) return this.ir.variables;
    const varsMatch = /variables\s*\{([^}]*)\}/i.exec(this.source);
    if (varsMatch) {
      const block = varsMatch[1];
      const re = /([A-Za-z0-9_]+)\s*:\s*([^;{=]+)(?:=\s*([^;{]+))?(?:\{([^}]*)\})?;/g;
      const parsed = [];
      let m;
      while ((m = re.exec(block)) !== null) {
        const name = m[1].trim();
        const data_type = m[2].trim();
        const defVal = m[3] ? m[3].trim() : null;
        const propsStr = m[4] || "";
        const capMatch = /caption\s*=\s*"([^"]+)"/.exec(propsStr);
        parsed.push({
          name,
          data_type,
          caption: capMatch ? capMatch[1] : name,
          default_value: defVal
        });
      }
      return parsed;
    }
    return [];
  }
  /** Page variables not placed on the page yet. */
  get unplacedVariables() {
    const placed = new Set(this.compiledControls.filter((c) => c.field).map((c) => (c.field.source ?? c.field.key ?? c.name).toLowerCase()));
    return this.variables.filter((v) => !placed.has(v.name.toLowerCase()));
  }

  /** Declares a new page variable inside the variables block in page source. */
  async addVariable({ name, data_type, caption, default_value }) {
    if (!name || !data_type) return;
    this.#push();
    let src = this.source;
    let varDef = `        ${name}: ${data_type}`;
    if (default_value) varDef += ` = ${default_value}`;
    if (caption && caption !== name) varDef += ` { caption = "${caption}"; }`;
    varDef += `;\n`;

    const varsIndex = src.search(/\bvariables\s*\{/i);
    if (varsIndex !== -1) {
      const openBrace = src.indexOf("{", varsIndex);
      let depth = 1;
      let closeBrace = -1;
      for (let i = openBrace + 1; i < src.length; i++) {
        if (src[i] === "{") depth++;
        else if (src[i] === "}") {
          depth--;
          if (depth === 0) {
            closeBrace = i;
            break;
          }
        }
      }
      if (closeBrace !== -1) {
        src = src.slice(0, closeBrace) + varDef + src.slice(closeBrace);
      }
    } else {
      const pageHeaderMatch = /page\s+[A-Za-z0-9_]+[^{]*\{/i.exec(src);
      if (pageHeaderMatch) {
        const insertPos = pageHeaderMatch.index + pageHeaderMatch[0].length;
        const block = `\n    variables {\n${varDef}    }\n`;
        src = src.slice(0, insertPos) + block + src.slice(insertPos);
      }
    }
    this.source = src;
    await this.#parse();
    this.onSourceChange?.(this.source);
    await this.refreshPreview(true);
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
      this.customizable = false;
      this.mode = "develop";
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
      this.notice = "This page's source is not open here; open the page object in the Studio to design it.";
      return;
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
    if (this.objectKind === "page") {
      return (
        path === "page" ||
        path === "header" ||
        path === "header:title" ||
        path === "header:instruction" ||
        Boolean(findNode([...this.nodes, ...this.actionNodes], path))
      );
    }
    return Boolean(findNode([...this.nodes, ...this.actionNodes], path));
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
    let effectivePath = path;
    let effectiveName = name;
    if (path === "header:title" || path === "header") {
      effectivePath = "page";
    } else if (path === "header:instruction") {
      effectivePath = "page";
      if (name === "caption") effectiveName = "instructional_text";
    }
    if (this.mode === "develop") {
      const op = bosValue === null ? { op: "removeProperty", path: effectivePath, name: effectiveName, block } : { op: "setProperty", path: effectivePath, name: effectiveName, value: bosValue, block };
      return this.applyOps([op]);
    }
    this.#push();
    if (effectivePath === "page") {
      const page = { ...(this.changes.page ?? {}) };
      if (block === "style") page.style = { ...(page.style ?? {}), [effectiveName]: raw ?? undefined };
      else page[effectiveName] = raw ?? undefined;
      this.changes = { ...this.changes, page };
    } else if (effectivePath.startsWith("action:")) {
      this.changes = patchAction($state.snapshot(this.changes), effectivePath.slice(7), { [effectiveName]: raw });
    } else {
      const control = lastSegment(effectivePath).name;
      if (block === "style") this.changes = patchControl($state.snapshot(this.changes), control, { style: { [effectiveName]: raw } });
      else if (block === "responsive")
        this.changes = patchControl($state.snapshot(this.changes), control, effectiveName === "hideBelow" ? { responsive: { hideBelow: raw } } : { responsive: { span: { [effectiveName]: raw } } });
      else this.changes = patchControl($state.snapshot(this.changes), control, { [effectiveName]: raw });
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
      if (item.kind === "action_group") {
        const name = "Group_" + Math.floor(Math.random() * 900 + 100);
        return this.addActionGroup({
          name,
          caption: item.label ?? "Menu Group",
          area: "Processing"
        });
      }
      if (item.category === "actions" || item.kind === "action") {
        const name = "Action_" + Math.floor(Math.random() * 900 + 100);
        return this.addAction({
          name,
          caption: item.label ?? "New Action",
          area: "Processing",
          primary: false,
          promoted: true
        });
      }
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

  /** Duplicates the selected control in develop mode. */
  async duplicate(path) {
    if (this.mode !== "develop" || !this.isAuthored(path)) {
      this.notice = "Only authored controls in Develop mode can be duplicated.";
      return;
    }
    const node = findNode([...this.nodes, ...this.actionNodes], path);
    if (!node) return;
    const { kind, name } = lastSegment(path);
    const newName = `${name}_Copy`;
    const source = `${kind}(${newName}) { caption = "${name} (Copy)"; }`;
    return this.insertAt({ kind, source }, node, "after");
  }

  /**
   * Adds an action to the page AST inside actions { area(...) { ... } }.
   */
  async addAction({
    name,
    caption,
    area = "Processing",
    group = null,
    groupCaption = null,
    icon = null,
    primary = false,
    promoted = false,
    promotedCategory = null,
    confirm = null,
    shortcut = null,
    tooltip = null,
    runCodeunit = null,
    openPage = null,
    runPageMode = null,
    runPageLink = null,
    script = null
  }) {
    if (this.mode !== "develop") {
      this.notice = "Actions can only be added in Develop mode.";
      return;
    }

    const lines = [`action(${name}) {`];
    if (caption) lines.push(`    caption = "${caption.replace(/"/g, '\\"')}";`);
    if (icon) lines.push(`    icon = "${icon}";`);
    if (primary) lines.push(`    primary = true;`);
    if (promoted) lines.push(`    promoted = true;`);
    if (promotedCategory) lines.push(`    promotedCategory = ${promotedCategory};`);
    if (confirm) lines.push(`    confirm = "${confirm.replace(/"/g, '\\"')}";`);
    if (shortcut) lines.push(`    shortcut = "${shortcut}";`);
    if (tooltip) lines.push(`    tooltip = "${tooltip.replace(/"/g, '\\"')}";`);
    if (runCodeunit) lines.push(`    runCodeunit = ${runCodeunit};`);
    if (openPage) lines.push(`    openPage = ${openPage};`);
    if (runPageMode) lines.push(`    runPageMode = ${runPageMode};`);
    if (runPageLink) lines.push(`    runPageLink = "${runPageLink}";`);
    if (script) {
      lines.push(`    trigger onAction() {`);
      lines.push(`        ${script}`);
      lines.push(`    }`);
    }
    lines.push(`}`);
    const actionSource = lines.join("\n");

    const flatActions = flatten(this.actionNodes);
    const areaNode = flatActions.find(
      (n) => n.kind === "area" && n.name.toLowerCase() === area.toLowerCase()
    );

    if (group) {
      const groupNode = flatActions.find(
        (n) => n.kind === "group" && n.name.toLowerCase() === group.toLowerCase()
      );
      if (groupNode) {
        await this.applyOps([{ op: "insert", parent: groupNode.path, before: null, source: actionSource }]);
      } else if (areaNode) {
        const grpCaption = groupCaption || group;
        const groupSource = `group(${group}) {\n    caption = "${grpCaption.replace(/"/g, '\\"')}";\n${actionSource.split("\n").map((l) => "    " + l).join("\n")}\n}`;
        await this.applyOps([{ op: "insert", parent: areaNode.path, before: null, source: groupSource }]);
      } else {
        const grpCaption = groupCaption || group;
        const combined = `area(${area}) {\n    group(${group}) {\n        caption = "${grpCaption.replace(/"/g, '\\"')}";\n${actionSource.split("\n").map((l) => "        " + l).join("\n")}\n    }\n}`;
        await this.applyOps([{ op: "insert", parent: "actions", before: null, source: combined }]);
      }
    } else {
      if (areaNode) {
        await this.applyOps([{ op: "insert", parent: areaNode.path, before: null, source: actionSource }]);
      } else {
        const combined = `area(${area}) {\n${actionSource.split("\n").map((l) => "    " + l).join("\n")}\n}`;
        await this.applyOps([{ op: "insert", parent: "actions", before: null, source: combined }]);
      }
    }

    this.selected = `action:${name}`;
    await this.refreshPreview();
  }

  /**
   * Adds an action menu group container inside an area.
   */
  async addActionGroup({ name, caption, area = "Processing" }) {
    if (this.mode !== "develop") {
      this.notice = "Action groups can only be added in Develop mode.";
      return;
    }
    const groupSource = `group(${name}) {\n    caption = "${caption.replace(/"/g, '\\"')}";\n}`;
    const flatActions = flatten(this.actionNodes);
    const areaNode = flatActions.find(
      (n) => n.kind === "area" && n.name.toLowerCase() === area.toLowerCase()
    );
    if (areaNode) {
      await this.applyOps([{ op: "insert", parent: areaNode.path, before: null, source: groupSource }]);
    } else {
      const combined = `area(${area}) {\n${groupSource.split("\n").map((l) => "    " + l).join("\n")}\n}`;
      await this.applyOps([{ op: "insert", parent: "actions", before: null, source: combined }]);
    }
    await this.refreshPreview();
  }

  /**
   * Deletes an action or action group.
   */
  async removeAction(nameOrPath) {
    if (this.mode !== "develop") {
      return this.remove(nameOrPath.startsWith("action:") ? nameOrPath : `action:${nameOrPath}`);
    }
    const flat = flatten(this.actionNodes);
    const target = nameOrPath.startsWith("action:") ? nameOrPath.slice(7) : nameOrPath;
    const node = flat.find((n) => n.name.toLowerCase() === target.toLowerCase() || n.path === nameOrPath);
    if (node) {
      if (this.selected?.toLowerCase().includes(target.toLowerCase())) this.selected = null;
      return this.applyOps([{ op: "delete", path: node.path }]);
    }
  }

  /**
   * Moves an action up or down relative to sibling actions.
   */
  async moveAction(nameOrPath, delta) {
    if (this.mode !== "develop") return;
    const flat = flatten(this.actionNodes);
    const target = nameOrPath.startsWith("action:") ? nameOrPath.slice(7) : nameOrPath;
    const node = flat.find((n) => n.name.toLowerCase() === target.toLowerCase() || n.path === nameOrPath);
    if (!node) return;
    const parent = node.path.slice(0, node.path.lastIndexOf("/"));
    const siblings = flat.filter((n) => n.path.slice(0, n.path.lastIndexOf("/")) === parent);
    const i = siblings.findIndex((n) => n.path === node.path);
    const other = siblings[i + delta];
    if (!other) return;
    const before = delta < 0 ? other.path : (siblings[i + 2]?.path ?? null);
    return this.applyOps([{ op: "move", path: node.path, parent, before }]);
  }

  /**
   * Reorders an action relative to a target action directly on the canvas action bar.
   */
  async reorderAction(sourceName, targetName) {
    if (this.mode !== "develop" || sourceName === targetName) return;
    const flat = flatten(this.actionNodes);
    const sourceNode = flat.find((n) => n.name.toLowerCase() === sourceName.toLowerCase());
    const targetNode = flat.find((n) => n.name.toLowerCase() === targetName.toLowerCase());
    if (!sourceNode || !targetNode) return;
    const parent = targetNode.path.slice(0, targetNode.path.lastIndexOf("/"));
    await this.applyOps([{ op: "move", path: sourceNode.path, parent, before: targetNode.path }]);
  }

  /**
   * 1-Click companion codeunit scaffolding to resolve BOS3004.
   */
  async scaffoldCodeunit(codeunitName) {
    if (!codeunitName) return;
    this.busy = true;
    try {
      const cuSource = `codeunit ${codeunitName} {\n    procedure Run() {\n        Message("${codeunitName} executed successfully.");\n    }\n}\n`;
      const res = await StudioApi.applyScript(cuSource, this.packageName);
      if (res?.applied === false || res?.success === false) {
        throw new Error(res.message || "Failed to scaffold codeunit");
      }
      this.notice = `Codeunit '${codeunitName}' created and registered in ${this.packageName}.`;
      await this.refreshPreview();
    } catch (e) {
      this.error = `Could not scaffold codeunit ${codeunitName}: ${e.message ?? e}`;
    } finally {
      this.busy = false;
    }
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
