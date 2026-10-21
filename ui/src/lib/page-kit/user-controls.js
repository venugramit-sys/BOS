/**
 * Registry of developer-supplied `usercontrol(Name)` components (NAV control add-ins).
 * A package's UI bundle registers its Svelte 5 component once at startup:
 *   registerUserControl("SignaturePad", SignaturePad);
 * The component receives `{ p, c }` (page controller and control metadata).
 */
import { SvelteMap } from "svelte/reactivity";

const registry = new SvelteMap();

export function registerUserControl(name, component) {
  if (typeof name !== "string" || !/^[A-Za-z][A-Za-z0-9_]*$/.test(name)) throw new Error(`invalid user control name: ${name}`);
  if (typeof component !== "function") throw new Error(`user control ${name} must be a Svelte component`);
  registry.set(name.toLowerCase(), component);
}

export function userControl(name) {
  return registry.get(String(name ?? "").toLowerCase()) ?? null;
}

export function userControlNames() {
  return [...registry.keys()];
}
