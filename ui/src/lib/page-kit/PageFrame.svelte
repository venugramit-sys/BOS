<script>
  /**
   * Outer frame of every compiled page: header (caption, record caption, status), action bar,
   * error / concurrency banners, body (content + FactBoxes), dialog footers and the confirm prompt.
   */
  import { onMount } from "svelte";
  import { Banner } from "$lib/bos-ui/banner/index.js";
  import { Button } from "$lib/bos-ui/button/index.js";
  import { Dialog } from "$lib/bos-ui/dialog/index.js";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { ChevronLeft, ChevronRight } from "lucide-svelte";
  import Icon from "./Icon.svelte";
  import ActionBar from "./ActionBar.svelte";
  import { styleVars } from "./style.js";
  import { focusFirstEditable, focusFieldError, handleEnterpriseFormKeyDown } from "./focus.js";

  let { p, actions = [], children } = $props();

  let frameEl = $state(null);

  const DIALOGS = new Set(["StandardDialog", "ConfirmationDialog", "NavigatePage"]);
  const dialog = $derived(DIALOGS.has(p.meta.page_type));
  const byName = $derived(new Map(actions.map((a) => [a.name.toLowerCase(), a])));
  const targetOf = (a) => (a.target ? (byName.get(a.target.toLowerCase()) ?? a) : a);

  const footer = $derived(actions.filter((a) => a.in_footer_bar && p.actionVisible(targetOf(a))));
  const lastStep = $derived(p.meta.page_type !== "NavigatePage" || p.stepCount === 0 || p.step >= p.stepCount - 1);
  const pageStyle = $derived(styleVars(p.meta.style));
  const status = $derived(dialog ? "" : p.mode === "create" ? "New" : p.dirty ? "Unsaved changes" : "");
  const showHeader = $derived(
    (!p.embedded || p.isList) &&
    p.meta.properties?.showHeader !== "false" &&
    p.meta.properties?.show_header !== "false"
  );
  const showActionBar = $derived(
    !dialog &&
    p.meta.properties?.showActionBar !== "false" &&
    p.meta.properties?.show_action_bar !== "false"
  );

  onMount(() => {
    if (!p.design) {
      const timer = setTimeout(() => {
        if (frameEl) focusFirstEditable(frameEl);
      }, 50);
      return () => clearTimeout(timer);
    }
  });

  $effect(() => {
    if (!p.design && p.loaded && frameEl) {
      const active = document.activeElement;
      if (!active || !frameEl.contains(active) || active === document.body) {
        focusFirstEditable(frameEl);
      }
    }
  });

  $effect(() => {
    // When field validation errors occur, auto-focus the first invalid field
    const errKeys = Object.keys(p.fieldErrors || {});
    if (errKeys.length > 0 && frameEl && !p.design) {
      focusFieldError(frameEl, errKeys[0]);
    }
  });

  function onKey(e) {
    if (p.design) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      p.save();
      return;
    }

    if (e.key === "PageUp" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      p.prevRecord();
      return;
    }

    if (e.key === "PageDown" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      p.nextRecord();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      if (p.meta?.insert_allowed && !dialog) {
        e.preventDefault();
        p.newRecord();
        return;
      }
    }

    if (e.key === "F5") {
      e.preventDefault();
      p.reload();
      return;
    }

    handleEnterpriseFormKeyDown(e, frameEl, {
      onPrimary: () => {
        const primaryAction = footer.find((a) => a.primary) || actions.find((a) => a.primary);
        if (primaryAction) {
          runFooter(primaryAction);
        } else if (dialog && p.meta.page_type === "StandardDialog") {
          confirmDialog();
        }
      }
    });
  }

  async function runFooter(a) {
    await p.runAction(a, actions);
  }

  async function confirmDialog() {
    const primary = footer.find((a) => a.primary) || actions.find((a) => a.primary);
    if (primary) {
      await runFooter(primary);
      return;
    }
    if (await p.save()) p.ctx.closePage?.({ result: "ok" });
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={frameEl}
  class={["bos-page", `bos-page-${p.meta.page_type}`, p.embedded ? "is-embedded" : "", dialog ? "is-dialog" : "", pageStyle ? "bos-styled" : ""].join(" ")}
  style={pageStyle}
  data-page={p.meta.name}
  data-bos-page={p.meta.name}
  data-bos-pagetype={p.meta.page_type}
  role="region"
  aria-label={p.caption}
  aria-busy={p.loading}
  onkeydown={onKey}
  {...p.design ? { "data-bos-node": "page", "data-bos-kind": "page" } : {}}
>
  {#if showHeader}
    <header class="bos-page-header" {...p.design ? { "data-bos-node": "header", "data-bos-kind": "header" } : {}}>
      {#if !p.embedded && !p.ctx.modal}
        <div class="flex items-center justify-between gap-3 min-w-0 flex-1" {...p.design ? { "data-bos-node": "header", "data-bos-kind": "header" } : {}}>
          <div class="min-w-0">
            <h1 class="bos-page-title" style={pageStyle} {...p.design ? { "data-bos-node": "header:title", "data-bos-kind": "headline", title: "Click to select Page Title & Typography" } : {}}>
              {#if p.meta.style?.icon}
                <Icon
                  name={p.meta.style.icon}
                  class="shrink-0"
                  size={p.meta.style.iconSize ?? "md"}
                  color={p.meta.style.iconColor}
                  background={p.meta.style.iconBackground}
                  radius={p.meta.style.iconRadius}
                />
              {/if}
              <span class="truncate">{p.caption}</span>
              {#if p.dataCaption}<span class="bos-page-subtitle">{p.dataCaption}</span>{/if}
            </h1>
            {#if p.meta.instructional_text}
              <p class="bos-hint" {...p.design ? { "data-bos-node": "header:instruction", "data-bos-kind": "label", title: "Click to select Subtitle / Instructional Text" } : {}}>{p.meta.instructional_text}</p>
            {/if}
          </div>

          <div class="flex items-center gap-2 shrink-0">
            {#if p.canNavigateRecords}
              <div class="bos-record-stepper inline-flex items-center rounded-md border border-[var(--bos-color-border)] bg-[var(--bos-color-surface)] shadow-2xs select-none">
                <button
                  type="button"
                  class="p-1.5 rounded-l-md hover:bg-[var(--bos-color-surfaceMuted)] disabled:opacity-30 disabled:pointer-events-none cursor-pointer text-slate-600 dark:text-slate-300 transition-colors"
                  disabled={!p.hasPrevRecord || p.stepping}
                  onclick={() => p.prevRecord()}
                  title="Previous record (Ctrl+PageUp)"
                  aria-label="Previous record"
                >
                  <ChevronLeft class="size-3.5" />
                </button>
                <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400 px-2 min-w-[3.5rem] text-center font-mono">
                  {#if p.stepping}
                    <Spinner size="xs" />
                  {:else}
                    {p.recordPositionText}
                  {/if}
                </span>
                <button
                  type="button"
                  class="p-1.5 rounded-r-md hover:bg-[var(--bos-color-surfaceMuted)] disabled:opacity-30 disabled:pointer-events-none cursor-pointer text-slate-600 dark:text-slate-300 transition-colors"
                  disabled={!p.hasNextRecord || p.stepping}
                  onclick={() => p.nextRecord()}
                  title="Next record (Ctrl+PageDown)"
                  aria-label="Next record"
                >
                  <ChevronRight class="size-3.5" />
                </button>
              </div>
            {/if}
            {#if status}<span class="bos-status">{status}</span>{/if}
          </div>
        </div>
      {/if}
      {#if showActionBar}<ActionBar {p} {actions} />{/if}
    </header>
  {/if}

  {#if p.conflict}
    <Banner tone="warning" title="This record was changed by someone else">
      <p class="text-xs">{p.conflict}</p>
      <div class="mt-2"><Button size="sm" variant="secondary" onclick={() => p.reload()}>Reload the latest version</Button></div>
    </Banner>
  {:else if p.error}
    <Banner
      tone="danger"
      title={p.meta?.page_type === "StandardDialog" && p.meta?.name?.toLowerCase().includes("login") ? "Sign In Failed" : (p.errorTitle || "Action Failed")}
      dismissible
      onDismiss={() => { p.error = ""; p.errorTitle = ""; }}
    >
      <p class="text-xs">{p.error}</p>
    </Banner>
  {/if}

  <div class="bos-page-body">
    {#if p.loading && !p.loaded}
      <div class="flex justify-center py-12"><Spinner label="Loading page" /></div>
    {:else}
      {@render children?.()}
    {/if}
  </div>

  {#if dialog && !p.embedded}
    <footer class="bos-page-footer">
      {#if p.meta.page_type === "StandardDialog" && footer.length === 0}
        <Button variant="secondary" onclick={() => p.ctx.closePage?.({ result: "cancel" })}>Cancel</Button>
        <Button onclick={confirmDialog} loading={p.saving}>OK</Button>
      {:else if p.meta.page_type === "ConfirmationDialog" && footer.length === 0}
        <Button variant="secondary" onclick={() => p.ctx.closePage?.({ result: "no" })}>No</Button>
        <Button onclick={() => p.ctx.closePage?.({ result: "yes" })}>Yes</Button>
      {:else}
        {#if p.ctx.closePage}
          <Button variant="secondary" onclick={() => p.ctx.closePage({ result: "cancel" })}>Cancel</Button>
        {/if}
        {#if lastStep}
          {#each footer as a (a.name)}
            {@const target = targetOf(a)}
            {@const iconName = a.icon || target.icon || a.properties?.icon || target.properties?.icon}
            {@const actionTarget = target.name}
            <Button
              variant={a.primary ? "primary" : "secondary"}
              disabled={!p.actionEnabled(target) || p.design}
              loading={p.busyAction === target.name}
              onclick={() => runFooter(a)}
              {...p.design ? { "data-bos-node": "action:" + actionTarget, "data-bos-kind": "action" } : {}}
            >
              {#if iconName && p.busyAction !== target.name}
                <Icon name={iconName} class="mr-1.5 size-4" />
              {/if}
              <span>{a.caption}</span>
            </Button>
          {/each}
        {/if}
      {/if}
    </footer>
  {/if}
</div>

<Dialog open={p.prompt !== null} title="Please confirm" size="sm" onClose={() => p.prompt?.resolve(false)}>
  <p class="text-sm">{p.prompt?.question ?? ""}</p>
  <div class="mt-4 flex justify-end gap-2">
    <Button variant="secondary" onclick={() => p.prompt?.resolve(false)}>No</Button>
    <Button onclick={() => p.prompt?.resolve(true)}>Yes</Button>
  </div>
</Dialog>
