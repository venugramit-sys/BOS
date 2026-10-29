import re

path = 'ui/src/lib/components/studio/designers/page/IconPicker.svelte'
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Add hoveredIcon state
target1 = 'let selectedCategory = $state("all");'
replacement1 = 'let selectedCategory = $state("all");\n  let hoveredIcon = $state(null);'
assert target1 in content, "target1 not found"
content = content.replace(target1, replacement1, 1)

# 2. Update trigger button preview icon size: size-5 -> size-6, size-3.5 -> size-4
target2 = '''        {#if value}
          <div class="flex size-5 shrink-0 items-center justify-center rounded bg-indigo-50 text-indigo-600 dark:bg-indigo-950/70 dark:text-indigo-400">
            <Icon name={value} class="size-3.5" />
          </div>'''
replacement2 = '''        {#if value}
          <div class="flex size-6 shrink-0 items-center justify-center rounded-md bg-indigo-50 text-indigo-600 dark:bg-indigo-950/70 dark:text-indigo-400 border border-indigo-200/50 dark:border-indigo-800/50">
            <Icon name={value} class="size-4" />
          </div>'''
assert target2 in content, "target2 not found"
content = content.replace(target2, replacement2, 1)

# 3. Update popover panel to include live hover/selected preview bar and spacious 5-column grid with 24px (size-6) icons
target3 = '''      <!-- Category Filter Pills -->
      <div class="flex items-center gap-1 overflow-x-auto pb-1 text-[10px] scrollbar-none">
        {#each CATEGORIES as cat (cat.id)}
          <button
            type="button"
            class={"rounded-full px-2 py-0.5 font-medium whitespace-nowrap transition-colors " +
              (selectedCategory === cat.id
                ? "bg-indigo-600 text-white font-semibold"
                : "bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-400 dark:hover:bg-slate-700")}
            onclick={() => {
              selectedCategory = cat.id;
              query = "";
            }}
          >
            {cat.label}
          </button>
        {/each}
      </div>

      <!-- Icon Grid -->
      <div
        class="grid max-h-56 grid-cols-6 sm:grid-cols-8 gap-1 overflow-y-auto p-1 rounded-lg border border-slate-100 bg-slate-50/30 dark:border-slate-800/60 dark:bg-slate-900/40"
        role="listbox"
        aria-label="Available Icons"
      >
        {#each matches as name (name)}
          <button
            type="button"
            role="option"
            aria-selected={value === name}
            title={name}
            class={"group relative flex aspect-square items-center justify-center rounded-lg p-1.5 transition-all " +
              (value === name
                ? "bg-indigo-600 text-white shadow-xs font-bold"
                : "text-slate-700 hover:bg-indigo-50 hover:text-indigo-600 dark:text-slate-300 dark:hover:bg-indigo-950/70 dark:hover:text-indigo-400")}
            onclick={() => {
              triggerChange(name);
              open = false;
            }}
          >
            <Icon {name} class="size-4 transition-transform group-hover:scale-110" />
            {#if value === name}
              <div class="absolute -top-1 -right-1 flex size-3 items-center justify-center rounded-full bg-white text-indigo-600 shadow-xs">
                <Check class="size-2" />
              </div>
            {/if}
          </button>
        {/each}'''

replacement3 = '''      <!-- Category Filter Pills -->
      <div class="flex items-center gap-1 overflow-x-auto pb-1 text-[10px] scrollbar-none">
        {#each CATEGORIES as cat (cat.id)}
          <button
            type="button"
            class={"rounded-full px-2.5 py-1 font-medium whitespace-nowrap transition-colors " +
              (selectedCategory === cat.id
                ? "bg-indigo-600 text-white font-semibold shadow-2xs"
                : "bg-slate-100 text-slate-600 hover:bg-slate-200 dark:bg-slate-800 dark:text-slate-400 dark:hover:bg-slate-700")}
            onclick={() => {
              selectedCategory = cat.id;
              query = "";
            }}
          >
            {cat.label}
          </button>
        {/each}
      </div>

      <!-- Live Hover / Active Preview Bar -->
      <div class="flex items-center gap-2.5 px-2.5 py-1.5 rounded-lg bg-indigo-50/80 dark:bg-indigo-950/50 border border-indigo-100 dark:border-indigo-900/60 min-h-10">
        <div class="flex size-7 shrink-0 items-center justify-center rounded-md bg-white dark:bg-slate-900 border border-indigo-200/80 dark:border-indigo-800/80 shadow-2xs">
          <Icon name={hoveredIcon || value || "sparkles"} class="size-5 text-indigo-600 dark:text-indigo-400" />
        </div>
        <div class="flex flex-col min-w-0">
          <span class="font-mono text-xs font-bold text-slate-900 dark:text-slate-100 truncate">
            {hoveredIcon || value || "Browse icons below"}
          </span>
          <span class="text-[10px] text-slate-500 dark:text-slate-400 truncate">
            {hoveredIcon ? "Click cell to apply" : value ? "Currently selected icon" : "1,686 Lucide vector icons available"}
          </span>
        </div>
      </div>

      <!-- Icon Grid: Spacious 5-column layout with large 24px (size-6) icons -->
      <div
        class="grid max-h-60 grid-cols-5 gap-1.5 overflow-y-auto p-1.5 rounded-xl border border-slate-100 bg-slate-50/40 dark:border-slate-800/70 dark:bg-slate-900/50"
        role="listbox"
        aria-label="Available Icons"
      >
        {#each matches as name (name)}
          <button
            type="button"
            role="option"
            aria-selected={value === name}
            title={name}
            class={"group relative flex aspect-square min-h-[46px] items-center justify-center rounded-xl p-2 transition-all " +
              (value === name
                ? "bg-indigo-600 text-white shadow-sm ring-2 ring-indigo-600 ring-offset-2 dark:ring-offset-slate-900 font-bold"
                : "border border-slate-200/70 bg-white text-slate-700 hover:border-indigo-400 hover:bg-indigo-50 hover:text-indigo-600 dark:border-slate-800 dark:bg-slate-900/90 dark:text-slate-300 dark:hover:border-indigo-700 dark:hover:bg-indigo-950/70 dark:hover:text-indigo-300 shadow-2xs")}
            onmouseenter={() => (hoveredIcon = name)}
            onmouseleave={() => (hoveredIcon = null)}
            onclick={() => {
              triggerChange(name);
              open = false;
            }}
          >
            <Icon {name} class="size-6 transition-transform group-hover:scale-115" />
            {#if value === name}
              <div class="absolute -top-1 -right-1 flex size-3.5 items-center justify-center rounded-full bg-white text-indigo-600 shadow-xs border border-indigo-200">
                <Check class="size-2.5 stroke-[3]" />
              </div>
            {/if}
          </button>
        {/each}'''

assert target3 in content, "target3 not found"
content = content.replace(target3, replacement3, 1)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print("IconPicker.svelte updated successfully!")
