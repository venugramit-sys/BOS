# Update PageSurface.svelte
with open('ui/src/lib/components/shell/PageSurface.svelte', 'r', encoding='utf-8') as f:
    ps_content = f.read()

target_ps = """  function onRecordChange(recordId) {
    // A new record got its id: keep the tab, re-key its params so a reload shows the saved record.
    shell.openTabs = shell.openTabs.map((t) => (t.id === tab.id ? { ...t, params: { ...(t.params ?? {}), recordId, mode: "edit" } } : t));
  }
</script>

{#key tab.id}
  <PageHost {page} {params} {openPage} {closePage} {onRecordChange} navigate={(url) => shell.navigate(url)} />
{/key}"""

replacement_ps = """  function onRecordChange(recordId) {
    // A new record got its id: keep the tab, re-key its params so a reload shows the saved record.
    shell.openTabs = shell.openTabs.map((t) => (t.id === tab.id ? { ...t, params: { ...(t.params ?? {}), recordId, mode: "edit" } } : t));
  }

  function onDirtyChange(isDirty) {
    shell.openTabs = shell.openTabs.map((t) => (t.id === tab.id ? { ...t, isDirty } : t));
  }
</script>

{#key tab.id}
  <PageHost {page} {params} {openPage} {closePage} {onRecordChange} {onDirtyChange} navigate={(url) => shell.navigate(url)} />
{/key}"""

if target_ps in ps_content:
    ps_content = ps_content.replace(target_ps, replacement_ps, 1)
    print("PageSurface.svelte updated (LF)")
elif target_ps.replace('\n', '\r\n') in ps_content:
    ps_content = ps_content.replace(target_ps.replace('\n', '\r\n'), replacement_ps.replace('\n', '\r\n'), 1)
    print("PageSurface.svelte updated (CRLF)")
else:
    print("Target not found in PageSurface.svelte")

with open('ui/src/lib/components/shell/PageSurface.svelte', 'w', encoding='utf-8') as f:
    f.write(ps_content)

# Update PageHost.svelte
with open('ui/src/lib/page-kit/PageHost.svelte', 'r', encoding='utf-8') as f:
    ph_content = f.read()

target_ph1 = "modal = false } = $props();"
replacement_ph1 = "modal = false, onDirtyChange = undefined } = $props();"

target_ph2 = "onActionError\n  });"
replacement_ph2 = "onActionError,\n    onDirtyChange\n  });"

if target_ph1 in ph_content:
    ph_content = ph_content.replace(target_ph1, replacement_ph1, 1)
    print("PageHost.svelte props updated")

if target_ph2 in ph_content:
    ph_content = ph_content.replace(target_ph2, replacement_ph2, 1)
    print("PageHost.svelte ctx updated (LF)")
elif target_ph2.replace('\n', '\r\n') in ph_content:
    ph_content = ph_content.replace(target_ph2.replace('\n', '\r\n'), replacement_ph2.replace('\n', '\r\n'), 1)
    print("PageHost.svelte ctx updated (CRLF)")

with open('ui/src/lib/page-kit/PageHost.svelte', 'w', encoding='utf-8') as f:
    f.write(ph_content)

print("PageSurface and PageHost updates complete")
