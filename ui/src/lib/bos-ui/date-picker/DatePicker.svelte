<script>
  import { cn } from "$lib/utils.js";
  import { Popover } from "bits-ui";
  import {
    Calendar as CalendarIcon,
    ChevronLeft,
    ChevronRight,
    Clock,
    X,
    Sparkles,
    AlertCircle,
    AlertTriangle,
    CheckCircle2,
    Info,
    Lock
  } from "lucide-svelte";
  import {
    evaluateDateFormula,
    formatISODate,
    formatDisplayDate,
    getAccountingPresets,
    parseISODate,
    shiftIsoDate,
    shiftIsoMonth,
    weekEdgeIso
  } from "./evaluator.js";
  import { resolveSurface } from "../theme/index.js";

  let {
    ref = $bindable(null),
    value = $bindable(""), // ISO string 'YYYY-MM-DD' or { start, end } if range=true
    range = false,
    showTime = false,
    timeValue = $bindable("12:00"),
    timeStep = 15,
    evalFormula = true,
    workDate = "",
    allowClosingDates = true,
    closedPeriods = [],
    label = "",
    helperText = "",
    errorMessage = "",
    warningMessage = "",
    placeholder = "",
    density = null, // 'sm' | 'md' | 'lg' (defaults to themeStore.density)
    variant = null, // 'outline' | 'filled' | 'underline' | 'ghost' (defaults to themeStore.variant)
    rounded = null, // 'none' | 'sm' | 'md' | 'pill' (defaults to themeStore.rounded)
    status = "default",
    required = false,
    disabled = false,
    readonly = false,
    clearable = true,
    minDate = null,
    maxDate = null,
    onChange = null,
    onFormulaCalculated = null,
    onClear = null,
    class: className = "",
    dropdownClass = "",
    open = $bindable(false),
    ...restProps
  } = $props();
  let textInputVal = $state("");
  let isFocused = $state(false);
  let cursorIso = $state(formatISODate(new Date()));
  let calendarMoved = $state(false);
  let isClosingDate = $state(false);
  let showCalcBadge = $state(false);
  let lastFormula = $state("");
  let calcBadgeTimer = $state(null);

  // Calendar Viewport month & year
  let viewYear = $state(new Date().getFullYear());
  let viewMonth = $state(new Date().getMonth()); // 0-indexed

  // Resolved effective status
  let resolvedStatus = $derived.by(() => {
    if (errorMessage) return "error";
    if (warningMessage) return "warning";
    return status;
  });

  // Effective placeholder text
  let resolvedPlaceholder = $derived.by(() => {
    if (placeholder) return placeholder;
    if (range) return "Start date .. End date (or formula)";
    return "YYYY-MM-DD or formula (e.g. cm+10d, t)";
  });

  // Synchronize text input from external value on mount/change
  $effect(() => {
    if (!isFocused) {
      if (range) {
        if (value && typeof value === "object") {
          const s = value.start ? formatISODate(new Date(value.start)) : "";
          const e = value.end ? formatISODate(new Date(value.end)) : "";
          textInputVal = s && e ? `${s} .. ${e}` : (s || e || "");
        } else {
          textInputVal = "";
        }
      } else {
        if (value && typeof value === "string") {
          if (value.startsWith("C")) {
            isClosingDate = true;
            textInputVal = value;
          } else {
            textInputVal = value;
          }
        } else {
          textInputVal = "";
        }
      }
    }
  });

  // Accounting Presets
  const presets = getAccountingPresets();

  // Calendar Generation Helpers
  const weekDays = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

  let calendarDays = $derived.by(() => {
    const days = [];
    const firstDayOfMonth = new Date(viewYear, viewMonth, 1);
    const lastDayOfMonth = new Date(viewYear, viewMonth + 1, 0);

    // Day of week for 1st of month: 0=Sun, 1=Mon, ..., 6=Sat
    let startDayOfWeek = firstDayOfMonth.getDay();
    startDayOfWeek = startDayOfWeek === 0 ? 6 : startDayOfWeek - 1; // Convert to Mon=0 ... Sun=6

    // Previous month padding
    const prevMonthLastDay = new Date(viewYear, viewMonth, 0).getDate();
    for (let i = startDayOfWeek - 1; i >= 0; i--) {
      const d = prevMonthLastDay - i;
      const dateObj = new Date(viewYear, viewMonth - 1, d);
      days.push({
        date: dateObj,
        dayNumber: d,
        isCurrentMonth: false,
        iso: formatISODate(dateObj)
      });
    }

    // Current month days
    for (let d = 1; d <= lastDayOfMonth.getDate(); d++) {
      const dateObj = new Date(viewYear, viewMonth, d);
      days.push({
        date: dateObj,
        dayNumber: d,
        isCurrentMonth: true,
        iso: formatISODate(dateObj)
      });
    }

    // Next month padding to fill complete weeks (multiples of 7)
    const remaining = (7 - (days.length % 7)) % 7;
    for (let d = 1; d <= remaining; d++) {
      const dateObj = new Date(viewYear, viewMonth + 1, d);
      days.push({
        date: dateObj,
        dayNumber: d,
        isCurrentMonth: false,
        iso: formatISODate(dateObj)
      });
    }

    return days;
  });

  function formulaBase() {
    return parseISODate(workDate) || new Date();
  }

  function isDateClosed(isoStr) {
    if (!closedPeriods || closedPeriods.length === 0) return false;
    return closedPeriods.some((p) => isoStr >= p.start && isoStr <= p.end);
  }

  function isDateSelected(isoStr) {
    if (range) {
      if (!value || typeof value !== "object") return false;
      return isoStr === value.start || isoStr === value.end;
    }
    return value === isoStr;
  }

  function isDateInRange(isoStr) {
    if (!range || !value || typeof value !== "object") return false;
    if (value.start && value.end) {
      return isoStr > value.start && isoStr < value.end;
    }
    return false;
  }

  function isToday(isoStr) {
    return isoStr === formatISODate(new Date());
  }

  function handleDayClick(dayObj) {
    if (isDateClosed(dayObj.iso)) return;

    if (range) {
      if (!value || typeof value !== "object" || (value.start && value.end)) {
        // Start new range selection
        value = { start: dayObj.iso, end: "" };
      } else if (value.start && !value.end) {
        if (dayObj.iso < value.start) {
          value = { start: dayObj.iso, end: value.start };
        } else {
          value = { start: value.start, end: dayObj.iso };
        }
        open = false;
        onChange?.(value);
      }
    } else {
      value = isClosingDate ? `C${dayObj.iso}` : dayObj.iso;
      textInputVal = value;
      open = false;
      onChange?.(value);
    }
  }

  function applyPreset(p) {
    const s = formatISODate(p.start);
    const e = formatISODate(p.end);
    if (range) {
      value = { start: s, end: e };
      textInputVal = `${s} .. ${e}`;
    } else {
      value = s;
      textInputVal = s;
    }
    viewYear = p.start.getFullYear();
    viewMonth = p.start.getMonth();
    open = false;
    onChange?.(value);
  }

  function handleTextInput() {
    // Keep local string in sync
  }

  function executeFormulaEvaluation() {
    if (!evalFormula || !textInputVal.trim()) return;

    const rawStr = textInputVal.trim();

    // Check if range syntax: e.g. "10/01/26 .. 10/31/26" or "cm .. cm+10d"
    if (range && rawStr.includes("..")) {
      const parts = rawStr.split("..").map((s) => s.trim());
      const resStart = evaluateDateFormula(parts[0], formulaBase());
      const resEnd = evaluateDateFormula(parts[1], formulaBase());

      if (resStart && resEnd) {
        const s = formatISODate(resStart.date);
        const e = formatISODate(resEnd.date);
        value = { start: s, end: e };
        textInputVal = `${s} .. ${e}`;
        lastFormula = `${rawStr} → ${s} .. ${e}`;
        showCalcBadge = true;
        flashCalcBadge();
        onChange?.(value);
        return;
      }
    }

    const evalResult = evaluateDateFormula(rawStr, formulaBase());
    if (evalResult) {
      const iso = formatISODate(evalResult.date);
      const finalVal = evalResult.isClosing && allowClosingDates ? `C${iso}` : iso;
      value = finalVal;
      textInputVal = finalVal;
      viewYear = evalResult.date.getFullYear();
      viewMonth = evalResult.date.getMonth();
      isClosingDate = evalResult.isClosing;

      lastFormula = `${rawStr} → ${finalVal}`;
      showCalcBadge = true;
      flashCalcBadge();
      onChange?.(value);
      onFormulaCalculated?.(evalResult);
    }
  }

  function flashCalcBadge() {
    if (calcBadgeTimer) clearTimeout(calcBadgeTimer);
    calcBadgeTimer = setTimeout(() => {
      showCalcBadge = false;
    }, 3500);
  }

  function seedCursor() {
    const raw = typeof value === "string" ? value.replace(/^C/, "") : "";
    const parsed = raw ? evaluateDateFormula(raw, formulaBase()) : null;
    const date = parsed?.date || new Date();
    cursorIso = formatISODate(date);
    viewYear = date.getFullYear();
    viewMonth = date.getMonth();
    calendarMoved = false;
  }

  function syncCursorView(iso) {
    const date = parseISODate(iso);
    if (!date) return;
    viewYear = date.getFullYear();
    viewMonth = date.getMonth();
  }

  function chooseCursor() {
    const day = calendarDays.find((item) => item.iso === cursorIso) || { iso: cursorIso };
    handleDayClick(day);
    calendarMoved = false;
  }

  /**
   * Arrow keys walk the open calendar. The highlighted day is what Enter stores.
   * @param {KeyboardEvent} e
   */
  function moveCalendar(e) {
    if (!open) return false;
    if (e.key === "ArrowLeft") cursorIso = shiftIsoDate(cursorIso, -1);
    else if (e.key === "ArrowRight") cursorIso = shiftIsoDate(cursorIso, 1);
    else if (e.key === "ArrowUp") cursorIso = shiftIsoDate(cursorIso, -7);
    else if (e.key === "ArrowDown" && !e.altKey) cursorIso = shiftIsoDate(cursorIso, 7);
    else if (e.key === "PageUp") cursorIso = shiftIsoMonth(cursorIso, e.altKey ? -12 : -1);
    else if (e.key === "PageDown") cursorIso = shiftIsoMonth(cursorIso, e.altKey ? 12 : 1);
    else if (e.key === "Home") cursorIso = weekEdgeIso(cursorIso, false);
    else if (e.key === "End") cursorIso = weekEdgeIso(cursorIso, true);
    else if (e.key === "Enter" && calendarMoved) {
      e.preventDefault();
      e.stopPropagation();
      chooseCursor();
      return true;
    } else {
      return false;
    }
    e.preventDefault();
    e.stopPropagation();
    calendarMoved = true;
    syncCursorView(cursorIso);
    return true;
  }

  function handleKeyDown(e) {
    if (e.key === "F4" || (e.altKey && e.key === "ArrowDown")) {
      e.preventDefault();
      e.stopPropagation();
      if (!open) seedCursor();
      open = !open;
    } else if (moveCalendar(e)) {
      return;
    } else if (e.key === "Enter") {
      executeFormulaEvaluation();
    } else if (e.key === "Escape") {
      if (open) {
        open = false;
      } else if (clearable && value) {
        handleClear();
      }
    }
  }

  let calendarArmed = false;
  $effect(() => {
    if (!open) {
      calendarMoved = false;
      calendarArmed = false;
      return;
    }
    if (!calendarArmed) {
      seedCursor();
      calendarArmed = true;
    }
  });

  function handleClear(e) {
    e?.stopPropagation();
    value = range ? { start: "", end: "" } : "";
    textInputVal = "";
    isClosingDate = false;
    onClear?.();
  }

  function prevMonth() {
    if (viewMonth === 0) {
      viewMonth = 11;
      viewYear--;
    } else {
      viewMonth--;
    }
  }

  function nextMonth() {
    if (viewMonth === 11) {
      viewMonth = 0;
      viewYear++;
    } else {
      viewMonth++;
    }
  }

  let surface = $derived(resolveSurface("date", { density, variant, rounded, status: resolvedStatus }));

  const monthNames = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December"
  ];
</script>

<div class={cn("flex flex-col gap-1 w-full text-left font-sans select-none", className)}>
  <!-- 1. Top Form Field Label -->
  {#if label}
    <div class="flex items-center justify-between text-xs">
      <label class="font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
        <span>{label}</span>
        {#if required}
          <span class="text-rose-500 font-bold" title="Required field">*</span>
        {/if}
        {#if isClosingDate}
          <span class="px-1.5 py-0.2 rounded text-[10px] font-mono font-bold bg-amber-100 dark:bg-amber-950 text-amber-700 dark:text-amber-400 border border-amber-300 dark:border-amber-800">
            CLOSING DATE
          </span>
        {/if}
      </label>
      <span class="text-[10px] text-slate-400 font-mono">Formula: 't', 'cm+10d'</span>
    </div>
  {/if}

  <!-- 2. Interactive Input Frame with Popover Trigger -->
  <Popover.Root bind:open>
    <div
      class={cn(
        "group relative flex items-center w-full transition-all outline-hidden gap-1",
        surface.controlClass,
        disabled && "opacity-50 cursor-not-allowed bg-slate-100 dark:bg-slate-800/40 pointer-events-none",
        readonly && "bg-slate-50 dark:bg-slate-950/60 pointer-events-none cursor-default"
      )}
    >
      <!-- Leading Calendar Icon / Popover Trigger -->
      <Popover.Trigger
        type="button"
        disabled={disabled}
        class="shrink-0 text-slate-400 hover:text-indigo-600 dark:hover:text-indigo-400 cursor-pointer transition-colors p-0.5"
        title="Open visual calendar (F4)"
        aria-label="Open visual calendar"
      >
        <CalendarIcon class={surface.iconClass} />
      </Popover.Trigger>

      <!-- High-Speed Keyboard Input Field -->
      <input
        bind:this={ref}
        type="text"
        bind:value={textInputVal}
        data-cursor-iso={cursorIso}
        data-calendar-moved={calendarMoved ? "1" : "0"}
        oninput={handleTextInput}
        onkeydown={handleKeyDown}
        onfocus={() => (isFocused = true)}
        onblur={() => {
          isFocused = false;
          if (!open) executeFormulaEvaluation();
        }}
        placeholder={resolvedPlaceholder}
        {disabled}
        {readonly}
        class="flex-1 min-w-0 bg-transparent px-1 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500 outline-hidden font-mono text-xs sm:text-sm"
        {...restProps}
      />

      <!-- Action Area Suffix -->
      <div class="shrink-0 flex items-center gap-1 self-center">
        <!-- Formula Calculation Badge -->
        {#if showCalcBadge}
          <span
            class="shrink-0 inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800 animate-in fade-in zoom-in-95 duration-150"
            title="Calculated: {lastFormula}"
          >
            <Sparkles class="size-2.5 text-indigo-600 dark:text-indigo-400" />
            <span class="max-w-[120px] truncate">{lastFormula}</span>
          </span>
        {/if}

        {#if showTime && timeValue}
          <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] font-mono bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300">
            <Clock class="size-2.5 text-slate-400" />
            <span>{timeValue}</span>
          </span>
        {/if}

        {#if clearable && (textInputVal || value) && !disabled && !readonly}
          <button
            type="button"
            onclick={handleClear}
            class="p-0.5 rounded text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors cursor-pointer"
            title="Clear date (Esc)"
            aria-label="Clear date"
          >
            <X class="size-3.5" />
          </button>
        {/if}
      </div>
    </div>

    <!-- 3. Visual Calendar Dropdown (Floating Portal) -->
    <Popover.Portal>
      <Popover.Content
        side="bottom"
        align="start"
        sideOffset={4}
        trapFocus={false}
        onOpenAutoFocus={(event) => event.preventDefault()}
        onkeydown={moveCalendar}
        class={cn(
          "z-50 overflow-hidden animate-in fade-in-50 zoom-in-95 duration-150 flex flex-col md:flex-row font-sans",
          surface.panelClass,
          dropdownClass
        )}
      >
        <!-- Left: Accounting Quick Presets Sidebar -->
        <div class="w-full md:w-44 p-2.5 bg-slate-50 dark:bg-slate-800/40 border-b md:border-b-0 md:border-r border-slate-100 dark:border-slate-800 flex flex-col justify-between shrink-0">
          <div>
            <span class="px-2 text-[10px] font-bold uppercase tracking-wider text-slate-400">
              Accounting Presets
            </span>
            <div class="mt-1.5 space-y-0.5">
              {#each presets as p}
                <button
                  type="button"
                  onclick={() => applyPreset(p)}
                  class="w-full text-left px-2 py-1.5 rounded-md text-xs font-medium text-slate-600 dark:text-slate-300 hover:bg-slate-200/70 dark:hover:bg-slate-800 hover:text-indigo-600 dark:hover:text-indigo-400 cursor-pointer transition-colors"
                >
                  {p.label}
                </button>
              {/each}
            </div>
          </div>

          <!-- Closing Date Accounting Toggle -->
          {#if allowClosingDates}
            <div class="mt-3 pt-2.5 border-t border-slate-200 dark:border-slate-700/60">
              <label class="flex items-center gap-2 px-1 text-xs text-slate-600 dark:text-slate-300 cursor-pointer">
                <input
                  type="checkbox"
                  bind:checked={isClosingDate}
                  class="rounded border-slate-300 text-amber-600 focus:ring-amber-500"
                />
                <span class="text-[11px] font-medium">Closing Date ('C')</span>
              </label>
              <p class="px-1 text-[9px] text-slate-400 mt-0.5">
                Year-end audit adjustment entries
              </p>
            </div>
          {/if}
        </div>

        <!-- Right: Dual-Mode Interactive Calendar Month -->
        <div class="p-3 flex flex-col space-y-3">
          <!-- Calendar Header: Month & Year Navigator -->
          <div class="flex items-center justify-between">
            <button
              type="button"
              onclick={prevMonth}
              class="p-1 rounded-md text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
              title="Previous Month"
            >
              <ChevronLeft class="size-4" />
            </button>

            <span class="text-xs sm:text-sm font-bold text-slate-900 dark:text-slate-100">
              {monthNames[viewMonth]} {viewYear}
            </span>

            <button
              type="button"
              onclick={nextMonth}
              class="p-1 rounded-md text-slate-500 hover:bg-slate-100 dark:hover:bg-slate-800 cursor-pointer transition-colors"
              title="Next Month"
            >
              <ChevronRight class="size-4" />
            </button>
          </div>

          <!-- Weekday Headers -->
          <div class="grid grid-cols-7 gap-1 text-center text-[10px] font-semibold uppercase tracking-wider text-slate-400">
            {#each weekDays as wd}
              <div class="py-1">{wd}</div>
            {/each}
          </div>

          <!-- Calendar Days Grid -->
          <div class="grid grid-cols-7 gap-1 text-center text-xs">
            {#each calendarDays as d}
              {@const isClosed = isDateClosed(d.iso)}
              {@const selected = isDateSelected(d.iso)}
              {@const inRange = isDateInRange(d.iso)}
              {@const currentDay = isToday(d.iso)}

              {@const keyed = open && cursorIso === d.iso}

              <button
                type="button"
                onclick={() => handleDayClick(d)}
                disabled={isClosed}
                data-cursor={keyed ? "1" : undefined}
                class={cn(
                  "size-8 rounded-lg flex items-center justify-center font-medium transition-colors relative cursor-pointer",
                  !d.isCurrentMonth && "opacity-35 text-slate-400",
                  d.isCurrentMonth && "text-slate-800 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800",
                  currentDay && !selected && "border border-indigo-500/50 font-bold text-indigo-600 dark:text-indigo-400",
                  inRange && "bg-indigo-50 dark:bg-indigo-950/60 rounded-none text-indigo-900 dark:text-indigo-200",
                  selected && "bg-indigo-600 text-white font-bold hover:bg-indigo-700 shadow-xs",
                  keyed && "ring-2 ring-inset ring-indigo-600",
                  isClosed && "opacity-25 cursor-not-allowed bg-rose-50 dark:bg-rose-950/30 text-rose-500 line-through"
                )}
                title={isClosed ? "Accounting period is closed" : d.iso}
              >
                <span>{d.dayNumber}</span>
                {#if isClosed}
                  <Lock class="size-2 text-rose-500 absolute bottom-0.5 right-0.5" />
                {/if}
              </button>
            {/each}
          </div>

          <!-- Embedded Time Selector (when showTime=true) -->
          {#if showTime}
            <div class="pt-2 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between text-xs">
              <span class="text-slate-500 flex items-center gap-1">
                <Clock class="size-3.5" />
                <span>Posting Time:</span>
              </span>
              <div class="flex items-center gap-1.5">
                <input
                  type="time"
                  step={timeStep * 60}
                  bind:value={timeValue}
                  class={cn(
                    "h-7 px-2 border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 font-mono text-xs outline-hidden focus:border-indigo-500",
                    surface.radiusClass
                  )}
                />
              </div>
            </div>
          {/if}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>

  <!-- 4. Bottom Feedback / Error / Warning Message -->
  {#if errorMessage || warningMessage || helperText}
    <div class="flex items-center gap-1.5 text-xs pt-0.5 animate-in fade-in-50 duration-150">
      {#if resolvedStatus === "error" && errorMessage}
        <AlertCircle class="size-3.5 text-rose-500 shrink-0" />
        <span class="text-rose-600 dark:text-rose-400 font-medium">{errorMessage}</span>
      {:else if resolvedStatus === "warning" && warningMessage}
        <AlertTriangle class="size-3.5 text-amber-500 shrink-0" />
        <span class="text-amber-600 dark:text-amber-400 font-medium">{warningMessage}</span>
      {:else if helperText}
        <Info class="size-3.5 text-slate-400 shrink-0" />
        <span class="text-slate-500 dark:text-slate-400">{helperText}</span>
      {/if}
    </div>
  {/if}
</div>
