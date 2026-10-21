<script>
  import { cn } from "$lib/utils.js";
  import { resolveSurface } from "../theme/index.js";
  import Field from "../field/Field.svelte";

  let {
    ref = $bindable(null),
    value = $bindable(""),
    rows = 3,
    mono = false,
    resize = "vertical",
    label = "",
    hint = "",
    error = "",
    required = false,
    disabled = false,
    readonly = false,
    placeholder = "",
    maxlength = undefined,
    status = "default",
    class: className = "",
    id = ""
  } = $props();

  const uid = $props.id();
  let fieldId = $derived(id || uid);
  let surface = $derived(resolveSurface("field", { status: error ? "error" : status, density: "sm" }));
  let resizeClass = $derived(
    resize === "none" ? "resize-none" : resize === "both" ? "resize" : "resize-y"
  );
</script>

<Field {label} {hint} error={error} {required} htmlFor={fieldId}>
  <textarea
    bind:this={ref}
    bind:value
    id={fieldId}
    {rows}
    {disabled}
    {readonly}
    {placeholder}
    {maxlength}
    aria-invalid={error ? "true" : undefined}
    class={cn(
      "w-full px-2 py-1.5 text-xs text-slate-900 dark:text-slate-100 placeholder:text-slate-400",
      "focus:outline-none",
      surface.controlClass,
      resizeClass,
      mono && "font-mono",
      className
    )}
  ></textarea>
</Field>
