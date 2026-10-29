with open('ui/src/lib/bos-ui/combobox/Combobox.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

target = """    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
    }
  }

  function handleSearchKeyDown(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const nextIndex = Math.min(activeIndex + 1, visibleItems.length - 1);
      activeIndex = nextIndex;
      scrollActiveIntoView(nextIndex);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const prevIndex = Math.max(activeIndex - 1, 0);
      activeIndex = prevIndex;
      scrollActiveIntoView(prevIndex);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (visibleItems[activeIndex]) {
        selectItem(visibleItems[activeIndex]);
      }"""

replacement = """    } else if (e.key === "Escape" && open) {
      e.preventDefault();
      open = false;
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (!open) {
        e.preventDefault();
        open = true;
        searchQuery = e.key;
      }
    }
  }

  function handleSearchKeyDown(e) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const nextIndex = Math.min(activeIndex + 1, visibleItems.length - 1);
      activeIndex = nextIndex;
      scrollActiveIntoView(nextIndex);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const prevIndex = Math.max(activeIndex - 1, 0);
      activeIndex = prevIndex;
      scrollActiveIntoView(prevIndex);
    } else if (e.key === "Enter" || e.key === "Tab") {
      if (visibleItems[activeIndex]) {
        e.preventDefault();
        selectItem(visibleItems[activeIndex]);
      }"""

target_crlf = target.replace('\n', '\r\n')
replacement_crlf = replacement.replace('\n', '\r\n')

if target in content:
    content = content.replace(target, replacement, 1)
    print("Combobox.svelte updated (LF)")
elif target_crlf in content:
    content = content.replace(target_crlf, replacement_crlf, 1)
    print("Combobox.svelte updated (CRLF)")
else:
    print("Target not found in Combobox.svelte")

with open('ui/src/lib/bos-ui/combobox/Combobox.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("Combobox.svelte update complete")
