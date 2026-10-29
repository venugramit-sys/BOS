/**
 * `createPage(getCtx, META, C, X)`: the page controller every compiled page creates once.
 *
 * Host context (`ctx`, read through the getter so changes are reactive):
 * - `page`: page name; `recordId`: `_id` of the record to show (card pages)
 * - `mode`: `"view" | "edit" | "create"`; `filters`: exact field filters (links of parts / openPage)
 * - `embedded`: rendered inside another page (part / FactBox); `design`: designer canvas
 * - `openPage(name, { recordId, mode, filters, modal })`, `closePage()`, `navigate(url)`
 */
import { untrack } from "svelte";
import { notify } from "$lib/components/notifications/index.js";
import * as api from "./api.js";
import {
  LIST_TYPES,
  writableKeys,
  newRecordValues,
  initialVariables,
  buildPatch,
  buildInsert,
  changedKeys,
  exactFilter,
  pendingConfirm,
  toastTone,
  normalizeDirectives,
  classifySaveError,
  dataCaption,
  missingRequired
} from "./record-model.js";

const MAX_CONFIRM_ROUNDS = 8;

function plain(value) {
  return value === undefined ? undefined : JSON.parse(JSON.stringify(value ?? null));
}

export class PageController {
  // ----- record (card-like pages) -----
  rec = $state({});
  original = $state.raw(null);
  vars = $state({});
  mode = $state("view");

  // ----- rows (list-like pages) -----
  rows = $state([]);
  total = $state(0);
  pageNo = $state(1);
  limit = $state(50);
  sort = $state("");
  search = $state("");
  searchField = $state("");
  selected = $state([]);
  currentId = $state(null);
  rowEdits = $state({});

  // ----- status -----
  loading = $state(false);
  saving = $state(false);
  busyAction = $state("");
  error = $state("");
  errorTitle = $state("");
  conflict = $state(null);
  fieldErrors = $state({});
  overrides = $state({ visible: {}, editable: {} });
  prompt = $state(null);
  loaded = $state(false);

  // ----- record navigation (card stepper) -----
  siblingIds = $state([]);
  stepping = $state(false);

  // ----- wizard (NavigatePage) -----
  step = $state(0);
  stepCount = $state(0);

  #getCtx;
  #loadSeq = 0;
  #lastLoadKey = null;
  #opened = false;

  /**
   * @param {() => any} getCtx
   * @param {any} META
   * @param {any[]} C
   * @param {Record<number, any>} X
   */
  constructor(getCtx, META, C, X) {
    this.#getCtx = getCtx;
    this.meta = META;
    this.C = C;
    this.x = X;
    this.keys = writableKeys(C);
    this.flowKeys = [...new Set(C.filter((c) => c.field?.binding === "flowField").map((c) => c.field.key))];
    const initialVars = initialVariables(META);
    const hostVars = this.ctx?.variables ?? this.ctx?.params?.variables ?? {};
    this.vars = { ...initialVars, ...hostVars };
    this.limit = META.page_type === "ListPart" ? 20 : 50;
    // Searchable columns (text-like, stored). The default is the first descriptive one (NAV searches by name
    // more often than by code); the list toolbar lets the user pick another.
    this.searchFields = C.filter((c) => c.field && c.field.binding === "column" && ["text", "email", "phone", "url", "lookup", "select"].includes(c.field.control))
      .map((c) => ({ key: c.field.key, caption: c.caption ?? c.name, pk: Boolean(c.field.is_primary_key) }))
      .filter((f, i, all) => all.findIndex((x) => x.key === f.key) === i);
    this.searchField = (this.searchFields.find((f) => !f.pk) ?? this.searchFields[0])?.key ?? "";

    $effect(() => {
      // Reload whenever the host changes record, mode or filters.
      const c = this.ctx;
      const key = JSON.stringify([c.page, c.recordId ?? null, c.mode ?? null, c.filters ?? null]);
      untrack(() => this.load(key));
    });

    $effect(() => {
      const d = this.dirty;
      untrack(() => {
        this.ctx?.onDirtyChange?.(d);
        if (typeof window !== "undefined" && !this.design) {
          const draftKey = "bos_draft:" + this.meta.name + ":" + (this.ctx?.recordId || "new");
          if (d) {
            try {
              const draftData = this.isList ? this.rowEdits : this.rec;
              localStorage.setItem(draftKey, JSON.stringify({ savedAt: Date.now(), data: draftData }));
            } catch {}
          }
        }
      });
    });
  }

  get ctx() {
    return this.#getCtx?.() ?? {};
  }
  get design() {
    return this.ctx.design !== undefined ? Boolean(this.ctx.design) : Boolean(this.meta.design);
  }
  get embedded() {
    return Boolean(this.ctx.embedded);
  }
  get isList() {
    return LIST_TYPES.has(this.meta.page_type);
  }
  get table() {
    return this.meta.source_sql_table;
  }
  get isNew() {
    return this.mode === "create";
  }
  get dirty() {
    if (this.isList) return Object.keys(this.rowEdits).length > 0;
    if (this.mode === "create") return true;
    return changedKeys(this.original, this.rec, this.keys).length > 0;
  }
  get caption() {
    return this.meta.caption;
  }
  get dataCaption() {
    return this.isList ? "" : dataCaption(this.meta, this.rec);
  }
  get current() {
    return this.rows.find((r) => r._id === this.currentId) ?? null;
  }

  get siblingIndex() {
    return this.rec?._id ? this.siblingIds.indexOf(this.rec._id) : -1;
  }
  get hasPrevRecord() {
    return !this.isList && this.siblingIndex > 0;
  }
  get hasNextRecord() {
    return !this.isList && this.siblingIndex >= 0 && this.siblingIndex < this.siblingIds.length - 1;
  }
  get recordPositionText() {
    if (this.isList || this.siblingIndex < 0) return "";
    return `${this.siblingIndex + 1} of ${this.siblingIds.length}`;
  }
  get canNavigateRecords() {
    return !this.isList && this.siblingIds.length > 1;
  }

  async prevRecord() {
    if (!this.hasPrevRecord || this.stepping) return;
    if (this.dirty) {
      const ok = await this.save();
      if (!ok) return;
    }
    const prevId = this.siblingIds[this.siblingIndex - 1];
    if (!prevId) return;
    this.stepping = true;
    try {
      const rec = await this.#fetchRecord(prevId);
      if (rec) {
        this.#setRecord(rec);
        if (this.ctx?.onRecordChange) this.ctx.onRecordChange(rec._id ?? prevId);
      }
    } finally {
      this.stepping = false;
    }
  }

  async nextRecord() {
    if (!this.hasNextRecord || this.stepping) return;
    if (this.dirty) {
      const ok = await this.save();
      if (!ok) return;
    }
    const nextId = this.siblingIds[this.siblingIndex + 1];
    if (!nextId) return;
    this.stepping = true;
    try {
      const rec = await this.#fetchRecord(nextId);
      if (rec) {
        this.#setRecord(rec);
        if (this.ctx?.onRecordChange) this.ctx.onRecordChange(rec._id ?? nextId);
      }
    } finally {
      this.stepping = false;
    }
  }
  /** NAV default order: the primary key. */
  get keySort() {
    return (this.meta.primary_key ?? []).map((k) => `${k}:asc`).join(",");
  }
  /** Lists edited in place (NAV Worksheet / ListPart / list without a card page). */
  get inlineEdit() {
    const t = this.meta.page_type;
    return t === "Worksheet" || t === "ListPart" || !this.meta.card_page;
  }
  get pageCount() {
    return Math.max(1, Math.ceil(this.total / this.limit));
  }

  /** Context-level exact filters (part links / openPage filters). */
  get filters() {
    return this.ctx.filters ?? {};
  }

  // ----------------------------------------------------
  // LOADING
  // ----------------------------------------------------

  async load(key = null) {
    if (key !== null && key === this.#lastLoadKey) return;
    if (key !== null) this.#lastLoadKey = key;
    const seq = ++this.#loadSeq;
    this.error = "";
    this.conflict = null;
    this.fieldErrors = {};
    if (!this.table || this.ctx.unbound || (this.meta.page_type === "StandardDialog" && !this.ctx.recordId && Object.keys(this.filters).length === 0)) {
      this.loaded = true;
      this.rec = {};
      this.original = null;
      this.mode = "view";
      await this.#openTrigger();
      return;
    }
    this.loading = true;
    try {
      if (this.isList) {
        await this.loadRows(seq);
      } else {
        await this.#loadRecord(seq);
      }
      if (seq === this.#loadSeq) {
        this.loaded = true;
        await this.#openTrigger();
      }
    } catch (e) {
      if (seq === this.#loadSeq) this.error = e.message ?? String(e);
    } finally {
      if (seq === this.#loadSeq) this.loading = false;
    }
  }

  /** NAV OnOpenPage: once per page instance, never on the designer canvas (scripts may write or navigate). */
  async #openTrigger() {
    if (this.#opened || this.design) return;
    this.#opened = true;
    if ((this.meta.triggers ?? []).some((t) => t.toLowerCase() === "onopenpage")) {
      await this.runTrigger("onOpenPage", this.rec).catch((e) => this.#toastError(e));
    }
  }

  #listFilter() {
    const filter = exactFilter(this.filters);
    if (this.search && this.searchField) filter[this.searchField] = this.search;
    return filter;
  }

  async loadRows(seq = this.#loadSeq) {
    const res = await api.listRecords(this.table, { page: this.pageNo, limit: this.limit, sort: this.sort || this.keySort, filter: this.#listFilter() });
    const rows = await this.#withFlowFields(res?.records ?? []);
    if (seq !== this.#loadSeq) return;
    this.rows = rows;
    this.total = res?.total ?? this.rows.length;
    this.rowEdits = {};
    if (!this.rows.some((r) => r._id === this.currentId)) this.currentId = this.rows[0]?._id ?? null;
    this.selected = this.selected.filter((id) => this.rows.some((r) => r._id === id));
  }

  async #loadRecord(seq) {
    const ctx = this.ctx;
    if (ctx.mode === "create") {
      this.#startNew();
      return;
    }
    let record = null;
    if (ctx.recordIds && Array.isArray(ctx.recordIds)) {
      this.siblingIds = ctx.recordIds;
    } else if (this.table && !this.isList && this.siblingIds.length === 0) {
      api.listRecords(this.table, {
        page: 1,
        limit: 200,
        sort: this.keySort || "_id:asc",
        filter: exactFilter(this.filters)
      }).then((res) => {
        if (res?.records) {
          this.siblingIds = res.records.map((r) => r._id);
        }
      }).catch(() => {});
    }

    if (ctx.recordId) {
      record = await api.getRecord(this.table, ctx.recordId);
    } else {
      // NAV semantics: a card without an explicit record shows the first record of its filter.
      const res = await api.listRecords(this.table, { page: 1, limit: 1, sort: this.keySort, filter: exactFilter(this.filters) });
      record = res?.records?.[0] ?? null;
    }
    if (record) [record] = await this.#withFlowFields([record]);
    if (seq !== this.#loadSeq) return;
    if (!record) {
      if (this.meta.insert_allowed && !this.embedded) this.#startNew();
      else {
        this.rec = {};
        this.original = null;
        this.mode = "view";
      }
      return;
    }
    this.#setRecord(record);
    this.mode = ctx.mode === "view" || !this.meta.modify_allowed ? "view" : "edit";
    if (!this.design && (this.meta.triggers ?? []).some((t) => t.toLowerCase() === "onaftergetrecord")) {
      await this.runTrigger("onAfterGetRecord", this.rec).catch((e) => this.#toastError(e));
    }
  }

  /** Adds FlowField values (sums, counts) to records: one query for all of them. */
  async #withFlowFields(records) {
    if (!this.flowKeys.length || !records.length) return records;
    const ids = records.map((r) => r._id).filter(Boolean);
    const values = await api.queryRecords(this.meta.source_table ?? this.table, { select: ["_id", ...this.flowKeys], filter: { _id: { $in: ids } }, limit: ids.length });
    const byId = new Map(values.map((v) => [v._id, v]));
    return records.map((r) => {
      const v = byId.get(r._id);
      if (!v) return r;
      const out = { ...r };
      for (const k of this.flowKeys) out[k] = v[k] ?? null;
      return out;
    });
  }

  async #fetchRecord(id) {
    const [record] = await this.#withFlowFields([await api.getRecord(this.table, id)]);
    return record;
  }

  #setRecord(record) {
    this.original = plain(record);
    this.rec = plain(record);
    this.fieldErrors = {};
    this.conflict = null;
  }

  #startNew() {
    const values = newRecordValues(this.C, this.filters);
    this.original = null;
    this.rec = values;
    this.mode = "create";
    if (typeof window !== "undefined" && !this.design) {
      try {
        const cached = localStorage.getItem("bos_draft:" + this.meta.name + ":new");
        if (cached) {
          const parsed = JSON.parse(cached);
          if (parsed?.data && typeof parsed.data === "object") {
            Object.assign(this.rec, parsed.data);
          }
        }
      } catch {}
    }
  }

  // ----------------------------------------------------
  // FIELD STATE
  // ----------------------------------------------------

  /** Value of a field on the current record (variables live on `vars`). */
  valueOf(field, row = null) {
    if (field.binding === "variable") return this.vars[field.key];
    if (row) return this.rowEdits[row._id]?.[field.key] ?? row[field.key];
    return this.rec[field.key];
  }

  visible(c, row = null) {
    if (this.overrides.visible[c.name] === false) return false;
    if (this.overrides.visible[c.name] === true) return true;
    if (!c.visible) return false;
    const show = this.x[c.i]?.show;
    return show ? this.evaluate(show, row) : true;
  }

  enabled(c, row = null) {
    const fn = this.x[c.i]?.enable;
    return fn ? this.evaluate(fn, row) : true;
  }

  editable(c, row = null) {
    const f = c.field;
    if (!f || this.design) return false;
    if (this.overrides.editable[c.name] !== undefined) return this.overrides.editable[c.name];
    const isNew = this.mode === "create" || row?._new;
    if (!f.editable && !(isNew && f.is_primary_key)) return false;
    if (f.binding === "variable") return this.enabled(c, row);
    if (this.isList ? (!this.meta.modify_allowed || this.ctx.mode === "view") && !row?._new : this.mode === "view") return false;
    if (f.is_primary_key && !isNew) return false;
    const fn = this.x[c.i]?.editable;
    if (fn && !this.evaluate(fn, row)) return false;
    return this.enabled(c, row);
  }

  styleResult(c, row = null) {
    const fn = this.x[c.i]?.style;
    if (!fn) return undefined;
    try {
      return fn(this.recordOf(row), this.vars);
    } catch {
      return undefined;
    }
  }

  evaluate(fn, row = null) {
    try {
      const v = fn(this.recordOf(row), this.vars);
      return v === true || (v !== false && v !== null && v !== undefined && v !== "" && v !== "false" && v !== 0);
    } catch {
      return false;
    }
  }

  /** Record a trigger sees for `row` (list rows include their unsaved edits). */
  recordOf(row = null) {
    return row ? { ...row, ...(this.rowEdits[row._id] ?? {}) } : this.rec;
  }

  /** Writes a field value locally (no server round trip; see `validate`). */
  setValue(c, value, row = null) {
    const f = c.field;
    if (f.binding === "variable") {
      this.vars[f.key] = value;
    } else if (row) {
      this.rowEdits = { ...this.rowEdits, [row._id]: { ...(this.rowEdits[row._id] ?? {}), [f.key]: value } };
    } else {
      this.rec[f.key] = value;
    }
    const toRemove = [f.key, f.source, c.name].filter(Boolean);
    let changed = false;
    let nextErrors = { ...this.fieldErrors };
    for (const k of toRemove) {
      if (nextErrors[k]) {
        delete nextErrors[k];
        changed = true;
      }
    }
    if (changed) {
      this.fieldErrors = nextErrors;
    }
  }

  /** Runs the control's server-side `onValidate` trigger (NAV OnValidate) after the user committed a value. */
  async validate(c, row = null) {
    if (this.design || !(c.triggers ?? []).some((t) => t.toLowerCase() === "onvalidate")) return;
    const f = c.field;
    try {
      const res = await this.runTrigger(`control.${c.name}.onValidate`, this.recordOf(row));
      if (res?.record && !row) Object.assign(this.rec, res.record);
      if (res?.record && row) this.rowEdits = { ...this.rowEdits, [row._id]: { ...(this.rowEdits[row._id] ?? {}), ...res.record } };
    } catch (e) {
      this.fieldErrors = { ...this.fieldErrors, [f.key]: e.message ?? String(e) };
    }
  }

  /** `setValue` followed by `validate` (pickers, switches, lookups: one gesture commits the value). */
  async commit(c, value, row = null) {
    this.setValue(c, value, row);
    await this.validate(c, row);
  }

  // ----------------------------------------------------
  // SAVE / DELETE / NEW
  // ----------------------------------------------------

  async save() {
    if (!this.table || this.design) return true;
    if (this.isList) return this.saveRows();
    const missingControls = this.C
      .filter((c) => c.field?.binding === "column" && c.field?.required)
      .filter((c) => {
        const v = c.field.binding === "variable" ? this.vars?.[c.field.key] : this.rec?.[c.field.key];
        return v === null || v === undefined || v === "";
      });
    if (missingControls.length) {
      const errMap = { ...this.fieldErrors };
      for (const c of missingControls) {
        errMap[c.field.key] = `${c.caption || c.name} is required`;
        if (c.field.source) errMap[c.field.source] = `${c.caption || c.name} is required`;
        errMap[c.name] = `${c.caption || c.name} is required`;
      }
      this.fieldErrors = errMap;
      // Single channel: in-field errors highlight the missing controls directly; suppress top banner
      this.error = "";
      const first = missingControls[0];
      const firstKey = first.field.key || first.name;
      if (typeof document !== "undefined") {
        setTimeout(() => {
          const el = document.querySelector(
            `[data-field="${first.name}"] input, [data-field="${first.name}"] textarea, [data-field="${first.name}"] select, [data-field="${first.name}"] [role="combobox"], [data-key="${firstKey}"] input, input[name="${firstKey}"]`
          );
          el?.focus?.();
          el?.scrollIntoView?.({ behavior: "smooth", block: "center" });
        }, 50);
      }
      return false;
    }
    this.saving = true;
    this.error = "";
    try {
      if (this.mode === "create") {
        const res = await api.createRecord(this.table, buildInsert(this.rec, this.keys));
        const id = res?._id ?? res?.id;
        this.#notify(res?.messages);
        if (id) {
          this.#setRecord(await this.#fetchRecord(id));
          this.mode = this.meta.modify_allowed ? "edit" : "view";
          this.ctx.onRecordChange?.(id);
        }
        this.ctx.onChanged?.();
      } else {
        const body = buildPatch(this.original, this.rec, this.keys);
        if (Object.keys(body).filter((k) => k !== "_system_version").length === 0) return true;
        const res = await api.updateRecord(this.table, this.original._id, body);
        this.#notify(res?.messages);
        this.#setRecord(await this.#fetchRecord(this.original._id));
        this.ctx.onChanged?.();
        if (typeof window !== "undefined") {
          try { localStorage.removeItem("bos_draft:" + this.meta.name + ":" + (this.ctx?.recordId || "new")); } catch {}
        }
      }
      return true;
    } catch (e) {
      this.#saveFailed(e);
      return false;
    } finally {
      this.saving = false;
    }
  }

  #saveFailed(e) {
    const r = classifySaveError(e);
    if (r.kind === "conflict" || r.kind === "version") this.conflict = r.message;
    else if (r.kind === "fields") {
      this.fieldErrors = { ...this.fieldErrors, ...r.fields };
      // Single channel: in-field errors highlight the fields directly; suppress top banner
      this.error = "";
      const firstKey = Object.keys(r.fields)[0];
      if (firstKey && typeof document !== "undefined") {
        setTimeout(() => {
          const el = document.querySelector(
            `[data-field="${firstKey}"] input, [data-field="${firstKey}"] textarea, [data-field="${firstKey}"] select, [data-field="${firstKey}"] [role="combobox"], [data-key="${firstKey}"] input, input[name="${firstKey}"]`
          );
          el?.focus?.();
          el?.scrollIntoView?.({ behavior: "smooth", block: "center" });
        }, 50);
      }
    } else {
      this.error = r.message;
    }
  }

  async saveRows() {
    this.saving = true;
    this.error = "";
    try {
      // Rows are saved one by one; each saved row leaves the pending set immediately, so a retry after
      // a failure never inserts or patches the same row twice.
      let saved = 0;
      try {
        for (const [id, edits] of Object.entries(this.rowEdits)) {
          const row = this.rows.find((r) => r._id === id);
          let stored = null;
          if (row) {
            if (row._new) {
              const res = await api.createRecord(this.table, buildInsert({ ...row, ...edits }, this.keys));
              stored = { ...row, ...edits, _id: res?._id ?? res?.id ?? id, _new: false };
            } else {
              await api.updateRecord(this.table, id, buildPatch(row, { ...row, ...edits }, this.keys));
              stored = { ...row, ...edits };
            }
            saved += 1;
          }
          const { [id]: _done, ...rest } = this.rowEdits;
          this.rowEdits = rest;
          if (stored) {
            this.rows = this.rows.map((r) => (r._id === id ? stored : r));
            if (this.currentId === id) this.currentId = stored._id;
          }
        }
      } finally {
        if (saved) this.ctx.onChanged?.();
      }
      await this.loadRows();
      return true;
    } catch (e) {
      this.#saveFailed(e);
      return false;
    } finally {
      this.saving = false;
    }
  }

  /** Reloads the record discarding local changes (also resolves a concurrency conflict). */
  async reload() {
    this.rowEdits = {};
    await this.load();
  }

  async deleteCurrent(row = null) {
    const target = row ?? (this.isList ? this.current : this.original);
    if (!target?._id || !this.meta.delete_allowed) return;
    if (!(await this.ask(`Delete this ${this.meta.source_table ?? "record"}?`))) return;
    try {
      await api.deleteRecord(this.table, target._id);
      toastStore.push({ tone: "success", title: "Deleted" });
      this.ctx.onChanged?.();
      if (this.isList) await this.loadRows();
      else if (this.embedded) await this.load();
      else this.ctx.closePage?.();
    } catch (e) {
      this.#toastError(e);
    }
  }

  newRecord() {
    if (!this.meta.insert_allowed) return;
    if (this.isList && this.inlineEdit) {
      const id = `new-${crypto.randomUUID()}`;
      this.rows = [{ _id: id, _new: true, ...newRecordValues(this.C, this.filters) }, ...this.rows];
      this.rowEdits = { ...this.rowEdits, [id]: {} };
      this.currentId = id;
      return;
    }
    if (this.isList) {
      this.ctx.openPage?.(this.meta.card_page, { mode: "create", filters: this.filters });
      return;
    }
    this.#startNew();
  }

  /** Opens the card page of a list row (NAV CardPageId). */
  openRow(row) {
    if (!row || row._new) return;
    this.currentId = row._id;
    if (this.meta.card_page) {
      this.ctx.openPage?.(this.meta.card_page, {
        recordId: row._id,
        recordIds: this.rows.map((r) => r._id),
        mode: this.meta.modify_allowed ? "edit" : "view"
      });
    }
  }

  setSort(key) {
    const [k, dir] = this.sort.split(":");
    this.sort = k === key ? (dir === "desc" ? "" : `${key}:desc`) : `${key}:asc`;
    this.pageNo = 1;
    this.loadRows().catch((e) => (this.error = e.message));
  }

  setPage(n) {
    this.pageNo = Math.min(Math.max(1, n), this.pageCount);
    this.loadRows().catch((e) => (this.error = e.message));
  }

  applySearch(text) {
    this.search = text;
    this.pageNo = 1;
    this.loadRows().catch((e) => (this.error = e.message));
  }

  // ----------------------------------------------------
  // ACTIONS & TRIGGERS
  // ----------------------------------------------------

  /** Asks the user (rendered by PageFrame with a ConfirmDialog). */
  ask(question) {
    return new Promise((resolve) => {
      this.prompt = {
        question,
        resolve: (answer) => {
          this.prompt = null;
          resolve(answer);
        }
      };
    });
  }

  /** Runs a server-side trigger, answering `ctx.ui.confirm` questions interactively. */
  async runTrigger(ref, record) {
    if (this.design) return null;
    const answers = {};
    for (let round = 0; round < MAX_CONFIRM_ROUNDS; round++) {
      try {
        const res = await api.runPageTrigger(this.meta.name, ref, { record: plain(record), variables: plain(this.vars), uiAnswers: answers });
        this.#applyTriggerResult(res);
        return res;
      } catch (e) {
        const question = pendingConfirm(e);
        if (!question || question in answers) throw e;
        answers[question] = await this.ask(question);
      }
    }
    throw new Error("Too many confirmation questions");
  }

  async #runCodeunit(target, params) {
    const answers = {};
    for (let round = 0; round < MAX_CONFIRM_ROUNDS; round++) {
      try {
        const res = await api.runCodeunit(target, params, answers);
        this.#notify(res?.messages);
        return res;
      } catch (e) {
        const question = pendingConfirm(e);
        if (!question || question in answers) throw e;
        answers[question] = await this.ask(question);
      }
    }
    throw new Error("Too many confirmation questions");
  }

  #applyTriggerResult(res) {
    if (!res) return;
    if (res.variables && typeof res.variables === "object") this.vars = { ...this.vars, ...res.variables };
    this.#notify(res.messages);
    for (const d of normalizeDirectives(res.directives)) {
      switch (d.type) {
        case "modal":
          this.ctx.openPage?.(d.page, { filters: d.params ?? {}, modal: true });
          break;
        case "navigate":
          if (typeof d.url === "string" && d.url.startsWith("/")) (this.ctx.navigate ?? ((u) => window.location.assign(u)))(d.url);
          break;
        case "fieldVisible":
          this.overrides = { ...this.overrides, visible: { ...this.overrides.visible, [d.field]: d.value } };
          break;
        case "fieldEditable":
          this.overrides = { ...this.overrides, editable: { ...this.overrides.editable, [d.field]: d.value } };
          break;
      }
    }
  }

  #notify(messages) {
    for (const m of Array.isArray(messages) ? messages : []) {
      const sev = (m.type ?? m.severity ?? "info").toLowerCase();
      const msg = m.message ?? m.description ?? "";
      const title = m.title ?? "";
      if (sev === "error" || sev === "danger") {
        notify.error(msg, { title: title || "Notification", page: this });
      } else if (sev === "warning") {
        notify.warning(msg, { title: title || "Warning" });
      } else if (sev === "success") {
        notify.success(msg, { title: title || "Success" });
      } else {
        notify.info(msg, { title: title || "Information" });
      }
    }
  }

  #toastError(e) {
    notify.error(e, { title: "Action Failed", page: this });
  }

  actionVisible(a) {
    if (!a) return true;
    if (a.visible === false || a.properties?.visible === "false") return false;
    return a.show ? this.evaluate(a.show, this.isList ? this.current : null) : true;
  }

  actionEnabled(a) {
    if (!a) return true;
    if (this.busyAction) return false;
    if (a.properties?.enabled === "false" || a.properties?.disabled === "true") return false;
    if (a.selection === "single" && this.isList && !this.current) return false;
    if (a.selection === "multiple" && this.isList && this.selected.length === 0) return false;
    return a.enable ? this.evaluate(a.enable, this.isList ? this.current : null) : true;
  }

  /** Runs a page action: confirm → script / codeunit / page navigation (actionrefs run their target). */
  async runAction(action, actions = []) {
    let a = action;
    if (a.target) a = actions.find((x) => x.name.toLowerCase() === a.target.toLowerCase()) ?? a;
    if (this.design || !this.actionEnabled(a)) return;
    const confirmMsg = a.confirm || a.properties?.confirm;
    if (confirmMsg && !(await this.ask(confirmMsg))) return;
    const row = this.isList ? this.current : null;
    const record = row ?? this.rec;
    this.busyAction = a.name;
    try {
      const codeunitTarget = a.run_codeunit || a.properties?.runCodeunit || a.properties?.codeunit;
      const openPageTarget = a.open_page || a.properties?.openPage || a.properties?.open_page;
      const hasScript = a.has_script || a.triggers?.includes("onAction") || a.triggers?.includes("onClick");

      if (!this.isList && this.dirty && this.mode !== "view" && (this.meta.modify_allowed || (this.mode === "create" && this.meta.insert_allowed)) && (hasScript || codeunitTarget)) {
        if (!(await this.save())) return;
      }
      let res = null;
      if (hasScript) {
        res = await this.runTrigger(`action.${a.name}`, record);
        if (res?.record && !this.isList) Object.assign(this.rec, res.record);
      }
      if (codeunitTarget) {
        const params = { record: plain(record), records: this.isList ? plain(this.rows.filter((r) => this.selected.includes(r._id))) : undefined };
        await this.#runCodeunit(codeunitTarget, params);
      }
      if (openPageTarget) {
        const values = {};
        for (const l of a.links ?? []) values[l.field] = l.value(record, this.vars);
        this.ctx.openPage?.(openPageTarget, { filters: values, mode: (a.run_page_mode ?? a.properties?.runPageMode ?? "View").toLowerCase() });
      }
      if (hasScript || codeunitTarget) {
        if (this.isList) await this.loadRows();
        else if (this.original?._id) this.#setRecord(await this.#fetchRecord(this.original._id));
      }
      if (this.ctx?.onActionComplete) {
        await this.ctx.onActionComplete(a.name, res, this);
      }
    } catch (e) {
      if (this.ctx?.onActionError) {
        await this.ctx.onActionError(a.name, e, this);
      } else {
        this.#toastError(e);
      }
    } finally {
      this.busyAction = "";
    }
  }
}

/** Entry point used by generated pages. */
export function createPage(getCtx, META, C, X) {
  return new PageController(getCtx, META, C, X);
}
