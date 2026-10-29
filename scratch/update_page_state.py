with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    c = f.read()

target = """    if (!this.table) {
      this.loaded = true;
      await this.#openTrigger();
      return;
    }"""

repl = """    if (!this.table || this.ctx.unbound || (this.meta.page_type === "StandardDialog" && !this.ctx.recordId && Object.keys(this.filters).length === 0)) {
      this.loaded = true;
      this.rec = {};
      this.original = null;
      this.mode = "view";
      await this.#openTrigger();
      return;
    }"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated page.svelte.js successfully")
