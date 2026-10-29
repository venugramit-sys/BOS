with open('ui/src/lib/components/studio/designers/page/designer-model.svelte.js', 'r', encoding='utf-8') as f:
    c = f.read()

target = """  /** Whether the selected node is defined in this source (not by an extension or the base page). */
  isAuthored(path) {
    return path === "page" || Boolean(findNode([...this.nodes, ...this.actionNodes], path));
  }"""

repl = """  /** Whether the selected node is defined in this source (not by an extension or the base page). */
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
  }"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('ui/src/lib/components/studio/designers/page/designer-model.svelte.js', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated designer-model.svelte.js isAuthored successfully")
