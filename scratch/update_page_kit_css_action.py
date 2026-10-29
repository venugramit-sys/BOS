path = 'ui/src/lib/page-kit/page-kit.css'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = ".bos-action-menu { display: inline-flex; }"
replacement = ".bos-action-wrap { display: inline-flex; align-items: center; }\n.bos-page-Custom .bos-page-body { padding: var(--bos-pad, 1.25rem); }\n.bos-action-menu { display: inline-flex; }"

if target in text:
    text = text.replace(target, replacement, 1)
else:
    text += "\n.bos-action-wrap { display: inline-flex; align-items: center; }\n.bos-page-Custom .bos-page-body { padding: var(--bos-pad, 1.25rem); }\n"

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated ui/src/lib/page-kit/page-kit.css successfully")
