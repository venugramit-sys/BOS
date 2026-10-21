/**
 * Lucide icon registry for pages and the designer's icon picker. Every icon is its own lazily
 * loaded chunk: pages name icons at runtime (`style.icon`, action `image`), so a static import list
 * is impossible and a full icon bundle would weigh about 1 MB.
 */
const PREFIX = "/node_modules/lucide-svelte/dist/icons/";
const ICONS = import.meta.glob("/node_modules/lucide-svelte/dist/icons/*.svelte");
const cache = new Map();

/** Every available icon name (kebab-case, as written in `.bos`: `icon = "truck"`). */
export const ICON_NAMES = Object.freeze(
  Object.keys(ICONS)
    .map((k) => k.slice(PREFIX.length, -".svelte".length))
    .sort()
);

export function hasIcon(name) {
  return Boolean(ICONS[`${PREFIX}${name}.svelte`]);
}

/** Resolves to the icon component, or `null` for an unknown name. */
export function loadIcon(name) {
  const key = `${PREFIX}${name}.svelte`;
  if (!ICONS[key]) return Promise.resolve(null);
  if (!cache.has(name)) cache.set(name, ICONS[key]().then((m) => m.default));
  return cache.get(name);
}
