<script>
  import { cn } from "$lib/utils.js";

  let {
    tone = "neutral",
    size = "sm",
    dot = false,
    class: className = "",
    children
  } = $props();

  const tones = {
    neutral: "bg-slate-100 text-slate-600 border-slate-200 dark:bg-slate-800 dark:text-slate-300 dark:border-slate-700",
    info: "bg-sky-50 text-sky-700 border-sky-200 dark:bg-sky-950/50 dark:text-sky-300 dark:border-sky-900",
    success: "bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-950/50 dark:text-emerald-300 dark:border-emerald-900",
    warning: "bg-amber-50 text-amber-800 border-amber-200 dark:bg-amber-950/50 dark:text-amber-300 dark:border-amber-900",
    danger: "bg-rose-50 text-rose-700 border-rose-200 dark:bg-rose-950/50 dark:text-rose-300 dark:border-rose-900"
  };

  const dots = {
    neutral: "bg-slate-400",
    info: "bg-sky-500",
    success: "bg-emerald-500",
    warning: "bg-amber-500",
    danger: "bg-rose-500"
  };
</script>

<span
  class={cn(
    "inline-flex items-center gap-1 rounded-full border font-medium whitespace-nowrap",
    size === "md" ? "px-2 py-0.5 text-xs" : "px-1.5 py-px text-[10px]",
    tones[tone] || tones.neutral,
    className
  )}
>
  {#if dot}
    <span class={cn("size-1.5 rounded-full", dots[tone] || dots.neutral)} aria-hidden="true"></span>
  {/if}
  {@render children?.()}
</span>
