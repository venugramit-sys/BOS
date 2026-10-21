<script>
  import { cn } from "$lib/utils.js";
  import { themeStore } from "../theme/index.js";
  import Spinner from "../spinner/Spinner.svelte";
  import Kbd from "../kbd/Kbd.svelte";
  import Tooltip from "../tooltip/Tooltip.svelte";

  let {
    ref = $bindable(null),
    variant = "primary",
    size = null,
    loading = false,
    disabled = false,
    type = "button",
    shortcut = "",
    showShortcut = false,
    tooltip = "",
    icon = null,
    iconOnly = false,
    class: className = "",
    children,
    ...restProps
  } = $props();

  let resolvedSize = $derived(
    size ?? (themeStore.density === "lg" ? "md" : themeStore.density === "sm" ? "xs" : "sm")
  );

  const sizes = {
    xs: "h-7 px-2 text-[11px] gap-1",
    sm: "h-8 px-2.5 text-xs gap-1.5",
    md: "h-10 px-3.5 text-sm gap-2"
  };

  const iconOnlySizes = {
    xs: "size-7",
    sm: "size-8",
    md: "size-10"
  };

  const variants = {
    primary: "bg-indigo-600 text-white border-indigo-600 hover:bg-indigo-700",
    secondary:
      "bg-white text-slate-700 border-slate-300 hover:bg-slate-50 dark:bg-slate-900 dark:text-slate-200 dark:border-slate-700 dark:hover:bg-slate-800",
    ghost:
      "bg-transparent text-slate-600 border-transparent hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800",
    destructive: "bg-rose-600 text-white border-rose-600 hover:bg-rose-700"
  };

  let isDisabled = $derived(disabled || loading);

  let buttonClass = $derived(
    cn(
      "inline-flex items-center justify-center rounded-md border font-semibold whitespace-nowrap cursor-pointer transition-colors",
      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/40",
      "disabled:opacity-50 disabled:pointer-events-none",
      iconOnly ? iconOnlySizes[resolvedSize] : sizes[resolvedSize],
      variants[variant] || variants.primary,
      className
    )
  );
</script>

{#snippet control(props = {})}
  <button
    bind:this={ref}
    {...props}
    {...restProps}
    {type}
    disabled={isDisabled}
    aria-busy={loading || undefined}
    aria-keyshortcuts={shortcut || undefined}
    aria-label={iconOnly ? restProps["aria-label"] || tooltip || undefined : restProps["aria-label"]}
    class={cn(buttonClass, props.class, restProps.class)}
    onclick={(event) => {
      props.onclick?.(event);
      restProps.onclick?.(event);
    }}
    onfocus={(event) => {
      props.onfocus?.(event);
      restProps.onfocus?.(event);
    }}
  >
    {#if loading}
      <Spinner size="xs" label="Working" />
    {:else if icon}
      {@const Icon = icon}
      <Icon class={resolvedSize === "md" ? "size-4 shrink-0" : "size-3.5 shrink-0"} />
    {/if}
    {#if !iconOnly}
      {@render children?.()}
      {#if showShortcut && shortcut}
        <Kbd keys={shortcut} />
      {/if}
    {/if}
  </button>
{/snippet}

{#if tooltip}
  <Tooltip content={tooltip} {shortcut}>
    {#snippet children(props)}
      {@render control(props)}
    {/snippet}
  </Tooltip>
{:else}
  {@render control()}
{/if}
