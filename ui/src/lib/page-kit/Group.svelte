<script>
  import { untrack } from "svelte";
  /** NAV FastTab: a collapsible section with a 12-column field grid and Promoted/Additional fields. */
  import { ChevronDown } from "lucide-svelte";
  import Icon from "./Icon.svelte";
  import { formatValue } from "./format.js";
  import { nodeAttrs, isShown, isHidden, group, setGroup, setSpanDefaults, spanDefaults, boxStyle, boxClass } from "./node.js";

  let { p, c, visible = undefined, children } = $props();

  const parent = group();
  const nested = Boolean(parent);
  const defaults = untrack(() => spanDefaults(p));
  if (nested) setSpanDefaults(defaults);

  const inside = $derived(p.C.filter((x) => x.path.startsWith(c.path + "/")));
  const hasAdditional = $derived(inside.some((x) => x.responsive?.importance === "Additional"));
  const promoted = $derived(inside.filter((x) => x.field && x.responsive?.importance === "Promoted"));

  const state = $state({ open: untrack(() => c.properties?.collapsed !== "true"), showMore: false });
  setGroup(state);

  const shown = $derived(isShown(p, c, visible));
  const summary = $derived(
    promoted
      .map((x) => formatValue(x.field, p.valueOf(x.field)))
      .filter(Boolean)
      .join(" · ")
  );
  const bodyId = $derived(`bos-g-${p.meta.name}-${c.i}`);
</script>

{#if shown}
  {#if nested || c.properties?.showCaption === "false" || c.properties?.header === "false" || !c.caption}
    <section class={boxClass(p, c, "bos-subgroup")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      {#if c.caption && c.properties?.showCaption !== "false"}<h4 class="bos-subgroup-title">{c.caption}</h4>{/if}
      <div class="bos-fields">{@render children?.()}</div>
    </section>
  {:else}
    <section class={boxClass(p, c, "bos-fasttab")} style={boxStyle(c)} {...nodeAttrs(p, c, isHidden(p, c, visible))}>
      <button type="button" class="bos-fasttab-header" aria-expanded={state.open} aria-controls={bodyId} onclick={() => (state.open = !state.open)}>
        <ChevronDown class={"size-4 shrink-0 transition-transform " + (state.open ? "" : "-rotate-90")} />
        {#if c.style?.icon}
          <Icon
            name={c.style.icon}
            class="shrink-0"
            size={c.style.iconSize}
            color={c.style.iconColor}
            background={c.style.iconBackground}
            radius={c.style.iconRadius}
          />
        {/if}
        <span class="font-semibold">{c.caption ?? c.name}</span>
        {#if !state.open && summary}
          <span class="ml-auto truncate text-xs font-normal text-[var(--bos-color-subordinate)]">{summary}</span>
        {/if}
      </button>
      {#if state.open}
        <div id={bodyId} class="bos-fasttab-body">
          {#if c.instructional_text}<p class="bos-hint mb-3">{c.instructional_text}</p>{/if}
          <div class="bos-fields">{@render children?.()}</div>
          {#if hasAdditional && !p.design}
            <button type="button" class="mt-3 text-xs font-medium text-[var(--bos-color-primary)] hover:underline" onclick={() => (state.showMore = !state.showMore)}>
              {state.showMore ? "Show less" : "Show more"}
            </button>
          {/if}
        </div>
      {/if}
    </section>
  {/if}
{/if}
