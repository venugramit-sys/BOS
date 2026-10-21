<script>
  import { nodeAttrs, boxStyle, boxClass } from "./node.js";

  let { p, c, children } = $props();
</script>

<section class={boxClass(p, c, "bos-area")} style={boxStyle(c)} data-area={c.name} {...nodeAttrs(p, c)}>
  {@render children?.()}
</section>
