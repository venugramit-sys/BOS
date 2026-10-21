<script>
  import { cn } from "$lib/utils.js";

  let {
    size = "sm",
    label = "Loading",
    class: className = ""
  } = $props();

  const sizes = {
    xs: "size-3",
    sm: "size-3.5",
    md: "size-4"
  };
</script>

<span class={cn("inline-flex", className)} role="status">
  <svg
    class={cn("animate-spin text-current", sizes[size] || sizes.sm)}
    viewBox="0 0 24 24"
    fill="none"
    aria-hidden="true"
  >
    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="3"></circle>
    <path class="opacity-90" fill="currentColor" d="M12 2a10 10 0 0 1 10 10h-3a7 7 0 0 0-7-7V2z"></path>
  </svg>
  <span class="sr-only">{label}</span>
</span>
