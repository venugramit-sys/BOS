with open('ui/src/lib/components/shell/SignIn.svelte', 'r', encoding='utf-8') as f:
    c = f.read()

target = """            params={{
              mode: "view",
              variables: {"""

repl = """            params={{
              mode: "view",
              unbound: true,
              variables: {"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('ui/src/lib/components/shell/SignIn.svelte', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated SignIn.svelte unbound: true successfully")
