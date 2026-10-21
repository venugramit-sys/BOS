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
  const required = $derived(c.field?.required && p.editable(c));
  const caption = $derived(c.caption ?? c.name);
  const showCaption = $derived(c.properties?.showCaption !== "false");
</script>

{#if shown && c.field}
  <div
    class={boxClass(p, c, "bos-cell bos-field")}
    style={boxStyle(c, defaults)}
    title={c.tooltip ?? undefined}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    {#if showCaption}
      <label class="bos-caption" for={id}>
        {#if c.style?.icon}<Icon name={c.style.icon} class="size-3.5 shrink-0" />{/if}
        <span class="truncate">{caption}</span>
        {#if required}<span class="text-[var(--bos-color-unfavorable)]" aria-hidden="true">*</span>{/if}
      </label>
    {/if}
    <FieldInput {p} {c} {id} label={caption} />
    {#if c.instructional_text}
      <p class="bos-hint">{c.instructional_text}</p>
    {/if}
  </div>
{/if}
