<script>
  import { cn } from "$lib/utils.js";
  import { X } from "lucide-svelte";

  let {
    tone = "info",
    title = "",
    dismissible = false,
    onDismiss = null,
    class: className = "",
    children
  } = $props();

  const tones = {
    info: "border-sky-200 bg-sky-50 text-sky-900 dark:border-sky-900 dark:bg-sky-950/40 dark:text-sky-100",
    success: "border-emerald-200 bg-emerald-50 text-emerald-900 dark:border-emerald-900 dark:bg-emerald-950/40 dark:text-emerald-100",
    warning: "border-amber-200 bg-amber-50 text-amber-950 dark:border-amber-900 dark:bg-amber-950/40 dark:text-amber-100",
    danger: "border-rose-200 bg-rose-50 text-rose-900 dark:border-rose-900 dark:bg-rose-950/40 dark:text-rose-100"
  };
</script>

<div
  role={tone === "danger" || tone === "warning" ? "alert" : "status"}
  class={cn("flex items-start gap-2 rounded-md border px-3 py-2 text-xs", tones[tone] || tones.info, className)}
>
  <div class="min-w-0 flex-1">
    {#if title}
      <p class="font-semibold">{title}</p>
    {/if}
    {#if children}
      <div class={title ? "mt-0.5" : ""}>{@render children()}</div>
    {/if}
  </div>
  {#if dismissible}
    <button
      type="button"
      class="p-0.5 rounded text-current/70 hover:text-current cursor-pointer"
      aria-label="Dismiss"
      onclick={() => onDismiss?.()}
    >
      <X class="size-3.5" />
    </button>
  {/if}
</div>
