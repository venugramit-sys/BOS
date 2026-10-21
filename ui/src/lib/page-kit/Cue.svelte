<script>
  /** Role Center cue tile (NAV CueGroup field): a big number that drills down to its list. */
  import Icon from "./Icon.svelte";
  import { formatValue } from "./format.js";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
  const value = $derived(c.field ? p.valueOf(c.field) : null);
  const text = $derived(c.field ? formatValue({ ...c.field, blank_zero: false }, value ?? 0) || "0" : "");
  const target = $derived(c.field?.drill_down?.page ?? c.properties?.drillDownPage ?? null);

  function open() {
    if (target && !p.design) p.ctx.openPage?.(target, { mode: "view" });
  }
</script>

{#if shown}
  <button
    type="button"
    class={boxClass(p, c, "bos-cue")}
    style={boxStyle(c)}
    disabled={!target || p.design}
    onclick={open}
    title={c.tooltip ?? undefined}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    <span class="bos-cue-value">{p.loading && !p.loaded ? "…" : text}</span>
    <span class="bos-cue-caption">
      {#if c.style?.icon}<Icon name={c.style.icon} class="size-3.5" />{/if}
      {c.caption ?? c.name}
    </span>
  </button>
{/if}
