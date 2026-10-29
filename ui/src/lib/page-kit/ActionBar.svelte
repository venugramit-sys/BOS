<script>
  /**
   * NAV action bar: system actions (New / Save / Discard / Delete / Refresh), promoted actions as
   * buttons, every other action area as a menu. On phones promoted actions fold into "More".
   */
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import Icon from "./Icon.svelte";
  import ActionMenu from "./ActionMenu.svelte";

  let { p, actions = [] } = $props();

  const AREAS = ["Processing", "Navigation", "Reporting", "Creation"];
  const AREA_CAPTION = { Processing: "Actions", Navigation: "Related", Reporting: "Reports", Creation: "New document" };
  const AREA_ICON = { Processing: "play", Navigation: "link-2", Reporting: "file-text", Creation: "plus-circle" };

  const byName = $derived(new Map(actions.map((a) => [a.name.toLowerCase(), a])));
  const targetOf = (a) => (a.target ? (byName.get(a.target.toLowerCase()) ?? a) : a);
  const groupCaption = (name) => p.meta.action_groups?.find((g) => g.name === name)?.caption ?? name;

  const visibleActions = $derived(actions.filter((a) => !a.in_footer_bar && p.actionVisible(targetOf(a))));
  const promoted = $derived(visibleActions.filter((a) => a.area === "Promoted" || a.primary || a.is_default || a.promoted_only));
  const menus = $derived(
    AREAS.map((area) => {
      const items = visibleActions.filter((a) => a.area === area && !a.promoted_only);
      const groups = [];
      for (const a of items) {
        const caption = a.group ? groupCaption(a.group) : "";
        let g = groups.find((x) => x.caption === caption);
        if (!g) groups.push((g = { caption, items: [] }));
        g.items.push(a);
      }
      return { area, label: AREA_CAPTION[area], icon: AREA_ICON[area], sections: groups };
    }).filter((m) => m.sections.length)
  );

  const editing = $derived(!p.isList && p.mode !== "view");
  const hasRecord = $derived(p.isList ? Boolean(p.current && !p.current._new) : Boolean(p.original?._id));
  const system = $derived(
    [
      p.meta.insert_allowed && { name: "__new", caption: p.isList && p.inlineEdit && p.embedded ? "New line" : "New", icon: "plus", run: () => p.newRecord() },
      (editing || (p.isList && p.dirty)) && {
        name: "__save",
        caption: "Save",
        icon: "save",
        run: () => p.save(),
        enabled: () => (p.isNew ? !p.saving : p.dirty && !p.saving)
      },
      p.dirty && hasRecord && { name: "__discard", caption: "Discard", icon: "undo-2", run: () => p.reload() },
      p.meta.delete_allowed && hasRecord && { name: "__delete", caption: "Delete", icon: "trash-2", run: () => p.deleteCurrent() },
      { name: "__refresh", caption: "Refresh", icon: "refresh-cw", run: () => p.reload(), enabled: () => !p.loading }
    ].filter(Boolean)
  );

  const isSystem = (a) => typeof a.run === "function";
  // Design builds: the canvas intercepts clicks (selection), so nothing here ever runs.
  const enabled = (a) => (p.design ? false : isSystem(a) ? (a.enabled ? a.enabled() : true) : p.actionEnabled(targetOf(a)));
  const run = (a) => (isSystem(a) ? a.run() : p.runAction(a, actions));
  const showSystem = $derived(!["StandardDialog", "ConfirmationDialog", "NavigatePage", "RoleCenter"].includes(p.meta.page_type));
</script>

<div class="bos-actionbar" role="toolbar" aria-label="Page actions">
  {#if showSystem}
    {#each system as a (a.name)}
      <button type="button" class="bos-action" data-bos-action={a.name.replace('__', '')} disabled={!enabled(a)} onclick={() => run(a)}>
        {#if a.name === "__save" && p.saving}<Spinner size="xs" />{:else}<Icon name={a.icon} class="size-4" />{/if}
        <span class={a.name === "__refresh" ? "sr-only" : "bos-action-label"}>{a.caption}</span>
      </button>
    {/each}
    {#if promoted.length || menus.length}<span class="bos-action-sep" aria-hidden="true"></span>{/if}
  {/if}
  {#each promoted as a (a.name)}
    {@const actionTarget = targetOf(a).name}
    <button
      type="button"
      class={"bos-action bos-promoted " + (a.primary ? "bos-action-primary" : "")}
      data-bos-action={actionTarget}
      {...p.design ? {
        "data-bos-node": "action:" + actionTarget,
        "data-bos-kind": "action",
        draggable: true,
        ondragstart: (e) => {
          e.dataTransfer?.setData("text/plain", actionTarget);
        },
        ondragover: (e) => {
          e.preventDefault();
        },
        ondrop: (e) => {
          e.preventDefault();
          const source = e.dataTransfer?.getData("text/plain");
          if (source && source !== actionTarget) {
            window.dispatchEvent(new CustomEvent("bos:reorder-action", {
              detail: { source, target: actionTarget }
            }));
          }
        }
      } : {}}
      disabled={!p.design && !enabled(a)}
      title={a.tooltip ?? a.shortcut ?? undefined}
      onclick={() => run(a)}
    >
      {#if p.busyAction === targetOf(a).name}<Spinner size="xs" />{:else if a.icon || targetOf(a).icon}<Icon name={a.icon ?? targetOf(a).icon} class="size-4" />{/if}
      <span>{a.caption}{a.ellipsis ? "…" : ""}</span>
    </button>
  {/each}
  {#if promoted.length}
    <ActionMenu class="bos-more" label="More" sections={[{ items: promoted }]} onrun={run} {enabled} />
  {/if}
  {#each menus as m (m.area)}
    <ActionMenu label={m.label} icon={m.icon} sections={m.sections} onrun={run} {enabled} />
  {/each}
  {#if p.design}
    <button
      type="button"
      data-bos-action-trigger="true"
      class="bos-action bos-action-designer-add inline-flex items-center gap-1 rounded border border-dashed border-indigo-400 bg-indigo-50/60 px-2 py-1 text-xs font-semibold text-indigo-700 hover:bg-indigo-100 dark:border-indigo-700 dark:bg-indigo-950/50 dark:text-indigo-300"
      title="Add new action or menu group to page"
      onclick={() => {
        if (typeof window !== "undefined") window.dispatchEvent(new CustomEvent("bos:open-action-builder"));
      }}
    >
      <Icon name="plus" class="size-3.5" />
      <span>Action</span>
    </button>
  {/if}
</div>
