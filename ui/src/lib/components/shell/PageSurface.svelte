<script>
  /**
   * Workspace tab → compiled page. Every BOS page in the app renders through `PageHost`
   * (`@bos/page-kit`); opening another page creates a tab `<Page>:<recordId|new>`.
   */
  import PageHost from "$lib/page-kit/PageHost.svelte";
  import { shell } from "./shellStore.svelte.js";

  /** @type {{ tab: { id: string, title: string, page?: string, params?: any } }} */
  let { tab } = $props();

  const page = $derived(tab.page ?? tab.id);
  const params = $derived(tab.params ?? {});

  function openPage(name, opts = {}) {
    const suffix = opts.recordId ? opts.recordId : opts.mode === "create" ? "new" : "";
    const filterKey = opts.filters && Object.keys(opts.filters).length ? JSON.stringify(opts.filters) : "";
    const id = [name, suffix, filterKey].filter(Boolean).join(":");
    shell.openTab({ id, title: name, type: "page", page: name, params: opts });
  }

  function closePage() {
    shell.closeTab(tab.id);
  }

  function onRecordChange(recordId) {
    // A new record got its id: keep the tab, re-key its params so a reload shows the saved record.
    shell.openTabs = shell.openTabs.map((t) => (t.id === tab.id ? { ...t, params: { ...(t.params ?? {}), recordId, mode: "edit" } } : t));
  }
</script>

{#key tab.id}
  <PageHost {page} {params} {openPage} {closePage} {onRecordChange} navigate={(url) => shell.navigate(url)} />
{/key}
