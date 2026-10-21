/**
 * Page manifest store and compiled-page loader.
 *
 * Production: `GET /api/v1/pages/manifest` lists every page with its content-addressed module URL
 * (`/page-artifacts/<db>/<Page>.<hash>.js`, immutable). The SSE stream announces republishes, so a
 * changed page is re-imported from its new URL while unchanged pages stay cached by the browser.
 *
 * Development (`vite dev`): pages are imported as Svelte source through the `bos-pages-dev` Vite
 * plugin (`/@bos-page/<Page>.svelte`), which also hot-reloads them on republish.
 */
import { fetchManifest } from "./api.js";

const DEV = import.meta.env?.DEV ?? false;

class ManifestStore {
  manifest = $state.raw(null);
  error = $state("");
  version = $state(0);

  #etag = "";
  #source = null;
  #loading = null;

  /** Loads the manifest once and subscribes to changes (idempotent). */
  start() {
    if (!this.#loading) this.#loading = this.refresh();
    if (!this.#source && typeof EventSource !== "undefined") {
      this.#source = new EventSource("/api/v1/pages/manifest/stream");
      this.#source.addEventListener("manifest", (e) => {
        if (e.data && e.data !== this.#etag) this.refresh();
      });
      this.#source.onerror = () => {
        // EventSource reconnects by itself; refresh once it is back so no change is missed.
        this.#source.onopen = () => this.refresh();
      };
    }
    return this.#loading;
  }

  async refresh() {
    try {
      const next = await fetchManifest(this.#etag);
      if (next) {
        this.#etag = next.etag;
        this.manifest = next;
        this.version += 1;
      }
      this.error = "";
    } catch (e) {
      this.error = e?.message ?? String(e);
    }
  }

  /** Manifest entry of a page (case-insensitive, like BOS object names). */
  entry(name) {
    const pages = this.manifest?.pages;
    if (!pages || !name) return null;
    if (pages[name]) return { name, ...pages[name] };
    const key = Object.keys(pages).find((k) => k.toLowerCase() === String(name).toLowerCase());
    return key ? { name: key, ...pages[key] } : null;
  }
}

export const pageManifest = new ManifestStore();

const modules = new Map();

/**
 * Imports the compiled component of `name`. Resolves to `{ component, entry }`.
 * Throws when the page does not exist or failed to compile.
 */
export async function loadPageComponent(name) {
  await pageManifest.start();
  const entry = pageManifest.entry(name);
  if (!entry) {
    if (pageManifest.error) throw new Error(`Page manifest unavailable: ${pageManifest.error}`);
    throw new Error(`Page "${name}" does not exist`);
  }
  if (entry.errors > 0) throw new Error(`Page "${entry.name}" has ${entry.errors} compile error(s); open it in the Studio to fix them`);
  const url = DEV ? `/@bos-page/${entry.name}.svelte` : entry.url;
  const key = DEV ? `${url}#${entry.hash}` : url;
  if (!modules.has(key)) {
    const promise = import(/* @vite-ignore */ url).then((m) => m.default);
    promise.catch(() => modules.delete(key));
    modules.set(key, promise);
  }
  return { component: await modules.get(key), entry };
}

/**
 * Imports the component of an unsaved page compiled by `POST /api/v1/pages/preview` (designer canvas).
 * Production imports the node's preview module; the Vite dev server compiles the generated source
 * itself (so it shares the dev runtime and hot-reloads).
 */
export async function loadPreview(result) {
  if (DEV) {
    const res = await fetch("/__bos-dev/preview", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ hash: result.hash, source: result.source })
    });
    if (!res.ok) throw new Error(`preview failed: ${res.status}`);
    const { url } = await res.json();
    return (await import(/* @vite-ignore */ url)).default;
  }
  if (!/^\/page-artifacts\/[0-9a-f]{16}\/preview\/[A-Za-z0-9_.-]+\.js$/.test(result.url)) throw new Error("invalid preview URL");
  return (await import(/* @vite-ignore */ result.url)).default;
}
