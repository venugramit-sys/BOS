/**
 * Design-token styling for compiled pages. Pages carry only tokens and validated hex colours
 * (`PageStyleIR`); these helpers turn them into CSS custom properties consumed by `app.css`
 * (`.bos-styled`, `.bos-cell`, emphasis classes). No raw CSS ever reaches the DOM.
 */

const HEX_RE = /^#[0-9a-f]{6}$/;
const TOKENS = new Set([
  "surface",
  "surfaceMuted",
  "primary",
  "secondary",
  "accent",
  "neutral",
  "info",
  "favorable",
  "unfavorable",
  "attention",
  "ambiguous",
  "subordinate",
  "strong"
]);
const RADIUS = { none: "0px", sm: "4px", md: "8px", lg: "12px", full: "9999px" };
const SHADOW = {
  none: "none",
  sm: "0 1px 2px rgb(15 23 42 / 0.08)",
  md: "0 4px 12px rgb(15 23 42 / 0.10)",
  lg: "0 12px 32px rgb(15 23 42 / 0.14)"
};
const DENSITY = { compact: "0.5rem", comfortable: "0.875rem", spacious: "1.25rem" };
const EMPHASIS = new Set([
  "Standard",
  "StandardAccent",
  "Strong",
  "StrongAccent",
  "Attention",
  "AttentionAccent",
  "Favorable",
  "Unfavorable",
  "Ambiguous",
  "Subordinate"
]);
const BREAKPOINTS = ["xs", "sm", "md", "lg", "xl"];

/** CSS value of a colour: token → var(--bos-color-token), validated hex → itself, anything else → null. */
export function colorValue(value) {
  if (typeof value !== "string") return null;
  if (TOKENS.has(value)) return `var(--bos-color-${value})`;
  const hex = value.toLowerCase();
  return HEX_RE.test(hex) ? hex : null;
}

/** Inline custom properties for a style object (empty string when nothing is set). */
export function styleVars(style) {
  if (!style) return "";
  const parts = [];
  const bg = colorValue(style.background);
  const fg = colorValue(style.textColor);
  const border = colorValue(style.borderColor);
  if (bg) parts.push(`--bos-bg:${bg}`);
  if (fg) parts.push(`--bos-fg:${fg}`);
  if (border) parts.push(`--bos-border-color:${border}`);
  if (Number.isInteger(style.borderWidth) && style.borderWidth >= 0 && style.borderWidth <= 4) {
    parts.push(`--bos-border-width:${style.borderWidth}px`);
  } else if (border) {
    parts.push("--bos-border-width:1px");
  }
  if (RADIUS[style.radius]) parts.push(`--bos-radius:${RADIUS[style.radius]}`);
  if (SHADOW[style.shadow]) parts.push(`--bos-shadow:${SHADOW[style.shadow]}`);
  if (DENSITY[style.density]) parts.push(`--bos-pad:${DENSITY[style.density]}`);
  return parts.join(";");
}

/** Emphasis class (NAV Style). `active=false` (StyleExpr evaluated to false) disables it. */
export function emphasisClass(emphasis, active = true) {
  if (!active || !EMPHASIS.has(emphasis) || emphasis === "Standard") return "";
  return `bos-emphasis-${emphasis}`;
}

/**
 * Resolves a control's emphasis with its optional StyleExpr result:
 * boolean → toggles the static emphasis; string → names the emphasis.
 */
export function resolveEmphasis(style, exprResult) {
  if (exprResult === undefined) return emphasisClass(style?.emphasis);
  if (typeof exprResult === "string" && EMPHASIS.has(exprResult)) return emphasisClass(exprResult);
  return emphasisClass(style?.emphasis, Boolean(exprResult) && exprResult !== "false");
}

/** Grid-cell custom properties for responsive spans (12-column grid) with page-type defaults. */
export function spanVars(responsive, defaults = { xs: 12, md: 6 }) {
  const span = { ...defaults, ...(responsive?.span ?? {}) };
  return BREAKPOINTS.filter((bp) => Number.isInteger(span[bp]) && span[bp] >= 1 && span[bp] <= 12)
    .map((bp) => `--bos-span-${bp}:${span[bp]}`)
    .join(";");
}

/** `bos-hide-below-<bp>` class for `hideBelow`. */
export function hideBelowClass(responsive) {
  const bp = responsive?.hideBelow;
  return BREAKPOINTS.includes(bp) && bp !== "xs" ? `bos-hide-below-${bp}` : "";
}

export function joinStyles(...parts) {
  return parts.filter(Boolean).join(";");
}
