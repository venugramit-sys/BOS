<script>
  /** Generic container for control kinds without a dedicated layout. */
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-cell bos-container")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
    <div class="bos-fields">{@render children?.()}</div>
  </div>
{/if}
