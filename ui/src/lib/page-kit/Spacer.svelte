<script>
  import { nodeAttrs, isShown, isHidden } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const size = $derived({ sm: "0.5rem", md: "1rem", lg: "2rem", xl: "3rem" }[c.properties?.size ?? "md"] ?? "1rem");
  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class="bos-spacer" style={`--bos-spacer:${size}`} aria-hidden="true" {...nodeAttrs(p, c, isHidden(p, c, visible))}></div>
{/if}
