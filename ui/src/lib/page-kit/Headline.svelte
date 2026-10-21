<script>
  /** Role Center headline: greeting plus optional instructional line. */
  import Icon from "./Icon.svelte";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
  const value = $derived(c.field ? p.valueOf(c.field) : null);
</script>

{#if shown}
  <header class={boxClass(p, c, "bos-headline")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.style?.icon}<Icon name={c.style.icon} class="size-6 shrink-0" />{/if}
    <div class="min-w-0">
      <h2>{c.caption ?? c.name}{#if value}<span> {value}</span>{/if}</h2>
      {#if c.instructional_text}<p>{c.instructional_text}</p>{/if}
    </div>
  </header>
{/if}
