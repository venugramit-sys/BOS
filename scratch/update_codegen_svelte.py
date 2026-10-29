path = 'crates/bosc-codegen-svelte/src/lib.rs'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = '            K::WizardStep => "WizardSteps",\n            _ => "Container",'
replacement = '            K::WizardStep => "WizardSteps",\n            K::Action => "Action",\n            _ => "Container",'

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated bosc-codegen-svelte/src/lib.rs successfully")
