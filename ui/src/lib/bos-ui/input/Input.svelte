<script>
  import { cn } from "$lib/utils.js";
  import {
    Eye,
    EyeOff,
    X,
    Copy,
    Check,
    Loader2,
    Plus,
    Minus,
    Search,
    Lock,
    Mail,
    Globe,
    Phone,
    User,
    MoreHorizontal,
    AlertCircle,
    AlertTriangle,
    CheckCircle2,
    Info,
    DollarSign,
    Hash,
    Calculator,
    Shield,
    ShieldOff
  } from "lucide-svelte";
  import { evaluateMath, maskValue, formatCurrencyNumber, formatWithPattern } from "./evaluator.js";
  import { resolveSurface } from "../theme/index.js";

  let {
    id = "",
    ref = $bindable(null),
    value = $bindable(""),
    type = "text",
    size = null,
    variant = null,
    rounded = null,
    pattern = "",
    status = "default",
    label = "",
    helperText = "",
    errorMessage = "",
    warningMessage = "",
    successMessage = "",
    placeholder = "",
    required = false,
    disabled = false,
    readonly = false,
    prefixText = "",
    prefixIcon = null,
    suffixText = "",
    suffixIcon = null,
    clearable = false,
    copyable = false,
    loading = false,
    stepper = false,
    autoUppercase = false,
    min = undefined,
    max = undefined,
    step = 1,
    maxlength = undefined,
    showCount = false,
    debounceMs = 300,
    // Enterprise Math & Masking extensions:
    evalMath = false,
    masked = false,
    maskType = "custom", // 'credit-card' | 'ssn' | 'iban' | 'custom'
    unmaskable = false,
    visibleCount = 4,
    decimals = 2,
    formatOnBlur = true,
    onCalculate = null,
    class: className = "",
    inputClass = "",
    onDebounce = null,
    onClear = null,
    onLookup = null,
    onCopy = null,
    onCopyAbove = null,
    onEnter = null,
    prefixSnippet = null,
    suffixSnippet = null,
    ...restProps
  } = $props();

  // Internal reactive states
  let showPassword = $state(false);
  let isCopied = $state(false);
  let debounceTimer = $state(null);
  let isFocused = $state(false);
  let isMaskRevealed = $state(false);
  let showCalcBadge = $state(false);
  let lastFormula = $state("");
  let calcBadgeTimer = $state(null);
  let isCapsLockOn = $state(false);

  // Resolved input type for native HTML input
  let resolvedNativeType = $derived.by(() => {
    if (type === "password") {
      return showPassword ? "text" : "password";
    }
    if (type === "number" || type === "currency") {
      return "text"; // Handled with numeric inputmode for maximum formatting control
    }
    if (type === "email") return "email";
    if (type === "url") return "url";
    if (type === "tel") return "tel";
    return "text";
  });

  // Effective status (auto-escalate if errorMessage provided)
  let resolvedStatus = $derived.by(() => {
    if (errorMessage) return "error";
    if (warningMessage) return "warning";
    if (successMessage) return "success";
    return status;
  });

  // Prefix icon is rendered only when explicitly provided
  let ResolvedPrefixIcon = $derived(prefixIcon || null);

  // Value length tracking
  let currentLength = $derived(String(value ?? "").length);

  let surface = $derived(resolveSurface("field", { size, variant, rounded, status: resolvedStatus }));

  // Display value formatting (masking & currency handling)
  let displayValue = $derived.by(() => {
    const rawStr = String(value ?? "");
    if (isFocused) {
      return rawStr;
    }
    if ((masked || type === "masked") && !isMaskRevealed && rawStr) {
      return maskValue(rawStr, maskType, visibleCount);
    }
    if (type === "currency" && formatOnBlur && rawStr !== "") {
      return formatCurrencyNumber(rawStr, decimals);
    }
    return rawStr;
  });

  // Debounced change trigger
  function handleInput(e) {
    let nextVal = e.target.value;

    // ERP Code: Auto uppercase enforcement
    if (autoUppercase || type === "code") {
      nextVal = nextVal.toUpperCase();
      e.target.value = nextVal;
    }

    // Live Pattern Masking (e.g. (###) ###-####, ##-#######, ###-##-####)
    if (pattern) {
      nextVal = formatWithPattern(nextVal, pattern);
      e.target.value = nextVal;
    }

    // Number constraint check (only if not an arithmetic formula)
    if (type === "number" && nextVal !== "" && !/[+\-*/]/.test(nextVal)) {
      const num = Number(nextVal);
      if (!isNaN(num)) {
        if (min !== undefined && num < min) nextVal = String(min);
        if (max !== undefined && num > max) nextVal = String(max);
      }
    }

    value = nextVal;

    if (onDebounce) {
      if (debounceTimer) clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        onDebounce(value);
      }, debounceMs);
    }
  }

  function executeMathEvaluation() {
    if (!evalMath && type !== "currency") return;
    if (value === null || value === undefined) return;

    const strVal = String(value).trim();
    if (!strVal || !/[+\-*/]/.test(strVal)) return;

    const evaluated = evaluateMath(strVal);
    if (evaluated !== null) {
      lastFormula = `${strVal} = ${evaluated}`;
      value = String(evaluated);
      showCalcBadge = true;
      if (calcBadgeTimer) clearTimeout(calcBadgeTimer);
      calcBadgeTimer = setTimeout(() => {
        showCalcBadge = false;
      }, 3500);
      onCalculate?.(evaluated, strVal);
    }
  }

  function handleFocus() {
    isFocused = true;
  }

  function handleBlur() {
    isFocused = false;
    isCapsLockOn = false;
    executeMathEvaluation();
  }

  function handleKeyDown(e) {
    if (type === "password" && typeof e.getModifierState === "function") {
      isCapsLockOn = e.getModifierState("CapsLock");
    }
    if (e.key === "F8") {
      e.preventDefault();
      onCopyAbove?.();
      return;
    }

    if (e.key === "Enter") {
      executeMathEvaluation();
      onEnter?.(value);
    } else if (e.key === "Escape") {
      if (clearable && value) {
        handleClear();
      } else {
        ref?.blur();
      }
    } else if ((type === "number" || stepper) && !disabled && !readonly) {
      if (e.key === "ArrowUp") {
        e.preventDefault();
        stepNumber(1);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        stepNumber(-1);
      }
    }
  }

  function handleClear() {
    value = "";
    onClear?.();
    ref?.focus();
    if (onDebounce) onDebounce("");
  }

  function stepNumber(delta) {
    const currentNum = Number(value) || 0;
    const stepVal = Number(step) || 1;
    let nextNum = currentNum + delta * stepVal;
    if (min !== undefined) nextNum = Math.max(min, nextNum);
    if (max !== undefined) nextNum = Math.min(max, nextNum);
    value = String(nextNum);
    onDebounce?.(value);
  }

  async function handleCopy() {
    if (!value) return;
    try {
      await navigator.clipboard.writeText(String(value));
      isCopied = true;
      onCopy?.(String(value));
      setTimeout(() => {
        isCopied = false;
      }, 2000);
    } catch {
      // Fallback
    }
  }
</script>

<div class={cn("flex flex-col gap-1 w-full text-left select-none font-sans", className)}>
  <!-- 1. Top Form Field Label & Optional Required Mark -->
  {#if label}
    <div class="flex items-center justify-between text-xs">
      <label class="font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
        <span>{label}</span>
        {#if required}
          <span class="text-rose-500 font-bold" title="Required field">*</span>
        {/if}
      </label>

      <!-- Optional Top Character Counter -->
      {#if showCount && maxlength}
        <span
          class={cn(
            "text-[10px] font-mono",
            currentLength >= maxlength
              ? "text-rose-500 font-bold"
              : currentLength >= maxlength * 0.85
                ? "text-amber-500"
                : "text-slate-400"
          )}
        >
          {currentLength} / {maxlength}
        </span>
      {/if}
    </div>
  {/if}

  <!-- 2. Integrated Composite Input Frame -->
  <div
    class={cn(
      "group relative flex items-center w-full transition-all overflow-hidden",
      surface.controlClass,
      disabled && "opacity-50 cursor-not-allowed bg-slate-100 dark:bg-slate-800/40 pointer-events-none",
      readonly && "bg-slate-50 dark:bg-slate-950/60"
    )}
  >
    <!-- Custom Leading Prefix Snippet Slot -->
    {#if prefixSnippet}
      <div class="shrink-0 flex items-center pl-2.5">
        {@render prefixSnippet()}
      </div>
    {/if}

    <!-- Static Addon Text (e.g. "https://", "$", "#") -->
    {#if prefixText}
      <span class="shrink-0 flex items-center px-2.5 bg-slate-100 dark:bg-slate-800 border-r border-slate-200 dark:border-slate-800 text-xs font-semibold text-slate-500 dark:text-slate-400 self-stretch">
        {prefixText}
      </span>
    {/if}

    <!-- Inferred or Custom Leading Icon -->
    {#if ResolvedPrefixIcon && !prefixSnippet && !prefixText}
      <div class="shrink-0 flex items-center pl-2.5 text-slate-400 dark:text-slate-500 pointer-events-none">
        <ResolvedPrefixIcon class={surface.iconClass} />
      </div>
    {/if}

    <!-- Core Primitive Native Input Element -->
    <input
      bind:this={ref}
      id={id || undefined}
      type={resolvedNativeType}
      value={displayValue}
      oninput={handleInput}
      onkeydown={handleKeyDown}
      onfocus={handleFocus}
      onblur={handleBlur}
      {placeholder}
      {disabled}
      {readonly}
      {maxlength}
      inputmode={evalMath ? "text" : (type === "number" ? "numeric" : (type === "currency" ? "decimal" : undefined))}
      data-slot="bos-input"
      class={cn(
        "flex-1 min-w-0 bg-transparent px-2.5 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500 outline-hidden font-normal",
        (autoUppercase || type === "code") && "font-mono uppercase tracking-wider",
        type === "currency" && "font-mono font-medium",
        type === "number" && "font-mono",
        (masked || type === "masked") && !isMaskRevealed && !isFocused && "tracking-widest font-mono",
        inputClass
      )}
      {...restProps}
    />

    <!-- Suffix Action Controls Area -->
    <div class="shrink-0 flex items-center gap-1 pr-1.5 self-stretch">
      <!-- Math Calculation Badge Indicator -->
      {#if showCalcBadge}
        <span
          class="shrink-0 inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 animate-in fade-in zoom-in-95 duration-150"
          title="Calculated: {lastFormula}"
        >
          <Calculator class="size-2.5 text-indigo-600 dark:text-indigo-400" />
          <span class="max-w-[130px] truncate">{lastFormula}</span>
        </span>
      {/if}

      <!-- Loading Indicator Spinner -->
      {#if loading}
        <div class="px-1 text-indigo-500 animate-spin">
          <Loader2 class={surface.iconClass} />
        </div>
      {/if}

      <!-- Clear Button (Visible when clearable/search and non-empty) -->
      {#if (clearable || type === "search") && value && !disabled && !readonly}
        <button
          type="button"
          onclick={handleClear}
          class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          title="Clear text (Esc)"
          aria-label="Clear field"
        >
          <X class={surface.density === "lg" ? "size-4" : "size-3.5"} />
        </button>
      {/if}

      <!-- Sensitive Field Mask / Unmask Toggle (Credit Card, SSN, IBAN, Secret) -->
      {#if (masked || type === "masked") && unmaskable && !disabled}
        <button
          type="button"
          onclick={() => (isMaskRevealed = !isMaskRevealed)}
          class="p-1 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
          title={isMaskRevealed ? "Mask sensitive value" : "Reveal sensitive value"}
          aria-label={isMaskRevealed ? "Mask sensitive value" : "Reveal sensitive value"}
        >
          {#if isMaskRevealed}
            <ShieldOff class={surface.iconClass} />
          {:else}
            <Shield class={surface.iconClass} />
          {/if}
        </button>
      {/if}

      <!-- CapsLock Warning Indicator -->
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
      {/if}

      <!-- Copy to Clipboard Button -->
      {#if copyable && value && !disabled}
        <button
          type="button"
          onclick={handleCopy}
          class={cn(
            "p-1 rounded cursor-pointer transition-colors",
            isCopied
              ? "text-emerald-500 bg-emerald-50 dark:bg-emerald-950/40"
              : "text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800"
          )}
          title={isCopied ? "Copied to clipboard!" : ((masked || type === "masked") ? "Copy raw unmasked value" : "Copy value")}
          aria-label="Copy value"
        >
          {#if isCopied}
            <Check class={surface.iconClass} />
          {:else}
            <Copy class={surface.iconClass} />
          {/if}
        </button>
      {/if}

      <!-- Stepper Controls (Increment / Decrement for Numbers) -->
      {#if (stepper || type === "number") && !disabled && !readonly}
        <div class="flex items-center border-l border-slate-200 dark:border-slate-800 pl-1 my-0.5 gap-0.5">
          <button
            type="button"
            onclick={() => stepNumber(-1)}
            disabled={min !== undefined && Number(value) <= min}
            class="p-1 rounded text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none cursor-pointer"
            title="Decrement"
            aria-label="Decrement number"
          >
            <Minus class="size-3" />
          </button>
          <button
            type="button"
            onclick={() => stepNumber(1)}
            disabled={max !== undefined && Number(value) >= max}
            class="p-1 rounded text-slate-500 hover:text-slate-800 dark:text-slate-400 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none cursor-pointer"
            title="Increment"
            aria-label="Increment number"
          >
            <Plus class="size-3" />
          </button>
        </div>
      {/if}

      <!-- Lookup / AssistEdit Trigger Button (BC / SAP Value Help pattern) -->
      {#if (type === "lookup" || onLookup) && !disabled}
        <button
          type="button"
          onclick={() => onLookup?.()}
          class="px-2 py-1 rounded text-indigo-600 dark:text-indigo-400 hover:bg-indigo-50 dark:hover:bg-indigo-950/60 border-l border-slate-200 dark:border-slate-800 cursor-pointer transition-colors text-xs font-semibold flex items-center gap-1"
          title="Open Relation Lookup (F4)"
          aria-label="Open lookup dialog"
        >
          <MoreHorizontal class="size-3.5" />
        </button>
      {/if}

      <!-- Static Suffix Text (e.g. "USD", "kg", "pcs") -->
      {#if suffixText}
        <span class="shrink-0 flex items-center px-2 bg-slate-100 dark:bg-slate-800 border-l border-slate-200 dark:border-slate-800 text-[11px] font-semibold text-slate-500 dark:text-slate-400 self-stretch">
          {suffixText}
        </span>
      {/if}

      <!-- Trailing Icon -->
      {#if suffixIcon}
        {@const SuffixIconComp = suffixIcon}
        <div class="px-1 text-slate-400 dark:text-slate-500 pointer-events-none">
          <SuffixIconComp class={surface.iconClass} />
        </div>
      {/if}

      <!-- Custom Trailing Snippet Slot -->
      {#if suffixSnippet}
        {@render suffixSnippet()}
      {/if}
    </div>
  </div>

  <!-- 3. Bottom Feedback / Description / Validation Message -->
  {#if errorMessage || warningMessage || successMessage || helperText}
    <div class="flex items-center gap-1.5 text-xs pt-0.5 animate-in fade-in-50 duration-150">
      {#if resolvedStatus === "error" && errorMessage}
        <AlertCircle class="size-3.5 text-rose-500 shrink-0" />
        <span class="text-rose-600 dark:text-rose-400 font-medium">{errorMessage}</span>
      {:else if resolvedStatus === "warning" && warningMessage}
        <AlertTriangle class="size-3.5 text-amber-500 shrink-0" />
        <span class="text-amber-600 dark:text-amber-400 font-medium">{warningMessage}</span>
      {:else if resolvedStatus === "success" && successMessage}
        <CheckCircle2 class="size-3.5 text-emerald-500 shrink-0" />
        <span class="text-emerald-600 dark:text-emerald-400 font-medium">{successMessage}</span>
      {:else if helperText}
        <Info class="size-3.5 text-slate-400 shrink-0" />
        <span class="text-slate-500 dark:text-slate-400">{helperText}</span>
      {/if}
    </div>
  {/if}
</div>
