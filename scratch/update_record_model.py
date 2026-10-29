with open('ui/src/lib/page-kit/record-model.js', 'r', encoding='utf-8') as f:
    content = f.read()

target = """/** Classifies a save error for the page frame. */
export function classifySaveError(error) {
  if (error?.status === 409) return { kind: "conflict", message: error.message };
  if (error?.status === 428) return { kind: "version", message: error.message };
  if (error?.fieldErrors && Object.keys(error.fieldErrors).length) return { kind: "fields", message: error.message, fields: error.fieldErrors };
  return { kind: "error", message: error?.message ?? String(error) };
}"""

replacement = """function toSnake(s) {
  return String(s).replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}
function toCamel(s) {
  return String(s).replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase());
}

/** Classifies a save error for the page frame. */
export function classifySaveError(error) {
  const msg = String(error?.message ?? error ?? "");
  if (error?.status === 409) return { kind: "conflict", message: msg };
  if (error?.status === 428) return { kind: "version", message: msg };

  const fields = {};
  if (error?.fieldErrors && typeof error.fieldErrors === "object") {
    for (const [k, v] of Object.entries(error.fieldErrors)) {
      fields[k] = v;
      fields[toSnake(k)] = v;
      fields[toCamel(k)] = v;
    }
  }

  // Extract field mentions from business logic / tablelogic errors
  const m =
    /Field ['"]?(\\w+)['"]? is required/i.exec(msg) ||
    /Validation Failed:\\s*Field ['"]?(\\w+)['"]?/i.exec(msg) ||
    /['"]?(\\w+)['"]? cannot be blank/i.exec(msg) ||
    /column ['"]?(\\w+)['"]? of relation/i.exec(msg);
  if (m) {
    const rawKey = m[1];
    fields[rawKey] = msg;
    fields[toSnake(rawKey)] = msg;
    fields[toCamel(rawKey)] = msg;
  }

  if (Object.keys(fields).length) {
    return { kind: "fields", message: msg, fields };
  }
  return { kind: "error", message: msg };
}"""

target_crlf = target.replace('\n', '\r\n')
replacement_crlf = replacement.replace('\n', '\r\n')

if target in content:
    content = content.replace(target, replacement, 1)
    print("Updated record-model.js (LF)")
elif target_crlf in content:
    content = content.replace(target_crlf, replacement_crlf, 1)
    print("Updated record-model.js (CRLF)")
else:
    print("Target not found in record-model.js")

with open('ui/src/lib/page-kit/record-model.js', 'w', encoding='utf-8') as f:
    f.write(content)
