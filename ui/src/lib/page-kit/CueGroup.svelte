<script>
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <section class={boxClass(p, c, "bos-cuegroup")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption}<h3 class="bos-section-title">{c.caption}</h3>{/if}
    <div class="bos-cues">{@render children?.()}</div>
  </section>
{/if}
