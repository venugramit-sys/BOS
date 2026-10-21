<script>
  /** FactBox pane: beside the content from `lg` up, below it on smaller screens. */
  import { nodeAttrs, boxStyle, boxClass } from "./node.js";

  let { p, c, children } = $props();
</script>

<aside class={boxClass(p, c, "bos-factboxes")} style={boxStyle(c)} aria-label="FactBoxes" {...nodeAttrs(p, c)}>
  {@render children?.()}
</aside>
