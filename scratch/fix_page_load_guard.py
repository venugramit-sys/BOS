with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

# 1. Add #lastLoadKey
old_seq = "  #getCtx;\n  #loadSeq = 0;\n  #opened = false;"
new_seq = "  #getCtx;\n  #loadSeq = 0;\n  #lastLoadKey = null;\n  #opened = false;"

assert old_seq in text_lf, "old_seq not found"
text_lf = text_lf.replace(old_seq, new_seq, 1)

# 2. Update load(key)
old_load = """  async load() {
    const seq = ++this.#loadSeq;"""

new_load = """  async load(key = null) {
    if (key !== null && key === this.#lastLoadKey) return;
    if (key !== null) this.#lastLoadKey = key;
    const seq = ++this.#loadSeq;"""

assert old_load in text_lf, "old_load not found"
text_lf = text_lf.replace(old_load, new_load, 1)

# 3. Guard siblingIds fetch in #loadRecord
old_fetch_siblings = """    if (ctx.recordIds && Array.isArray(ctx.recordIds)) {
      this.siblingIds = ctx.recordIds;
    } else if (this.table && !this.isList) {"""

new_fetch_siblings = """    if (ctx.recordIds && Array.isArray(ctx.recordIds)) {
      this.siblingIds = ctx.recordIds;
    } else if (this.table && !this.isList && this.siblingIds.length === 0) {"""

assert old_fetch_siblings in text_lf, "old_fetch_siblings not found"
text_lf = text_lf.replace(old_fetch_siblings, new_fetch_siblings, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("page.svelte.js loop fixed with #lastLoadKey guard")
