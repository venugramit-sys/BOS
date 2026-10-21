<script>
  import { Tabs as TabBar } from "$lib/bos-ui/tabs/index.js";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  /** @type {{ p: any, c: any, visible?: boolean, tabs: { c: any, body: import('svelte').Snippet, visible?: boolean }[] }} */
  let { p, c, visible = undefined, tabs = [] } = $props();

  const shown = $derived(isShown(p, c, visible));
  const available = $derived(tabs.filter((t) => isShown(p, t.c, t.visible)));
  let active = $state("");
  const current = $derived(available.find((t) => t.c.name === active) ?? available[0] ?? null);
  const items = $derived(available.map((t) => ({ id: t.c.name, label: t.c.caption ?? t.c.name })));
</script>

{#if shown && available.length}
  <section class={boxClass(p, c, "bos-tabs")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    <div class="overflow-x-auto">
      <TabBar variant="underline" items={items} value={current?.c.name ?? ""} onValueChange={(v) => (active = v)} />
    </div>
    {#if current}
      <div class="bos-tab-panel" role="tabpanel" {...nodeAttrs(p, current.c, isHidden(p, current.c, current.visible))}>
        <div class="bos-fields">{@render current.body()}</div>
      </div>
    {/if}
  </section>
{/if}
