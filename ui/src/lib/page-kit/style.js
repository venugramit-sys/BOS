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
  if (Number.isInteger(style.borderWidth) && style.borderWidth >= 0 && style.borderWidth <= 8) {
    parts.push(`--bos-border-width:${style.borderWidth}px`);
  } else if (border) {
    parts.push("--bos-border-width:1px");
  }
  if (RADIUS[style.radius]) parts.push(`--bos-radius:${RADIUS[style.radius]}`);
  else if (style.radius && typeof style.radius === "string") parts.push(`--bos-radius:${style.radius}`);
  if (SHADOW[style.shadow]) parts.push(`--bos-shadow:${SHADOW[style.shadow]}`);
  if (DENSITY[style.density]) parts.push(`--bos-pad:${DENSITY[style.density]}`);

  // Hybrid Token + Numeric Units for Box Model, Typography & Layout
  if (style.fontSize) {
    const s = String(style.fontSize).trim();
    parts.push(`--bos-font-size:${s.match(/^\d+$/) ? `${s}px` : s}`);
  }
  if (style.fontWeight) parts.push(`--bos-font-weight:${style.fontWeight}`);
  if (style.lineHeight) parts.push(`--bos-line-height:${style.lineHeight}`);
  if (style.letterSpacing) parts.push(`--bos-letter-spacing:${style.letterSpacing}`);
  if (style.textAlign) parts.push(`--bos-text-align:${style.textAlign}`);
  if (style.textTransform) parts.push(`--bos-text-transform:${style.textTransform}`);
  if (style.textDecoration) parts.push(`--bos-text-decoration:${style.textDecoration}`);

  if (style.borderStyle) parts.push(`--bos-border-style:${style.borderStyle}`);
  if (style.padding) {
    const p = String(style.padding).trim();
    parts.push(`--bos-pad:${p.match(/^\d+$/) ? `${p}px` : p}`);
  }
  if (style.margin) {
    const m = String(style.margin).trim();
    parts.push(`--bos-margin:${m.match(/^\d+$/) ? `${m}px` : m}`);
  }
  if (style.gap) {
    const g = String(style.gap).trim();
    parts.push(`--bos-gap:${g.match(/^\d+$/) ? `${g}px` : g}`);
  }

  // Dimensions & Sizing
  if (style.width) {
    const w = String(style.width).trim();
    parts.push(`--bos-w:${w.match(/^\d+$/) ? `${w}px` : w}`);
  }
  if (style.maxWidth) {
    const mw = String(style.maxWidth).trim();
    parts.push(`--bos-max-w:${mw.match(/^\d+$/) ? `${mw}px` : mw}`);
  }
  if (style.minWidth) {
    const minw = String(style.minWidth).trim();
    parts.push(`--bos-min-w:${minw.match(/^\d+$/) ? `${minw}px` : minw}`);
  }
  if (style.height) {
    const h = String(style.height).trim();
    parts.push(`--bos-h:${h.match(/^\d+$/) ? `${h}px` : h}`);
  }
  if (style.minHeight) {
    const minh = String(style.minHeight).trim();
    parts.push(`--bos-min-h:${minh.match(/^\d+$/) ? `${minh}px` : minh}`);
  }
  if (style.maxHeight) {
    const maxh = String(style.maxHeight).trim();
    parts.push(`--bos-max-h:${maxh.match(/^\d+$/) ? `${maxh}px` : maxh}`);
  }
  if (style.opacity) parts.push(`--bos-opacity:${style.opacity}`);
  if (style.overflow) parts.push(`--bos-overflow:${style.overflow}`);
  if (style.display) parts.push(`--bos-display:${style.display}`);
  if (style.justifyContent) parts.push(`--bos-justify:${style.justifyContent}`);
  if (style.alignItems) parts.push(`--bos-align:${style.alignItems}`);
  if (style.flexDirection) parts.push(`--bos-flex-dir:${style.flexDirection}`);
  if (style.flexWrap) parts.push(`--bos-flex-wrap:${style.flexWrap}`);

  // Icon Customization (size, color, background, shape/radius)
  if (style.iconSize) {
    const isz = String(style.iconSize).trim();
    const map = { xs: "0.875rem", sm: "1rem", md: "1.25rem", lg: "1.5rem", xl: "2rem", "2xl": "2.5rem" };
    parts.push(`--bos-icon-size:${map[isz] ?? (isz.match(/^\d+$/) ? `${isz}px` : isz)}`);
  }
  const iconCol = colorValue(style.iconColor);
  if (iconCol) parts.push(`--bos-icon-color:${iconCol}`);
  const iconBg = colorValue(style.iconBackground ?? style.iconBg);
  if (iconBg) parts.push(`--bos-icon-bg:${iconBg}`);
  if (style.iconRadius) {
    parts.push(`--bos-icon-radius:${RADIUS[style.iconRadius] ?? style.iconRadius}`);
  }
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
