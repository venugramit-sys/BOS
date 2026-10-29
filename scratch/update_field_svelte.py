with open('ui/src/lib/page-kit/Field.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

target = """  const isInlineControl = $derived(c.field?.control === "switch" || c.field?.control === "checkbox");
</script>

{#if shown && c.field}
  <div
    class={boxClass(p, c, [
      "bos-cell bos-field",
      isHorizontal ? "is-horizontal" : "",
      isLabelRight ? "is-label-right" : "",
      isInlineControl ? "is-inline-control" : ""
    ].filter(Boolean).join(" "))}
    style={boxStyle(c, defaults)}
    title={c.tooltip ?? undefined}
    data-field={c.name}
    data-key={c.field?.key}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    {#if showCaption}
      <label class="bos-caption" for={id}>
        <span class="truncate">{caption}</span>
        {#if required}<span class="text-[var(--bos-color-unfavorable)]" aria-hidden="true">*</span>{/if}
      </label>
    {/if}
    <FieldInput {p} {c} {id} label={caption} />
    {#if c.instructional_text}
      <p class="bos-hint">{c.instructional_text}</p>
    {/if}
  </div>
{/if}"""

replacement = """  const isInlineControl = $derived(c.field?.control === "switch" || c.field?.control === "checkbox");
  const errorText = $derived(
    p.fieldErrors[c.field?.key] ??
    p.fieldErrors[c.field?.source] ??
    p.fieldErrors[c.name] ??
    ""
  );
</script>

{#if shown && c.field}
  <div
    class={boxClass(p, c, [
      "bos-cell bos-field",
      isHorizontal ? "is-horizontal" : "",
      isLabelRight ? "is-label-right" : "",
      isInlineControl ? "is-inline-control" : "",
      errorText ? "has-error border-rose-500/70 dark:border-rose-500/80" : ""
    ].filter(Boolean).join(" "))}
    style={boxStyle(c, defaults)}
    title={c.tooltip ?? undefined}
    data-field={c.name}
    data-key={c.field?.key}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    {#if showCaption}
      <label class="bos-caption" for={id}>
        <span class="truncate">{caption}</span>
        {#if required}<span class="text-[var(--bos-color-unfavorable)]" aria-hidden="true">*</span>{/if}
      </label>
    {/if}
    <FieldInput {p} {c} {id} label={caption} />
    {#if errorText}
      <p class="bos-field-error text-[11px] font-medium text-rose-600 dark:text-rose-400 mt-1 flex items-center gap-1 animate-in fade-in-50 duration-150">
        <span aria-hidden="true">⚠</span>
        <span>{errorText}</span>
      </p>
    {/if}
    {#if c.instructional_text}
      <p class="bos-hint">{c.instructional_text}</p>
    {/if}
  </div>
{/if}"""

target_crlf = target.replace('\n', '\r\n')
replacement_crlf = replacement.replace('\n', '\r\n')

if target in content:
    content = content.replace(target, replacement, 1)
    print("Field.svelte updated (LF)")
elif target_crlf in content:
    content = content.replace(target_crlf, replacement_crlf, 1)
    print("Field.svelte updated (CRLF)")
else:
    print("Target not found in Field.svelte")

with open('ui/src/lib/page-kit/Field.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
