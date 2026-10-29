with open('ui/src/lib/page-kit/PageHost.svelte', 'r', encoding='utf-8') as f:
    c = f.read()

target = """   *   onActionError?: (actionName: string, err: any, p: any) => void,
   *   design?: boolean,"""

repl = """   *   onActionError?: (actionName: string, err: any, p: any) => void,
   *   onHostError?: (err: string | null) => void,
   *   design?: boolean,"""

target2 = "onActionError = undefined, design = false"
repl2 = "onActionError = undefined, onHostError = undefined, design = false"

target3 = """    loadPageComponent(name)
      .then(({ component }) => {
        if (alive) Component = component;
      })
      .catch((e) => {
        if (alive) {
          Component = null;
          error = e?.message ?? String(e);
        }
      });"""

repl3 = """    loadPageComponent(name)
      .then(({ component }) => {
        if (alive) {
          Component = component;
          onHostError?.(null);
        }
      })
      .catch((e) => {
        if (alive) {
          Component = null;
          error = e?.message ?? String(e);
          onHostError?.(error);
        }
      });"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')
    target3 = target3.replace('\n', '\r\n')
    repl3 = repl3.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

assert target2 in c, "target2 not found"
c = c.replace(target2, repl2, 1)

assert target3 in c, "target3 not found"
c = c.replace(target3, repl3, 1)

with open('ui/src/lib/page-kit/PageHost.svelte', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated PageHost.svelte successfully")
