<script>
  import { untrack } from "svelte";
  /** `grid(...)` / `fixed(...)`: fields in `cols` equal columns (default 2) from `md` up. */
  import { nodeAttrs, isShown, isHidden, setSpanDefaults, spanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  // Layout context is fixed when the control mounts (controls of a compiled page never change shape).
  const outer = untrack(() => spanDefaults(p));
  const cols = untrack(() => Math.min(Math.max(Number.parseInt(c.properties?.cols ?? "2", 10) || 2, 1), 6));
  setSpanDefaults({ xs: 12, sm: cols > 1 ? 6 : 12, md: Math.max(1, Math.floor(12 / cols)) });

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-cell bos-grid-box")} style={boxStyle(c, { ...outer, md: 12, sm: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
    <div class="bos-fields">{@render children?.()}</div>
  </div>
{/if}
