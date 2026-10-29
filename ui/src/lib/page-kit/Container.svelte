<script>
  /** Generic container for control kinds and semantic HTML layout wrappers. */
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const shown = $derived(isShown(p, c, visible));
  const tag = $derived((c.properties?.tag || c.kind || "div").toLowerCase());
</script>

{#if shown}
  {#if tag === "section"}
    <section class={boxClass(p, c, "bos-cell bos-container bos-section")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </section>
  {:else if tag === "nav"}
    <nav class={boxClass(p, c, "bos-cell bos-container bos-nav")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </nav>
  {:else if tag === "footer"}
    <footer class={boxClass(p, c, "bos-cell bos-container bos-footer")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </footer>
  {:else if tag === "article"}
    <article class={boxClass(p, c, "bos-cell bos-container bos-article")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </article>
  {:else if tag === "aside"}
    <aside class={boxClass(p, c, "bos-cell bos-container bos-aside")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </aside>
  {:else if tag === "header"}
    <header class={boxClass(p, c, "bos-cell bos-container bos-header")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </header>
  {:else}
    <div class={boxClass(p, c, "bos-cell bos-container")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </div>
  {/if}
{/if}
