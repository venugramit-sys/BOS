/**
 * Dynamics 365 Business Central / NAV Date Formula Engine
 * 
 * Safely evaluates shorthand and accounting date formulas:
 * - 't' or 'today' -> Today's date
 * - 'w' or 'work' -> Work date (defaults to today or configured work date)
 * - '15' -> 15th of current month and year
 * - '1215' -> December 15th of current year
 * - 'cm' -> Current Month End (e.g. 2026-10-31)
 * - '-cm' -> Current Month Start (e.g. 2026-10-01)
 * - 'cm+10d' -> Current Month End + 10 days
 * - '+30d', '-5d', '2w', '1m', '1y' -> Relative shifts
 * - 'cq' / '-cq' -> Current Quarter End / Start
 * - 'cy' / '-cy' -> Current Year End / Start
 * - 'C123126' -> Closing Date (Year-end adjustment entries)
 */

/**
 * Evaluates an ERP date formula or shorthand input string into a Date object or closing date object
 * @param {string} input - Formula string (e.g. 't', 'cm+15d', '+30d', 'C123126')
 * @param {Date} [baseDate=new Date()] - Reference date (defaults to current date or work date)
 * @returns {{ date: Date, isClosing: boolean, formula: string } | null}
 */
export function evaluateDateFormula(input, baseDate = new Date()) {
  if (!input || typeof input !== "string") return null;
  const raw = input.trim();
  if (!raw) return null;

  // 1. Check for Closing Date prefix 'C' (e.g. C123126 or C12/31/2026)
  let isClosing = false;
  let cleanInput = raw;
  if (/^c[0-9/.-]+/i.test(cleanInput)) {
    isClosing = true;
    cleanInput = cleanInput.slice(1).trim();
  }

  const baseYear = baseDate.getFullYear();
  const baseMonth = baseDate.getMonth(); // 0-indexed
  const baseDay = baseDate.getDate();

  // 2. Direct standard ISO / YYYY-MM-DD or standard date format
  const standardMatch = cleanInput.match(/^(\d{4})[-/](\d{1,2})[-/](\d{1,2})$/);
  if (standardMatch) {
    const y = parseInt(standardMatch[1], 10);
    const m = parseInt(standardMatch[2], 10) - 1;
    const d = parseInt(standardMatch[3], 10);
    const res = new Date(y, m, d);
    if (!isNaN(res.getTime())) {
      return { date: res, isClosing, formula: raw };
    }
  }

  // 3. Shorthand: Single day number '15' -> 15th of current month
  if (/^\d{1,2}$/.test(cleanInput)) {
    const dayNum = parseInt(cleanInput, 10);
    if (dayNum >= 1 && dayNum <= 31) {
      const res = new Date(baseYear, baseMonth, dayNum);
      return { date: res, isClosing, formula: `${dayNum} -> ${formatISODate(res)}` };
    }
  }

  // 4. Shorthand: 4-digit MMDD '1215' -> Dec 15 of current year
  if (/^\d{4}$/.test(cleanInput)) {
    const m = parseInt(cleanInput.slice(0, 2), 10) - 1;
    const d = parseInt(cleanInput.slice(2, 4), 10);
    if (m >= 0 && m <= 11 && d >= 1 && d <= 31) {
      const res = new Date(baseYear, m, d);
      return { date: res, isClosing, formula: `${cleanInput} -> ${formatISODate(res)}` };
    }
  }

  // 5. BC Date Formula Parser
  // Normalize tokens to uppercase
  let expr = cleanInput.toUpperCase().replace(/\s+/g, "");

  // Base starting point
  let curr = new Date(baseYear, baseMonth, baseDay);

  // If starts with 'T' (Today) or 'W' (WorkDate)
  if (expr.startsWith("T") || expr.startsWith("TODAY")) {
    curr = new Date();
    expr = expr.replace(/^(TODAY|T)/, "");
  } else if (expr.startsWith("W") || expr.startsWith("WORK")) {
    curr = new Date(baseDate);
    expr = expr.replace(/^(WORK|W)/, "");
  }

  // Check Current Month / Quarter / Year anchors (CM, CQ, CY, -CM, -CQ, -CY)
  if (expr.startsWith("-CM")) {
    // Beginning of current month
    curr = new Date(curr.getFullYear(), curr.getMonth(), 1);
    expr = expr.slice(3);
  } else if (expr.startsWith("CM")) {
    // End of current month
    curr = new Date(curr.getFullYear(), curr.getMonth() + 1, 0);
    expr = expr.slice(2);
  } else if (expr.startsWith("-CQ")) {
    // Beginning of current quarter
    const qStartMonth = Math.floor(curr.getMonth() / 3) * 3;
    curr = new Date(curr.getFullYear(), qStartMonth, 1);
    expr = expr.slice(3);
  } else if (expr.startsWith("CQ")) {
    // End of current quarter
    const qEndMonth = Math.floor(curr.getMonth() / 3) * 3 + 3;
    curr = new Date(curr.getFullYear(), qEndMonth, 0);
    expr = expr.slice(2);
  } else if (expr.startsWith("-CY")) {
    // Beginning of current year
    curr = new Date(curr.getFullYear(), 0, 1);
    expr = expr.slice(3);
  } else if (expr.startsWith("CY")) {
    // End of current year
    curr = new Date(curr.getFullYear(), 11, 31);
    expr = expr.slice(2);
  }

  // Parse relative delta terms (e.g. +10D, -5D, +2W, +1M, +1Y)
  if (expr.length > 0) {
    const termRegex = /([+-]?\d+)([DWMQY])/g;
    let match;
    let matchedAny = false;

    while ((match = termRegex.exec(expr)) !== null) {
      matchedAny = true;
      const count = parseInt(match[1], 10);
      const unit = match[2];

      switch (unit) {
        case "D": // Days
          curr.setDate(curr.getDate() + count);
          break;
        case "W": // Weeks
          curr.setDate(curr.getDate() + count * 7);
          break;
        case "M": // Months
          curr.setMonth(curr.getMonth() + count);
          break;
        case "Q": // Quarters
          curr.setMonth(curr.getMonth() + count * 3);
          break;
        case "Y": // Years
          curr.setFullYear(curr.getFullYear() + count);
          break;
      }
    }

    if (!matchedAny && expr !== "") {
      // Unrecognized leftover syntax, try native Date parse as fallback
      const fallback = new Date(raw);
      if (!isNaN(fallback.getTime())) {
        return { date: fallback, isClosing, formula: raw };
      }
      return null;
    }
  }

  return { date: curr, isClosing, formula: raw };
}

/**
 * Formats a Date object to YYYY-MM-DD
 * @param {Date} date
 * @returns {string}
 */
export function formatISODate(date) {
  if (!date || isNaN(date.getTime())) return "";
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/**
 * @param {string} iso
 * @returns {Date | null}
 */
export function parseISODate(iso) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(String(iso || ""));
  if (!match) return null;
  const date = new Date(Number(match[1]), Number(match[2]) - 1, Number(match[3]));
  if (formatISODate(date) !== iso) return null;
  return date;
}

/**
 * @param {string} iso
 * @param {number} days
 */
export function shiftIsoDate(iso, days) {
  const date = parseISODate(iso) || new Date();
  date.setDate(date.getDate() + days);
  return formatISODate(date);
}

/**
 * @param {string} iso
 * @param {number} months
 */
export function shiftIsoMonth(iso, months) {
  const date = parseISODate(iso) || new Date();
  const day = date.getDate();
  const next = new Date(date.getFullYear(), date.getMonth() + months, 1);
  const last = new Date(next.getFullYear(), next.getMonth() + 1, 0).getDate();
  next.setDate(Math.min(day, last));
  return formatISODate(next);
}

/**
 * Monday is the start of the week, matching the calendar grid.
 * @param {string} iso
 * @param {boolean} end
 */
export function weekEdgeIso(iso, end) {
  const date = parseISODate(iso) || new Date();
  const mondayIndex = date.getDay() === 0 ? 6 : date.getDay() - 1;
  date.setDate(date.getDate() + (end ? 6 - mondayIndex : -mondayIndex));
  return formatISODate(date);
}

/**
 * Formats a Date object for human display (e.g. Oct 14, 2026)
 * @param {Date} date
 * @param {boolean} [isClosing=false]
 * @returns {string}
 */
export function formatDisplayDate(date, isClosing = false) {
  if (!date || isNaN(date.getTime())) return "";
  const prefix = isClosing ? "C " : "";
  const str = date.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric"
  });
  return `${prefix}${str}`;
}

/**
 * Predefined Accounting Date Presets
 */
export function getAccountingPresets(baseDate = new Date()) {
  const y = baseDate.getFullYear();
  const m = baseDate.getMonth();
  const today = new Date(y, m, baseDate.getDate());

  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  // This Week (Mon to Sun)
  const dayOfWeek = today.getDay(); // 0 is Sun, 1 is Mon
  const diffToMon = dayOfWeek === 0 ? -6 : 1 - dayOfWeek;
  const startOfWeek = new Date(today);
  startOfWeek.setDate(today.getDate() + diffToMon);
  const endOfWeek = new Date(startOfWeek);
  endOfWeek.setDate(startOfWeek.getDate() + 6);

  // Month to Date (MTD)
  const startOfMonth = new Date(y, m, 1);
  const endOfMonth = new Date(y, m + 1, 0);

  // Quarter to Date (QTD)
  const qStartMonth = Math.floor(m / 3) * 3;
  const startOfQuarter = new Date(y, qStartMonth, 1);
  const endOfQuarter = new Date(y, qStartMonth + 3, 0);

  // Year to Date (YTD)
  const startOfYear = new Date(y, 0, 1);
  const endOfYear = new Date(y, 11, 31);

  return [
    { id: "today", label: "Today", start: today, end: today },
    { id: "yesterday", label: "Yesterday", start: yesterday, end: yesterday },
    { id: "this-week", label: "This Week", start: startOfWeek, end: endOfWeek },
    { id: "mtd", label: "Month to Date (MTD)", start: startOfMonth, end: today },
    { id: "full-month", label: "Full Month", start: startOfMonth, end: endOfMonth },
    { id: "qtd", label: "Quarter to Date (QTD)", start: startOfQuarter, end: today },
    { id: "ytd", label: "Year to Date (YTD)", start: startOfYear, end: today },
    { id: "full-year", label: "Full Fiscal Year", start: startOfYear, end: endOfYear }
  ];
}
