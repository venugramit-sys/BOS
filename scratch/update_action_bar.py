with open('ui/src/lib/page-kit/ActionBar.svelte', 'r', encoding='utf-8') as f:
    text = f.read()

has_crlf = '\r\n' in text
text_lf = text.replace('\r\n', '\n')

old_areas = """  const AREAS = ["Processing", "Navigation", "Reporting", "Creation"];
  const AREA_CAPTION = { Processing: "Actions", Navigation: "Related", Reporting: "Reports", Creation: "New document" };"""

new_areas = """  const AREAS = ["Processing", "Navigation", "Reporting", "Creation"];
  const AREA_CAPTION = { Processing: "Actions", Navigation: "Related", Reporting: "Reports", Creation: "New document" };
  const AREA_ICON = { Processing: "play", Navigation: "link-2", Reporting: "file-text", Creation: "plus-circle" };"""

assert old_areas in text_lf, "old_areas not found"
text_lf = text_lf.replace(old_areas, new_areas, 1)

old_return = """      return { area, label: AREA_CAPTION[area], sections: groups };"""
new_return = """      return { area, label: AREA_CAPTION[area], icon: AREA_ICON[area], sections: groups };"""

assert old_return in text_lf, "old_return not found"
text_lf = text_lf.replace(old_return, new_return, 1)

old_menu_render = """  {#each menus as m (m.area)}
    <ActionMenu label={m.label} sections={m.sections} onrun={run} {enabled} />
  {/each}"""

new_menu_render = """  {#each menus as m (m.area)}
    <ActionMenu label={m.label} icon={m.icon} sections={m.sections} onrun={run} {enabled} />
  {/each}"""

assert old_menu_render in text_lf, "old_menu_render not found"
text_lf = text_lf.replace(old_menu_render, new_menu_render, 1)

if has_crlf:
    text_lf = text_lf.replace('\n', '\r\n')

with open('ui/src/lib/page-kit/ActionBar.svelte', 'w', encoding='utf-8') as f:
    f.write(text_lf)

print("ActionBar.svelte updated with area icons")
