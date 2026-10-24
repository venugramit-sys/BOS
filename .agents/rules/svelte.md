---
trigger: glob
description: "Svelte 5 UI rules for ui/"
globs: "ui/**/*.svelte, ui/**/*.js, ui/**/*.ts, ui/**/*.css"
---
# Svelte 5 UI (ui/)
- Read ui/AGENTS.md first. Runes only ($state, $derived, $props, $effect); no `$:`; no effect mutation loops; clean teardown.
- shadcn-svelte/bits-ui, Tailwind, Lucide; responsive (375/1024/1920); WAI-ARIA; themeable; real backend data only.
- page-kit (ui/src/lib/page-kit) is a contract with crates/bosc-codegen-svelte: change both together, run npm run test:page-kit and the golden tests.
- Never read node_modules/, dist/, .bos-pages/.
