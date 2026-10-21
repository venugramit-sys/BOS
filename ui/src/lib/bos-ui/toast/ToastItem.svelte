<script>
  import { X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import { toastStore } from "./toastStore.svelte.js";

  let { item } = $props();

  const tones = {
    info: "border-sky-200 dark:border-sky-900",
    success: "border-emerald-200 dark:border-emerald-900",
    warning: "border-amber-200 dark:border-amber-900",
    danger: "border-rose-200 dark:border-rose-900"
  };

  let paused = $state(false);

  $effect(() => {
    if (paused || !item.timeout) return;
    const timer = setTimeout(() => toastStore.dismiss(item.id), item.timeout);
    return () => clearTimeout(timer);
  });
</script>

<article
  role={item.tone === "danger" || item.tone === "warning" ? "alert" : "status"}
  class={cn(
    "rounded-lg border bg-white px-3 py-2 shadow-lg dark:bg-slate-900",
    tones[item.tone] || tones.info
  )}
  onpointerenter={() => (paused = true)}
  onpointerleave={() => (paused = false)}
>
  <div class="flex items-start gap-2">
    <div class="min-w-0 flex-1">
      {#if item.title}
        <p class="text-xs font-semibold text-slate-900 dark:text-slate-100">{item.title}</p>
      {/if}
      {#if item.description}
        <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-0.5">{item.description}</p>
      {/if}
    </div>
    <button
      type="button"
      class="p-0.5 text-slate-400 hover:text-slate-700 cursor-pointer"
      aria-label="Dismiss notification"
      onclick={() => toastStore.dismiss(item.id)}
    >
      <X class="size-3.5" />
    </button>
  </div>
</article>
