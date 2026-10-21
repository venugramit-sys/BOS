<script>
  /** Side-by-side panes from `md` up (`ratio = "1:2"`), stacked on phones. */
  import { nodeAttrs, isShown, isHidden, setSpanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, panes = [] } = $props();

  setSpanDefaults({ xs: 12 });

  const shown = $derived(isShown(p, c, visible));
  const available = $derived(panes.filter((t) => p.design || t.visible !== false));
  const columns = $derived.by(() => {
    const parts = String(c.properties?.ratio ?? "")
      .split(":")
      .map((n) => Number.parseInt(n, 10))
      .filter((n) => Number.isInteger(n) && n > 0 && n <= 12);
    const ratio = parts.length === available.length ? parts : available.map(() => 1);
    return ratio.map((n) => `minmax(0,${n}fr)`).join(" ");
  });
</script>

{#if shown && available.length}
  <div class={boxClass(p, c, "bos-split")} style={boxStyle(c) + `;--bos-split-cols:${columns}`} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#each available as pane (pane.c.i)}
      <div class="min-w-0">{@render pane.body()}</div>
    {/each}
  </div>
{/if}
