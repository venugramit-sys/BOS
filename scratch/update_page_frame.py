with open('ui/src/lib/page-kit/PageFrame.svelte', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

# 1. Add Chevron imports
target_imports = 'import { Spinner } from "$lib/bos-ui/spinner/index.js";\n  import Icon from "./Icon.svelte";'
replacement_imports = 'import { Spinner } from "$lib/bos-ui/spinner/index.js";\n  import { ChevronLeft, ChevronRight } from "lucide-svelte";\n  import Icon from "./Icon.svelte";'

assert target_imports in text_lf, "target_imports not found"
text_lf = text_lf.replace(target_imports, replacement_imports, 1)

# 2. Add keyboard shortcuts in onKey
target_keys = """    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      p.save();
      return;
    }"""

replacement_keys = """    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      p.save();
      return;
    }

    if (e.key === "PageUp" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      p.prevRecord();
      return;
    }

    if (e.key === "PageDown" && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      p.nextRecord();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      if (p.meta?.insert_allowed && !dialog) {
        e.preventDefault();
        p.newRecord();
        return;
      }
    }

    if (e.key === "F5") {
      e.preventDefault();
      p.reload();
      return;
    }"""

assert target_keys in text_lf, "target_keys not found"
text_lf = text_lf.replace(target_keys, replacement_keys, 1)

# 3. Add record stepper and flex container to header
target_header = """      {#if !p.embedded && !p.ctx.modal}
        <div class="min-w-0" {...p.design ? { "data-bos-node": "header", "data-bos-kind": "header" } : {}}>
          <h1 class="bos-page-title" style={pageStyle} {...p.design ? { "data-bos-node": "header:title", "data-bos-kind": "headline", title: "Click to select Page Title & Typography" } : {}}>
            {#if p.meta.style?.icon}
              <Icon
                name={p.meta.style.icon}
                class="shrink-0"
                size={p.meta.style.iconSize ?? "md"}
                color={p.meta.style.iconColor}
                background={p.meta.style.iconBackground}
                radius={p.meta.style.iconRadius}
              />
            {/if}
            <span class="truncate">{p.caption}</span>
            {#if p.dataCaption}<span class="bos-page-subtitle">{p.dataCaption}</span>{/if}
          </h1>
          {#if p.meta.instructional_text}
            <p class="bos-hint" {...p.design ? { "data-bos-node": "header:instruction", "data-bos-kind": "label", title: "Click to select Subtitle / Instructional Text" } : {}}>{p.meta.instructional_text}</p>
          {/if}
        </div>
        {#if status}<span class="bos-status">{status}</span>{/if}
      {/if}"""

replacement_header = """      {#if !p.embedded && !p.ctx.modal}
        <div class="flex items-center justify-between gap-3 min-w-0 flex-1" {...p.design ? { "data-bos-node": "header", "data-bos-kind": "header" } : {}}>
          <div class="min-w-0">
            <h1 class="bos-page-title" style={pageStyle} {...p.design ? { "data-bos-node": "header:title", "data-bos-kind": "headline", title: "Click to select Page Title & Typography" } : {}}>
              {#if p.meta.style?.icon}
                <Icon
                  name={p.meta.style.icon}
                  class="shrink-0"
                  size={p.meta.style.iconSize ?? "md"}
                  color={p.meta.style.iconColor}
                  background={p.meta.style.iconBackground}
                  radius={p.meta.style.iconRadius}
                />
              {/if}
              <span class="truncate">{p.caption}</span>
              {#if p.dataCaption}<span class="bos-page-subtitle">{p.dataCaption}</span>{/if}
            </h1>
            {#if p.meta.instructional_text}
              <p class="bos-hint" {...p.design ? { "data-bos-node": "header:instruction", "data-bos-kind": "label", title: "Click to select Subtitle / Instructional Text" } : {}}>{p.meta.instructional_text}</p>
            {/if}
          </div>

          <div class="flex items-center gap-2 shrink-0">
            {#if p.canNavigateRecords}
              <div class="bos-record-stepper inline-flex items-center rounded-md border border-[var(--bos-color-border)] bg-[var(--bos-color-surface)] shadow-2xs select-none">
                <button
                  type="button"
                  class="p-1.5 rounded-l-md hover:bg-[var(--bos-color-surfaceMuted)] disabled:opacity-30 disabled:pointer-events-none cursor-pointer text-slate-600 dark:text-slate-300 transition-colors"
                  disabled={!p.hasPrevRecord || p.stepping}
                  onclick={() => p.prevRecord()}
                  title="Previous record (Ctrl+PageUp)"
                  aria-label="Previous record"
                >
                  <ChevronLeft class="size-3.5" />
                </button>
                <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400 px-2 min-w-[3.5rem] text-center font-mono">
                  {#if p.stepping}
                    <Spinner size="xs" />
                  {:else}
                    {p.recordPositionText}
                  {/if}
                </span>
                <button
                  type="button"
                  class="p-1.5 rounded-r-md hover:bg-[var(--bos-color-surfaceMuted)] disabled:opacity-30 disabled:pointer-events-none cursor-pointer text-slate-600 dark:text-slate-300 transition-colors"
                  disabled={!p.hasNextRecord || p.stepping}
                  onclick={() => p.nextRecord()}
                  title="Next record (Ctrl+PageDown)"
                  aria-label="Next record"
                >
                  <ChevronRight class="size-3.5" />
                </button>
              </div>
            {/if}
            {#if status}<span class="bos-status">{status}</span>{/if}
          </div>
        </div>
      {/if}"""

assert target_header in text_lf, "target_header not found"
text_lf = text_lf.replace(target_header, replacement_header, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/PageFrame.svelte', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("PageFrame.svelte updated with record stepper & shortcuts")
