/**
 * Single surface definition for every bos-ui control.
 *
 * Density, frame variant, and corner radius are read from themeStore
 * (the one personalization setting). A component may pass an override;
 * otherwise the store value is used.
 */
import { cn } from "$lib/utils.js";
import { themeStore } from "./themeStore.svelte.js";

export const surfaceDensity = {
  field: {
    sm: "h-7 sm:h-8 text-xs py-1",
    md: "h-9 text-xs sm:text-sm py-1.5",
    lg: "h-11 text-sm sm:text-base py-2.5"
  },
  lookup: {
    sm: "min-h-7 text-xs py-1 px-2",
    md: "min-h-9 text-xs sm:text-sm py-1.5 px-2.5",
    lg: "min-h-11 text-sm sm:text-base py-2 px-3"
  },
  date: {
    sm: "h-7 text-xs px-2",
    md: "h-9 text-xs sm:text-sm px-2.5",
    lg: "h-11 text-sm sm:text-base px-3"
  },
  menu: {
    sm: "h-7 text-[11px] px-1.5",
    md: "h-9 text-xs px-2",
    lg: "h-11 text-sm px-2.5"
  }
};

export const surfaceIcon = {
  sm: "size-3.5",
  md: "size-4",
  lg: "size-4.5"
};

export const surfaceText = {
  sm: "text-[11px] py-1",
  md: "text-xs py-1.5",
  lg: "text-sm py-2"
};

export const surfaceRadius = {
  none: "rounded-none",
  sm: "rounded-xs",
  md: "rounded-md",
  lg: "rounded-lg",
  pill: "rounded-full"
};

/** Floating panels stay readable when the control radius is a pill. */
export const panelRadius = {
  none: "rounded-none",
  sm: "rounded-sm",
  md: "rounded-lg",
  lg: "rounded-xl",
  pill: "rounded-2xl"
};

export const surfaceVariant = {
  outline: "border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-2xs",
  filled: "border-b-2 border-transparent bg-slate-100 dark:bg-slate-800/80 hover:bg-slate-200/70 dark:hover:bg-slate-800",
  underline: "border-b-2 border-slate-300 dark:border-slate-700 bg-transparent rounded-none px-0 shadow-none",
  ghost: "border border-transparent bg-transparent hover:bg-slate-100/70 dark:hover:bg-slate-800/60 shadow-none"
};

/** Menu bar chrome. Underline does not strip horizontal padding. */
export const chromeVariant = {
  outline: "border border-slate-200 dark:border-slate-800 bg-white/95 dark:bg-slate-900/95 shadow-xs backdrop-blur-sm",
  filled: "border border-transparent dark:border-transparent bg-slate-100 dark:bg-slate-800/80 shadow-none",
  underline: "border-x-0 border-t-0 border-b-2 border-slate-300 dark:border-slate-700 bg-transparent rounded-none shadow-none",
  ghost: "border border-transparent dark:border-transparent bg-transparent shadow-none"
};

export const panelVariant = {
  outline: "border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-xl",
  filled: "border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-950 shadow-xl",
  underline: "border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 shadow-xl",
  ghost: "border border-slate-200/80 dark:border-slate-800 bg-white/95 dark:bg-slate-900/95 shadow-lg backdrop-blur-sm"
};

export const surfaceStatus = {
  default: "focus-within:border-indigo-500 dark:focus-within:border-indigo-400 focus-within:ring-1 focus-within:ring-indigo-500/20",
  error: "border-rose-500 dark:border-rose-500 focus-within:border-rose-600 focus-within:ring-1 focus-within:ring-rose-500/20",
  warning: "border-amber-500 dark:border-amber-500 focus-within:border-amber-600 focus-within:ring-1 focus-within:ring-amber-500/20",
  success: "border-emerald-500 dark:border-emerald-500 focus-within:border-emerald-600 focus-within:ring-1 focus-within:ring-emerald-500/20",
  info: "border-sky-500 dark:border-sky-500 focus-within:border-sky-600 focus-within:ring-1 focus-within:ring-sky-500/20"
};

/**
 * Resolve the shared surface for a bos-ui control.
 * Call this inside `$derived` so themeStore updates repaint the control.
 *
 * @param {'field' | 'lookup' | 'date' | 'menu'} kind
 * @param {{ density?: string, size?: string, variant?: string, rounded?: string, status?: string }} [overrides]
 */
export function resolveSurface(kind = "field", overrides = {}) {
  const density = overrides.density ?? overrides.size ?? themeStore.density ?? "md";
  const variant = overrides.variant ?? themeStore.variant ?? "outline";
  const rounded = overrides.rounded ?? themeStore.rounded ?? "md";
  const status = overrides.status ?? "default";

  const densityMap = surfaceDensity[kind] || surfaceDensity.field;
  const densityClass = densityMap[density] || densityMap.md;
  const radiusClass = surfaceRadius[rounded] || surfaceRadius.md;
  const variantMap = kind === "menu" ? chromeVariant : surfaceVariant;
  const variantClass = variantMap[variant] || variantMap.outline;
  const statusClass = surfaceStatus[status] || surfaceStatus.default;

  return {
    density,
    variant,
    rounded,
    iconClass: surfaceIcon[density] || surfaceIcon.md,
    textClass: surfaceText[density] || surfaceText.md,
    radiusClass,
    controlClass: cn(densityClass, radiusClass, variantClass, kind === "menu" ? "" : statusClass),
    menuClass: cn(densityClass, radiusClass, variantClass),
    panelClass: cn(panelRadius[rounded] || panelRadius.md, panelVariant[variant] || panelVariant.outline),
    chipRadius: radiusClass === "rounded-full" ? "rounded-full" : radiusClass
  };
}
