<script>
  import { loadIcon } from "./icons.js";
  import { colorValue } from "./style.js";

  let {
    name = "",
    class: className = "size-4",
    size = null,
    color = null,
    background = null,
    radius = null,
    style = ""
  } = $props();

  let Component = $state(null);

  const RADIUS_MAP = { none: "0px", sm: "4px", md: "8px", lg: "12px", full: "9999px" };
  const SIZE_MAP = { xs: "0.875rem", sm: "1rem", md: "1.25rem", lg: "1.5rem", xl: "2rem", "2xl": "2.5rem" };

  const styleString = $derived.by(() => {
    const parts = [];
    if (size) {
      const isz = String(size).trim();
      const val = SIZE_MAP[isz] ?? (isz.match(/^\d+$/) ? `${isz}px` : isz);
      parts.push(`width: ${val}`, `height: ${val}`, `min-width: ${val}`);
    }
    if (color) {
      const col = colorValue(color) ?? color;
      parts.push(`color: ${col}`);
    }
    if (background) {
      const bg = colorValue(background) ?? background;
      parts.push(`background: ${bg}`, `padding: 0.25rem`);
    }
    if (radius) {
      parts.push(`border-radius: ${RADIUS_MAP[radius] ?? radius}`);
    }
    if (style) parts.push(style);
    return parts.join("; ");
  });

  function toKebab(str) {
    if (!str) return "";
    return String(str)
      .trim()
      .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
      .replace(/_/g, "-")
      .toLowerCase();
  }

  $effect(() => {
    const n = toKebab(name);
    let alive = true;
    if (!/^[a-z0-9-]+$/.test(n)) {
      Component = null;
      return;
    }
    loadIcon(n).then((c) => {
      if (alive) Component = c;
    });
    return () => {
      alive = false;
    };
  });
</script>

{#if Component}
  <Component
    class={className}
    style={styleString || undefined}
    aria-hidden="true"
    data-bos-icon={name}
  />
{:else if name}
  <span
    class={"inline-block " + className}
    style={styleString || undefined}
    aria-hidden="true"
  ></span>
{/if}
