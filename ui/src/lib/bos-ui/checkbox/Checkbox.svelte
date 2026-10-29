<script>
  import { Checkbox as CheckboxPrimitive } from "bits-ui";
  import { Check, Minus } from "lucide-svelte";
  import { cn } from "$lib/utils.js";

  let {
    ref = $bindable(null),
    checked = $bindable(),
    indeterminate = $bindable(),
    disabled = false,
    readonly = false,
    required = false,
    name = undefined,
    label = "",
    density = "sm",
    class: className = "",
    onCheckedChange = null
  } = $props();

  const uid = $props.id();
</script>

<div class={cn("inline-flex items-center gap-2.5 min-h-[44px] sm:min-h-0", className)}>
  <CheckboxPrimitive.Root
    bind:ref
    checked={checked ?? false}
    indeterminate={indeterminate ?? false}
    {disabled}
    {readonly}
    {required}
    {name}
    id={uid}
    onkeydown={(e) => {
      if (e.key === "Enter") {
        e.preventDefault();
      }
    }}
    onCheckedChange={(value) => {
      if (value === "indeterminate") {
        indeterminate = true;
        checked = false;
      } else {
        indeterminate = false;
        checked = value === true;
      }
      onCheckedChange?.(checked);
    }}
    class={cn(
      "inline-flex shrink-0 items-center justify-center rounded border border-slate-300 bg-white text-white",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/40",
      "disabled:opacity-50 disabled:pointer-events-none dark:border-slate-600 dark:bg-slate-950",
      checked || indeterminate ? "bg-indigo-600 border-indigo-600" : "",
      density === "md" ? "size-4" : "size-3.5"
    )}
  >
    {#snippet children({ checked: isChecked, indeterminate: isMixed })}
      {#if isMixed}
        <Minus class="size-3" />
      {:else if isChecked}
        <Check class="size-3" />
      {/if}
    {/snippet}
  </CheckboxPrimitive.Root>
  {#if label}
    <label for={uid} class="text-xs text-slate-700 dark:text-slate-200 cursor-pointer select-none">
      {label}
    </label>
  {/if}
</div>
