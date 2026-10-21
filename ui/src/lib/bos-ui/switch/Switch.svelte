<script>
  import { Switch as SwitchPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    disabled = false,
    required = false,
    name = undefined,
    label = "",
    class: className = "",
    onCheckedChange = null
  } = $props();

  const uid = $props.id();
</script>

<div class={cn("inline-flex items-center gap-2", className)}>
  <SwitchPrimitive.Root
    bind:ref
    bind:checked
    {disabled}
    {required}
    {name}
    id={uid}
    onCheckedChange={(value) => onCheckedChange?.(value)}
    class={cn(
      "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full border transition-colors",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/40",
      "disabled:opacity-50 disabled:pointer-events-none",
      checked
        ? "bg-indigo-600 border-indigo-600"
        : "bg-slate-200 border-slate-300 dark:bg-slate-700 dark:border-slate-600"
    )}
  >
    {#snippet children({ checked: on })}
      <span
        class={cn(
          "pointer-events-none block size-3.5 rounded-full bg-white shadow-sm transition-transform",
          on ? "translate-x-4" : "translate-x-0.5"
        )}
      ></span>
    {/snippet}
  </SwitchPrimitive.Root>
  {#if label}
    <label for={uid} class="text-xs text-slate-700 dark:text-slate-200 cursor-pointer select-none">
      {label}
    </label>
  {/if}
</div>
