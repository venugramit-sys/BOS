<script>
  /**
   * Action.svelte
   * Inline action button control in page layouts.
   * Renders an interactive button wired directly to PageController triggers, codeunits, or page navigation.
   */
  import { Button } from "$lib/bos-ui/button/index.js";
  import Icon from "./Icon.svelte";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
  const busy = $derived(p.busyAction === c.name);
  const disabled = $derived(!p.actionEnabled(c) || p.design || c.properties?.disabled === "true" || c.properties?.enabled === "false");
  const primary = $derived(c.properties?.primary === "true" || c.properties?.variant === "primary");
  const variant = $derived(c.properties?.variant ?? (primary ? "primary" : "secondary"));
  const icon = $derived(c.icon || c.style?.icon || c.properties?.icon);
  const caption = $derived(c.caption || c.name);

  async function handleClick(e) {
    if (p.design) return;
    e.stopPropagation();
    await p.runAction(c);
  }
</script>

{#if shown}
  <div
    class={boxClass(p, c, "bos-cell bos-action-wrap")}
    style={boxStyle(c, { xs: 12 })}
    {...nodeAttrs(p, c, isHidden(p, c, visible))}
  >
    <Button
      {variant}
      {disabled}
      loading={busy}
      onclick={handleClick}
      aria-label={caption}
      class="w-full sm:w-auto"
    >
      {#if icon && !busy}
        <Icon name={icon} class="mr-1.5 size-4" />
      {/if}
      <span>{caption}</span>
    </Button>
  </div>
{/if}
