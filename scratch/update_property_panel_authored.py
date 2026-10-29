with open('ui/src/lib/components/studio/designers/page/PropertyPanel.svelte', 'r', encoding='utf-8') as f:
    c = f.read()

target = """      {#if !authored}
        <div class="p-2 rounded-lg bg-amber-50 dark:bg-amber-950/60 text-amber-800 dark:text-amber-200 text-[11px] border border-amber-200 dark:border-amber-800 flex items-start gap-1.5">
          <Info class="size-3.5 shrink-0 mt-0.5 text-amber-600" />
          <span>Defined by a page extension. Edit that extension object directly in code or designer.</span>
        </div>
      {/if}"""

repl = """      {#if !authored}
        <div class="p-2.5 rounded-lg bg-amber-50 dark:bg-amber-950/60 text-amber-800 dark:text-amber-200 text-[11px] border border-amber-200 dark:border-amber-800 flex items-start gap-2">
          <Info class="size-4 shrink-0 mt-0.5 text-amber-600" />
          <div>
            {#if model.objectKind === "pageextension"}
              <span>Defined by the base page <strong>{model.targetPage}</strong>. Switch to the <strong>{model.targetPage}</strong> tab to edit this element directly.</span>
            {:else}
              <span>Defined by a page extension. Edit that extension object directly in code or designer.</span>
            {/if}
          </div>
        </div>
      {/if}"""

is_crlf = '\r\n' in c
if is_crlf:
    target = target.replace('\n', '\r\n')
    repl = repl.replace('\n', '\r\n')

assert target in c, "target not found"
c = c.replace(target, repl, 1)

with open('ui/src/lib/components/studio/designers/page/PropertyPanel.svelte', 'w', encoding='utf-8', newline='') as f:
    f.write(c)

print("Updated PropertyPanel.svelte successfully")
