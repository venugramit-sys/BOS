with open('ui/src/lib/bos-ui/input/Input.svelte', 'r', encoding='utf-8') as f:
    content = f.read()

target_state = '''  let isMaskRevealed = $state(false);
  let showCalcBadge = $state(false);
  let lastFormula = $state("");
  let calcBadgeTimer = $state(null);'''

replacement_state = '''  let isMaskRevealed = $state(false);
  let showCalcBadge = $state(false);
  let lastFormula = $state("");
  let calcBadgeTimer = $state(null);
  let isCapsLockOn = $state(false);'''

target_key = '''  function handleKeyDown(e) {
    if (e.key === "F8") {'''

replacement_key = '''  function handleKeyDown(e) {
    if (type === "password" && typeof e.getModifierState === "function") {
      isCapsLockOn = e.getModifierState("CapsLock");
    }
    if (e.key === "F8") {'''

target_blur = '''  function handleBlur() {
    isFocused = false;
    executeMathEvaluation();
  }'''

replacement_blur = '''  function handleBlur() {
    isFocused = false;
    isCapsLockOn = false;
    executeMathEvaluation();
  }'''

target_button = '''      <!-- Password Reveal / Mask Toggle -->
      {#if type === "password" && !disabled}
        <button
          type="button"
          onclick={() => (showPassword = !showPassword)}
          class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          title={showPassword ? "Hide password" : "Show password"}
          aria-label={showPassword ? "Hide password" : "Show password"}
        >
          {#if showPassword}
            <EyeOff class={surface.iconClass} />
          {:else}
            <Eye class={surface.iconClass} />
          {/if}
        </button>
      {/if}'''

replacement_button = '''      <!-- CapsLock Warning Indicator -->
      {#if isCapsLockOn && type === "password"}
        <span
          class="shrink-0 inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-800 animate-in fade-in"
          title="Caps Lock is on"
          aria-label="Caps Lock is on"
        >
          <span>Caps Lock</span>
        </span>
      {/if}

      <!-- Password Reveal / Mask Toggle -->
      {#if type === "password" && !disabled}
        <button
          type="button"
          onclick={() => (showPassword = !showPassword)}
          class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          title={showPassword ? "Hide password" : "Show password"}
          aria-label={showPassword ? "Hide password" : "Show password"}
        >
          {#if showPassword}
            <EyeOff class={surface.iconClass} />
          {:else}
            <Eye class={surface.iconClass} />
          {/if}
        </button>
      {/if}'''

assert target_state in content, "target_state not found"
assert target_key in content, "target_key not found"
assert target_blur in content, "target_blur not found"
assert target_button in content, "target_button not found"

content = content.replace(target_state, replacement_state)
content = content.replace(target_key, replacement_key)
content = content.replace(target_blur, replacement_blur)
content = content.replace(target_button, replacement_button)

with open('ui/src/lib/bos-ui/input/Input.svelte', 'w', encoding='utf-8') as f:
    f.write(content)
print("Input.svelte updated with CapsLock indicator successfully!")
