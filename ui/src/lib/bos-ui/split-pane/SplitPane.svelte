<script>
  import { cn } from "$lib/utils.js";

  let {
    direction = "row",
    size = $bindable(null),
    defaultSize = 280,
    min = 120,
    max = 720,
    onSizeChange = null,
    class: className = "",
    primary,
    secondary
  } = $props();

  let current = $state(size ?? defaultSize);

  $effect(() => {
    if (size != null) current = size;
  });

  function apply(next) {
    const clamped = Math.min(max, Math.max(min, Math.round(next)));
    current = clamped;
    size = clamped;
    onSizeChange?.(clamped);
  }

  function onPointerDown(event) {
    event.preventDefault();
    const start = direction === "row" ? event.clientX : event.clientY;
    const origin = current;
    const move = (ev) => {
      const point = direction === "row" ? ev.clientX : ev.clientY;
      apply(origin + (point - start));
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function onKeydown(event) {
    const step = event.shiftKey ? 32 : 8;
    if (direction === "row" && (event.key === "ArrowLeft" || event.key === "ArrowRight")) {
      event.preventDefault();
      apply(current + (event.key === "ArrowRight" ? step : -step));
    }
    if (direction === "column" && (event.key === "ArrowUp" || event.key === "ArrowDown")) {
      event.preventDefault();
      apply(current + (event.key === "ArrowDown" ? step : -step));
    }
  }

  let horizontal = $derived(direction !== "column");
</script>

<div class={cn("flex min-h-0 min-w-0", horizontal ? "flex-row" : "flex-col", className)}>
  <div
    class="min-h-0 min-w-0 overflow-hidden"
    style={horizontal ? `width:${current}px;flex:0 0 auto` : `height:${current}px;flex:0 0 auto`}
  >
    {@render primary?.()}
  </div>
  <button
    type="button"
    role="separator"
    aria-label="Resize panes"
    aria-orientation={horizontal ? "vertical" : "horizontal"}
    aria-valuenow={current}
    aria-valuemin={min}
    aria-valuemax={max}
    class={cn(
      "shrink-0 bg-slate-200 hover:bg-indigo-400 focus-visible:bg-indigo-500 focus-visible:outline-none dark:bg-slate-800",
      horizontal ? "w-1 cursor-col-resize" : "h-1 cursor-row-resize"
    )}
    onpointerdown={onPointerDown}
    onkeydown={onKeydown}
  ></button>
  <div class="min-h-0 min-w-0 flex-1 overflow-hidden">
    {@render secondary?.()}
  </div>
</div>
