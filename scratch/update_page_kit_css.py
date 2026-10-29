with open('ui/src/lib/page-kit/page-kit.css', 'r', encoding='utf-8') as f:
    c = f.read()

target = ".bos-page-title { display: flex; align-items: baseline; gap: 0.5rem; min-width: 0; font-size: 1.25rem; font-weight: 700; letter-spacing: -0.01em; }"

repl = """.bos-page-title {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  min-width: 0;
  font-size: var(--bos-font-size, 1.25rem);
  font-weight: var(--bos-font-weight, 700);
  color: var(--bos-fg, inherit);
  letter-spacing: var(--bos-letter-spacing, -0.01em);
  line-height: var(--bos-line-height, inherit);
  text-align: var(--bos-text-align, inherit);
  text-transform: var(--bos-text-transform, inherit);
  text-decoration: var(--bos-text-decoration, inherit);
}"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('ui/src/lib/page-kit/page-kit.css', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated page-kit.css successfully")
