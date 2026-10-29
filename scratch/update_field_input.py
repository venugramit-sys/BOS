with open('ui/src/lib/page-kit/FieldInput.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

replacements = [
    ('<Textarea {id}', '<Textarea {id} name={f.key || c.name}'),
    ('<Input {id} type="number"', '<Input {id} name={f.key || c.name} type="number"'),
    ('<input {id} type="datetime-local"', '<input {id} name={f.key || c.name} type="datetime-local"'),
    ('<input {id} type="time"', '<input {id} name={f.key || c.name} type="time"'),
    ('<Select {id}', '<Select {id} name={f.key || c.name}'),
    ('<Combobox value={value ?? null}', '<Combobox name={f.key || c.name} value={value ?? null}'),
    ('<input {id} class={inputClass + " font-mono"}', '<input {id} name={f.key || c.name} class={inputClass + " font-mono"}')
]

for old, new in replacements:
    if old in content:
        content = content.replace(old, new, 1)
        print(f"Replaced: {old}")
    else:
        print(f"NOT found: {old}")

with open('ui/src/lib/page-kit/FieldInput.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("Done updating FieldInput.svelte")
