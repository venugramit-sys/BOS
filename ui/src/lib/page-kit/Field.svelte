<script>
  import { untrack } from "svelte";
  import FieldInput from "./FieldInput.svelte";
  import Icon from "./Icon.svelte";
  import { nodeAttrs, isShown, isHidden, spanDefaults, group, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const defaults = untrack(() => spanDefaults(p));
  const fastTab = group();
  const id = $derived(`bos-f-${p.meta.name}-${c.i}`);

  const additional = $derived(c.responsive?.importance === "Additional");
  const shown = $derived(isShown(p, c, visible) && (p.design || !additional || !fastTab || fastTab.showMore));
  const required = $derived(c.field?.required && (p.design || p.editable(c)));
  const caption = $derived(c.caption ?? c.name);
  const labelPosition = $derived(c.properties?.labelPosition ?? "top");
  const showCaption = $derived(c.properties?.showCaption !== "false" && labelPosition !== "none");
  const isHorizontal = $derived(labelPosition === "left" || labelPosition === "right");
  const isLabelRight = $derived(labelPosition === "right");
  const isInlineControl = $derived(c.field?.control === "switch" || c.field?.control === "checkbox");
  const errorText = $derived(
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
    {#if errorText && !controlHandlesError}
      <p class="bos-field-error text-[11px] font-medium text-rose-600 dark:text-rose-400 mt-1 flex items-center gap-1 animate-in fade-in-50 duration-150">
        <span aria-hidden="true">⚠</span>
        <span>{errorText}</span>
      </p>
    {/if}
    {#if c.instructional_text}
      <p class="bos-hint">{c.instructional_text}</p>
    {/if}
  </div>
{/if}
