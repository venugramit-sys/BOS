<script>
  import { cn } from "$lib/utils.js";

  let {
    label = "",
    hint = "",
    error = "",
    required = false,
    htmlFor = "",
    class: className = "",
    children
  } = $props();
</script>

<div class={cn("flex flex-col gap-1 min-w-0", className)}>
  {#if label}
    <label for={htmlFor} class="text-[11px] font-medium text-slate-500 dark:text-slate-400">
      {label}
      {#if required}<span class="text-rose-500" aria-hidden="true"> *</span>{/if}
    </label>
  {/if}
  {@render children?.()}
  {#if error}
    <p class="text-[11px] text-rose-600 dark:text-rose-400" role="alert">{error}</p>
  {:else if hint}
    <p class="text-[11px] text-slate-400">{hint}</p>
  {/if}
</div>
