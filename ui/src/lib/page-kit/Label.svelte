<script>
  import Icon from "./Icon.svelte";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-cell bos-label")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.style?.icon}<Icon name={c.style.icon} class="size-4 shrink-0" />{/if}
    <div>
      <p>{c.caption ?? ""}</p>
      {#if c.instructional_text}<p class="bos-hint">{c.instructional_text}</p>{/if}
    </div>
  </div>
{/if}
