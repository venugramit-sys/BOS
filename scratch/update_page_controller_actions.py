path = 'ui/src/lib/page-kit/page.svelte.js'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = """  actionVisible(a) {
    return a.show ? this.evaluate(a.show, this.isList ? this.current : null) : true;
  }

  actionEnabled(a) {
    if (this.busyAction) return false;
    if (a.selection === "single" && this.isList && !this.current) return false;
    if (a.selection === "multiple" && this.isList && this.selected.length === 0) return false;
    return a.enable ? this.evaluate(a.enable, this.isList ? this.current : null) : true;
  }

  /** Runs a page action: confirm → script / codeunit / page navigation (actionrefs run their target). */
  async runAction(action, actions = []) {
    let a = action;
    if (a.target) a = actions.find((x) => x.name.toLowerCase() === a.target.toLowerCase()) ?? a;
    if (this.design || !this.actionEnabled(a)) return;
    if (a.confirm && !(await this.ask(a.confirm))) return;
    const row = this.isList ? this.current : null;
    const record = row ?? this.rec;
    this.busyAction = a.name;
    try {
      if (!this.isList && this.dirty && this.mode !== "view" && (this.meta.modify_allowed || (this.mode === "create" && this.meta.insert_allowed)) && (a.has_script || a.run_codeunit)) {
        if (!(await this.save())) return;
      }
      let res = null;
      if (a.has_script) {
        res = await this.runTrigger(`action.${a.name}`, record);
        if (res?.record && !this.isList) Object.assign(this.rec, res.record);
      }
      if (a.run_codeunit) {
        const params = { record: plain(record), records: this.isList ? plain(this.rows.filter((r) => this.selected.includes(r._id))) : undefined };
        await this.#runCodeunit(a.run_codeunit, params);
      }
      if (a.open_page) {
        const values = {};
        for (const l of a.links ?? []) values[l.field] = l.value(record, this.vars);
        this.ctx.openPage?.(a.open_page, { filters: values, mode: (a.run_page_mode ?? "View").toLowerCase() });
      }
      if (a.has_script || a.run_codeunit) {
        if (this.isList) await this.loadRows();
        else if (this.original?._id) this.#setRecord(await this.#fetchRecord(this.original._id));
      }"""

replacement = """  actionVisible(a) {
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
      }"""

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated ui/src/lib/page-kit/page.svelte.js successfully")
