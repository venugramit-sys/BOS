<script>
  /** Searchable lucide icon picker (icons load lazily as they scroll into the list). */
  import { X } from "lucide-svelte";
  import Icon from "$lib/page-kit/Icon.svelte";
  import { ICON_NAMES } from "$lib/page-kit/icons.js";

  let { value = null, onChange } = $props();

  let open = $state(false);
  let query = $state("");
  const LIMIT = 120;
  const matches = $derived(
    (query ? ICON_NAMES.filter((n) => n.includes(query.trim().toLowerCase())) : ICON_NAMES).slice(0, LIMIT)
  );
</script>

<div class="flex flex-col gap-1">
  <div class="flex items-center gap-1">
    <button
      type="button"
      class="flex min-w-0 flex-1 items-center gap-2 rounded-md border border-slate-200 px-2 py-1 text-left text-xs hover:border-indigo-400 dark:border-slate-700"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      {#if value}<Icon name={value} class="size-4" />{/if}
      <span class="truncate">{value || "No icon"}</span>
    </button>
    {#if value}
      <button type="button" class="rounded p-1 hover:bg-slate-100 dark:hover:bg-slate-800" aria-label="Remove icon" onclick={() => onChange(null)}><X class="size-3.5" /></button>
    {/if}
  </div>
  {#if open}
    <div class="rounded-md border border-slate-200 p-2 dark:border-slate-700">
      <input class="mb-2 w-full rounded border border-slate-200 bg-transparent px-2 py-1 text-xs outline-none dark:border-slate-700" placeholder={`Search ${ICON_NAMES.length} icons…`} bind:value={query} />
      <div class="grid max-h-48 grid-cols-6 gap-1 overflow-y-auto" role="listbox" aria-label="Icons">
        {#each matches as name (name)}
          <button
            type="button"
            role="option"
            aria-selected={value === name}
            title={name}
            class={"grid aspect-square place-items-center rounded hover:bg-indigo-50 dark:hover:bg-indigo-950 " + (value === name ? "bg-indigo-100 text-indigo-700 dark:bg-indigo-900" : "")}
            onclick={() => {
              onChange(name);
              open = false;
            }}
          >
            <Icon {name} class="size-4" />
          </button>
        {/each}
      </div>
      {#if matches.length === LIMIT}<p class="mt-1 text-[10px] text-slate-400">Showing the first {LIMIT}; refine the search.</p>{/if}
    </div>
  {/if}
</div>
