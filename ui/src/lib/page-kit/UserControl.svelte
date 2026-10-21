<script>
  import { userControl } from "./user-controls.js";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const name = $derived(c.properties?.control ?? c.name);
  const Component = $derived(userControl(name));
  const shown = $derived(isShown(p, c, visible));
</script>

{#if shown}
  <div class={boxClass(p, c, "bos-cell bos-usercontrol")} style={boxStyle(c, { xs: 12 })} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if Component}
      <Component {p} {c} />
    {:else}
      <div class="bos-notice" role="status">User control "{name}" is not registered in this client.</div>
    {/if}
  </div>
{/if}
