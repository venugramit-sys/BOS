<script>
  import { notificationStore } from "$lib/services/notificationStore.svelte.js";
  import { CheckCircle2, AlertTriangle, AlertCircle, Info, X, Copy, Check, ChevronDown, ChevronUp } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let isDetailsExpanded = $state(false);
  let hasCopied = $state(false);

  const alert = $derived(notificationStore.modalAlert);

  function handleCopyDetails() {
    if (!alert) return;
    const textToCopy = [
      alert.title ? `Title: ${alert.title}` : "",
      `Message: ${alert.message}`,
      alert.code ? `Code: ${alert.code}` : "",
      alert.details ? `Details:\n${alert.details}` : "",
      `Timestamp: ${alert.timestamp}`,
    ].filter(Boolean).join("\n");

    navigator.clipboard.writeText(textToCopy).then(() => {
      hasCopied = true;
      setTimeout(() => (hasCopied = false), 2000);
    });
  }

  function getSeverityConfig(severity) {
    switch (severity) {
      case "success":
        return {
          icon: CheckCircle2,
          color: "text-emerald-500",
          bgColor: "bg-emerald-50 dark:bg-emerald-950/30",
          border: "border-emerald-500/30",
          btnColor: "bg-emerald-600 hover:bg-emerald-700 text-white",
        };
      case "warning":
        return {
          icon: AlertTriangle,
          color: "text-amber-500",
          bgColor: "bg-amber-50 dark:bg-amber-950/30",
          border: "border-amber-500/30",
          btnColor: "bg-amber-600 hover:bg-amber-700 text-white",
        };
      case "error":
        return {
          icon: AlertCircle,
          color: "text-rose-500",
          bgColor: "bg-rose-50 dark:bg-rose-950/30",
          border: "border-rose-500/30",
          btnColor: "bg-rose-600 hover:bg-rose-700 text-white",
        };
      case "info":
      default:
        return {
          icon: Info,
          color: "text-indigo-500",
          bgColor: "bg-indigo-50 dark:bg-indigo-950/30",
          border: "border-indigo-500/30",
          btnColor: "bg-indigo-600 hover:bg-indigo-700 text-white",
        };
    }
  }
</script>

{#if alert}
  {@const conf = getSeverityConfig(alert.severity)}
  {@const Icon = conf.icon}
  <div
    role="presentation"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/50 backdrop-blur-xs animate-in fade-in duration-150"
    onclick={() => notificationStore.closePopup()}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="alert-modal-title"
      tabindex="-1"
      class="relative w-full max-w-md bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl p-6 overflow-hidden animate-in zoom-in-95 duration-150"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Close icon button -->
      <button
        type="button"
        onclick={() => notificationStore.closePopup()}
        class="absolute top-4 right-4 p-1.5 rounded-lg text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
        aria-label="Close alert"
      >
        <X class="size-4" />
      </button>

      <div class="flex items-start gap-4">
        <!-- Severity Icon Badge -->
        <div class={cn("p-3 rounded-xl shrink-0 border", conf.bgColor, conf.border)}>
          <Icon class={cn("size-6", conf.color)} />
        </div>

        <div class="flex-1 min-w-0 pr-6">
          <h3 id="alert-modal-title" class="text-base font-bold text-slate-900 dark:text-slate-100 leading-tight">
            {alert.title || (alert.severity === "error" ? "System Error" : "Notification")}
          </h3>
          <p class="mt-2 text-sm text-slate-600 dark:text-slate-400 leading-relaxed break-words">
            {alert.message}
          </p>
        </div>
      </div>

      <!-- Expandable Technical Details -->
      {#if alert.details || alert.code}
        <div class="mt-4 pt-4 border-t border-slate-100 dark:border-slate-800">
          <button
            type="button"
            onclick={() => (isDetailsExpanded = !isDetailsExpanded)}
            class="flex items-center justify-between w-full text-xs font-semibold text-slate-500 hover:text-slate-800 dark:hover:text-slate-200 cursor-pointer"
          >
            <span class="flex items-center gap-1.5">
              <span>Technical Information</span>
              {#if alert.code}
                <span class="px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800 font-mono text-[10px] text-slate-500">
                  {alert.code}
                </span>
              {/if}
            </span>
            {#if isDetailsExpanded}
              <ChevronUp class="size-3.5" />
            {:else}
              <ChevronDown class="size-3.5" />
            {/if}
          </button>

          {#if isDetailsExpanded}
            <div class="mt-2 relative">
              <pre class="p-3 rounded-lg bg-slate-950 text-slate-200 font-mono text-xs overflow-x-auto max-h-48 whitespace-pre-wrap leading-tight">{alert.details || `Error Code: ${alert.code}`}</pre>
              <button
                type="button"
                onclick={handleCopyDetails}
                class="absolute top-2 right-2 p-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-[11px] flex items-center gap-1 px-2 cursor-pointer shadow-sm transition-colors"
                title="Copy technical details to clipboard"
              >
                {#if hasCopied}
                  <Check class="size-3 text-emerald-400" />
                  <span class="text-emerald-400 font-medium">Copied</span>
                {:else}
                  <Copy class="size-3" />
                  <span>Copy</span>
                {/if}
              </button>
            </div>
          {/if}
        </div>
      {/if}

      <!-- Modal Footer / Actions -->
      <div class="mt-6 flex justify-end gap-2.5">
        <button
          type="button"
          onclick={() => notificationStore.closePopup()}
          class={cn("px-4 py-2 rounded-xl text-xs font-semibold transition-colors cursor-pointer shadow-sm", conf.btnColor)}
        >
          Acknowledge
        </button>
      </div>
    </div>
  </div>
{/if}
