with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

# 1. Add state fields for record navigation
target_state = "  prompt = $state(null);\n  loaded = $state(false);"
replacement_state = """  prompt = $state(null);
  loaded = $state(false);

  // ----- record navigation (card stepper) -----
  siblingIds = $state([]);
  stepping = $state(false);"""

assert target_state in text_lf, "target_state not found"
text_lf = text_lf.replace(target_state, replacement_state, 1)

# 2. Add navigation getters and methods
target_methods = "  get current() {\n    return this.rows.find((r) => r._id === this.currentId) ?? null;\n  }"
replacement_methods = """  get current() {\n    return this.rows.find((r) => r._id === this.currentId) ?? null;\n  }

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
        if (this.ctx?.onRecordChange) this.ctx.onRecordChange(rec);
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
        if (this.ctx?.onRecordChange) this.ctx.onRecordChange(rec);
      }
    } finally {
      this.stepping = false;
    }
  }"""

assert target_methods in text_lf, "target_methods not found"
text_lf = text_lf.replace(target_methods, replacement_methods, 1)

# 3. In #loadRecord, initialize siblingIds
target_load = """    let record = null;
    if (ctx.recordId) {
      record = await api.getRecord(this.table, ctx.recordId);
    } else {"""

replacement_load = """    let record = null;
    if (ctx.recordIds && Array.isArray(ctx.recordIds)) {
      this.siblingIds = ctx.recordIds;
    } else if (this.table && !this.isList) {
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
    } else {"""

assert target_load in text_lf, "target_load not found"
text_lf = text_lf.replace(target_load, replacement_load, 1)

# 4. In openPage(card_page), pass recordIds
target_open_card = """      this.ctx.openPage?.(this.meta.card_page, { recordId: row._id, mode: this.meta.modify_allowed ? "edit" : "view" });"""
replacement_open_card = """      this.ctx.openPage?.(this.meta.card_page, {
        recordId: row._id,
        recordIds: this.rows.map((r) => r._id),
        mode: this.meta.modify_allowed ? "edit" : "view"
      });"""

assert target_open_card in text_lf, "target_open_card not found"
text_lf = text_lf.replace(target_open_card, replacement_open_card, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("page.svelte.js updated with record stepper support")
