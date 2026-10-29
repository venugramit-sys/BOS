with open('ui/src/lib/page-kit/ActionMenu.svelte', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

old_props = """  /** @type {{ label: string, sections: { caption?: string, items: any[] }[], onrun: (a: any) => void, enabled: (a: any) => boolean, class?: string }} */
  let { label, sections = [], onrun, enabled, class: className = "" } = $props();"""

new_props = """  /** @type {{ label: string, icon?: string, sections: { caption?: string, items: any[] }[], onrun: (a: any) => void, enabled: (a: any) => boolean, class?: string }} */
  let { label, icon = "", sections = [], onrun, enabled, class: className = "" } = $props();"""

assert old_props in text_lf, "old_props not found"
text_lf = text_lf.replace(old_props, new_props, 1)

old_btn = """  <button type="button" class="bos-action" aria-haspopup="menu" aria-expanded={open} aria-controls={menuId} onclick={() => (open = !open)}>
    {label}
    <ChevronDown class="size-3.5" />
  </button>"""

new_btn = """  <button type="button" class="bos-action inline-flex items-center gap-1.5" aria-haspopup="menu" aria-expanded={open} aria-controls={menuId} onclick={() => (open = !open)}>
    {#if icon}<Icon name={icon} class="size-3.5 shrink-0 opacity-70" />{/if}
    <span>{label}</span>
    <ChevronDown class="size-3.5 shrink-0 opacity-60" />
  </button>"""

assert old_btn in text_lf, "old_btn not found"
text_lf = text_lf.replace(old_btn, new_btn, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/ActionMenu.svelte', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("ActionMenu.svelte updated with icon support")
