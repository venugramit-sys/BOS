<script>
  import { notificationStore } from "$lib/services/notificationStore.svelte.js";
  import { CheckCircle2, AlertTriangle, AlertCircle, Info, X } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  /**
   * @param {'info' | 'success' | 'warning' | 'error'} severity
   */
  function getSeverityConfig(severity) {
    switch (severity) {
      case "success":
        return {
          icon: CheckCircle2,
          border: "border-emerald-500/40 dark:border-emerald-500/30",
          bg: "bg-white dark:bg-slate-900",
          iconColor: "text-emerald-500 dark:text-emerald-400",
          barColor: "bg-emerald-500",
          titleColor: "text-emerald-950 dark:text-emerald-200",
        };
      case "warning":
        return {
          icon: AlertTriangle,
          border: "border-amber-500/40 dark:border-amber-500/30",
          bg: "bg-white dark:bg-slate-900",
          iconColor: "text-amber-500 dark:text-amber-400",
          barColor: "bg-amber-500",
          titleColor: "text-amber-950 dark:text-amber-200",
        };
      case "error":
        return {
          icon: AlertCircle,
          border: "border-rose-500/50 dark:border-rose-500/30",
          bg: "bg-white dark:bg-slate-900",
          iconColor: "text-rose-500 dark:text-rose-400",
          barColor: "bg-rose-500",
          titleColor: "text-rose-950 dark:text-rose-200",
        };
      case "info":
      default:
        return {
          icon: Info,
          border: "border-indigo-500/40 dark:border-indigo-500/30",
          bg: "bg-white dark:bg-slate-900",
          iconColor: "text-indigo-500 dark:text-indigo-400",
          barColor: "bg-indigo-500",
          titleColor: "text-indigo-950 dark:text-indigo-200",
        };
    }
  }
</script>

<div
  aria-live="polite"
  aria-atomic="true"
  class="fixed bottom-4 right-4 z-50 flex flex-col gap-2.5 max-w-sm w-full pointer-events-none px-3 sm:px-0"
>
  {#each notificationStore.toasts as toast (toast.id)}
    {@const conf = getSeverityConfig(toast.severity)}
    {@const Icon = conf.icon}
    <div
      role="alert"
      class={cn(
        "pointer-events-auto relative overflow-hidden rounded-xl border shadow-xl transition-all duration-200 p-3.5 flex gap-3 items-start",
        conf.bg,
        conf.border
      )}
    >
      <!-- Severity Icon -->
      <div class="mt-0.5 shrink-0">
        <Icon class={cn("size-4", conf.iconColor)} />
      </div>

      <!-- Message Content -->
      <div class="flex-1 min-w-0 pr-4">
        {#if toast.title}
          <div class={cn("font-bold text-xs uppercase tracking-wider mb-0.5", conf.titleColor)}>
            {toast.title}
          </div>
        {/if}
        <div class="text-xs text-slate-700 dark:text-slate-300 font-medium leading-relaxed break-words">
          {toast.message}
        </div>
        {#if toast.code}
          <div class="mt-1 font-mono text-[10px] text-slate-400 dark:text-slate-500">
            Code: {toast.code}
          </div>
        {/if}
      </div>

      <!-- Dismiss Button -->
      <button
        type="button"
        onclick={() => notificationStore.dismissToast(toast.id)}
        aria-label="Dismiss notification"
        class="absolute top-2.5 right-2.5 p-1 rounded-md text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
      >
        <X class="size-3.5" />
      </button>

      <!-- Auto-dismiss Progress Bar -->
      {#if toast.duration_ms}
        <div
          class={cn("absolute bottom-0 left-0 h-0.5 w-full opacity-60", conf.barColor)}
          style="animation: toastProgress {toast.duration_ms}ms linear forwards;"
        ></div>
      {/if}
    </div>
  {/each}
</div>

<style>
  @keyframes toastProgress {
    from {
      width: 100%;
    }
    to {
      width: 0%;
    }
  }
</style>
