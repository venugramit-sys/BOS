import * as prettier from "prettier/standalone";
import * as babelPlugin from "prettier/plugins/babel";
import * as estreePlugin from "prettier/plugins/estree";
import * as typescriptPlugin from "prettier/plugins/typescript";

const PRETTIER_OPTIONS = {
  tabWidth: 4,
  useTabs: false,
  semi: true,
  singleQuote: false,
  trailingComma: "all",
  printWidth: 100,
  arrowParens: "always",
  bracketSpacing: true,
};

const plugins = [babelPlugin, estreePlugin, typescriptPlugin];

function skipStringOrComment(source, index) {
  const ch = source[index];
  const next = source[index + 1];
  if (ch === "/" && next === "/") {
    const newline = source.indexOf("\n", index);
    return newline === -1 ? source.length : newline;
  }
  if (ch === "/" && next === "*") {
    const end = source.indexOf("*/", index + 2);
    return end === -1 ? source.length : end + 2;
  }
  if (ch === "'" || ch === '"') {
    let cursor = index + 1;
    while (cursor < source.length) {
      if (source[cursor] === "\\") {
        cursor += 2;
        continue;
      }
      if (source[cursor] === ch) return cursor + 1;
      if (source[cursor] === "\n") return cursor;
      cursor += 1;
    }
    return cursor;
  }
  if (ch === "`") {
    let cursor = index + 1;
    while (cursor < source.length) {
      if (source[cursor] === "\\") {
        cursor += 2;
        continue;
      }
      if (source[cursor] === "`") return cursor + 1;
      if (source[cursor] === "$" && source[cursor + 1] === "{") {
        cursor += 2;
        let depth = 1;
        while (cursor < source.length && depth > 0) {
          const skipped = skipStringOrComment(source, cursor);
          if (skipped !== cursor) {
            cursor = skipped;
            continue;
          }
          if (source[cursor] === "{") depth += 1;
          else if (source[cursor] === "}") depth -= 1;
          if (depth > 0) cursor += 1;
        }
        continue;
      }
      cursor += 1;
    }
    return cursor;
  }
  return index;
}

function headerBeforeBrace(source, braceIndex) {
  const lineStart = source.lastIndexOf("\n", braceIndex - 1) + 1;
  let header = source.slice(lineStart, braceIndex).trim();
  if (!header) {
    const previousEnd = lineStart - 1;
    const previousStart = Math.max(0, source.lastIndexOf("\n", previousEnd - 1) + 1);
    header = source.slice(previousStart, Math.max(previousStart, previousEnd)).trim();
  }
  return header;
}

const NON_SCRIPT_CALL_HEADERS = new Set([
  "field",
  "group",
  "repeater",
  "part",
  "area",
  "action",
  "column",
  "value",
  "key",
  "primaryKey",
  "tier",
  "state",
  "transition",
]);

function isScriptHeader(header) {
  if (
    /^(?:export\s+)?(?:async\s+)?(?:public|private|protected|procedure|function|guard)\b/.test(header)
    || /^(?:on|before|after)[A-Za-z]\w*\s*\(/.test(header)
    || /^validate\b/.test(header)
  ) {
    return true;
  }
  const bareMethodMatch = /^(?:@[A-Za-z_]\w*(?:\([^)]*\))?\s+)*([A-Za-z_]\w*)\s*\([^)]*\)\s*(?::\s*[A-Za-z0-9_<>\[\],\s|]+)?$/.exec(header);
  if (bareMethodMatch && !NON_SCRIPT_CALL_HEADERS.has(bareMethodMatch[1])) {
    return true;
  }
  return false;
}

function findMatchingBrace(source, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    const skipped = skipStringOrComment(source, index);
    if (skipped !== index) {
      index = skipped - 1;
      continue;
    }
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function findScriptBodies(source) {
  const bodies = [];
  for (let index = 0; index < source.length; index += 1) {
    const skipped = skipStringOrComment(source, index);
    if (skipped !== index) {
      index = skipped - 1;
      continue;
    }
    if (source[index] !== "{") continue;
    if (!isScriptHeader(headerBeforeBrace(source, index))) continue;
    const close = findMatchingBrace(source, index);
    if (close < 0) continue;
    bodies.push({ start: index + 1, end: close });
    index = close;
  }
  return bodies;
}

async function formatTypeScriptFragment(source, parser = "typescript") {
  const wrapped = `async function __bosFormat() {\n${source}\n}\n`;
  const formatted = await prettier.format(wrapped, {
    ...PRETTIER_OPTIONS,
    parser,
    plugins,
  });
  const open = formatted.indexOf("{");
  const close = formatted.lastIndexOf("}");
  if (open < 0 || close <= open) return source;
  return formatted.slice(open + 1, close).replace(/^\n/, "").replace(/\s+$/, "");
}

export async function formatTypeScriptSource(source = "", parser = "typescript") {
  const text = String(source ?? "");
  if (!text.trim()) return text;
  try {
    return await prettier.format(text, {
      ...PRETTIER_OPTIONS,
      parser: parser === "babel" ? "babel" : "typescript",
      plugins,
    });
  } catch {
    return formatBosShell(text);
  }
}

async function formatScriptBody(body) {
  if (!body.trim()) return body;
  try {
    return await formatTypeScriptFragment(body, "typescript");
  } catch {
    return body;
  }
}

function indentFormattedBody(body, indent) {
  const lines = String(body).replace(/^\n/, "").replace(/\s+$/, "").split("\n");
  if (!lines.some((line) => line.trim())) return "";
  return lines
    .map((line) => {
      if (!line.trim()) return "";
      const stripped = line.startsWith("    ") ? line.slice(4) : line.trimStart();
      return `${indent}${stripped}`;
    })
    .join("\n");
}

/**
 * Format a BOS document with Prettier inside triggers, validates, and methods.
 * The table and logic headers stay structural.
 */
export async function formatBosScriptSource(source = "") {
  const normalized = String(source ?? "").replace(/\r\n/g, "\n");
  const bodies = findScriptBodies(normalized);
  const formattedBodies = [];
  let working = normalized;
  for (let index = bodies.length - 1; index >= 0; index -= 1) {
    const body = bodies[index];
    const formatted = await formatScriptBody(normalized.slice(body.start, body.end));
    formattedBodies[index] = formatted;
    working = `${working.slice(0, body.start)}\n/*__BOS_SCRIPT_${index}__*/\n${working.slice(body.end)}`;
  }

  const shelled = formatBosShell(working);
  const rebuilt = shelled.replace(/^([ \t]*)\/\*__BOS_SCRIPT_(\d+)__\*\/[ \t]*$/gm, (_match, indent, id) => {
    return indentFormattedBody(formattedBodies[Number(id)] || "", indent);
  });
  return `${rebuilt.replace(/\s+$/, "")}\n`;
}

export function formatBosShell(source = "") {
  const rawLines = String(source).replace(/\r\n/g, "\n").split("\n");
  const formattedLines = [];
  let indentLevel = 0;

  for (const rawLine of rawLines) {
    const trimmed = rawLine.trim();
    if (!trimmed) {
      if (formattedLines.length > 0 && formattedLines[formattedLines.length - 1] !== "") {
        formattedLines.push("");
      }
      continue;
    }

    if (/^[}\])]/.test(trimmed)) {
      indentLevel = Math.max(0, indentLevel - 1);
    }

    formattedLines.push(`${"    ".repeat(indentLevel)}${trimmed}`);

    if (!trimmed.startsWith("//")) {
      const strippedStrings = trimmed.replace(/"([^"\\]|\\.)*"|'([^'\\]|\\.)*'|`([^`\\]|\\.)*`/g, '""');
      const withoutLineComment = strippedStrings.replace(/\/\/.*$/, "");
      let openCount = 0;
      let closeCount = 0;
      for (const ch of withoutLineComment) {
        if (ch === "{" || ch === "[") openCount += 1;
        else if (ch === "}" || ch === "]") closeCount += 1;
      }
      if (/^[}\]]/.test(withoutLineComment)) closeCount = Math.max(0, closeCount - 1);
      indentLevel = Math.max(0, indentLevel + openCount - closeCount);
    }
  }

  return `${formattedLines.join("\n").trimEnd()}\n`;
}

export async function formatRangeText(source = "", parser = "typescript") {
  const text = String(source ?? "");
  if (!text.trim()) return text;
  try {
    const formatted = await formatTypeScriptFragment(text, parser);
    return text.endsWith("\n") ? `${formatted}\n` : formatted;
  } catch {
    const shelled = formatBosShell(text);
    return text.endsWith("\n") ? shelled : shelled.replace(/\n$/, "");
  }
}
