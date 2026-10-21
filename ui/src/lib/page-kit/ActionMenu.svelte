<script>
  /** Disclosure menu of page actions (one per action area, plus the phone overflow menu). */
  import { ChevronDown } from "lucide-svelte";
  import Icon from "./Icon.svelte";

  /** @type {{ label: string, sections: { caption?: string, items: any[] }[], onrun: (a: any) => void, enabled: (a: any) => boolean, class?: string }} */
  let { label, sections = [], onrun, enabled, class: className = "" } = $props();

  let open = $state(false);
  let root = $state(null);
  const menuId = `bos-menu-${Math.random().toString(36).slice(2, 9)}`;

  $effect(() => {
    if (!open) return;
    const onDown = (e) => {
      if (root && !root.contains(e.target)) open = false;
    };
    const onKey = (e) => {
      if (e.key === "Escape") open = false;
    };
    document.addEventListener("pointerdown", onDown);
    document.addEventListener("keydown", onKey);
    queueMicrotask(() => root?.querySelector('[role="menuitem"]:not([disabled])')?.focus());
    return () => {
      document.removeEventListener("pointerdown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  });

  function onMenuKey(e) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = [...root.querySelectorAll('[role="menuitem"]:not([disabled])')];
    const i = items.indexOf(document.activeElement);
    items[(i + (e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length]?.focus();
  }
</script>

<div class={"relative " + className} bind:this={root}>
  <button type="button" class="bos-action" aria-haspopup="menu" aria-expanded={open} aria-controls={menuId} onclick={() => (open = !open)}>
    {label}
    <ChevronDown class="size-3.5" />
  </button>
  {#if open}
    <div id={menuId} class="bos-menu" role="menu" tabindex="-1" onkeydown={onMenuKey}>
      {#each sections as section, si (si)}
        {#if section.items.length}
          {#if section.caption}<div class="bos-menu-caption" role="presentation">{section.caption}</div>{/if}
          {#each section.items as a (a.name)}
            <button
              type="button"
              role="menuitem"
              class="bos-menu-item"
              disabled={!enabled(a)}
              title={a.tooltip ?? undefined}
              data-bos-node={a.run ? undefined : "action:" + (a.target ?? a.name)}
              onclick={() => {
                open = false;
                onrun(a);
              }}
            >
              {#if a.icon}<Icon name={a.icon} class="size-4 shrink-0" />{:else}<span class="size-4 shrink-0"></span>{/if}
              <span class="truncate">{a.caption}{a.ellipsis ? "…" : ""}</span>
            </button>
          {/each}
        {/if}
      {/each}
    </div>
  {/if}
</div>
