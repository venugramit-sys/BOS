<script>
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    value = $bindable(""),
    type = "text",
    class: className = "",
    ...restProps
  } = $props();
</script>

<input
  bind:this={ref}
  data-slot="input"
  {type}
  bind:value
  class={cn(
    "flex h-9 w-full min-w-0 rounded-md border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 px-3 py-1 text-xs sm:text-sm shadow-2xs transition-colors",
    "placeholder:text-slate-400 dark:placeholder:text-slate-500 text-slate-900 dark:text-slate-100",
    "focus-visible:border-indigo-500 dark:focus-visible:border-indigo-400 focus-visible:ring-1 focus-visible:ring-indigo-500/30 outline-hidden",
    "disabled:cursor-not-allowed disabled:opacity-50 disabled:bg-slate-100 dark:disabled:bg-slate-800/50",
    "aria-invalid:border-rose-500 dark:aria-invalid:border-rose-500 aria-invalid:ring-1 aria-invalid:ring-rose-500/20",
    className
  )}
  {...restProps}
/>
