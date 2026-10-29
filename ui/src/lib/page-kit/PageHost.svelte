<script>
  /**
   * Mounts one compiled page by name: manifest → dynamic import → `<Page {ctx} />`.
   * Dialog-type pages (and `openPage(..., { modal: true })`) open in a modal stack owned by the host;
   * other pages go to `openPage` of the embedding shell (a new workspace tab).
   */
  import { Dialog } from "$lib/bos-ui/dialog/index.js";
  import { Spinner } from "$lib/bos-ui/spinner/index.js";
  import { Banner } from "$lib/bos-ui/banner/index.js";
  import { Button } from "$lib/bos-ui/button/index.js";
  import { RotateCcw } from "lucide-svelte";
  import { loadPageComponent, pageManifest } from "./loader.svelte.js";
  import Self from "./PageHost.svelte";

  /**
   * @type {{
   *   page: string,
   *   params?: { recordId?: string, mode?: string, filters?: Record<string, any> },
   *   openPage?: (name: string, params: any) => void,
   *   closePage?: (result?: any) => void,
   *   navigate?: (url: string) => void,
   *   onRecordChange?: (id: string) => void,
   *   onActionComplete?: (actionName: string, res: any, p: any) => void,
   *   onActionError?: (actionName: string, err: any, p: any) => void,
   *   onHostError?: (err: string | null) => void,
   *   design?: boolean,
   *   embedded?: boolean,
   *   modal?: boolean,
   * }}
   */
  let { page, params = {}, openPage = undefined, closePage = undefined, navigate = undefined, onRecordChange = undefined, onActionComplete = undefined, onActionError = undefined, onHostError = undefined, design = false, embedded = false, modal = false, onDirtyChange = undefined } = $props();

  const DIALOG_TYPES = new Set(["StandardDialog", "ConfirmationDialog", "NavigatePage"]);

  let Component = $state(null);
  let error = $state("");
  let modals = $state([]);
  let reloadKey = $state(0);

  $effect(() => {
    const name = page;
    pageManifest.version; // re-import after a republish
    reloadKey; // explicit retry trigger
    let alive = true;
    error = "";
    loadPageComponent(name)
      .then(({ component }) => {
        if (alive) {
          Component = component;
          onHostError?.(null);
        }
      })
      .catch((e) => {
        if (alive) {
          Component = null;
          error = e?.message ?? String(e);
          onHostError?.(error);
        }
      });
    return () => {
      alive = false;
    };
  });

  function open(name, opts = {}) {
    const entry = pageManifest.entry(name);
    if (opts.modal || (entry && DIALOG_TYPES.has(entry.page_type)) || !openPage) {
      modals = [...modals, { key: `${name}-${Date.now()}`, page: entry?.name ?? name, caption: entry?.caption ?? name, params: opts }];
    } else {
      openPage(entry?.name ?? name, opts);
    }
  }

  function closeModal(key) {
    modals = modals.filter((m) => m.key !== key);
  }

  const ctx = $derived({
    page,
    recordId: params.recordId ?? null,
    mode: params.mode ?? null,
    filters: params.filters ?? {},
    variables: params.variables ?? {},
    design,
    embedded,
    modal,
    openPage: open,
    closePage,
    navigate,
    onRecordChange,
    onActionComplete,
    onActionError,
    onDirtyChange
  });
</script>

{#if error}
  <div class="flex items-center justify-center p-8">
    <div class="w-full max-w-lg">
      <Banner tone="danger" title="This page cannot be opened">
        <p class="text-xs break-all">{error}</p>
        <div class="mt-3 flex items-center gap-2">
          <Button
            size="sm"
            variant="secondary"
            onclick={() => {
              error = "";
              pageManifest.refresh();
              reloadKey += 1;
            }}
          >
            <RotateCcw class="size-3.5 mr-1.5" />
            Retry
          </Button>
        </div>
      </Banner>
    </div>
  </div>
{:else if Component}
  <Component {ctx} />
{:else}
  <div class="flex justify-center py-12"><Spinner label="Loading page" /></div>
{/if}

{#each modals as m (m.key)}
  <Dialog open title={m.caption} size="lg" onClose={() => closeModal(m.key)}>
    <Self page={m.page} params={m.params} modal {navigate} closePage={() => closeModal(m.key)} openPage={(name, opts) => open(name, opts)} />
  </Dialog>
{/each}
