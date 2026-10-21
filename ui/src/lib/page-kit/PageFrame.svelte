<script>
  /**
   * Outer frame of every compiled page: header (caption, record caption, status), action bar,
   * error / concurrency banners, body (content + FactBoxes), dialog footers and the confirm prompt.
   */
  import { Banner } from "$lib/bos-ui/banner/index.js";
  import { Button } from "$lib/bos-ui/button/index.js";
  import { Dialog } from "$lib/bos-ui/dialog/index.js";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import Icon from "./Icon.svelte";
  import ActionBar from "./ActionBar.svelte";
  import { styleVars } from "./style.js";

  let { p, actions = [], children } = $props();

  const DIALOGS = new Set(["StandardDialog", "ConfirmationDialog", "NavigatePage"]);
  const dialog = $derived(DIALOGS.has(p.meta.page_type));
  const byName = $derived(new Map(actions.map((a) => [a.name.toLowerCase(), a])));
  const targetOf = (a) => (a.target ? (byName.get(a.target.toLowerCase()) ?? a) : a);

  const footer = $derived(actions.filter((a) => a.in_footer_bar && p.actionVisible(targetOf(a))));
  const lastStep = $derived(p.meta.page_type !== "NavigatePage" || p.stepCount === 0 || p.step >= p.stepCount - 1);
  const pageStyle = $derived(styleVars(p.meta.style));
  const status = $derived(dialog ? "" : p.mode === "create" ? "New" : p.dirty ? "Unsaved changes" : "");

  function onKey(e) {
    if (p.design) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      p.save();
    }
  }

  async function runFooter(a) {
    await p.runAction(a, actions);
  }

  async function confirmDialog() {
    if (await p.save()) p.ctx.closePage?.({ result: "ok" });
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class={["bos-page", `bos-page-${p.meta.page_type}`, p.embedded ? "is-embedded" : "", dialog ? "is-dialog" : "", pageStyle ? "bos-styled" : ""].join(" ")}
  style={pageStyle}
  data-page={p.meta.name}
  role="region"
  aria-label={p.caption}
  aria-busy={p.loading}
  onkeydown={onKey}
  {...p.design ? { "data-bos-node": "page", "data-bos-kind": "page" } : {}}
>
  {#if !p.embedded || p.isList}
    <header class="bos-page-header">
      {#if !p.embedded && !p.ctx.modal}
        <div class="min-w-0">
          <h1 class="bos-page-title">
            {#if p.meta.style?.icon}<Icon name={p.meta.style.icon} class="size-5 shrink-0" />{/if}
            <span class="truncate">{p.caption}</span>
            {#if p.dataCaption}<span class="bos-page-subtitle">{p.dataCaption}</span>{/if}
          </h1>
          {#if p.meta.instructional_text}<p class="bos-hint">{p.meta.instructional_text}</p>{/if}
        </div>
        {#if status}<span class="bos-status">{status}</span>{/if}
      {/if}
      {#if !dialog}<ActionBar {p} {actions} />{/if}
    </header>
  {/if}

  {#if p.conflict}
    <Banner tone="warning" title="This record was changed by someone else">
      <p class="text-xs">{p.conflict}</p>
      <div class="mt-2"><Button size="sm" variant="secondary" onclick={() => p.reload()}>Reload the latest version</Button></div>
    </Banner>
  {:else if p.error}
    <Banner tone="danger" title="Something went wrong" dismissible onDismiss={() => (p.error = "")}>
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
            <Button variant={a.primary ? "primary" : "secondary"} disabled={!p.actionEnabled(targetOf(a)) || p.design} loading={p.busyAction === targetOf(a).name} onclick={() => runFooter(a)}>
              {a.caption}
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
