path = 'ui/src/lib/page-kit/PageFrame.svelte'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target1 = """  const pageStyle = $derived(styleVars(p.meta.style));
  const status = $derived(dialog ? "" : p.mode === "create" ? "New" : p.dirty ? "Unsaved changes" : "");"""

replacement1 = """  const pageStyle = $derived(styleVars(p.meta.style));
  const status = $derived(dialog ? "" : p.mode === "create" ? "New" : p.dirty ? "Unsaved changes" : "");
  const showHeader = $derived(
    (!p.embedded || p.isList) &&
    p.meta.properties?.showHeader !== "false" &&
    p.meta.properties?.show_header !== "false"
  );
  const showActionBar = $derived(
    !dialog &&
    p.meta.properties?.showActionBar !== "false" &&
    p.meta.properties?.show_action_bar !== "false"
  );"""

assert target1 in text, "target1 not found"
text = text.replace(target1, replacement1, 1)

target2 = "{#if !p.embedded || p.isList}\n    <header class=\"bos-page-header\""
replacement2 = "{#if showHeader}\n    <header class=\"bos-page-header\""
assert target2 in text, "target2 not found"
text = text.replace(target2, replacement2, 1)

target3 = "{#if !dialog}<ActionBar {p} {actions} />{/if}"
replacement3 = "{#if showActionBar}<ActionBar {p} {actions} />{/if}"
assert target3 in text, "target3 not found"
text = text.replace(target3, replacement3, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated ui/src/lib/page-kit/PageFrame.svelte successfully")
