/**
 * Safe Mathematical Expression Evaluator & Enterprise Masking Engine
 * Zero unsafe eval() or Function constructor - 100% pure tokenized parser.
 */

/**
 * Safely evaluates arithmetic expressions (+, -, *, /, parentheses)
 * @param {string} expr - e.g. "50 * 1.2" or "(1000 - 150) / 2"
 * @returns {number | null} - Evaluated numeric result or null if invalid
 */
export function evaluateMath(expr) {
  if (!expr || typeof expr !== 'string') return null;
  const clean = expr.trim().replace(/\s+/g, '');
  if (!clean) return null;

  // Must contain at least one arithmetic operator to be treated as a formula
  if (!/[+\-*/]/.test(clean)) {
    const directNum = Number(clean.replace(/,/g, ''));
    return isNaN(directNum) ? null : directNum;
  }

  // Tokenize safely: numbers (including decimals) and operators
  const tokens = [];
  let i = 0;
  while (i < clean.length) {
    const ch = clean[i];
    if ('+-*/()'.includes(ch)) {
      // Check for unary minus
      if (ch === '-' && (i === 0 || '+-*/('.includes(clean[i - 1]))) {
        let numStr = '-';
        i++;
        while (i < clean.length && (/[0-9.]/.test(clean[i]))) {
          numStr += clean[i];
          i++;
        }
        const num = parseFloat(numStr);
        if (isNaN(num)) return null;
        tokens.push({ type: 'number', value: num });
        continue;
      }
      tokens.push({ type: 'op', value: ch });
      i++;
    } else if (/[0-9.]/.test(ch)) {
      let numStr = '';
      while (i < clean.length && (/[0-9.]/.test(clean[i]))) {
        numStr += clean[i];
        i++;
      }
      const num = parseFloat(numStr);
      if (isNaN(num)) return null;
      tokens.push({ type: 'number', value: num });
    } else {
      // Illegal character
      return null;
    }
  }

  // Shunting-Yard algorithm to produce Reverse Polish Notation (RPN)
  const precedence = { '+': 1, '-': 1, '*': 2, '/': 2 };
  const outputQueue = [];
  const opStack = [];

  for (const token of tokens) {
    if (token.type === 'number') {
      outputQueue.push(token.value);
    } else if (token.type === 'op') {
      if (token.value === '(') {
        opStack.push(token.value);
      } else if (token.value === ')') {
        while (opStack.length > 0 && opStack[opStack.length - 1] !== '(') {
          outputQueue.push(opStack.pop());
        }
        if (opStack.length === 0) return null; // Mismatched parens
        opStack.pop(); // Discard '('
      } else {
        while (
          opStack.length > 0 &&
          opStack[opStack.length - 1] !== '(' &&
          precedence[opStack[opStack.length - 1]] >= precedence[token.value]
        ) {
          outputQueue.push(opStack.pop());
        }
        opStack.push(token.value);
      }
    }
  }

  while (opStack.length > 0) {
    const op = opStack.pop();
    if (op === '(' || op === ')') return null;
    outputQueue.push(op);
  }

  // Evaluate RPN
  const evalStack = [];
  for (const item of outputQueue) {
    if (typeof item === 'number') {
      evalStack.push(item);
    } else {
      const b = evalStack.pop();
      const a = evalStack.pop();
      if (a === undefined || b === undefined) return null;
      let res = 0;
      switch (item) {
        case '+': res = a + b; break;
        case '-': res = a - b; break;
        case '*': res = a * b; break;
        case '/':
          if (b === 0) return null; // Zero division guard
          res = a / b;
          break;
        default: return null;
      }
      evalStack.push(res);
    }
  }

  if (evalStack.length !== 1 || isNaN(evalStack[0])) return null;
  return Number(evalStack[0].toFixed(6)); // Round floating noise
}

/**
 * Enterprise Field Masking Function
 * @param {string} val - Raw unmasked string
 * @param {'credit-card' | 'ssn' | 'iban' | 'custom'} [maskType='custom']
 * @param {number} [visibleCount=4] - Number of unmasked trailing characters
 * @returns {string} - Masked representation
 */
export function maskValue(val, maskType = 'custom', visibleCount = 4) {
  if (!val || typeof val !== 'string') return '';
  const str = String(val).trim();
  if (str.length === 0) return '';

  if (maskType === 'credit-card') {
    const digits = str.replace(/\D/g, '');
    const last4 = digits.slice(-4) || str.slice(-4);
    return `•••• •••• •••• ${last4}`;
  }

  if (maskType === 'ssn') {
    const digits = str.replace(/\D/g, '');
    const last4 = digits.slice(-4) || str.slice(-4);
    return `•••-••-${last4}`;
  }

  if (maskType === 'iban') {
    const clean = str.replace(/\s+/g, '');
    const prefix = clean.slice(0, 4) || 'IBAN';
    const last4 = clean.slice(-4);
    return `${prefix} •••• •••• •••• ${last4}`;
  }

  // Custom / General: Mask all except last `visibleCount` characters
  if (str.length <= visibleCount) return str;
  const maskedPart = '•'.repeat(Math.max(1, str.length - visibleCount));
  const visiblePart = str.slice(-visibleCount);
  return `${maskedPart} ${visiblePart}`;
}

/**
 * Formats a numeric value with commas and fixed decimals
 * @param {number | string} val
 * @param {number} [decimals=2]
 * @returns {string}
 */
export function formatCurrencyNumber(val, decimals = 2) {
  if (val === '' || val === null || val === undefined) return '';
  const num = typeof val === 'number' ? val : parseFloat(String(val).replace(/,/g, ''));
  if (isNaN(num)) return String(val);

  return num.toLocaleString('en-US', {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals
  });
}

/**
 * Formats a raw alphanumeric value against a template pattern (e.g. "(###) ###-####" or "##-#######")
 * '#' = digit, 'A' = uppercase letter, 'a' = lowercase letter, '*' = any alphanumeric
 * @param {string} val
 * @param {string} pattern
 * @returns {string}
 */
export function formatWithPattern(val, pattern) {
  if (!val || !pattern) return val || '';
  const clean = String(val).replace(/[^a-zA-Z0-9]/g, '');
  let result = '';
  let cleanIdx = 0;

  for (let i = 0; i < pattern.length && cleanIdx < clean.length; i++) {
    const maskChar = pattern[i];
    if (maskChar === '#') {
      while (cleanIdx < clean.length && !/[0-9]/.test(clean[cleanIdx])) {
        cleanIdx++;
      }
      if (cleanIdx < clean.length) {
        result += clean[cleanIdx];
        cleanIdx++;
      }
    } else if (maskChar === 'A') {
      while (cleanIdx < clean.length && !/[a-zA-Z]/.test(clean[cleanIdx])) {
        cleanIdx++;
      }
      if (cleanIdx < clean.length) {
        result += clean[cleanIdx].toUpperCase();
        cleanIdx++;
      }
    } else if (maskChar === 'a') {
      while (cleanIdx < clean.length && !/[a-zA-Z]/.test(clean[cleanIdx])) {
        cleanIdx++;
      }
      if (cleanIdx < clean.length) {
        result += clean[cleanIdx].toLowerCase();
        cleanIdx++;
      }
    } else if (maskChar === '*') {
      result += clean[cleanIdx];
      cleanIdx++;
    } else {
      // Literal symbol in pattern (e.g. '(', ')', ' ', '-', '/')
      result += maskChar;
    }
  }
  return result;
}

