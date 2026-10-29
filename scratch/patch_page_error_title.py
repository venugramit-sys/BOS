with open('ui/src/lib/page-kit/page.svelte.js', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''  error = $state("");
  conflict = $state(null);'''

replacement = '''  error = $state("");
  errorTitle = $state("");
  conflict = $state(null);'''

assert target in content, "target not found in page.svelte.js"
content = content.replace(target, replacement)

with open('ui/src/lib/page-kit/page.svelte.js', 'w', encoding='utf-8') as f:
    f.write(content)
print("page.svelte.js updated with errorTitle state!")
