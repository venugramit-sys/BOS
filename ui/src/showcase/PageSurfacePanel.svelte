<script>
  /**
   * Showcase panel for compiled pages: pick any page of the connected node's manifest and render it
   * live through PageHost (real records, real triggers). No sample data.
   */
  import { onMount } from "svelte";
  import { Select } from "$lib/bos-ui/select/index.js";
  import { Banner } from "$lib/bos-ui/banner/index.js";
  import PageHost from "$lib/page-kit/PageHost.svelte";
  import { pageManifest } from "$lib/page-kit/loader.svelte.js";

  let selected = $state(new URLSearchParams(window.location.search).get("page") ?? "");
  let params = $state({});

  onMount(() => {
    pageManifest.start();
  });

  const options = $derived(
    Object.entries(pageManifest.manifest?.pages ?? {})
      .filter(([, e]) => e.page_type !== "Api")
      .map(([name, e]) => ({ value: name, label: `${e.caption} (${e.page_type})` }))
  );
</script>

<div class="space-y-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-full sm:w-80">
      <Select label="Compiled page" options={options} bind:value={selected} placeholder="Choose a page" onValueChange={() => (params = {})} />
    </div>
    {#if pageManifest.manifest}
      <p class="text-xs text-slate-500">
        {options.length} pages · Svelte {pageManifest.manifest.svelte ?? "?"} · manifest {pageManifest.manifest.etag.slice(0, 8)}
      </p>
    {/if}
  </div>

  {#if pageManifest.error}
    <Banner tone="danger" title="Page manifest unavailable"><p class="text-xs">{pageManifest.error}</p></Banner>
  {:else if pageManifest.manifest?.compiler_error}
    <Banner tone="warning" title="Page compiler unavailable"><p class="text-xs">{pageManifest.manifest.compiler_error}</p></Banner>
  {/if}

  {#if selected}
    {#key selected + JSON.stringify(params)}
      <div class="rounded-xl border border-slate-200 bg-white p-4 dark:border-slate-800 dark:bg-slate-950">
        <PageHost page={selected} {params} openPage={(name, opts) => { selected = name; params = opts; }} />
      </div>
    {/key}
  {/if}
</div>
