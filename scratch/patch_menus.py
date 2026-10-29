import re

for filepath in ["packages/bos.auth/auth.bos", "packages/bos.auth/package.bos"]:
    with open(filepath, "rb") as f:
        text = f.read().decode("utf-8")
    text = text.replace("\r\n", "\n")

    target = """        item(Databases) {
            caption = "Databases & Companies";
            icon = "Database";
            page = "databases";
        }"""
    
    if target in text:
        text = text.replace(target, "")
        with open(filepath, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"SUCCESS: Removed Databases item from {filepath}")
    else:
        print(f"WARNING: target not found in {filepath}")
