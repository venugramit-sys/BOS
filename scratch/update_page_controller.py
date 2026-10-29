with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add isNew getter and dirty sync
target_dirty = """  get dirty() {
    if (this.isList) return Object.keys(this.rowEdits).length > 0;
    if (this.mode === "create") return true;
    return changedKeys(this.original, this.rec, this.keys).length > 0;
  }"""

replacement_dirty = """  get isNew() {
    return this.mode === "create";
  }
  get dirty() {
    if (this.isList) return Object.keys(this.rowEdits).length > 0;
    if (this.mode === "create") return true;
    return changedKeys(this.original, this.rec, this.keys).length > 0;
  }"""

# 2. Add dirty change notifier in constructor
target_effect = """    $effect(() => {
      // Reload whenever the host changes record, mode or filters.
      const c = this.ctx;
      const key = JSON.stringify([c.page, c.recordId ?? null, c.mode ?? null, c.filters ?? null]);
      untrack(() => this.load(key));
    });"""

replacement_effect = """    $effect(() => {
      // Reload whenever the host changes record, mode or filters.
      const c = this.ctx;
      const key = JSON.stringify([c.page, c.recordId ?? null, c.mode ?? null, c.filters ?? null]);
      untrack(() => this.load(key));
    });

    $effect(() => {
      const d = this.dirty;
      untrack(() => {
        this.ctx?.onDirtyChange?.(d);
      });
    });"""

# 3. Update setValue
target_setvalue = """    if (this.fieldErrors[f.key]) {
      const { [f.key]: _removed, ...rest } = this.fieldErrors;
      this.fieldErrors = rest;
    }"""

replacement_setvalue = """    const toRemove = [f.key, f.source, c.name].filter(Boolean);
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
    }"""

# 4. Update save and #saveFailed
target_save = """  async save() {
    if (!this.table || this.design) return true;
    if (this.isList) return this.saveRows();
    const missing = missingRequired(this.C.filter((c) => c.field?.binding === "column"), this.rec, this.vars);
    if (missing.length) {
      this.error = `Fill in the required fields: ${missing.join(", ")}`;
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
    else if (r.kind === "fields") this.fieldErrors = { ...r.fields };
    this.error = r.message;
  }"""

replacement_save = """  async save() {
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
      this.error = `Fill in the required fields: ${missingControls.map((c) => c.caption || c.name).join(", ")}`;
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
    }
    this.error = r.message;
  }"""

for name, t, r in [
    ("dirty", target_dirty, replacement_dirty),
    ("effect", target_effect, replacement_effect),
    ("setvalue", target_setvalue, replacement_setvalue),
    ("save", target_save, replacement_save)
]:
    t_crlf = t.replace('\n', '\r\n')
    r_crlf = r.replace('\n', '\r\n')
    if t in content:
        content = content.replace(t, r, 1)
        print(f"Replaced {name} (LF)")
    elif t_crlf in content:
        content = content.replace(t_crlf, r_crlf, 1)
        print(f"Replaced {name} (CRLF)")
    else:
        print(f"NOT found: {name}")

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(content)
print("page.svelte.js update complete")
