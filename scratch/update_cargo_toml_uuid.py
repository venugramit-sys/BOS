path = 'Cargo.toml'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = 'uuid = { version = "1.10", features = ["v4", "serde"] }'
replacement = 'uuid = { version = "1.10", features = ["v4", "v7", "serde"] }'

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated root Cargo.toml with uuid v7 feature")
