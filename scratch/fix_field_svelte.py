with open('ui/src/lib/page-kit/Field.svelte', 'rb') as f:
    content = f.read().decode('utf-8')

target = """  const errorText = $derived(
    p.fieldErrors[c.field?.key] ??
    p.fieldErrors[c.field?.source] ??
    p.fieldErrors[c.name] ??
    ""
  );"""

replacement = """  const errorText = $derived(
    p.fieldErrors[c.field?.key] ??
    p.fieldErrors[c.field?.source] ??
    p.fieldErrors[c.name] ??
    ""
  );
  // Controls that render their own error UI (Input, Textarea, Select, DatePicker, Combobox)
  // must not have Field.svelte render a duplicate error element.
  const controlHandlesError = $derived(
    !c.field?.control ||
    ["text", "number", "email", "phone", "url", "password", "masked", "textArea", "select", "date", "lookup", "json"].includes(c.field.control)
  );"""

if target not in content and target.replace('\n', '\r\n') in content:
    target = target.replace('\n', '\r\n')
    replacement = replacement.replace('\n', '\r\n')

assert target in content, "target not found in Field.svelte"
content = content.replace(target, replacement, 1)

target2 = """    {#if errorText}
      <p class="bos-field-error text-[11px] font-medium text-rose-600 dark:text-rose-400 mt-1 flex items-center gap-1 animate-in fade-in-50 duration-150">
        <span aria-hidden="true">⚠</span>
        <span>{errorText}</span>
      </p>
    {/if}"""

replacement2 = """    {#if errorText && !controlHandlesError}
      <p class="bos-field-error text-[11px] font-medium text-rose-600 dark:text-rose-400 mt-1 flex items-center gap-1 animate-in fade-in-50 duration-150">
        <span aria-hidden="true">⚠</span>
        <span>{errorText}</span>
      </p>
    {/if}"""

if target2 not in content and target2.replace('\n', '\r\n') in content:
    target2 = target2.replace('\n', '\r\n')
    replacement2 = replacement2.replace('\n', '\r\n')

assert target2 in content, "target2 not found in Field.svelte"
content = content.replace(target2, replacement2, 1)

with open('ui/src/lib/page-kit/Field.svelte', 'wb') as f:
    f.write(content.encode('utf-8'))

print("Field.svelte successfully updated.")
