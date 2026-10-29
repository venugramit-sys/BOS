import re

# Update StyleEditor.svelte
path = 'ui/src/lib/components/studio/designers/page/StyleEditor.svelte'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

target1 = '''let { style = {}, schema, inherited = {}, onSet } = $props();'''
replacement1 = '''let {
    style = {},
    schema,
    inherited = {},
    activeIcon = null,
    onSet,
    onSetIcon = null
  } = $props();

  const effectiveIcon = $derived(value("icon") || activeIcon || inherited?.icon || null);

  function handleIconChange(newIcon) {
    if (onSetIcon) {
      onSetIcon(newIcon);
    } else {
      onSet("icon", newIcon);
    }
  }'''
assert target1 in content, "target1 not found"
content = content.replace(target1, replacement1, 1)

target2 = '''      <!-- Icon Selector & Preview -->
      <div class="flex flex-col gap-2 rounded-lg border border-slate-200/80 bg-white p-2.5 shadow-2xs dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <span class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-1.5">
            <span>Icon</span>
            {#if value("icon")}
              <span class="rounded bg-indigo-50 px-1.5 py-0.2 text-[10px] font-mono text-indigo-600 dark:bg-indigo-950 dark:text-indigo-400">
                active
              </span>
            {/if}
          </span>
          {#if value("icon")}
            <button
              type="button"
              class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-rose-600 dark:hover:text-rose-400"
              onclick={() => {
                onSet("icon", null);
                onSet("iconSize", null);
                onSet("iconColor", null);
                onSet("iconBackground", null);
                onSet("iconRadius", null);
              }}
            >
              <X class="size-3" />Clear Icon
            </button>
          {/if}
        </div>

        <IconPicker
          value={value("icon")}
          placeholder="Select an icon..."
          onChange={(icon) => onSet("icon", icon)}
          onSelect={(icon) => onSet("icon", icon)}
        />

        {#if value("icon")}
          <!-- Live Preview Box -->
          <div class="flex items-center gap-3 rounded-lg border border-slate-100 bg-slate-50/70 p-2 dark:border-slate-800/80 dark:bg-slate-950/60 mt-1">
            <div class="flex size-12 shrink-0 items-center justify-center rounded-lg border border-slate-200 bg-white shadow-2xs dark:border-slate-800 dark:bg-slate-900 overflow-hidden">
              <Icon
                name={value("icon")}
                size={value("iconSize") ?? "md"}
                color={value("iconColor")}
                background={value("iconBackground")}
                radius={value("iconRadius")}
              />
            </div>
            <div class="flex flex-col min-w-0">
              <span class="font-mono text-[11px] font-bold text-slate-800 dark:text-slate-200 truncate">{value("icon")}</span>
              <span class="text-[10px] text-slate-400">
                Size: {value("iconSize") ?? "md (default)"} · Color: {value("iconColor") ?? "inherit"}
              </span>
            </div>
          </div>'''

replacement2 = '''      <!-- Icon Selector & Preview -->
      <div class="flex flex-col gap-2 rounded-lg border border-slate-200/80 bg-white p-2.5 shadow-2xs dark:border-slate-800 dark:bg-slate-900">
        <div class="flex items-center justify-between">
          <div class="flex flex-col">
            <span class="font-semibold text-slate-800 dark:text-slate-100 flex items-center gap-1.5">
              <span>Icon</span>
              {#if effectiveIcon}
                <span class="rounded bg-indigo-50 px-1.5 py-0.2 text-[10px] font-mono text-indigo-600 dark:bg-indigo-950 dark:text-indigo-400">
                  active
                </span>
              {/if}
            </span>
            <span class="text-[10px] text-slate-400">
              {effectiveIcon ? "Synced with element properties" : "Assign an icon and customize its appearance"}
            </span>
          </div>
          {#if effectiveIcon}
            <button
              type="button"
              class="flex items-center gap-0.5 text-[10px] text-slate-400 hover:text-rose-600 dark:hover:text-rose-400"
              onclick={() => {
                handleIconChange(null);
                onSet("icon", null);
                onSet("iconSize", null);
                onSet("iconColor", null);
                onSet("iconBackground", null);
                onSet("iconRadius", null);
              }}
            >
              <X class="size-3" />Clear Icon
            </button>
          {/if}
        </div>

        <IconPicker
          value={effectiveIcon}
          placeholder="Select an icon..."
          onChange={handleIconChange}
          onSelect={handleIconChange}
        />

        {#if effectiveIcon}
          <!-- Live Preview Box -->
          <div class="flex items-center gap-3 rounded-lg border border-slate-100 bg-slate-50/70 p-2.5 dark:border-slate-800/80 dark:bg-slate-950/60 mt-1">
            <div class="flex size-12 shrink-0 items-center justify-center rounded-lg border border-slate-200 bg-white shadow-2xs dark:border-slate-800 dark:bg-slate-900 overflow-hidden">
              <Icon
                name={effectiveIcon}
                size={value("iconSize") ?? "md"}
                color={value("iconColor")}
                background={value("iconBackground")}
                radius={value("iconRadius")}
              />
            </div>
            <div class="flex flex-col min-w-0">
              <span class="font-mono text-[11px] font-bold text-slate-800 dark:text-slate-200 truncate">{effectiveIcon}</span>
              <span class="text-[10px] text-slate-500 dark:text-slate-400">
                Size: {value("iconSize") ?? "md (default)"} · Color: {value("iconColor") ?? "inherit"}
              </span>
            </div>
          </div>'''

assert target2 in content, "target2 not found"
content = content.replace(target2, replacement2, 1)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print("StyleEditor.svelte updated successfully!")
