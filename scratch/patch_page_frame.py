with open('ui/src/lib/page-kit/PageFrame.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

target = '''  {:else if p.error}
    <Banner tone="danger" title="Something went wrong" dismissible onDismiss={() => (p.error = "")}>
      <p class="text-xs">{p.error}</p>
    </Banner>
  {/if}'''

replacement = '''  {:else if p.error}
    <Banner
      tone="danger"
      title={p.meta?.page_type === "StandardDialog" && p.meta?.name?.toLowerCase().includes("login") ? "Sign In Failed" : (p.errorTitle || "Action Failed")}
      dismissible
      onDismiss={() => { p.error = ""; p.errorTitle = ""; }}
    >
      <p class="text-xs">{p.error}</p>
    </Banner>
  {/if}'''

assert target in content, "target not found in PageFrame.svelte"
content = content.replace(target, replacement)

with open('ui/src/lib/page-kit/PageFrame.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("PageFrame.svelte updated with contextual error banner title!")
