<script>
  /** `flex(Row|Column)`: controls side by side (wrapping) or stacked. */
  import { nodeAttrs, isShown, isHidden, setSpanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const column = $derived(String(c.properties?.direction ?? c.name).toLowerCase() === "column");
  const gap = $derived({ none: "0", sm: "0.5rem", md: "1rem", lg: "1.5rem" }[c.properties?.gap ?? "md"] ?? "1rem");
  setSpanDefaults({ xs: 12 });

  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div
    class={boxClass(p, c, "bos-cell bos-flex " + (column ? "bos-flex-col" : "bos-flex-row"))}
    style={boxStyle(c, { xs: 12 }) + `;--bos-gap:${gap}`}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    {@render children?.()}
  </div>
{/if}
