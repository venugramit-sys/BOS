<script>
  import { loadIcon } from "./icons.js";

  let { name = "", class: className = "size-4" } = $props();

  let Component = $state(null);

  $effect(() => {
    const n = String(name ?? "").trim().toLowerCase();
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
  <Component class={className} aria-hidden="true" />
{:else if name}
  <span class={"inline-block " + className} aria-hidden="true"></span>
{/if}
