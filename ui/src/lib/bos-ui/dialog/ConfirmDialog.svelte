<script>
  import { AlertDialog as AlertDialogPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    open = $bindable(false),
    title = "Confirm",
    description = "",
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    tone = "default",
    loading = false,
    onConfirm = null,
    onCancel = null
  } = $props();
</script>

<AlertDialogPrimitive.Root bind:open>
  <AlertDialogPrimitive.Portal>
    <AlertDialogPrimitive.Overlay class="fixed inset-0 z-50 bg-slate-950/60" />
    <AlertDialogPrimitive.Content
      class="fixed left-1/2 top-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-800 dark:bg-slate-900"
    >
      <AlertDialogPrimitive.Title class="text-sm font-semibold text-slate-900 dark:text-slate-100">
        {title}
      </AlertDialogPrimitive.Title>
      {#if description}
        <AlertDialogPrimitive.Description class="mt-2 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
          {description}
        </AlertDialogPrimitive.Description>
      {/if}
      <div class="mt-4 flex items-center justify-end gap-2">
        <AlertDialogPrimitive.Cancel
          class="px-3 py-1.5 rounded-md text-xs font-medium text-slate-600 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800 cursor-pointer"
          onclick={() => onCancel?.()}
        >
          {cancelLabel}
        </AlertDialogPrimitive.Cancel>
        <AlertDialogPrimitive.Action
          class={cn(
            "px-3 py-1.5 rounded-md text-xs font-semibold text-white cursor-pointer disabled:opacity-50",
            tone === "danger" ? "bg-rose-600 hover:bg-rose-700" : "bg-indigo-600 hover:bg-indigo-700"
          )}
          disabled={loading}
          onclick={() => onConfirm?.()}
        >
          {loading ? "Working..." : confirmLabel}
        </AlertDialogPrimitive.Action>
      </div>
    </AlertDialogPrimitive.Content>
  </AlertDialogPrimitive.Portal>
</AlertDialogPrimitive.Root>
