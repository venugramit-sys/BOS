/**
 * table.bos / table.bl.bos document model.
 * Emits `table` as structure only. Every trigger, field validate, and method
 * is emitted as `tablelogic` so the backend stores and runs them as the companion.
 * and rebuilds the same model from the meta-object definition JSON.
 */

let seq = 1;
function uid(prefix) {
  seq += 1;
  return `${prefix}${seq}`;
}

export function createField(partial = {}) {
  const dataType = partial.dataType || "String(50)";
  return {
    id: partial.id || uid("f"),
    name: partial.name || "field",
    dataType,
    length: partial.length ?? lengthOf(dataType),
    caption: partial.caption || "",
    notNull: Boolean(partial.notNull),
    unique: Boolean(partial.unique),
    isPrimaryKey: Boolean(partial.isPrimaryKey),
    defaultVal: partial.defaultVal || "",
    editable: partial.editable !== false,
    visible: partial.visible !== false,
    searchable: Boolean(partial.searchable),
    mask: partial.mask || "",
    dataClassification: partial.dataClassification || "",
    foreignTable: partial.foreignTable || "",
    foreignColumn: partial.foreignColumn || "",
    onDeleteAction: partial.onDeleteAction || "Restrict",
    computed: partial.computed || "",
    flowField: partial.flowField || null,
    drillDownPage: partial.drillDownPage || "",
    validate: partial.validate || null,
    extras: partial.extras || [],
  };
}

export function createEmptyTable(name = "Table") {
  return {
    name,
    caption: "",
    description: "",
    tableType: "Normal",
    dataPerCompany: true,
    lookupPage: "",
    drillDownPage: "",
    dataClassification: "",
    obsoleteState: "",
    obsoleteReason: "",
    packageName: "",
    logicName: `${name}Logic`,
    fields: [],
    primaryKey: [],
    keys: [],
    triggers: [],
    methods: [],
    extensions: [],
  };
}

export function bosTypeFromSql(sqlType) {
  const raw = String(sqlType || "").trim();
  const compact = raw.replace(/\s+/g, "").toUpperCase();
  if (!compact) return "String";
  if (compact === "UUID") return "UUID";
  if (compact === "INTEGER" || compact === "INT" || compact === "INT4" || compact === "SMALLINT" || compact === "INT2" || compact === "SERIAL") return "Integer";
  if (compact === "BIGINT" || compact === "INT8" || compact === "BIGSERIAL") return "BigInt";
  if (compact === "BOOLEAN" || compact === "BOOL") return "Boolean";
  if (compact === "DATE") return "Date";
  if (compact.startsWith("TIMESTAMP") || compact === "TIMESTAMPTZ" || compact === "TIMESTAMPWITHTIMEZONE" || compact === "TIMESTAMPWITHOUTTIMEZONE") return "DateTime";
  if (compact === "JSONB" || compact === "JSON") return "Json";
  if (compact === "BYTEA" || compact === "BLOB") return "Blob";
  if (compact === "TEXT" || compact === "VARCHAR" || compact === "CHARACTERVARYING" || compact === "BPCHAR") return "String";
  const varchar = /^(?:VARCHAR|CHARACTERVARYING|CHAR)\((\d+)\)$/.exec(compact);
  if (varchar) return `String(${varchar[1]})`;
  const str = /^STRING\((\d+)\)$/.exec(compact);
  if (str) return `String(${str[1]})`;
  if (compact === "STRING") return "String";
  const numeric = /^(?:NUMERIC|DECIMAL)\((\d+),(\d+)\)$/.exec(compact);
  if (numeric) return `Decimal(${numeric[1]}, ${numeric[2]})`;
  if (compact === "NUMERIC" || compact === "DECIMAL" || compact === "FLOAT" || compact === "REAL" || compact === "DOUBLEPRECISION") {
    return "Decimal(18, 2)";
  }
  if (/^STRING\(\d+\)$/.test(compact) || /^(UUID|INTEGER|BIGINT|BOOLEAN|DATE|DATETIME|JSON|BLOB)$/.test(compact)) {
    return raw.replace(/\s+/g, " ");
  }
  if (compact.startsWith("STRING") || compact.startsWith("DECIMAL") || compact.startsWith("ENUM")) return raw;
  return raw;
}

export const FIELD_TYPES = ["UUID", "String", "Integer", "BigInt", "Decimal", "Boolean", "Date", "DateTime", "Json", "Blob"].map((id) => ({
  id,
  name: id,
}));

export function typeParts(dataType) {
  const normalized = bosTypeFromSql(dataType);
  const compact = String(normalized || "").replace(/\s+/g, "");
  const stringLength = /^String\((\d+)\)$/i.exec(compact);
  if (stringLength) return { base: "String", length: stringLength[1], precision: "", scale: "" };
  const decimal = /^Decimal\((\d+),(\d+)\)$/i.exec(compact);
  if (decimal) return { base: "Decimal", length: "", precision: decimal[1], scale: decimal[2] };
  const named = /^([A-Za-z_][A-Za-z0-9_]*)/.exec(compact);
  return { base: named ? named[1] : "String", length: "", precision: "", scale: "" };
}

export function lengthOf(dataType) {
  const parts = typeParts(dataType);
  if (parts.base === "String") return parts.length;
  if (parts.base === "Decimal" && (parts.precision || parts.scale)) return [parts.precision, parts.scale].filter(Boolean).join(", ");
  return "";
}

export function applyBaseType(field, base) {
  const next = String(base || "String");
  const parts = typeParts(field.dataType);
  if (next === "String") {
    const length = parts.length || "50";
    field.length = length;
    field.dataType = `String(${length})`;
    return;
  }
  if (next === "Decimal") {
    const precision = parts.precision || "18";
    const scale = parts.scale || "2";
    field.length = `${precision}, ${scale}`;
    field.dataType = `Decimal(${precision}, ${scale})`;
    return;
  }
  field.length = "";
  field.dataType = next;
}

export function applyDataType(field, dataType) {
  field.dataType = String(dataType || "").trim() || "String";
  field.length = lengthOf(field.dataType);
}

export function applyLength(field, length) {
  const digits = String(length ?? "").replace(/\D/g, "");
  field.length = digits;
  field.dataType = digits ? `String(${digits})` : "String";
}

export function applyPrecision(field, precision, scale) {
  const current = typeParts(field.dataType);
  const nextPrecision = String(precision ?? current.precision ?? "").replace(/\D/g, "");
  const nextScale = String(scale ?? current.scale ?? "").replace(/\D/g, "");
  field.length = [nextPrecision, nextScale].filter(Boolean).join(", ");
  if (nextPrecision && nextScale) field.dataType = `Decimal(${nextPrecision}, ${nextScale})`;
  else if (nextPrecision) field.dataType = `Decimal(${nextPrecision}, 2)`;
  else field.dataType = "Decimal";
}

export function syncPrimaryKey(model) {
  const next = model.fields.filter((field) => field.isPrimaryKey).map((field) => field.name);
  const same = next.length === model.primaryKey.length && next.every((name, index) => name === model.primaryKey[index]);
  if (!same) model.primaryKey = next;
  for (const field of model.fields) {
    if (field.isPrimaryKey && !field.notNull) field.notNull = true;
  }
}

export function addField(model, partial = {}) {
  const field = createField({
    name: partial.name || `field_${model.fields.length + 1}`,
    dataType: partial.dataType || "String(50)",
    caption: partial.caption || "",
    ...partial,
  });
  model.fields = [...model.fields, field];
  return field;
}

export function addSecondaryKey(model) {
  const key = { id: uid("k"), fields: [] };
  model.keys = [...model.keys, key];
  return key;
}

export function ensureValidate(field, { home = "logic" } = {}) {
  if (field.validate) return field.validate;
  const required = field.notNull
    ? `if (val == null || val === "") {\n    throw new Error("${field.name} is required");\n}`
    : `// ${field.name}\n`;
  field.validate = { param: "val", oldParam: "", body: required, home };
  return field.validate;
}

export const TABLE_EVENTS = [
  ["beforeInsert", "Before Insert"],
  ["onInsert", "On Insert"],
  ["afterInsert", "After Insert"],
  ["beforeModify", "Before Modify"],
  ["onModify", "On Modify"],
  ["afterModify", "After Modify"],
  ["beforeDelete", "Before Delete"],
  ["onDelete", "On Delete"],
  ["afterDelete", "After Delete"],
  ["onDraftActivate", "On Draft Activate"],
].map(([name, label]) => ({ name, label }));

export function ensureEvent(model, name) {
  const existing = model.triggers.find((trigger) => trigger.name === name);
  if (existing) return existing;
  return ensureTrigger(model, name, { home: "logic" });
}

export function eventLine(script, token) {
  const lines = String(script || "").split("\n");
  const escaped = String(token || "").replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`\\b${escaped}\\s*\\(`);
  const index = lines.findIndex((line) => pattern.test(line));
  return index >= 0 ? index + 1 : 1;
}

export function ensureTrigger(model, name, { home = "logic", params = ["rec", "ctx"] } = {}) {
  const existing = model.triggers.find((trigger) => trigger.name === name && trigger.home === home);
  if (existing) return existing;
  const trigger = {
    id: uid("t"),
    name,
    params,
    body: "",
    home,
  };
  model.triggers = [...model.triggers, trigger];
  return trigger;
}

export function ensureMethod(model) {
  const method = {
    id: uid("m"),
    name: "check",
    params: "rec",
    isPublic: true,
    body: "return true;",
    home: "logic",
  };
  model.methods = [...model.methods, method];
  return method;
}

export function moveLogicToCompanion(model) {
  for (const field of model.fields) {
    if (field.validate) field.validate.home = "logic";
  }
  for (const trigger of model.triggers) trigger.home = "logic";
}

export function addDocumentStatus(model) {
  const existing = model.fields.find((field) => field.name === "status");
  if (existing) return existing;
  return addField(model, {
    name: "status",
    dataType: "String(20)",
    caption: "Status",
    notNull: true,
    defaultVal: '"Draft"',
  });
}

function skipString(src, index) {
  const quote = src[index];
  let i = index + 1;
  if (quote !== "`") {
    while (i < src.length) {
      if (src[i] === "\\") {
        i += 2;
        continue;
      }
      if (src[i] === quote) return i + 1;
      i += 1;
    }
    return i;
  }
  while (i < src.length) {
    if (src[i] === "\\") {
      i += 2;
      continue;
    }
    if (src[i] === "`") return i + 1;
    if (src[i] === "$" && src[i + 1] === "{") {
      i += 2;
      let depth = 1;
      while (i < src.length && depth > 0) {
        const ch = src[i];
        if (ch === '"' || ch === "'" || ch === "`") {
          i = skipString(src, i);
          continue;
        }
        if (ch === "{") depth += 1;
        else if (ch === "}") depth -= 1;
        i += 1;
      }
      continue;
    }
    i += 1;
  }
  return i;
}

function splitStatements(src) {
  const out = [];
  let start = 0;
  let depth = 0;
  let i = 0;
  while (i < src.length) {
    const ch = src[i];
    if (ch === "/" && src[i + 1] === "/") {
      i = src.indexOf("\n", i);
      if (i === -1) break;
      continue;
    }
    if (ch === "/" && src[i + 1] === "*") {
      const end = src.indexOf("*/", i + 2);
      i = end === -1 ? src.length : end + 2;
      continue;
    }
    if (ch === '"' || ch === "'" || ch === "`") {
      i = skipString(src, i);
      continue;
    }
    if (ch === "{") {
      depth += 1;
      i += 1;
      continue;
    }
    if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        let j = i + 1;
        while (src[j] === " " || src[j] === "\n" || src[j] === "\r" || src[j] === "\t") j += 1;
        const end = src[j] === ";" ? j + 1 : i + 1;
        const stmt = src.slice(start, end).trim();
        if (stmt) out.push(stmt);
        i = end;
        start = i;
        continue;
      }
      i += 1;
      continue;
    }
    if (ch === ";" && depth === 0) {
      const stmt = src.slice(start, i + 1).trim();
      if (stmt) out.push(stmt);
      i += 1;
      start = i;
      continue;
    }
    i += 1;
  }
  const tail = src.slice(start).trim();
  if (tail) out.push(tail);
  return out;
}

function extractObjects(source) {
  const found = [];
  const re = /\b(tablelogicextension|tablelogic|tableextension|table)\s+([A-Za-z_]\w*)/g;
  let match;
  while ((match = re.exec(source))) {
    const keyword = match[1];
    const name = match[2];
    let i = match.index + match[0].length;
    let target = "";
    const rest = source.slice(i);
    const forMatch = /^\s+for\s+([A-Za-z_]\w*)/.exec(rest);
    const extendsMatch = /^\s+extends\s+([A-Za-z_]\w*)/.exec(rest);
    if (forMatch) {
      target = forMatch[1];
      i += forMatch[0].length;
    } else if (extendsMatch) {
      target = extendsMatch[1];
      i += extendsMatch[0].length;
    }
    while (i < source.length && source[i] !== "{") i += 1;
    if (source[i] !== "{") continue;
    const open = i;
    let depth = 0;
    i += 1;
    while (i < source.length) {
      const ch = source[i];
      if (ch === '"' || ch === "'" || ch === "`") {
        i = skipString(source, i);
        continue;
      }
      if (ch === "{") depth += 1;
      else if (ch === "}") {
        if (depth === 0) break;
        depth -= 1;
      }
      i += 1;
    }
    found.push({ keyword, name, target, body: source.slice(open + 1, i) });
    re.lastIndex = i + 1;
  }
  return found;
}

function parseTrigger(stmt, home) {
  const match = /^(on|before|after)([A-Za-z]\w*)\s*\(([^)]*)\)\s*\{([\s\S]*)\}\s*;?$/.exec(stmt.trim());
  if (!match) return null;
  const params = match[3].split(",").map((part) => part.trim()).filter(Boolean);
  return {
    id: uid("t"),
    name: `${match[1]}${match[2]}`,
    params: params.length ? params : [],
    body: stripIndent(match[4]),
    home,
  };
}

function parseValidate(stmt, home) {
  const match = /^validate(?:\s+([A-Za-z_]\w*))?\s*\(\s*([A-Za-z_]\w*)\s*(?:,\s*([A-Za-z_]\w*))?\s*\)\s*\{([\s\S]*)\}\s*;?$/.exec(stmt.trim());
  if (!match) return null;
  return {
    fieldName: match[1] || "",
    param: match[2],
    oldParam: match[3] || "",
    body: stripIndent(match[4]),
    home,
  };
}

export function stripIndent(body) {
  const normalized = String(body || "").replace(/\r\n/g, "\n").replace(/^\n+/, "").replace(/\s+$/, "");
  if (!normalized) return "";
  const lines = normalized.split("\n");
  if (lines.length === 1) return lines[0].trim();

  const getIndent = (line) => line.match(/^[ \t]*/)?.[0].length || 0;
  const firstIndent = getIndent(lines[0]);
  const restIndents = lines
    .slice(1)
    .filter((line) => line.trim())
    .map(getIndent);
  const restMin = restIndents.length ? Math.min(...restIndents) : 0;

  // Handle legacy DB scripts where line 0 had leading spaces stripped by `.trim()` while lines 1..N kept 8+ spaces
  if (firstIndent === 0 && restMin > 0) {
    return lines
      .map((line, idx) => {
        if (!line.trim()) return "";
        if (idx === 0) return line.trim();
        return line.slice(Math.min(restMin, getIndent(line))).trimEnd();
      })
      .join("\n");
  }

  const allIndents = lines.filter((line) => line.trim()).map(getIndent);
  const min = allIndents.length ? Math.min(...allIndents) : 0;
  return lines
    .map((line) => (line.trim() ? line.slice(Math.min(min, getIndent(line))).trimEnd() : ""))
    .join("\n");
}

function parseRelation(raw) {
  const text = raw.replace(/;$/, "").trim();
  const match = /^([A-Za-z_]\w*)\.([A-Za-z_]\w*)(?:\s*\{([\s\S]*)\})?$/.exec(text);
  if (!match) return { foreignTable: "", foreignColumn: "", onDeleteAction: "Restrict", raw: text };
  const onDelete = /onDelete\s*=\s*([A-Za-z_]\w*)/.exec(match[3] || "");
  return {
    foreignTable: match[1],
    foreignColumn: match[2],
    onDeleteAction: onDelete?.[1] || "Restrict",
    raw: "",
  };
}

function parseComputed(raw) {
  const text = raw.replace(/;$/, "").trim();
  const match = /^(sum|avg|average|count|min|max|lookup|exist)\(\s*([A-Za-z_]\w*)(?:\.([A-Za-z_]\w*))?\s+where\s*\{\s*([\s\S]*?)\s*\}\s*\)$/i.exec(text);
  if (!match) return { computed: text, flowField: null };
  return {
    computed: text,
    flowField: {
      method: match[1].toLowerCase() === "average" ? "avg" : match[1].toLowerCase(),
      targetTable: match[2],
      targetField: match[3] || "",
      filter: match[4].trim(),
    },
  };
}

function assignProp(field, name, value) {
  const expr = value.replace(/;$/, "").trim();
  if (name === "notNull") field.notNull = expr === "true";
  else if (name === "unique") field.unique = expr === "true";
  else if (name === "caption") field.caption = unquote(expr);
  else if (name === "default") field.defaultVal = expr;
  else if (name === "editable") field.editable = expr !== "false";
  else if (name === "visible") field.visible = expr !== "false";
  else if (name === "searchable") field.searchable = expr === "true";
  else if (name === "mask") field.mask = unquote(expr);
  else if (name === "dataClassification") field.dataClassification = expr.replace(/;$/, "");
  else if (name === "drillDownPage") field.drillDownPage = expr;
  else if (name === "onDelete") field.onDeleteAction = expr;
  else if (name === "relation") Object.assign(field, parseRelation(expr));
  else if (name === "computed") Object.assign(field, parseComputed(expr));
  else field.extras.push({ name, value: expr });
}

function parseFieldStatement(stmt) {
  const named = /^([A-Za-z_]\w*)\s*:\s*([\s\S]+)$/.exec(stmt.trim());
  if (!named) return null;
  const rest = named[2];
  let depth = 0;
  let i = 0;
  while (i < rest.length) {
    const ch = rest[i];
    if (ch === '"' || ch === "'" || ch === "`") {
      i = skipString(rest, i);
      continue;
    }
    if (ch === "(") depth += 1;
    else if (ch === ")") depth -= 1;
    else if (depth === 0 && (ch === "{" || ch === "=")) break;
    i += 1;
  }
  const field = createField({ name: named[1], dataType: rest.slice(0, i).trim().replace(/;$/, "") });
  const tail = rest.slice(i).trim();
  if (tail.startsWith("=")) {
    field.defaultVal = tail.slice(1).replace(/;$/, "").trim();
  } else if (tail.startsWith("{")) {
    const inner = tail.slice(1, tail.lastIndexOf("}"));
    for (const prop of splitStatements(inner)) {
      const validate = parseValidate(prop, "table");
      if (validate && !validate.fieldName) {
        field.validate = { param: validate.param, oldParam: validate.oldParam, body: validate.body, home: "table" };
        continue;
      }
      const assignment = /^([A-Za-z_]\w*)\s*=\s*([\s\S]+)$/.exec(prop.trim());
      if (assignment) assignProp(field, assignment[1], assignment[2]);
    }
  }
  field.length = lengthOf(field.dataType);
  return field;
}

function unquote(value) {
  const text = String(value || "").trim();
  if ((text.startsWith('"') && text.endsWith('"')) || (text.startsWith("'") && text.endsWith("'"))) {
    return text.slice(1, -1);
  }
  return text;
}

function quote(value) {
  return `"${String(value).replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function parseMembers(body, home, model, intoExtension) {
  for (const stmt of splitStatements(body)) {
    const validate = parseValidate(stmt, home);
    if (validate?.fieldName) {
      const field = model.fields.find((item) => item.name === validate.fieldName);
      const record = { param: validate.param, oldParam: validate.oldParam, body: validate.body, home };
      if (field && !intoExtension) field.validate = record;
      else intoExtension?.validations.push({ fieldName: validate.fieldName, ...record });
      continue;
    }
    const trigger = parseTrigger(stmt, home);
    if (trigger) {
      if (intoExtension) intoExtension.triggers.push(trigger);
      else model.triggers.push(trigger);
      continue;
    }
    const method = /^(?:(public|private)\s+)?([A-Za-z_]\w*)\s*\(([^)]*)\)\s*\{([\s\S]*)\}\s*;?$/.exec(stmt.trim());
    if (method && method[2] !== "validate") {
      const record = {
        id: uid("m"),
        name: method[2],
        isPublic: method[1] !== "private",
        params: method[3].trim(),
        body: stripIndent(method[4]),
        home,
      };
      if (intoExtension) intoExtension.methods.push(record);
      else model.methods.push(record);
    }
  }
}

export function parseTableDocument(source, fallbackName = "Table") {
  const model = createEmptyTable(fallbackName);
  if (!source || !/\btable\b/.test(source)) return model;
  const objects = extractObjects(source);
  const table = objects.find((item) => item.keyword === "table" || item.keyword === "tableextension");
  if (!table) return model;
  model.name = table.keyword === "table" ? table.name : table.target || fallbackName;
  model.logicName = `${model.name}Logic`;
  for (const stmt of splitStatements(table.body)) {
    const pk = /^primaryKey\s*\(([^)]*)\)\s*;?$/.exec(stmt.trim());
    if (pk) {
      model.primaryKey = pk[1].split(",").map((part) => part.trim()).filter(Boolean);
      continue;
    }
    const key = /^key\s*\(([^)]*)\)\s*;?$/.exec(stmt.trim());
    if (key) {
      model.keys.push({ id: uid("k"), fields: key[1].split(",").map((part) => part.trim()).filter(Boolean) });
      continue;
    }
    const trigger = parseTrigger(stmt, "table");
    if (trigger) {
      model.triggers.push(trigger);
      continue;
    }
    if (/^[A-Za-z_]\w*\s*:/.test(stmt)) {
      const field = parseFieldStatement(stmt);
      if (field) model.fields.push(field);
      continue;
    }
    const prop = /^([A-Za-z_]\w*)\s*=\s*([\s\S]+)$/.exec(stmt.trim());
    if (!prop) continue;
    const value = prop[2].replace(/;$/, "").trim();
    if (prop[1] === "caption") model.caption = unquote(value);
    else if (prop[1] === "description") model.description = unquote(value);
    else if (prop[1] === "tableType") model.tableType = value;
    else if (prop[1] === "dataPerCompany") model.dataPerCompany = value !== "false";
    else if (prop[1] === "lookupPage") model.lookupPage = value;
    else if (prop[1] === "drillDownPage") model.drillDownPage = value;
    else if (prop[1] === "dataClassification") model.dataClassification = value;
    else if (prop[1] === "obsoleteState") model.obsoleteState = value;
    else if (prop[1] === "obsoleteReason") model.obsoleteReason = unquote(value);
  }
  for (const field of model.fields) field.isPrimaryKey = model.primaryKey.includes(field.name);

  for (const object of objects) {
    if (object.keyword === "tablelogic") {
      model.logicName = object.name || model.logicName;
      parseMembers(object.body, "logic", model, null);
    } else if (object.keyword === "tablelogicextension") {
      const extension = { name: object.name, validations: [], triggers: [], methods: [] };
      parseMembers(object.body, "extension", model, extension);
      model.extensions.push(extension);
    }
  }
  return model;
}

function emitBlock(body) {
  const text = stripIndent(body);
  if (!text) return "{}";
  const lines = text.split("\n").map((line) => (line.trim() ? `        ${line}` : ""));
  return `{\n${lines.join("\n")}\n    }`;
}

function emitField(field) {
  const props = [];
  if (field.caption) props.push(`caption = ${quote(field.caption)};`);
  if (field.notNull) props.push("notNull = true;");
  if (field.unique) props.push("unique = true;");
  if (field.defaultVal) props.push(`default = ${field.defaultVal};`);
  if (field.editable === false) props.push("editable = false;");
  if (field.visible === false) props.push("visible = false;");
  if (field.searchable) props.push("searchable = true;");
  if (field.mask) props.push(`mask = ${quote(field.mask)};`);
  if (field.dataClassification) props.push(`dataClassification = ${field.dataClassification};`);
  if (field.foreignTable && field.foreignColumn) props.push(`relation = ${field.foreignTable}.${field.foreignColumn};`);
  if (field.onDeleteAction && field.onDeleteAction !== "Restrict" && field.foreignTable) props.push(`onDelete = ${field.onDeleteAction};`);
  if (field.computed) props.push(`computed = ${field.computed.replace(/;$/, "")};`);
  if (field.drillDownPage) props.push(`drillDownPage = ${field.drillDownPage};`);
  for (const extra of field.extras || []) props.push(`${extra.name} = ${extra.value};`);
  if (!props.length && !field.defaultVal) return `    ${field.name}: ${field.dataType};`;
  if (!props.length) return `    ${field.name}: ${field.dataType} = ${field.defaultVal};`;
  return `    ${field.name}: ${field.dataType} {\n        ${props.join("\n        ")}\n    };`;
}

function emitTrigger(trigger) {
  const params = (trigger.params || []).join(", ");
  return `    ${trigger.name}(${params}) ${emitBlock(trigger.body)}`;
}

function emitValidate(fieldName, validate) {
  const params = validate.oldParam ? `${validate.param}, ${validate.oldParam}` : validate.param || "val";
  return `    validate ${fieldName}(${params}) ${emitBlock(validate.body)}`;
}

function emitMethod(method) {
  const vis = method.isPublic === false ? "private " : "public ";
  return `    ${vis}${method.name}(${method.params || ""}) ${emitBlock(method.body)}`;
}

export function emitTableScript(model) {
  const lines = [`table ${model.name} {`];
  if (model.caption) lines.push(`    caption = ${quote(model.caption)};`);
  if (model.description) lines.push(`    description = ${quote(model.description)};`);
  if (model.tableType && model.tableType !== "Normal") lines.push(`    tableType = ${model.tableType};`);
  if (model.dataPerCompany === false) lines.push("    dataPerCompany = false;");
  if (model.lookupPage) lines.push(`    lookupPage = ${model.lookupPage};`);
  if (model.drillDownPage) lines.push(`    drillDownPage = ${model.drillDownPage};`);
  if (model.dataClassification) lines.push(`    dataClassification = ${model.dataClassification};`);
  if (model.obsoleteState && model.obsoleteState !== "None") lines.push(`    obsoleteState = ${model.obsoleteState};`);
  if (model.obsoleteReason) lines.push(`    obsoleteReason = ${quote(model.obsoleteReason)};`);
  if (lines.length > 1) lines.push("");
  for (const field of model.fields) lines.push(emitField(field));
  if (model.primaryKey.length) lines.push("", `    primaryKey(${model.primaryKey.join(", ")});`);
  for (const key of model.keys) {
    const cols = (key.fields || []).filter(Boolean);
    if (cols.length) lines.push(`    key(${cols.join(", ")});`);
  }
  lines.push("}");
  return `${lines.join("\n")}\n`;
}

function emitLogicBody(model, bucket) {
  const lines = [];
  const validations = model.fields.filter((field) => field.validate).map((field) => emitValidate(field.name, field.validate));
  lines.push(...validations);
  for (const trigger of model.triggers) lines.push(emitTrigger(trigger));
  for (const method of bucket === "logic" ? model.methods : []) lines.push(emitMethod(method));
  return lines;
}

export function emitLogicScript(model) {
  const lines = emitLogicBody(model, "logic");
  if (!lines.length) return "";
  return `tablelogic ${model.logicName || `${model.name}Logic`} for ${model.name} {\n${lines.join("\n")}\n}\n`;
}

export function emitExtensionScripts(model) {
  return (model.extensions || []).map((extension) => {
    const lines = [];
    for (const validate of extension.validations || []) lines.push(emitValidate(validate.fieldName, validate));
    for (const trigger of extension.triggers || []) lines.push(emitTrigger(trigger));
    for (const method of extension.methods || []) lines.push(emitMethod(method));
    return `tablelogicextension ${extension.name} for ${model.name} {\n${lines.join("\n")}\n}\n`;
  }).join("\n");
}

export function emitDocument(model) {
  moveLogicToCompanion(model);
  const tableScript = emitTableScript(model);
  const logicScript = emitLogicScript(model);
  const extensionScript = emitExtensionScripts(model);
  return {
    tableScript,
    logicScript,
    combined: [tableScript, logicScript, extensionScript].filter(Boolean).join("\n"),
  };
}

function flowFromIr(flow) {
  if (!flow) return { computed: "", flowField: null };
  const func = String(flow.func || "Sum").toLowerCase() === "average" ? "avg" : String(flow.func || "sum").toLowerCase();
  const target = flow.target_column ? `${flow.target_table}.${flow.target_column}` : flow.target_table;
  const computed = `${func}(${target} where { 1 == 1 })`;
  return parseComputed(computed);
}

export function documentFromDetails(name, details, companions = []) {
  const definition = details?.definition || {};
  const summary = details?.summary || {};
  const model = createEmptyTable(summary.object_name || name);
  model.caption = definition.caption || summary.caption || "";
  if (model.caption === model.name) model.caption = "";
  const tableType = definition.table_type || summary.sub_type || "Normal";
  model.tableType = typeof tableType === "string" ? tableType : "Normal";
  model.dataPerCompany = definition.data_per_company !== false;
  model.packageName = summary.package_name || "";
  model.logicName = `${model.name}Logic`;

  const primary = new Set((definition.primary_key || []).map(String));
  const relations = new Map((details?.relations || []).map((relation) => [relation.from_column, relation]));
  const flows = new Map((definition.flow_fields || []).map((flow) => [flow.name, flow]));
  const columns = Array.isArray(definition.columns) && definition.columns.length
    ? definition.columns
    : (details?.fields || []).map((field) => ({
        name: field.field_name,
        sql_type: field.sql_type || field.field_type,
        data_type: field.field_type,
        is_nullable: field.is_nullable,
        is_unique: field.is_unique,
        is_primary_key: field.is_primary_key,
        default_val: field.default_val,
        caption: field.caption,
      }));

  model.fields = columns.map((column) => {
    const columnName = column.name || column.field_name;
    const relation = relations.get(columnName) || column.foreign_key;
    const flow = flowFromIr(flows.get(columnName));
    const caption = column.caption && column.caption !== columnName ? column.caption : "";
    return createField({
      name: columnName,
      dataType: bosTypeFromSql(column.data_type || column.field_type || column.sql_type),
      notNull: column.is_nullable === false || primary.has(columnName) || column.is_primary_key,
      unique: Boolean(column.is_unique),
      isPrimaryKey: primary.has(columnName) || Boolean(column.is_primary_key),
      defaultVal: column.default_val || column.default_value || "",
      caption,
      foreignTable: relation?.to_table || relation?.target_table || "",
      foreignColumn: relation?.to_column || relation?.target_column || "",
      onDeleteAction: normalizeDelete(relation?.on_delete),
      computed: flow.computed,
      flowField: flow.flowField,
    });
  });
  model.primaryKey = model.fields.filter((field) => field.isPrimaryKey).map((field) => field.name);

  for (const index of definition.indexes || []) {
    const fields = index.columns || [];
    const sameAsPk = fields.length === model.primaryKey.length && fields.every((col, i) => col === model.primaryKey[i]);
    if (!sameAsPk && fields.length) model.keys.push({ id: uid("k"), fields: [...fields] });
  }

  for (const trigger of definition.triggers || []) {
    const synthesized = String(trigger.name || "").match(/^onValidate_(.+)$/i);
    if (synthesized) {
      const raw = synthesized[1].toLowerCase();
      const field = model.fields.find((candidate) => {
        const snake = candidate.name.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`).replace(/^_/, "").toLowerCase();
        return candidate.name.toLowerCase() === raw || snake === raw;
      });
      if (field && !field.validate) {
        field.validate = {
          param: trigger.params?.[0] || "val",
          oldParam: "",
          body: stripIndent(trigger.script || ""),
          home: "logic",
        };
      }
      continue;
    }
    model.triggers.push({
      id: uid("t"),
      name: trigger.name,
      params: trigger.params?.length ? trigger.params : ["rec", "ctx"],
      body: stripIndent(trigger.script || ""),
      home: "logic",
    });
  }

  for (const companion of companions) {
    const def = companion.details?.definition || companion.definition || companion;
    const kind = companion.kind || "tablelogic";
    if (kind === "tablelogicextension") {
      const extension = {
        name: def.name || companion.details?.summary?.object_name || "Extension",
        validations: (def.field_validations || []).map((item) => ({
          fieldName: item.field_name,
          param: item.param_name || "val",
          oldParam: item.old_param_name || "",
          body: stripIndent(item.script || ""),
          home: "extension",
        })),
        triggers: (def.triggers || []).map((trigger) => ({
          id: uid("t"),
          name: trigger.name,
          params: trigger.params?.length ? trigger.params : ["rec", "ctx"],
          body: stripIndent(trigger.script || ""),
          home: "extension",
        })),
        methods: (def.methods || []).map((method) => ({
          id: uid("m"),
          name: method.name,
          isPublic: method.is_public !== false,
          params: Array.isArray(method.params) ? method.params.map((param) => param[0] || param).join(", ") : "",
          body: stripIndent(method.script || ""),
          home: "extension",
        })),
      };
      model.extensions.push(extension);
      continue;
    }
    model.logicName = def.name || companion.details?.summary?.object_name || model.logicName;
    for (const item of def.field_validations || []) {
      const field = model.fields.find((candidate) => candidate.name === item.field_name);
      const validate = {
        param: item.param_name || "val",
        oldParam: item.old_param_name || "",
        body: stripIndent(item.script || ""),
        home: "logic",
      };
      if (field) field.validate = validate;
    }
    for (const trigger of def.triggers || []) {
      const existing = model.triggers.find((item) => item.name === trigger.name);
      const next = {
        id: existing?.id || uid("t"),
        name: trigger.name,
        params: trigger.params?.length ? trigger.params : ["rec", "ctx"],
        body: stripIndent(trigger.script || existing?.body || ""),
        home: "logic",
      };
      if (existing) Object.assign(existing, next);
      else model.triggers.push(next);
    }
    for (const method of def.methods || []) {
      model.methods.push({
        id: uid("m"),
        name: method.name,
        isPublic: method.is_public !== false,
        params: Array.isArray(method.params) ? method.params.map((param) => (Array.isArray(param) ? param[0] : param)).join(", ") : "",
        body: stripIndent(method.script || ""),
        home: "logic",
      });
    }
  }
  return model;
}

function normalizeDelete(value) {
  const text = String(value || "Restrict").toLowerCase();
  if (text.includes("cascade")) return "Cascade";
  if (text.includes("null")) return "SetNull";
  return "Restrict";
}

export function adoptModel(target, next) {
  target.name = next.name;
  target.caption = next.caption;
  target.description = next.description;
  target.tableType = next.tableType;
  target.dataPerCompany = next.dataPerCompany;
  target.lookupPage = next.lookupPage;
  target.drillDownPage = next.drillDownPage;
  target.dataClassification = next.dataClassification;
  target.obsoleteState = next.obsoleteState;
  target.obsoleteReason = next.obsoleteReason;
  target.packageName = next.packageName;
  target.logicName = next.logicName;
  target.fields = next.fields;
  target.primaryKey = next.primaryKey;
  target.keys = next.keys;
  target.triggers = next.triggers;
  target.methods = next.methods;
  target.extensions = next.extensions;
}
