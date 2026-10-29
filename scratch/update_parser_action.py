path = 'crates/bosc-syntax/src/parser.rs'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = """                | "container"
                | "systempart"
                | "usercontrol"
                | "label"
                | "image"
                | "spacer"
                | "divider"
                | "wrap\""""

replacement = """                | "container"
                | "systempart"
                | "usercontrol"
                | "label"
                | "image"
                | "spacer"
                | "divider"
                | "action"
                | "button"
                | "cta"
                | "wrap\""""

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated parser.rs successfully")
