path = 'ui/src/lib/page-kit/index.js'
with open(path, 'r', encoding='utf-8') as f:
    text = f.read()

target = 'export { default as WizardSteps } from "./WizardSteps.svelte";\nexport { default as Icon } from "./Icon.svelte";'
replacement = 'export { default as WizardSteps } from "./WizardSteps.svelte";\nexport { default as Action } from "./Action.svelte";\nexport { default as Icon } from "./Icon.svelte";'

assert target in text, "target not found"
text = text.replace(target, replacement, 1)

with open(path, 'w', encoding='utf-8', newline='') as f:
    f.write(text)

print("Updated ui/src/lib/page-kit/index.js")
