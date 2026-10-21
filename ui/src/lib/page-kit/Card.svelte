<script>
  import { untrack } from "svelte";
  import Icon from "./Icon.svelte";
  import { nodeAttrs, isShown, isHidden, spanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const outer = untrack(() => spanDefaults(p));
  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-cell bos-card")} style={boxStyle(c, { ...outer, md: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption}
      <h4 class="bos-subgroup-title">
        {#if c.style?.icon}<Icon name={c.style.icon} class="size-4" />{/if}
        {c.caption}
      </h4>
    {/if}
    <div class="bos-fields">{@render children?.()}</div>
  </div>
{/if}
