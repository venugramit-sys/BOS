<script>
  import { ChevronRight, ChevronDown } from "lucide-svelte";
  import { cn } from "$lib/utils.js";
  import Badge from "../badge/Badge.svelte";
  import EmptyState from "../empty-state/EmptyState.svelte";

  let {
    nodes = [],
    expanded = $bindable([]),
    selectedId = $bindable(null),
    filter = "",
    label = "Objects",
    onActivate = null,
    class: className = ""
  } = $props();

  const rowHeight = 32;
  let scrollEl = $state(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(320);
  let focusIndex = $state(0);

  function matches(node, query) {
    if (!query) return true;
    const haystack = `${node.label || ""} ${node.id || ""}`.toLowerCase();
    if (haystack.includes(query)) return true;
    return (node.children || []).some((child) => matches(child, query));
  }

  function flatten(list, depth, query) {
    const rows = [];
    for (const node of list) {
      if (!matches(node, query)) continue;
      const children = node.children || [];
      const forced = Boolean(query);
      const open = forced || expanded.includes(node.id);
      rows.push({
        id: node.id,
        node,
        depth,
        hasChildren: children.length > 0,
        open
      });
      if (open && children.length) rows.push(...flatten(children, depth + 1, query));
    }
    return rows;
  }

  let query = $derived(filter.trim().toLowerCase());
  let rows = $derived(flatten(nodes, 0, query));

  let windowed = $derived.by(() => {
    if (rows.length < 50) {
      return { start: 0, end: rows.length, padTop: 0, padBottom: 0 };
    }
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - 6);
    const count = Math.ceil(viewportHeight / rowHeight) + 12;
    const end = Math.min(rows.length, start + count);
    return {
      start,
      end,
      padTop: start * rowHeight,
      padBottom: Math.max(0, (rows.length - end) * rowHeight)
    };
  });

  $effect(() => {
    const el = scrollEl;
    if (!el) return;
    viewportHeight = el.clientHeight;
    const observer = new ResizeObserver(() => {
      viewportHeight = el.clientHeight;
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  function toggle(id) {
    expanded = expanded.includes(id) ? expanded.filter((item) => item !== id) : [...expanded, id];
  }

  function selectIndex(index) {
    const next = Math.min(rows.length - 1, Math.max(0, index));
    focusIndex = next;
    const row = rows[next];
    if (!row) return;
    selectedId = row.id;
    const top = next * rowHeight;
    if (scrollEl && top < scrollEl.scrollTop) scrollEl.scrollTop = top;
    if (scrollEl && top + rowHeight > scrollEl.scrollTop + scrollEl.clientHeight) {
      scrollEl.scrollTop = top + rowHeight - scrollEl.clientHeight;
    }
  }

  function onKeydown(event) {
    if (!rows.length) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectIndex(focusIndex + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selectIndex(focusIndex - 1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      const row = rows[focusIndex];
      if (row?.hasChildren && !row.open) toggle(row.id);
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      const row = rows[focusIndex];
      if (row?.open && row.hasChildren) toggle(row.id);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const row = rows[focusIndex];
      if (row) onActivate?.(row.node);
    }
  }
</script>

<div class={cn("flex h-full min-h-0 flex-col", className)}>
  <div
    bind:this={scrollEl}
    role="tree"
    aria-label={label}
    tabindex="0"
    class="min-h-0 flex-1 overflow-auto outline-none focus-visible:ring-2 focus-visible:ring-indigo-500/30"
    onscroll={() => (scrollTop = scrollEl?.scrollTop || 0)}
    onkeydown={onKeydown}
  >
    {#if rows.length === 0}
      <EmptyState title="No objects" description="Nothing matches this filter." />
    {:else}
      <div style={`height:${windowed.padTop}px`}></div>
      {#each rows.slice(windowed.start, windowed.end) as row, offset (row.id)}
        {@const index = windowed.start + offset}
        <div
          role="treeitem"
          aria-selected={selectedId === row.id}
          aria-label={row.node.label}
          aria-expanded={row.hasChildren ? row.open : undefined}
          aria-level={row.depth + 1}
          class={cn(
            "flex items-center gap-1 pr-2 text-xs cursor-pointer",
            selectedId === row.id
              ? "bg-indigo-50 text-indigo-700 dark:bg-indigo-950/50 dark:text-indigo-200"
              : "text-slate-700 hover:bg-slate-100 dark:text-slate-200 dark:hover:bg-slate-800"
          )}
          style={`height:${rowHeight}px;padding-left:${8 + row.depth * 14}px`}
          onclick={() => {
            selectIndex(index);
            onActivate?.(row.node);
          }}
          oncontextmenu={() => selectIndex(index)}
        >
          {#if row.hasChildren}
            <button
              type="button"
              class="p-0.5 rounded text-slate-400 cursor-pointer"
              aria-label={row.open ? `Collapse ${row.node.label}` : `Expand ${row.node.label}`}
              onclick={(event) => {
                event.stopPropagation();
                toggle(row.id);
              }}
            >
              {#if row.open}
                <ChevronDown class="size-3.5" />
              {:else}
                <ChevronRight class="size-3.5" />
              {/if}
            </button>
          {:else}
            <span class="w-4"></span>
          {/if}
          {#if row.node.icon}
            {@const Icon = row.node.icon}
            <Icon class="size-3.5 shrink-0 text-indigo-500" />
          {/if}
          <span class="truncate">{row.node.label}</span>
          {#if row.node.badge}
            <Badge tone={row.node.badgeTone || "neutral"} class="ml-auto">{row.node.badge}</Badge>
          {/if}
        </div>
      {/each}
      <div style={`height:${windowed.padBottom}px`}></div>
    {/if}
  </div>
</div>
