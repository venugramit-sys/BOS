<script>
  /**
   * `systempart(Notes|Links|...)`. BOS has no built-in record notes/links store, so the part
   * states that plainly instead of rendering an empty or placeholder panel.
   */
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <section class={boxClass(p, c, "bos-part")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    <h3 class="bos-part-title">{c.caption ?? c.name}</h3>
    <div class="bos-notice" role="status">The system part "{c.properties?.system ?? c.name}" is not available on this BOS node.</div>
  </section>
{/if}
