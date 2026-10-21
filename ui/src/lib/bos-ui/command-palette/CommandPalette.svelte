<script>
  import { Dialog as DialogPrimitive, Command as CommandPrimitive } from "bits-ui";
  import { cn } from "$lib/utils.js";
  import { eventMatchesShortcut } from "../hotkey.js";
  import Kbd from "../kbd/Kbd.svelte";

  let {
    open = $bindable(false),
    groups = [],
    placeholder = "Search objects and commands...",
    hotkey = "Ctrl+K",
    recentKey = "",
    onRun = null
  } = $props();

  let recentIds = $state([]);

  function readRecent() {
    if (!recentKey || typeof localStorage === "undefined") return [];
    try {
      const parsed = JSON.parse(localStorage.getItem(recentKey) || "[]");
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }

  function remember(item) {
    if (!recentKey || !item?.id || typeof localStorage === "undefined") return;
    const next = [item.id, ...readRecent().filter((id) => id !== item.id)].slice(0, 5);
    localStorage.setItem(recentKey, JSON.stringify(next));
    recentIds = next;
  }

  let catalog = $derived(groups.flatMap((group) => group.items || []));

  let recentItems = $derived(
    recentIds
      .map((id) => catalog.find((item) => item.id === id))
      .filter(Boolean)
  );

  let visibleGroups = $derived(
    recentItems.length
      ? [{ heading: "Recent", items: recentItems }, ...groups]
      : groups
  );

  function run(item) {
    if (!item || item.disabled) return;
    remember(item);
    open = false;
    item.action?.();
    onRun?.(item);
  }

  $effect(() => {
    recentIds = readRecent();
  });

  $effect(() => {
    if (!hotkey) return;
    const onKeydown = (event) => {
      if (!eventMatchesShortcut(event, hotkey)) return;
      event.preventDefault();
      open = !open;
    };
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

<DialogPrimitive.Root bind:open>
  <DialogPrimitive.Portal>
    <DialogPrimitive.Overlay class="fixed inset-0 z-50 bg-slate-950/50" />
    <DialogPrimitive.Content
      class="fixed left-1/2 top-[15%] z-50 w-[calc(100%-2rem)] max-w-xl -translate-x-1/2 overflow-hidden rounded-xl border border-slate-200 bg-white shadow-xl dark:border-slate-800 dark:bg-slate-900"
    >
      <DialogPrimitive.Title class="sr-only">Command palette</DialogPrimitive.Title>
      <DialogPrimitive.Description class="sr-only">
        Search meta-objects and run studio commands.
      </DialogPrimitive.Description>
      <CommandPrimitive.Root class="flex flex-col" loop>
        <CommandPrimitive.Input
          placeholder={placeholder}
          class="h-11 w-full border-b border-slate-200 bg-transparent px-3 text-sm text-slate-900 outline-none placeholder:text-slate-400 dark:border-slate-800 dark:text-slate-100"
        />
        <CommandPrimitive.List class="max-h-80 overflow-y-auto p-1">
          <CommandPrimitive.Empty class="px-3 py-6 text-center text-xs text-slate-400">
            No matching commands.
          </CommandPrimitive.Empty>
          {#each visibleGroups as group (group.heading)}
            <CommandPrimitive.Group>
              <CommandPrimitive.GroupHeading class="px-2 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                {group.heading}
              </CommandPrimitive.GroupHeading>
              <CommandPrimitive.GroupItems>
                {#each group.items || [] as item (item.id || item.label)}
                  <CommandPrimitive.Item
                    value={item.label}
                    keywords={item.keywords || []}
                    disabled={item.disabled}
                    onSelect={() => run(item)}
                    class={cn(
                      "flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-xs text-slate-700 outline-none",
                      "data-selected:bg-indigo-50 data-selected:text-indigo-700",
                      "dark:text-slate-200 dark:data-selected:bg-indigo-950/50 dark:data-selected:text-indigo-200",
                      "data-disabled:opacity-40"
                    )}
                  >
                    {#if item.icon}
                      {@const Icon = item.icon}
                      <Icon class="size-3.5 shrink-0" />
                    {/if}
                    <span class="flex-1 truncate">{item.label}</span>
                    {#if item.shortcut}
                      <Kbd keys={item.shortcut} />
                    {/if}
                  </CommandPrimitive.Item>
                {/each}
              </CommandPrimitive.GroupItems>
            </CommandPrimitive.Group>
          {/each}
        </CommandPrimitive.List>
      </CommandPrimitive.Root>
    </DialogPrimitive.Content>
  </DialogPrimitive.Portal>
</DialogPrimitive.Root>
