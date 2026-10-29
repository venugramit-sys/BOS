with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    c = f.read()

target = """  editable(c, row = null) {
    const f = c.field;
    if (!f || this.design) return false;
    if (this.overrides.editable[c.name] !== undefined) return this.overrides.editable[c.name];
    if (!f.editable) return false;
    if (f.binding === "variable") return this.enabled(c, row);
    if (this.isList ? (!this.meta.modify_allowed || this.ctx.mode === "view") && !row?._new : this.mode === "view") return false;
    if (f.is_primary_key && this.mode !== "create" && !row?._new) return false;
    const fn = this.x[c.i]?.editable;
    if (fn && !this.evaluate(fn, row)) return false;
    return this.enabled(c, row);
  }"""

repl = """  editable(c, row = null) {
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
  }"""

c = c.replace(target.replace('\n', '\r\n'), repl.replace('\n', '\r\n'))
c = c.replace(target, repl)

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(c)

print('Updated page.svelte.js successfully')
