path = 'crates/bosc-analyzer/src/page_source.rs'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = '    "wizardstep", "container", "systempart", "usercontrol", "label", "image", "spacer", "divider", "wrap", "replace",'
replacement = '    "wizardstep", "container", "systempart", "usercontrol", "label", "image", "spacer", "divider", "action", "button", "cta", "wrap", "replace",'

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated page_source.rs successfully")
