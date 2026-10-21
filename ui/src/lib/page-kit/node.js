/**
 * Shared helpers of page-kit components: designer node attributes, visibility and layout context.
 */
import { getContext, setContext } from "svelte";
import { styleVars, spanVars, hideBelowClass, resolveEmphasis, joinStyles } from "./style.js";

const SPAN_KEY = Symbol("bos-span-defaults");
const GROUP_KEY = Symbol("bos-group");

/** `data-bos-node` attributes the designer overlay uses to find controls (design builds only). */
export function nodeAttrs(p, c, hidden = false) {
  if (!p.design || !c) return {};
  return { "data-bos-node": c.path, "data-bos-kind": c.kind, "data-bos-hidden": hidden ? "true" : undefined };
}

/**
 * Whether a control renders. Runtime builds hide it; design builds always render it
 * (dimmed through `data-bos-hidden`) so super users can select and un-hide it.
 */
export function isShown(p, c, visible) {
  if (p.design) return true;
  return visible !== false && p.visible(c);
}

export function isHidden(p, c, visible) {
  return visible === false || !c.visible || !p.visible(c);
}

/** Default responsive spans of fields placed inside a container (12-column grid). */
export function setSpanDefaults(defaults) {
  setContext(SPAN_KEY, defaults);
}

export function spanDefaults(p) {
  const fromParent = getContext(SPAN_KEY);
  if (fromParent) return fromParent;
  const t = p.meta.page_type;
  if (p.embedded || t === "CardPart" || t === "StandardDialog" || t === "ConfirmationDialog" || t === "NavigatePage") return { xs: 12 };
  return { xs: 12, md: 6 };
}

/** FastTab state shared with its fields (Promoted / Additional importance). */
export function setGroup(state) {
  setContext(GROUP_KEY, state);
}

export function group() {
  return getContext(GROUP_KEY) ?? null;
}

/** Inline style of a control box: style tokens + grid span. */
export function boxStyle(c, defaults) {
  return joinStyles(styleVars(c.style), defaults ? spanVars(c.responsive, defaults) : "");
}

/** Class list of a control box (styled box, hide-below, emphasis). */
export function boxClass(p, c, base = "", row = null) {
  return [
    base,
    c.style && styleVars(c.style) ? "bos-styled" : "",
    hideBelowClass(c.responsive),
    resolveEmphasis(c.style, p.styleResult(c, row))
  ]
    .filter(Boolean)
    .join(" ");
}
