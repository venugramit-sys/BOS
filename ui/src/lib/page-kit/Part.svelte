<script>
  /**
   * `part(Name; SubPage)`: embeds another compiled page filtered by its `link` (NAV SubPageLink).
   * The sub page shows nothing until every link value is known (e.g. the header is saved).
   */
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { loadPageComponent, pageManifest } from "./loader.svelte.js";
  import { evaluateLinks, linksComplete } from "./record-model.js";
  import { nodeAttrs, isShown, isHidden, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined } = $props();

  const shown = $derived(isShown(p, c, visible));
  const inFactBox = $derived(c.path.toLowerCase().split("/")[0] === "area:factboxes");
  const links = $derived(p.x[c.i]?.links ?? []);
  const filterValues = $derived(evaluateLinks(links, p.rec, p.vars));
  const filterKey = $derived(JSON.stringify(filterValues));
  const ready = $derived(links.length === 0 || linksComplete(filterValues));
  const filters = $derived(JSON.parse(filterKey));

  let Component = $state(null);
  let error = $state("");

  $effect(() => {
    if (!shown || !c.part?.page) return;
    // Re-import when the sub page is republished (manifest version bump).
    pageManifest.version;
    let alive = true;
    error = "";
    loadPageComponent(c.part.page)
      .then(({ component }) => {
        if (alive) Component = component;
      })
      .catch((e) => {
        if (alive) error = e?.message ?? String(e);
      });
    return () => {
      alive = false;
    };
  });

  // Update propagation (NAV UpdatePropagation=Both): a change in the part refreshes the parent record.
  function onChanged() {
    if (c.part?.update_propagation && !p.isList) p.reload();
  }

  const childCtx = $derived({
    page: c.part?.page,
    embedded: true,
    design: p.design,
    filters,
    // FactBoxes are read-only (NAV); other parts follow the host page's mode.
    mode: inFactBox || p.mode === "view" ? "view" : "edit",
    openPage: p.ctx.openPage,
    navigate: p.ctx.navigate,
    onChanged
  });
</script>

{#if shown}
  <section class={boxClass(p, c, "bos-part")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
    {#if c.caption && c.properties?.showCaption !== "false"}<h3 class="bos-part-title">{c.caption}</h3>{/if}
    {#if error}
      <div class="bos-notice bos-notice-error" role="alert">{error}</div>
    {:else if !ready && !p.design}
      <div class="bos-notice" role="status">{p.mode === "create" ? "Save the record to add lines." : "No related records."}</div>
    {:else if Component}
      <Component ctx={childCtx} />
    {:else}
      <div class="flex justify-center py-6"><Spinner /></div>
    {/if}
  </section>
{/if}
