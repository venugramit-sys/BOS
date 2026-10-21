<script>
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-divider")} style={boxStyle(c)} role="separator" {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption}<span>{c.caption}</span>{/if}
  </div>
{/if}
