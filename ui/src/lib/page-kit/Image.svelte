<script>
  import { untrack } from "svelte";
  /** Static image (`image(Logo) { src = "/assets/logo.png"; }`) or the value of a Blob/URL field. */
  import { nodeAttrs, isShown, isHidden, spanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const defaults = untrack(() => spanDefaults(p));
  const shown = $derived(isShown(p, c, visible));
  const src = $derived.by(() => {
    const s = String((c.field ? p.valueOf(c.field) : c.properties?.src) ?? "");
    return /^(https?:\/\/|\/|data:image\/(png|jpe?g|gif|webp|svg\+xml);base64,)/i.test(s) ? s : "";
  });
  const height = $derived({ sm: "4rem", md: "8rem", lg: "14rem" }[c.properties?.size ?? "md"] ?? "8rem");
</script>

{#if shown}
  <figure class={boxClass(p, c, "bos-cell bos-image")} style={boxStyle(c, defaults) + `;--bos-img-h:${height}`} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if src}
      <img {src} alt={c.caption ?? c.name} loading="lazy" />
    {:else}
      <div class="bos-image-empty" role="img" aria-label={c.caption ?? c.name}>No image</div>
    {/if}
    {#if c.caption && c.properties?.showCaption !== "false"}<figcaption class="bos-hint">{c.caption}</figcaption>{/if}
  </figure>
{/if}
