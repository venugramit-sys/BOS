/**
 * BOS-Script Custom Language Definition & Full IntelliSense Suite for Monaco Editor
 * Supports all 14 BOS Meta-Objects, embedded TypeScript, live Catalog completion,
 * Hover documentation, Signature Help, Document Symbols, Code Actions, and Formatting.
 */

import { formatBosScriptSource, formatRangeText, formatTypeScriptSource } from "./bosPrettier.js";

const registeredDisposablesByMonaco =
    globalThis.__bosMonacoRegisteredDisposables ||
    (globalThis.__bosMonacoRegisteredDisposables = new WeakMap());

let openObjectCallback = null;

/**
 * Register a callback invoked when the user triggers Go-To-Definition (`F12` / `Ctrl+Click`)
 * on a catalog meta-object defined outside the current editor buffer.
 * @param {(name: string, mode: string, type: string) => void} handler
 */
export function setBosScriptOpenObjectHandler(handler) {
    openObjectCallback = typeof handler === "function" ? handler : null;
}

/**
 * Live catalog context shared with Monaco providers (persisted on globalThis across HMR).
 * Updated dynamically by MonacoCodeStudio / monacoLspProvider.
 */
const liveWorkspaceCatalog =
    globalThis.__bosLiveWorkspaceCatalog ||
    (globalThis.__bosLiveWorkspaceCatalog = {
        tables: new Map(),      // name -> { name, fields: [{ name, data_type, is_primary_key, is_flow_field }], description }
        enums: new Map(),       // name -> { name, values: [string] }
        codeunits: new Map(),   // name -> { name, methods: [{ name, params, return_type }] }
        interfaces: new Map(),  // name -> { name, methods: [string] }
        pages: new Map(),       // name -> { name, source_table }
        views: new Map(),       // name -> { name }
        queries: new Map()      // name -> { name }
    });

/**
 * Update the live catalog context used by BOS-Script IntelliSense providers.
 * When `catalogData.replace` is true, replaces the catalog maps with only the active database's objects
 * (while preserving already-loaded field schemas for tables that still exist in the active database).
 * @param {object} catalogData
 */
export function updateBosScriptCatalogContext(catalogData = {}) {
    const shouldReplace = Boolean(catalogData.replace);

    if (Array.isArray(catalogData.tables)) {
        const previousTables = new Map(liveWorkspaceCatalog.tables);
        if (shouldReplace) {
            liveWorkspaceCatalog.tables.clear();
        }
        for (const t of catalogData.tables) {
            if (!t || !t.name) continue;
            const existing = previousTables.get(t.name) || { name: t.name, fields: [] };
            liveWorkspaceCatalog.tables.set(t.name, {
                ...existing,
                ...t,
                fields: Array.isArray(t.fields) && t.fields.length > 0 ? t.fields : existing.fields
            });
        }
    }
    if (Array.isArray(catalogData.enums)) {
        if (shouldReplace) {
            liveWorkspaceCatalog.enums.clear();
        }
        for (const e of catalogData.enums) {
            if (!e || !e.name) continue;
            liveWorkspaceCatalog.enums.set(e.name, {
                name: e.name,
                values: Array.isArray(e.values) ? e.values : []
            });
        }
    }
    if (Array.isArray(catalogData.codeunits)) {
        if (shouldReplace) {
            liveWorkspaceCatalog.codeunits.clear();
        }
        for (const c of catalogData.codeunits) {
            if (!c || !c.name) continue;
            liveWorkspaceCatalog.codeunits.set(c.name, {
                name: c.name,
                methods: Array.isArray(c.methods) ? c.methods : []
            });
        }
    }
    if (Array.isArray(catalogData.interfaces)) {
        if (shouldReplace) {
            liveWorkspaceCatalog.interfaces.clear();
        }
        for (const iface of catalogData.interfaces) {
            if (!iface || !iface.name) continue;
            liveWorkspaceCatalog.interfaces.set(iface.name, iface);
        }
    }
}

/**
 * Register a single table schema (e.g., from /api/meta/tables/:name/schema) into live IntelliSense.
 */
export function registerTableSchemaForIntelliSense(tableName, fields = []) {
    if (!tableName) return;
    const normalizedFields = (Array.isArray(fields) ? fields : [])
        .map((f) => ({
            name: f.name || f.field_name || f.column_name,
            data_type: f.data_type || f.field_type || f.sql_type || f.type || "any",
            is_primary_key: Boolean(f.is_primary_key),
            is_flow_field: Boolean(f.is_flow_field || f.is_flowfield)
        }))
        .filter((f) => Boolean(f.name));

    const existing = liveWorkspaceCatalog.tables.get(tableName) || { name: tableName, fields: [] };
    liveWorkspaceCatalog.tables.set(tableName, {
        ...existing,
        name: tableName,
        fields: normalizedFields.length > 0 ? normalizedFields : existing.fields
    });
}

/**
 * Parse `declare interface <Table>Record { ... }` and `declare type <Enum>Enum = ...`
 * from `/api/meta/ambient.d.ts` so every active database table has its exact fields
 * populated in `liveWorkspaceCatalog.tables` immediately.
 */
export function hydrateCatalogFromAmbientDts(dtsText = "") {
    if (!dtsText || typeof dtsText !== "string") return;

    const recordRe = /declare\s+interface\s+([A-Za-z_][A-Za-z0-9_]*)Record\s*\{([\s\S]*?)\n\}/g;
    let match;
    while ((match = recordRe.exec(dtsText)) !== null) {
        const tableName = match[1];
        const body = match[2] || "";
        const fields = [];
        const lines = body.split("\n");
        let lastComment = "";

        for (const rawLine of lines) {
            const line = rawLine.trim();
            if (line.startsWith("/**") && line.endsWith("*/")) {
                lastComment = line;
                continue;
            }
            const propMatch = line.match(/^(readonly\s+)?([A-Za-z_][A-Za-z0-9_]*)\??\s*:\s*([^;(]+);$/);
            if (propMatch) {
                const isReadonly = Boolean(propMatch[1]);
                const fName = propMatch[2];
                const fType = propMatch[3].trim();
                const isFlow = isReadonly || lastComment.includes("FlowField");
                fields.push({
                    name: fName,
                    data_type: fType,
                    is_primary_key: fName === "id" || fName === "code" || fName === "no",
                    is_flow_field: isFlow
                });
            }
            lastComment = "";
        }

        if (tableName && fields.length > 0) {
            registerTableSchemaForIntelliSense(tableName, fields);
        }
    }

    const enumRe = /declare\s+type\s+([A-Za-z_][A-Za-z0-9_]*)Enum\s*=\s*([^;]+);/g;
    let enumMatch;
    while ((enumMatch = enumRe.exec(dtsText)) !== null) {
        const enumName = enumMatch[1];
        const rawVals = enumMatch[2] || "";
        const values = Array.from(rawVals.matchAll(/"([^"]+)"/g)).map((m) => m[1]);
        if (enumName && values.length > 0) {
            liveWorkspaceCatalog.enums.set(enumName, { name: enumName, values });
        }
    }
}

/**
 * Resolve a known table name (case-insensitive) from the active catalog or current document.
 */
function resolveKnownTableName(candidate, docSymbols = [], targetTable = null) {
    if (!candidate) return null;
    const clean = candidate.replace(/Record$/, "");
    if (liveWorkspaceCatalog.tables.has(clean)) return clean;
    if (targetTable && targetTable.toLowerCase() === clean.toLowerCase()) return targetTable;
    for (const key of liveWorkspaceCatalog.tables.keys()) {
        if (key.toLowerCase() === clean.toLowerCase()) return key;
    }
    for (const sym of docSymbols) {
        if (sym.kind === "table" && sym.name && sym.name.toLowerCase() === clean.toLowerCase()) {
            return sym.name;
        }
    }
    return null;
}

/**
 * Map a BOS-Script or SQL field type (e.g., `String(100)`, `Decimal(18, 2)`, `Integer`, `Boolean`, `DateTime`, `Json`)
 * to its corresponding primitive runtime category for IntelliSense (`string`, `decimal`, `number`, `boolean`, `date`, `json`).
 */
function inferPrimitiveKindFromFieldType(dataType = "") {
    const dt = String(dataType || "").toLowerCase().trim();
    if (!dt) return null;
    if (dt === "decimal" || dt.startsWith("decimal(") || dt.startsWith("numeric(")) {
        return "decimal";
    }
    if (
        dt === "string" ||
        dt.startsWith("string(") ||
        dt === "text" ||
        dt === "code" ||
        dt.startsWith("code(") ||
        dt === "uuid" ||
        dt === "guid" ||
        dt.startsWith("varchar") ||
        dt.startsWith("char")
    ) {
        return "string";
    }
    if (
        dt === "integer" ||
        dt === "int" ||
        dt === "bigint" ||
        dt === "biginteger" ||
        dt === "number" ||
        dt === "float"
    ) {
        return "number";
    }
    if (dt === "boolean" || dt === "bool") {
        return "boolean";
    }
    if (dt === "date" || dt === "datetime" || dt === "time" || dt.startsWith("timestamp")) {
        return "date";
    }
    if (dt === "json" || dt === "jsonb" || dt === "object") {
        return "json";
    }
    return null;
}

/**
 * Look up a field's declared type on a table (consulting both live catalog and current buffer `localFields`).
 */
function resolveTableFieldPrimitiveKind(tableName, fieldName, localFields = [], targetTable = null) {
    if (!fieldName) return null;
    const fLower = fieldName.toLowerCase();
    if (!tableName || (targetTable && tableName.toLowerCase() === targetTable.toLowerCase())) {
        const localHit = localFields.find((lf) => lf.name.toLowerCase() === fLower);
        if (localHit && localHit.data_type) {
            return inferPrimitiveKindFromFieldType(localHit.data_type);
        }
    }
    const tbl = tableName ? liveWorkspaceCatalog.tables.get(tableName) : null;
    if (tbl && Array.isArray(tbl.fields)) {
        const catHit = tbl.fields.find((f) => (f.name || f.column_name || "").toLowerCase() === fLower);
        if (catHit) {
            return inferPrimitiveKindFromFieldType(catHit.data_type || catHit.type);
        }
    }
    return null;
}

/**
 * Parse the current editor buffer to extract local tables, fields, target table, states, methods,
 * and local variable / parameter types (`localVarTypes`) so completions work immediately on unsaved code.
 */
export function parseDocumentSymbolsAndContext(source = "") {
    const lines = source.split("\n");
    const symbols = [];
    const localFields = [];
    const localStates = [];
    const localMethods = [];
    const localVarTypes = new Map(); // varName -> { kind: "model" | "record" | "record_array" | "model_or_record", table: string }
    let currentMetaObject = null;
    let targetTable = null;

    const SINGLE_RECORD_METHODS = new Set([
        "get", "getOrThrow", "findById", "first", "findFirst", "last",
        "create", "insert", "update", "modify", "upsert", "lock"
    ]);
    const ARRAY_RECORD_METHODS = new Set(["findMany", "createMany", "getMany"]);

    const metaHeaderRe = /^\s*(table|tableextension|tablelogic|tablelogicextension|statemachine|statemachineextension|codeunit|interface|enum|enumextension|permissionset|view|query|page|pageextension|report|custommetadata)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+(?:extends|for|implements)\s+([A-Za-z_][A-Za-z0-9_]*))?/;
    const fieldRe = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_<>()[\]\s,|]*?)(?:\s+(?:notNull|unique|default|flowField|caption|editable|validate|;)|\s*(?:;|=|\{))/;
    const methodRe = /^\s*(?:async\s+)?(?:procedure|function)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)(?:\s*:\s*([A-Za-z_][A-Za-z0-9_<>[\]\s|]*))?/;
    const triggerRe = /^\s*(onInsert|onModify|onDelete|onRename|onOpenPage|onAfterGetRecord|onPreReport|onPostReport|guard|effect)\s*\(([^)]*)\)/;
    const stateBlockRe = /^\s*(?:initial\s+)?state\s+([A-Za-z_][A-Za-z0-9_]*)/;
    const statesArrayRe = /^\s*states\s*:\s*\[([^\]]*)\]/;
    const transitionRe = /^\s*transition\s+([A-Za-z_][A-Za-z0-9_]*)\s+from\s+([A-Za-z_][A-Za-z0-9_]*)\s+to\s+([A-Za-z_][A-Za-z0-9_]*)/;
    const keyRe = /^\s*(primaryKey|key)\s*\(([^)]*)\)/;
    const sourceTablePropRe = /^\s*(?:sourceTable|source_table|dataset)\s*:\s*([A-Za-z_][A-Za-z0-9_]*)/;

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        const lineNum = i + 1;

        const headerMatch = line.match(metaHeaderRe);
        if (headerMatch) {
            const [, kind, name, related] = headerMatch;
            currentMetaObject = { kind, name, related, line: lineNum, children: [] };
            if (kind === "table") {
                targetTable = name;
            } else if (related && ["tableextension", "tablelogic", "tablelogicextension", "statemachine", "page", "report"].includes(kind)) {
                targetTable = related;
            }
            symbols.push(currentMetaObject);
            continue;
        }

        const srcTblMatch = line.match(sourceTablePropRe);
        if (srcTblMatch && !targetTable) {
            targetTable = srcTblMatch[1];
        }

        const statesArrMatch = line.match(statesArrayRe);
        if (statesArrMatch) {
            const rawStates = statesArrMatch[1].split(",").map((s) => s.trim()).filter(Boolean);
            for (const st of rawStates) {
                if (!localStates.includes(st)) localStates.push(st);
            }
        }

        const stateMatch = line.match(stateBlockRe);
        if (stateMatch) {
            const stName = stateMatch[1];
            if (!localStates.includes(stName)) localStates.push(stName);
            if (currentMetaObject) {
                currentMetaObject.children.push({
                    kind: "state",
                    name: stName,
                    detail: "State",
                    line: lineNum
                });
            }
            continue;
        }

        const transMatch = line.match(transitionRe);
        if (transMatch) {
            const [, trName, fromSt, toSt] = transMatch;
            if (currentMetaObject) {
                currentMetaObject.children.push({
                    kind: "transition",
                    name: `${trName} (${fromSt} -> ${toSt})`,
                    detail: "Transition",
                    line: lineNum
                });
            }
            continue;
        }

        const methodMatch = line.match(methodRe);
        if (methodMatch) {
            const [, fnName, params, retType] = methodMatch;
            localMethods.push({ name: fnName, params: params.trim(), returnType: (retType || "void").trim(), line: lineNum });
            if (currentMetaObject) {
                currentMetaObject.children.push({
                    kind: "method",
                    name: `${fnName}(${params.trim()})`,
                    detail: retType ? `: ${retType.trim()}` : "procedure",
                    line: lineNum
                });
            }
            // Extract typed parameters `paramName: TableName` or `paramName: string` / `number` / `Decimal`
            for (const rawParam of params.split(",")) {
                const pm = rawParam.trim().match(/^([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_]*)(\[\])?/);
                if (pm) {
                    const [, pName, pType, isArr] = pm;
                    const primKind = !isArr ? inferPrimitiveKindFromFieldType(pType) : null;
                    if (primKind) {
                        localVarTypes.set(pName, { kind: primKind, dataType: pType });
                    } else {
                        const resolvedTbl = resolveKnownTableName(pType, symbols, targetTable) || pType;
                        localVarTypes.set(pName, {
                            kind: isArr ? "record_array" : "record",
                            table: resolvedTbl
                        });
                    }
                }
            }
            continue;
        }

        const validateMatch = line.match(/^\s*validate\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)/);
        if (validateMatch && targetTable) {
            const [, valFieldName, valParamName] = validateMatch;
            const fieldPrim = resolveTableFieldPrimitiveKind(targetTable, valFieldName, localFields, targetTable) || "string";
            localVarTypes.set(valParamName, { kind: fieldPrim, field: valFieldName, table: targetTable });
        }

        const trigMatch = line.match(triggerRe);
        if (trigMatch) {
            const [, trigName, params] = trigMatch;
            if (currentMetaObject && !["guard", "effect"].includes(trigName)) {
                currentMetaObject.children.push({
                    kind: "event",
                    name: `${trigName}(${params.trim()})`,
                    detail: "Lifecycle Trigger",
                    line: lineNum
                });
            }
            if (targetTable) {
                const paramNames = params.split(",").map((p) => p.trim().split(":")[0].trim()).filter(Boolean);
                if (paramNames[0]) localVarTypes.set(paramNames[0], { kind: "record", table: targetTable });
                if (trigName === "onModify" && paramNames[1]) {
                    localVarTypes.set(paramNames[1], { kind: "record", table: targetTable });
                }
            }
            continue;
        }

        const keyMatch = line.match(keyRe);
        if (keyMatch) {
            const [, keyKind, cols] = keyMatch;
            if (currentMetaObject) {
                currentMetaObject.children.push({
                    kind: "key",
                    name: `${keyKind}(${cols.trim()})`,
                    detail: "Index / Key",
                    line: lineNum
                });
            }
            continue;
        }

        const fieldMatch = line.match(fieldRe);
        if (fieldMatch && currentMetaObject && ["table", "tableextension", "custommetadata"].includes(currentMetaObject.kind)) {
            const [, fName, fType] = fieldMatch;
            const reservedProps = [
                "tableType", "extensible", "dataPerCompany", "caption", "description",
                "pageType", "sourceTable", "usageCategory", "applicationArea", "states",
                "initialState", "dataset", "columns", "filters", "orderBy", "permissions"
            ];
            if (!reservedProps.includes(fName)) {
                localFields.push({ name: fName, data_type: fType.trim(), line: lineNum });
                currentMetaObject.children.push({
                    kind: "field",
                    name: fName,
                    detail: fType.trim(),
                    line: lineNum
                });
            }
            continue;
        }

        // Infer local variable types in procedural blocks (`codeunit`, `tablelogic`, `statemachine`)
        // 1. `const x = models.Note.first(...)` or `ctx.db.Note.findMany(...)` or `Note.get(...)`
        const repoCallMatch = line.match(
            /\b(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:await\s+)?(?:(?:models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*))\.([A-Za-z_][A-Za-z0-9_]*)\s*\(/
        );
        if (repoCallMatch) {
            const [, varName, explicitTbl, directReceiver, methodName] = repoCallMatch;
            let tbl = explicitTbl || null;
            if (!tbl && directReceiver) {
                if (localVarTypes.get(directReceiver)?.kind === "model") {
                    tbl = localVarTypes.get(directReceiver).table;
                } else {
                    tbl = resolveKnownTableName(directReceiver, symbols, targetTable);
                }
            }
            if (tbl) {
                if (SINGLE_RECORD_METHODS.has(methodName)) {
                    localVarTypes.set(varName, { kind: "record", table: tbl });
                } else if (ARRAY_RECORD_METHODS.has(methodName)) {
                    localVarTypes.set(varName, { kind: "record_array", table: tbl });
                }
            }
            continue;
        }

        // 2. `const note = models.Note` or `ctx.db.Note` or `ctx.db.getModel("Note")` or `const note = Note`
        const modelAliasMatch = line.match(
            /\b(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:(?:models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)|ctx\.db\.getModel\(\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\)|([A-Za-z_][A-Za-z0-9_]*))\s*;?\s*$/
        );
        if (modelAliasMatch) {
            const [, varName, explicitTbl, getModelTbl, bareIdent] = modelAliasMatch;
            const tbl = explicitTbl || getModelTbl || resolveKnownTableName(bareIdent, symbols, targetTable);
            if (tbl) {
                localVarTypes.set(varName, { kind: "model", table: tbl });
                continue;
            }
        }

        // 3. Explicit TypeScript variable type annotation: `const note: Note` / `const s: string` / `let d: Decimal`
        const typedVarMatch = line.match(
            /\b(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?:TableModel<\s*([A-Za-z_][A-Za-z0-9_]*)>|([A-Za-z_][A-Za-z0-9_]*)(\[\])?)/
        );
        if (typedVarMatch) {
            const [, varName, modelGeneric, typeIdent, isArr] = typedVarMatch;
            if (modelGeneric) {
                const tbl = resolveKnownTableName(modelGeneric, symbols, targetTable) || modelGeneric.replace(/Record$/, "");
                localVarTypes.set(varName, { kind: "model", table: tbl });
                continue;
            }
            if (typeIdent) {
                const primKind = !isArr ? inferPrimitiveKindFromFieldType(typeIdent) : null;
                if (primKind) {
                    localVarTypes.set(varName, { kind: primKind, dataType: typeIdent });
                    continue;
                }
                const tbl = resolveKnownTableName(typeIdent, symbols, targetTable);
                if (tbl) {
                    localVarTypes.set(varName, { kind: isArr ? "record_array" : "record", table: tbl });
                    continue;
                }
            }
        }

        // 3b. Primitive / BosDecimal / Field-access variable inference:
        // e.g. `const text = rec.content || ""`, `const amt = ctx.decimal(rec.amount)`, `const count = Note.count(...)`, `const ok = Note.exists(...)`
        const varAssignMatch = line.match(/\b(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.+?)\s*;?\s*$/);
        if (varAssignMatch && !localVarTypes.has(varAssignMatch[1])) {
            const [, varName, rhsRaw] = varAssignMatch;
            const rhs = rhsRaw.trim();

            if (/\b(?:ctx\.decimal|Decimal)\s*\(/.test(rhs)) {
                localVarTypes.set(varName, { kind: "decimal" });
                continue;
            }
            if (/\bnew\s+Date\s*\(/.test(rhs)) {
                localVarTypes.set(varName, { kind: "date", dataType: "Date" });
                continue;
            }
            if (/\bnew\s+Map\b/.test(rhs)) {
                localVarTypes.set(varName, { kind: "map", dataType: "Map" });
                continue;
            }
            if (/\bnew\s+Set\b/.test(rhs)) {
                localVarTypes.set(varName, { kind: "set", dataType: "Set" });
                continue;
            }
            const newRecordMatch = rhs.match(/\bnew\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/);
            if (newRecordMatch) {
                const candidateTbl = resolveKnownTableName(newRecordMatch[1], symbols, targetTable);
                if (candidateTbl) {
                    localVarTypes.set(varName, { kind: "record", table: candidateTbl });
                    continue;
                }
            }
            if (/\.\s*split\s*\(/.test(rhs) || /^\[.*\]$/.test(rhs) || /\bnew\s+Array\b/.test(rhs)) {
                localVarTypes.set(varName, { kind: "array" });
                continue;
            }
            if (
                /\.\s*(?:count|indexOf|lastIndexOf|toNumber)\s*\(/.test(rhs) ||
                /\.\s*length\b/.test(rhs) ||
                /^[-+]?\d+(?:\.\d+)?$/.test(rhs) ||
                /\b(?:Math|Number|parseInt|parseFloat)\b/.test(rhs)
            ) {
                localVarTypes.set(varName, { kind: "number" });
                continue;
            }
            if (
                /\.\s*(?:exists|testField|hasRole|canTransition|includes|startsWith|endsWith|eq|gt|gte|lt|lte|isZero|isNegative|isPositive)\s*\(/.test(rhs) ||
                /^(?:true|false)\b/.test(rhs) ||
                /^Boolean\s*\(/.test(rhs)
            ) {
                localVarTypes.set(varName, { kind: "boolean" });
                continue;
            }
            if (/\.\s*(?:add|sub|mul|div|round|abs|neg)\s*\(/.test(rhs)) {
                localVarTypes.set(varName, { kind: "decimal" });
                continue;
            }
            if (
                /^["'`]/.test(rhs) ||
                /\|\|\s*["'`]/.test(rhs) ||
                /\?\?\s*["'`]/.test(rhs) ||
                /\bString\s*\(/.test(rhs) ||
                /\bctx\.(?:now|crypto\.sha256|tenantId|companyId|userId)\b/.test(rhs) ||
                /\.\s*(?:trim|trimStart|trimEnd|toLowerCase|toUpperCase|slice|substring|replace|replaceAll|padStart|padEnd| toString)\s*\(/.test(rhs)
            ) {
                localVarTypes.set(varName, { kind: "string" });
                continue;
            }

            // Record field access: `const text = rec.content` or `const x = item.amount`
            const fieldAccessMatch = rhs.match(/^([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)(?:\s*(?:\|\||\?\?)\s*(.+))?$/);
            if (fieldAccessMatch) {
                const [, recv, propName, fallbackExpr] = fieldAccessMatch;
                let recvTable = null;
                if (["rec", "xRec", "Rec", "this"].includes(recv)) {
                    recvTable = targetTable;
                } else if (localVarTypes.get(recv)?.kind === "record") {
                    recvTable = localVarTypes.get(recv).table;
                }
                if (recvTable) {
                    const fieldPrim = resolveTableFieldPrimitiveKind(recvTable, propName, localFields, targetTable);
                    if (fieldPrim) {
                        localVarTypes.set(varName, { kind: fieldPrim, field: propName, table: recvTable });
                        continue;
                    }
                    if (fallbackExpr) {
                        if (/^["'`]/.test(fallbackExpr.trim())) {
                            localVarTypes.set(varName, { kind: "string" });
                            continue;
                        }
                        if (/^[-+]?\d/.test(fallbackExpr.trim())) {
                            localVarTypes.set(varName, { kind: "number" });
                            continue;
                        }
                    }
                    // Even if field isn't in catalog yet, a property of `rec.<field>` is a scalar field value (default to string), NOT a TableModel!
                    localVarTypes.set(varName, { kind: "string", field: propName, table: recvTable });
                    continue;
                }
            }
        }

        // 4. `for (const item of notes)` or `for (const item of models.Note.findMany(...))`
        const forOfMatch = line.match(
            /\bfor\s*\(\s*(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s+of\s+(?:(?:models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)\.findMany|([A-Za-z_][A-Za-z0-9_]*))/
        );
        if (forOfMatch) {
            const [, loopVar, explicitTbl, iterVar] = forOfMatch;
            if (explicitTbl) {
                localVarTypes.set(loopVar, { kind: "record", table: explicitTbl });
            } else if (iterVar && localVarTypes.get(iterVar)?.kind === "record_array") {
                localVarTypes.set(loopVar, { kind: "record", table: localVarTypes.get(iterVar).table });
            }
        }

        // 5. Array higher-order callbacks: `notes.filter(n => ...)` / `notes.map((n) => ...)` / `notes.forEach(n => ...)`
        const arrayCbMatch = line.match(
            /\b([A-Za-z_][A-Za-z0-9_]*)\.(?:filter|map|find|forEach|some|every)\s*\(\s*\(?([A-Za-z_][A-Za-z0-9_]*)\)?\s*=>/
        );
        if (arrayCbMatch) {
            const [, arrVar, itemVar] = arrayCbMatch;
            if (localVarTypes.get(arrVar)?.kind === "record_array") {
                localVarTypes.set(itemVar, { kind: "record", table: localVarTypes.get(arrVar).table });
            }
        }

        // 6. Incomplete variable declaration matching a table name case-insensitively (e.g. `var note` when `Note` exists)
        const bareDeclMatch = line.match(/\b(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\b/);
        if (bareDeclMatch) {
            const varName = bareDeclMatch[1];
            if (!localVarTypes.has(varName)) {
                const matchedTable = resolveKnownTableName(varName, symbols, targetTable);
                if (matchedTable) {
                    localVarTypes.set(varName, { kind: "model_or_record", table: matchedTable });
                }
            }
        }
    }

    return {
        symbols,
        localFields,
        localStates,
        localMethods,
        localVarTypes,
        targetTable,
        currentMetaObject
    };
}

/**
 * Brace-, bracket-, and property-key-aware backward scanner for `TableModel` / `ctx.db` calls.
 * Ensures zero manual property typing across `findMany`, `first`, `findFirst`, `last`, `get`,
 * `getOrThrow`, `findById`, `exists`, `count`, `create`, `insert`, `createMany`, `update`,
 * `modify`, and `upsert` at any nesting depth (including after closing nested `{ contains: ... }`
 * or inside `OR: [{ | }]`, `AND: [{ | }]`, `select: ["|"]`, or `field: |` value positions).
 */
export function analyzeEnclosingModelCallContext(model, position, docCtx) {
    const startLine = Math.max(1, position.lineNumber - 60);
    const lines = [];
    for (let ln = startLine; ln <= position.lineNumber; ln++) {
        const text = model.getLineContent(ln);
        lines.push(ln === position.lineNumber ? text.slice(0, position.column - 1) : text);
    }
    const textBefore = lines.join("\n");

    // Walk forward from the start of `textBefore` to accurately track string literals and open `(`, `{`, `[`
    // so an unclosed quote right at the cursor position (e.g. `select: ["|`) never corrupts backward parenthesis matching!
    const openParens = [];
    let scanInString = null;
    for (let i = 0; i < textBefore.length; i++) {
        const ch = textBefore[i];
        const prev = i > 0 ? textBefore[i - 1] : "";

        if (scanInString) {
            if (ch === scanInString && prev !== "\\") {
                scanInString = null;
            }
            continue;
        }
        if (ch === '"' || ch === "'" || ch === "`") {
            scanInString = ch;
            continue;
        }
        if (ch === "(") {
            openParens.push(i);
        } else if (ch === ")" && openParens.length > 0) {
            openParens.pop();
        }
    }

    let openParenIdx = -1;
    let callMatch = null;
    for (let k = openParens.length - 1; k >= 0; k--) {
        const idx = openParens[k];
        const prefixBeforeParen = textBefore.slice(0, idx);
        const m = prefixBeforeParen.match(
            /\b(?:(?:models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*))\.(findMany|getMany|first|findFirst|last|get|getOrThrow|findById|exists|count|create|insert|createMany|update|modify|upsert)\s*$/
        );
        if (m) {
            openParenIdx = idx;
            callMatch = m;
            break;
        }
    }

    if (openParenIdx === -1 || !callMatch) return null;

    const [, explicitTbl, receiverIdent, calledMethod] = callMatch;
    let queryTable = explicitTbl || null;
    if (!queryTable && receiverIdent) {
        const varInfo = docCtx.localVarTypes?.get(receiverIdent);
        if (varInfo?.table) {
            queryTable = varInfo.table;
        } else {
            queryTable = resolveKnownTableName(receiverIdent, docCtx.symbols, docCtx.targetTable);
        }
    }
    if (!queryTable) return null;

    // Now scan forward from `openParenIdx + 1` to the cursor to build the exact stack of open `{` and `[` frames!
    const argsSlice = textBefore.slice(openParenIdx + 1);
    const stack = []; // Each frame: { type: "brace" | "bracket", key: string | null, keysDefined: Set<string>, lastPropKey: string | null, afterColon: boolean }
    let fInString = null;
    let lastIdentifier = null;
    let sawColonAfterIdent = false;
    let topLevelArgIndex = 0;

    for (let i = 0; i < argsSlice.length; i++) {
        const ch = argsSlice[i];
        const prev = i > 0 ? argsSlice[i - 1] : "";

        if (fInString) {
            if (ch === fInString && prev !== "\\") {
                fInString = null;
            }
            continue;
        }
        if (ch === '"' || ch === "'" || ch === "`") {
            fInString = ch;
            continue;
        }

        if (/[A-Za-z_]/.test(ch)) {
            let j = i + 1;
            while (j < argsSlice.length && /[A-Za-z0-9_]/.test(argsSlice[j])) j++;
            const ident = argsSlice.slice(i, j);
            const top = stack[stack.length - 1];
            if (!top || !top.afterColon) {
                lastIdentifier = ident;
            }
            i = j - 1;
            continue;
        }

        if (ch === ":") {
            const top = stack[stack.length - 1];
            if (top && top.type === "brace" && !top.afterColon && lastIdentifier) {
                top.keysDefined.add(lastIdentifier);
                top.lastPropKey = lastIdentifier;
                top.afterColon = true;
                sawColonAfterIdent = true;
            }
            continue;
        }

        if (ch === ",") {
            const top = stack[stack.length - 1];
            if (!top) {
                topLevelArgIndex++;
            } else if (top.type === "brace") {
                top.afterColon = false;
                top.lastPropKey = null;
            }
            lastIdentifier = null;
            sawColonAfterIdent = false;
            continue;
        }

        if (ch === "{") {
            const parent = stack[stack.length - 1];
            let frameKey = null;
            if (parent) {
                if (parent.type === "brace" && parent.afterColon && parent.lastPropKey) {
                    frameKey = parent.lastPropKey;
                } else if (parent.type === "bracket" && parent.key) {
                    frameKey = `${parent.key}[]`;
                }
            }
            stack.push({
                type: "brace",
                key: frameKey,
                keysDefined: new Set(),
                lastPropKey: null,
                afterColon: false
            });
            lastIdentifier = null;
            sawColonAfterIdent = false;
            continue;
        }

        if (ch === "}") {
            if (stack.length > 0 && stack[stack.length - 1].type === "brace") {
                stack.pop();
            }
            lastIdentifier = null;
            sawColonAfterIdent = false;
            continue;
        }

        if (ch === "[") {
            const parent = stack[stack.length - 1];
            let frameKey = null;
            if (parent && parent.type === "brace" && parent.afterColon && parent.lastPropKey) {
                frameKey = parent.lastPropKey;
            }
            stack.push({
                type: "bracket",
                key: frameKey,
                keysDefined: new Set(),
                lastPropKey: null,
                afterColon: false
            });
            lastIdentifier = null;
            sawColonAfterIdent = false;
            continue;
        }

        if (ch === "]") {
            if (stack.length > 0 && stack[stack.length - 1].type === "bracket") {
                stack.pop();
            }
            lastIdentifier = null;
            sawColonAfterIdent = false;
            continue;
        }
    }

    return {
        queryTable,
        calledMethod,
        topLevelArgIndex,
        stack,
        inStringQuote: fInString,
        argsSlice
    };
}

const HOVER_DOCS = {
    // Meta-Objects
    table: {
        title: "Meta-Object: `table`",
        body: "Declares a physical PostgreSQL entity table with automatic multi-tenant isolation (`tenant_id`, `company_id`), optimistic concurrency (`$system_version`), FlowFields, and constraints. Lifecycle scripts belong in `tablelogic`.\n\n```bos\ntable Customer {\n    tableType: Master;\n    id: UUID notNull;\n    name: String(100) notNull;\n    primaryKey(id);\n}\n```"
    },
    tableextension: {
        title: "Meta-Object: `tableextension`",
        body: "Non-invasively appends new columns, secondary indexes, or field validation rules onto an existing base `table` without modifying base package source.\n\n```bos\ntableextension CustomerCrmExt extends Customer {\n    loyalty_tier: String(20) default(\"Standard\");\n}\n```"
    },
    tablelogic: {
        title: "Meta-Object: `tablelogic`",
        body: "Holds every executable script for one table: field `validate field(val)`, lifecycle triggers (`onInsert`, `onModify`, `onDelete`, `before*` / `after*`, draft hooks), and record methods. The table file stays structural."
    },
    codeunit: {
        title: "Meta-Object: `codeunit`",
        body: "Encapsulates transactional domain procedures, ledger posting routines, and event handlers (`@businessEvent`, `@subscribeEvent`, `@pushdown`). Executed in the QuickJS sandbox or compiled to PL/pgSQL."
    },
    statemachine: {
        title: "Meta-Object: `statemachine`",
        body: "Defines a deterministic finite-state workflow governing a table's lifecycle states, transitions, guards, and side effects.\n\n```bos\nstatemachine SalesOrderWorkflow for SalesHeader.status {\n    initial state Draft { allowModify: true; allowDelete: true; }\n    state Released { allowModify: false; }\n    transition release from Draft to Released {}\n}\n```"
    },
    page: {
        title: "Meta-Object: `page`",
        body: "Declarative UI layout definition (`Card`, `List`, `Document`, `RoleCenter`, `Worksheet`) bound to a `sourceTable` with `layout` groups/repeaters and header `actions`."
    },
    pageextension: {
        title: "Meta-Object: `pageextension`",
        body: "Extends an existing `page` with additional fields, groups, or ribbon actions."
    },
    view: {
        title: "Meta-Object: `view`",
        body: "Declares a database SQL view (`CREATE OR REPLACE VIEW`) for high-speed analytical projection."
    },
    query: {
        title: "Meta-Object: `query`",
        body: "Read-only multi-table analytical query meta-object supporting joins, aggregations (`sum`, `count`, `average`), filters, and ordering."
    },
    enum: {
        title: "Meta-Object: `enum`",
        body: "Strongly-typed enumeration of domain values (`extensible: true | false`)."
    },
    interface: {
        title: "Meta-Object: `interface`",
        body: "Polymorphic contract defining method signatures that one or more `codeunit` objects can implement."
    },
    permissionset: {
        title: "Meta-Object: `permissionset`",
        body: "Role-Based Access Control (RBAC) matrix granting `read`, `insert`, `modify`, `delete`, `execute` rights and RLS row filters."
    },
    report: {
        title: "Meta-Object: `report`",
        body: "Structured document/analytical report bound to a `dataset` table with columns, filters, and pre/post triggers."
    },
    custommetadata: {
        title: "Meta-Object: `custommetadata`",
        body: "Deployable configuration records and application settings shipped inside a `.bospkg` bundle."
    },
    // Types
    UUID: { title: "Type: `UUID`", body: "128-bit universally unique identifier (`UUID` in PostgreSQL, `string` in TypeScript)." },
    Decimal: { title: "Type: `Decimal(precision, scale)`", body: "Exact fixed-point currency/quantity number (`NUMERIC(p, s)` in PostgreSQL). Avoids IEEE-754 floating-point rounding bugs." },
    String: { title: "Type: `String(maxLength)`", body: "Variable-length UTF-8 string up to `maxLength` characters (`VARCHAR(n)` in PostgreSQL)." },
    Text: { title: "Type: `Text`", body: "Unlimited-length UTF-8 string (`TEXT` in PostgreSQL)." },
    Integer: { title: "Type: `Integer`", body: "32-bit signed integer (`INTEGER` in PostgreSQL)." },
    BigInteger: { title: "Type: `BigInteger`", body: "64-bit signed integer (`BIGINT` in PostgreSQL)." },
    Boolean: { title: "Type: `Boolean`", body: "Logical `true` / `false` (`BOOLEAN` in PostgreSQL)." },
    Date: { title: "Type: `Date`", body: "Calendar date without time (`DATE` in PostgreSQL, `YYYY-MM-DD`)." },
    DateTime: { title: "Type: `DateTime`", body: "UTC timestamp with timezone (`TIMESTAMPTZ` in PostgreSQL)." },
    Json: { title: "Type: `Json`", body: "Binary indexed JSON document (`JSONB` in PostgreSQL)." },
    // FlowFields & Triggers
    flowField: {
        title: "Property: `flowField(...)`",
        body: "Declarative virtual calculated field evaluated at query time via SQL subquery.\nSupported formulas: `sum`, `count`, `average`, `min`, `max`, `lookup`, `exist`.\n\n```bos\nbalance: Decimal(18, 2) flowField(sum(CustomerLedgerEntry.amount where (customer_id == this.id)));\n```"
    },
    onInsert: {
        title: "Trigger: `onInsert(rec)`",
        body: "Executed inside the database transaction immediately before a new record is inserted. Mutate `rec.<field>` or call `ctx.ui.toast(...)` / `error(...)` to abort."
    },
    onModify: {
        title: "Trigger: `onModify(rec, xRec)`",
        body: "Executed before an existing record is updated. `rec` holds the new values; `xRec` holds the prior committed snapshot."
    },
    onDelete: {
        title: "Trigger: `onDelete(rec)`",
        body: "Executed before a record is deleted. Throw an error or call `error(...)` to prevent deletion."
    },
    validate: {
        title: "Field Trigger: `validate(val)`",
        body: "Executed whenever the field value is modified via UI or codeunit. `val` is the incoming value; `this` / `rec` references the current record."
    },
    ctx: {
        title: "Runtime Context: `ctx: BosSessionContext`",
        body: "Global transactional execution context providing:\n- `ctx.db`: Parameterized SQL & ORM operations\n- `ctx.ui`: Toast notifications, dialogs, dynamic field visibility\n- `ctx.events`: Transactional outbox event publishing\n- `ctx.crypto`: Native SHA-256 & CSPRNG utilities\n- `ctx.audit`: System audit log recording\n- `ctx.dimension`: Accounting dimension validation\n- `ctx.workflow`: State machine transitions\n- `ctx.security`: Permission & role checks"
    },
    models: {
        title: "Typed ORM Registry: `models`",
        body: "Strongly-typed repository access for all registered BOS tables.\n\n```ts\nconst cust = models.Customer.findById(rec.customer_id);\nconst openOrders = models.SalesHeader.findMany({ status: \"Draft\" });\n```"
    }
};

function dedupeCompletionResult(result) {
    if (!result || !Array.isArray(result.suggestions)) return result;
    const seen = new Set();
    const unique = [];
    for (const item of result.suggestions) {
        if (!item) continue;
        const labelStr = typeof item.label === "object" && item.label !== null ? item.label.label : String(item.label ?? "");
        const key = `${labelStr}`;
        if (seen.has(key)) continue;
        seen.add(key);
        unique.push(item);
    }
    return {
        ...result,
        incomplete: false,
        suggestions: unique
    };
}

export function registerBosScriptLanguage(monaco) {
    if (!monaco || !monaco.languages) return;

    // Dispose any previously registered providers on this monaco.languages singleton (including across Vite HMR reloads)
    const keyObj = monaco.languages || monaco;
    const prevDisposables = registeredDisposablesByMonaco.get(keyObj) || globalThis.__bosLastMonacoDisposables;
    if (Array.isArray(prevDisposables)) {
        for (const d of prevDisposables) {
            try {
                if (d && typeof d.dispose === "function") d.dispose();
            } catch (_) {}
        }
    }
    const disposables = [];
    const track = (disposable) => {
        if (disposable && typeof disposable.dispose === "function") {
            disposables.push(disposable);
        }
        return disposable;
    };
    registeredDisposablesByMonaco.set(keyObj, disposables);
    globalThis.__bosLastMonacoDisposables = disposables;

    const alreadyRegisteredLang =
        typeof monaco.languages.getLanguages === "function" &&
        monaco.languages.getLanguages().some((l) => l.id === "bos-script");
    if (!alreadyRegisteredLang) {
        monaco.languages.register({
            id: "bos-script",
            extensions: [".bos"],
            aliases: ["BOS-Script", "bos", "bos-script"]
        });
    }

    // 1. Monarch Tokenizer for all 14 Meta-Objects + Embedded TypeScript
    track(monaco.languages.setMonarchTokensProvider("bos-script", {
        defaultToken: "",
        tokenPostfix: ".bos",

        metaObjectKeywords: [
            "table", "tableextension", "tablelogic", "tablelogicextension",
            "statemachine", "statemachineextension", "codeunit", "interface",
            "enum", "enumextension", "permissionset", "view", "query",
            "page", "pageextension", "report", "custommetadata"
        ],

        keywords: [
            "extends", "implements", "for", "where", "from", "to", "initial",
            "state", "states", "transition", "guard", "effect", "procedure",
            "function", "async", "await", "validate", "onInsert", "onModify",
            "onDelete", "onRename", "onTransition", "onOpenPage", "onAfterGetRecord",
            "onPreReport", "onPostReport", "primaryKey", "key", "indexes",
            "flowField", "notNull", "unique", "default", "caption", "description",
            "editable", "visible", "tableType", "extensible", "dataPerCompany",
            "pageType", "sourceTable", "usageCategory", "applicationArea",
            "layout", "area", "group", "repeater", "part", "field", "actions",
            "action", "dataset", "columns", "filters", "orderBy", "permissions",
            "return", "if", "else", "for", "while", "do", "switch", "case",
            "break", "continue", "let", "const", "var", "new", "throw", "try",
            "catch", "finally", "typeof", "instanceof", "in", "of", "as"
        ],

        typeKeywords: [
            "UUID", "String", "Code", "Text", "Integer", "BigInteger", "Decimal",
            "Boolean", "Date", "DateTime", "Time", "Json", "Blob", "Option", "Enum",
            "Master", "Document", "Ledger", "Setup", "Supplemental", "Register", "Temporary",
            "Card", "List", "RoleCenter", "Worksheet", "ListPart", "CardPart",
            "void", "any", "string", "number", "boolean", "Record", "Promise", "Array"
        ],

        builtinFunctions: [
            "sum", "count", "average", "min", "max", "lookup", "exist",
            "error", "calcFields", "testField", "init", "validateField"
        ],

        contextVariables: [
            "this", "rec", "xRec", "Rec", "ctx", "models", "true", "false", "null", "undefined", "Math", "JSON", "Date", "console"
        ],

        operators: [
            "=", ">", "<", "!", "~", "?", ":", "==", "<=", ">=", "!=",
            "===", "!==", "&&", "||", "??", "++", "--", "+", "-", "*", "/",
            "&", "|", "^", "%", "<<", ">>", ">>>", "+=", "-=", "*=", "/=",
            "=>", "?."
        ],

        symbols: /[=><!~?:&|+\-*\/\^%]+/,
        escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

        tokenizer: {
            root: [
                [/@[a-zA-Z_]\w*/, "annotation"],

                [/[a-zA-Z_$][\w$]*/, {
                    cases: {
                        "@metaObjectKeywords": "keyword.declaration",
                        "@keywords": "keyword",
                        "@typeKeywords": "type",
                        "@builtinFunctions": "predefined",
                        "@contextVariables": "variable.predefined",
                        "@default": "identifier"
                    }
                }],

                { include: "@whitespace" },

                [/[{}()\[\]]/, "@brackets"],
                [/[<>](?!@symbols)/, "@brackets"],
                [/@symbols/, {
                    cases: {
                        "@operators": "operator",
                        "@default": ""
                    }
                }],

                [/\d*\.\d+([eE][\-+]?\d+)?/, "number.float"],
                [/0[xX][0-9a-fA-F]+/, "number.hex"],
                [/\d+/, "number"],

                [/[;,.]/, "delimiter"],

                [/"([^"\\]|\\.)*$/, "string.invalid"],
                [/'([^'\\]|\\.)*$/, "string.invalid"],
                [/"/, "string", "@string_double"],
                [/'/, "string", "@string_single"],
                [/`/, "string", "@string_backtick"]
            ],

            whitespace: [
                [/[ \t\r\n]+/, "white"],
                [/\/\*/, "comment", "@comment"],
                [/\/\/.*$/, "comment"]
            ],

            comment: [
                [/[^\/*]+/, "comment"],
                [/\*\//, "comment", "@pop"],
                [/[\/*]/, "comment"]
            ],

            string_double: [
                [/[^\\"]+/, "string"],
                [/@escapes/, "string.escape"],
                [/\\./, "string.escape.invalid"],
                [/"/, "string", "@pop"]
            ],

            string_single: [
                [/[^\\']+/, "string"],
                [/@escapes/, "string.escape"],
                [/\\./, "string.escape.invalid"],
                [/'/, "string", "@pop"]
            ],

            string_backtick: [
                [/\$\{/, { token: "delimiter.bracket", next: "@bracketCounting" }],
                [/[^\\`$]+/, "string"],
                [/@escapes/, "string.escape"],
                [/\\./, "string.escape.invalid"],
                [/`/, "string", "@pop"]
            ],

            bracketCounting: [
                [/\{/, "delimiter.bracket", "@bracketCounting"],
                [/\}/, "delimiter.bracket", "@pop"],
                { include: "root" }
            ]
        }
    }));

    // 2. Language Configuration (auto-closing pairs, brackets, folding, indentation)
    track(monaco.languages.setLanguageConfiguration("bos-script", {
        comments: {
            lineComment: "//",
            blockComment: ["/*", "*/"]
        },
        brackets: [
            ["{", "}"],
            ["[", "]"],
            ["(", ")"]
        ],
        autoClosingPairs: [
            { open: "{", close: "}" },
            { open: "[", close: "]" },
            { open: "(", close: ")" },
            { open: '"', close: '"', notIn: ["string"] },
            { open: "'", close: "'", notIn: ["string", "comment"] },
            { open: "`", close: "`", notIn: ["string", "comment"] }
        ],
        surroundingPairs: [
            { open: "{", close: "}" },
            { open: "[", close: "]" },
            { open: "(", close: ")" },
            { open: '"', close: '"' },
            { open: "'", close: "'" },
            { open: "`", close: "`" }
        ],
        indentationRules: {
            increaseIndentPattern: /^((?!\/\/).)*(\{[^}"'`]*|\([^)"'`]*|\[[^\]"'`]*)$/,
            decreaseIndentPattern: /^\s*(\}|\)|\])$/
        }
    }));

    // Cursor is in quoted text or in the literal part of a template, not in a `${}` hole.
    function scriptContextAt(model, position) {
        const text = model.getValueInRange({
            startLineNumber: 1,
            startColumn: 1,
            endLineNumber: position.lineNumber,
            endColumn: position.column
        });
        let mode = "code";
        const holes = [];
        for (let i = 0; i < text.length; i++) {
            const c = text[i];
            const n = text[i + 1];
            if (mode === "line") {
                if (c === "\n") mode = "code";
                continue;
            }
            if (mode === "block") {
                if (c === "*" && n === "/") {
                    mode = "code";
                    i += 1;
                }
                continue;
            }
            if (mode === "sq" || mode === "dq") {
                const quote = mode === "sq" ? "'" : '"';
                if (c === "\\") {
                    i += 1;
                    continue;
                }
                if (c === quote) mode = "code";
                continue;
            }
            if (mode === "tpl") {
                if (c === "\\") {
                    i += 1;
                    continue;
                }
                if (c === "`") {
                    mode = "code";
                    continue;
                }
                if (c === "$" && n === "{") {
                    holes.push(1);
                    mode = "code";
                    i += 1;
                }
                continue;
            }
            if (c === "/" && n === "/") {
                mode = "line";
                i += 1;
                continue;
            }
            if (c === "/" && n === "*") {
                mode = "block";
                i += 1;
                continue;
            }
            if (c === "'") {
                mode = "sq";
                continue;
            }
            if (c === '"') {
                mode = "dq";
                continue;
            }
            if (c === "`") {
                mode = "tpl";
                continue;
            }
            if (holes.length && c === "{") {
                holes[holes.length - 1] += 1;
                continue;
            }
            if (holes.length && c === "}") {
                holes[holes.length - 1] -= 1;
                if (holes[holes.length - 1] === 0) {
                    holes.pop();
                    mode = "tpl";
                }
            }
        }
        return mode === "code" ? "code" : "string";
    }

    // 3. Context-Aware Completion Item Provider
    const rawProvideCompletionItems = (model, position) => {
            if (scriptContextAt(model, position) === "string") {
                return { suggestions: [] };
            }
            const word = model.getWordUntilPosition(position);
            const range = {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: word.startColumn,
                endColumn: word.endColumn
            };

            const linePrefix = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
            const fullSource = model.getValue();
            const docCtx = parseDocumentSymbolsAndContext(fullSource);

            // Resolve all fields for a table (merging live catalog + local unsaved fields)
            const getFieldsForTable = (tblName) => {
                const merged = new Map();
                const resolvedName = resolveKnownTableName(tblName, docCtx.symbols, docCtx.targetTable) || tblName;
                const catalogTbl = resolvedName ? liveWorkspaceCatalog.tables.get(resolvedName) : null;
                if (catalogTbl && Array.isArray(catalogTbl.fields)) {
                    for (const f of catalogTbl.fields) {
                        const fName = f.name || f.column_name;
                        if (fName) {
                            merged.set(fName, {
                                name: fName,
                                data_type: f.data_type || f.type || "any",
                                is_primary_key: Boolean(f.is_primary_key),
                                is_flow_field: Boolean(f.is_flow_field)
                            });
                        }
                    }
                }
                if (!resolvedName || resolvedName === docCtx.targetTable) {
                    for (const lf of docCtx.localFields) {
                        merged.set(lf.name, {
                            name: lf.name,
                            data_type: lf.data_type || "any",
                            is_primary_key: false,
                            is_flow_field: false
                        });
                    }
                }
                return Array.from(merged.values());
            };

            const getRepoMethodSuggestions = (tblName, targetRange = range) => {
                const fields = getFieldsForTable(tblName);
                const pkField = fields.find((f) => f.is_primary_key)?.name || fields[0]?.name || "id";
                const sampleField = fields.find((f) => !f.is_primary_key)?.name || pkField;
                const repoMethods = [
                    { label: "get", insert: `get(\${1:${pkField}})`, detail: `(pk) => ${tblName}Record | null`, doc: `Fetch a single \`${tblName}\` record by primary key or return null.` },
                    { label: "getOrThrow", insert: `getOrThrow(\${1:${pkField}})`, detail: `(pk) => ${tblName}Record`, doc: `Fetch a single \`${tblName}\` record by primary key or throw an error.` },
                    { label: "findById", insert: `findById(\${1:${pkField}})`, detail: `(pk) => ${tblName}Record | null`, doc: `Fetch a single \`${tblName}\` record by primary key.` },
                    { label: "first", insert: `first({ where: { \${1:${sampleField}}: \${2:val} } })`, detail: `(query?) => ${tblName}Record | null`, doc: `Find the first \`${tblName}\` record matching filter criteria.` },
                    { label: "findFirst", insert: `findFirst({ where: { \${1:${sampleField}}: \${2:val} } })`, detail: `(query?) => ${tblName}Record | null`, doc: `Find the first \`${tblName}\` record matching filter criteria.` },
                    { label: "last", insert: `last({ where: { \${1:${sampleField}}: \${2:val} } })`, detail: `(query?) => ${tblName}Record | null`, doc: `Find the last \`${tblName}\` record matching filter criteria.` },
                    { label: "findMany", insert: `findMany({ where: { \${1:${sampleField}}: \${2:val} } })`, detail: `(query?) => ${tblName}Record[]`, doc: `Query an array of \`${tblName}\` records matching filter criteria.` },
                    {
                        label: "findMany (Filter Operators & Sort)",
                        insert: `findMany({\n    where: { \${1:${sampleField}}: { \${2|contains,eq,ne,gt,gte,lt,lte,startsWith,endsWith,in|}: \${3:val} } },\n    orderBy: { \${4:${pkField}}: "\${5|asc,desc|}" },\n    take: \${6:50}\n})`,
                        detail: `Advanced Filter + Sort + Pagination`,
                        doc: `Query \`${tblName}\` records using comparison/text operators (\`contains\`, \`gt\`, \`lte\`, \`in\`, etc.), \`orderBy\`, and \`take\`/\`skip\`.`
                    },
                    { label: "create", insert: `create({ \${1:${sampleField}}: \${2:val} })`, detail: `(data) => ${tblName}Record`, doc: `Insert a new \`${tblName}\` record (runs table validations & onInsert).` },
                    { label: "insert", insert: `insert({ \${1:${sampleField}}: \${2:val} })`, detail: `(data) => ${tblName}Record`, doc: `Insert a new \`${tblName}\` record (runs table validations & onInsert).` },
                    { label: "getMany", insert: `getMany([\${1:key}])`, detail: `(keys[]) => ${tblName}Record[]`, doc: `Load many \`${tblName}\` rows by primary key in one query. The compiler hoists \`Model.get(row.field)\` inside \`for...of\` into this.` },
                    { label: "createMany", insert: `createMany([{ \${1:${sampleField}}: \${2:val} }])`, detail: `(records[]) => ${tblName}Record[]`, doc: `Insert many \`${tblName}\` records in one SQL statement.` },
                    { label: "update", insert: `update(\${1:rec.${pkField}}, { \${2:${sampleField}}: \${3:val} })`, detail: `(pk, patch) => ${tblName}Record`, doc: `Update an existing \`${tblName}\` record by primary key.` },
                    { label: "modify", insert: "modify(${1:rec})", detail: `(record) => ${tblName}Record`, doc: `Update an existing \`${tblName}\` record (runs onModify & OCC check).` },
                    { label: "upsert", insert: `upsert({ where: { \${1:${pkField}}: \${2:val} }, create: { \${3} }, update: { \${4} } })`, detail: `(options) => ${tblName}Record`, doc: `Insert or update a \`${tblName}\` record atomically.` },
                    { label: "delete", insert: `delete(\${1:rec.${pkField}})`, detail: `(pk) => boolean`, doc: `Delete a \`${tblName}\` record by primary key.` },
                    { label: "exists", insert: `exists({ \${1:${sampleField}}: \${2:val} })`, detail: `(where?) => boolean`, doc: `Return true if at least one \`${tblName}\` record matches the filter.` },
                    { label: "count", insert: `count({ \${1:${sampleField}}: \${2:val} })`, detail: `(where?) => number`, doc: `Count \`${tblName}\` records matching optional filter.` },
                    { label: "lock", insert: `lock(\${1:rec.${pkField}})`, detail: `(pk) => ${tblName}Record | null`, doc: `Acquire a row-level \`FOR UPDATE\` lock on a \`${tblName}\` record.` },
                    { label: "modifyAll", insert: `modifyAll({ \${1:${sampleField}}: \${2:val} }, { \${3:${sampleField}}: \${4:filterVal} })`, detail: `(patch, where?) => number`, doc: `Set-based bulk SQL UPDATE across all matching \`${tblName}\` records (zero N+1 overhead).` },
                    { label: "deleteAll", insert: `deleteAll({ \${1:${sampleField}}: \${2:filterVal} })`, detail: `(where?) => number`, doc: `Set-based bulk SQL DELETE across all matching \`${tblName}\` records.` },
                    { label: "sumField", insert: `sumField("\${1:${sampleField}}", { \${2} })`, detail: `(fieldName, where?) => BosDecimal`, doc: `Compute exact SQL \`SUM(field)\` across matching \`${tblName}\` records as a \`BosDecimal\`.` },
                    { label: "avgField", insert: `avgField("\${1:${sampleField}}", { \${2} })`, detail: `(fieldName, where?) => BosDecimal`, doc: `Compute exact SQL \`AVG(field)\` across matching \`${tblName}\` records as a \`BosDecimal\`.` },
                    { label: "minField", insert: `minField("\${1:${sampleField}}", { \${2} })`, detail: `(fieldName, where?) => BosDecimal`, doc: `Compute SQL \`MIN(field)\` across matching \`${tblName}\` records as a \`BosDecimal\`.` },
                    { label: "maxField", insert: `maxField("\${1:${sampleField}}", { \${2} })`, detail: `(fieldName, where?) => BosDecimal`, doc: `Compute SQL \`MAX(field)\` across matching \`${tblName}\` records as a \`BosDecimal\`.` }
                ];
                return repoMethods.map((m) => ({
                    label: m.label,
                    kind: monaco.languages.CompletionItemKind.Method,
                    insertText: m.insert,
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    detail: m.detail,
                    documentation: m.doc,
                    range: targetRange
                }));
            };

            const getRecordMemberSuggestions = (tblName, targetRange = range) => {
                const fields = getFieldsForTable(tblName);
                const firstField = fields[0]?.name || "field_name";
                const fieldItems = fields.map((f) => ({
                    label: f.name,
                    kind: monaco.languages.CompletionItemKind.Field,
                    insertText: f.name,
                    detail: `${tblName || "Record"}.${f.name}: ${f.data_type}${f.is_primary_key ? " (PK)" : ""}${f.is_flow_field ? " (FlowField)" : ""}`,
                    documentation: `Field \`${f.name}\` (${f.data_type}) on table \`${tblName || docCtx.targetTable || "CurrentTable"}\`.`,
                    range: targetRange
                }));
                const recordHelpers = [
                    {
                        label: "testField",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: `testField("\${1:${firstField}}")`,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `(fieldName, expectedValue?) => boolean`,
                        documentation: "Assert that a field on this record is non-blank (or equals `expectedValue`), throwing a validation error if not.",
                        range: targetRange
                    },
                    {
                        label: "calcFields",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: `calcFields("\${1:${firstField}}")`,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `(...fieldNames) => this`,
                        documentation: "Evaluate declarative FlowFields on this record via SQL subqueries.",
                        range: targetRange
                    },
                    {
                        label: "validate",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: `validate("\${1:${firstField}}", \${2:newValue})`,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `(fieldName, newValue?) => this`,
                        documentation: "Assign a field value and trigger its field-level `validate` handler.",
                        range: targetRange
                    },
                    {
                        label: "save",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: "save()",
                        detail: `() => this`,
                        documentation: `Persist this \`${tblName || "Record"}\` instance to PostgreSQL (inserts if new, updates if existing).`,
                        range: targetRange
                    },
                    {
                        label: "insert",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: "insert()",
                        detail: `() => this`,
                        documentation: `Insert this \`${tblName || "Record"}\` instance into PostgreSQL.`,
                        range: targetRange
                    },
                    {
                        label: "modify",
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: "modify()",
                        detail: `() => this`,
                        documentation: `Update this \`${tblName || "Record"}\` instance in PostgreSQL.`,
                        range: targetRange
                    }
                ];
                return [...fieldItems, ...recordHelpers];
            };

            const getArrayMemberSuggestions = (tblName, targetRange = range) => {
                const fields = getFieldsForTable(tblName);
                const sampleField = fields[0]?.name || "id";
                const arrMethods = [
                    { label: "filter", insert: `filter((item) => item.\${1:${sampleField}} === \${2:val})`, detail: `(predicate) => ${tblName}Record[]`, doc: `Filter the array of \`${tblName}\` records in memory.` },
                    { label: "map", insert: `map((item) => item.\${1:${sampleField}})`, detail: `(mapper) => any[]`, doc: `Transform each \`${tblName}\` record in the array.` },
                    { label: "find", insert: `find((item) => item.\${1:${sampleField}} === \${2:val})`, detail: `(predicate) => ${tblName}Record | undefined`, doc: `Find the first matching \`${tblName}\` element in the array.` },
                    { label: "forEach", insert: `forEach((item) => {\n    \${1}\n})`, detail: `(callback) => void`, doc: `Iterate over each \`${tblName}\` record.` },
                    { label: "some", insert: `some((item) => item.\${1:${sampleField}} === \${2:val})`, detail: `(predicate) => boolean`, doc: `Return true if at least one \`${tblName}\` record satisfies the predicate.` },
                    { label: "every", insert: `every((item) => Boolean(item.\${1:${sampleField}}))`, detail: `(predicate) => boolean`, doc: `Return true if all \`${tblName}\` records satisfy the predicate.` },
                    { label: "reduce", insert: `reduce((sum, item) => sum + (item.\${1:${sampleField}} || 0), 0)`, detail: `(reducer, initialValue) => any`, doc: `Aggregate values across \`${tblName}\` records.` },
                    { label: "length", insert: "length", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: `Number of \`${tblName}\` records in the array.` }
                ];
                return arrMethods.map((m) => ({
                    label: m.label,
                    kind: m.kind || monaco.languages.CompletionItemKind.Method,
                    insertText: m.insert,
                    insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                    detail: m.detail,
                    documentation: m.doc,
                    range: targetRange
                }));
            };

            const getPrimitiveMemberSuggestions = (primKind, targetRange = range) => {
                if (primKind === "decimal") {
                    const decMethods = [
                        { label: "add", insert: "add(${1:other})", detail: "(other: Decimal | number | string) => BosDecimal", doc: "Exact fixed-point addition (`this + other`)." },
                        { label: "sub", insert: "sub(${1:other})", detail: "(other: Decimal | number | string) => BosDecimal", doc: "Exact fixed-point subtraction (`this - other`)." },
                        { label: "mul", insert: "mul(${1:other})", detail: "(other: Decimal | number | string) => BosDecimal", doc: "Exact fixed-point multiplication (`this * other`)." },
                        { label: "div", insert: "div(${1:other})", detail: "(other: Decimal | number | string) => BosDecimal", doc: "Exact fixed-point division (`this / other`)." },
                        { label: "round", insert: "round(${1:2})", detail: "(scale?: number) => BosDecimal", doc: "Round to `scale` decimal places using banker's / half-up rounding." },
                        { label: "abs", insert: "abs()", detail: "() => BosDecimal", doc: "Return the absolute value of this Decimal." },
                        { label: "neg", insert: "neg()", detail: "() => BosDecimal", doc: "Negate this Decimal (`-this`)." },
                        { label: "eq", insert: "eq(${1:other})", detail: "(other) => boolean", doc: "Return true if equal to `other`." },
                        { label: "gt", insert: "gt(${1:other})", detail: "(other) => boolean", doc: "Return true if strictly greater than `other`." },
                        { label: "gte", insert: "gte(${1:other})", detail: "(other) => boolean", doc: "Return true if greater than or equal to `other`." },
                        { label: "lt", insert: "lt(${1:other})", detail: "(other) => boolean", doc: "Return true if strictly less than `other`." },
                        { label: "lte", insert: "lte(${1:other})", detail: "(other) => boolean", doc: "Return true if less than or equal to `other`." },
                        { label: "isZero", insert: "isZero()", detail: "() => boolean", doc: "Return true if this Decimal equals zero." },
                        { label: "isNegative", insert: "isNegative()", detail: "() => boolean", doc: "Return true if this Decimal is strictly negative (`< 0`)." },
                        { label: "isPositive", insert: "isPositive()", detail: "() => boolean", doc: "Return true if this Decimal is strictly positive (`> 0`)." },
                        { label: "toNumber", insert: "toNumber()", detail: "() => number", doc: "Convert this Decimal to a JavaScript number." },
                        { label: "toFixed", insert: "toFixed(${1:2})", detail: "(fractionDigits?: number) => string", doc: "Format this Decimal with a fixed number of decimal places." },
                        { label: "toString", insert: "toString()", detail: "() => string", doc: "Return the exact string representation of this Decimal." }
                    ];
                    return decMethods.map((m) => ({
                        label: m.label,
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "number") {
                    const numMethods = [
                        { label: "toFixed", insert: "toFixed(${1:2})", detail: "(fractionDigits?: number) => string", doc: "Format a number using fixed-point notation." },
                        { label: "toString", insert: "toString()", detail: "(radix?: number) => string", doc: "Return a string representation of the number." },
                        { label: "toLocaleString", insert: "toLocaleString()", detail: "() => string", doc: "Return a locale-sensitive representation of the number." },
                        { label: "toPrecision", insert: "toPrecision(${1:4})", detail: "(precision?: number) => string", doc: "Return a string containing the number represented to the specified precision." }
                    ];
                    return numMethods.map((m) => ({
                        label: m.label,
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "boolean") {
                    return [
                        {
                            label: "toString",
                            kind: monaco.languages.CompletionItemKind.Method,
                            insertText: "toString()",
                            detail: "() => string",
                            documentation: 'Return `"true"` or `"false"`.',
                            range: targetRange
                        }
                    ];
                }

                if (primKind === "array") {
                    const arrMembers = [
                        { label: "length", insert: "length", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: "The number of elements in the array." },
                        { label: "join", insert: 'join("${1:, }")', detail: "(separator?: string) => string", doc: "Adds all the elements of an array into a string, separated by the specified separator string." },
                        { label: "includes", insert: "includes(${1:searchElement})", detail: "(searchElement: any) => boolean", doc: "Determines whether an array includes a certain element." },
                        { label: "map", insert: "map((item) => ${1:item})", detail: "(callbackfn) => any[]", doc: "Calls a defined callback function on each element of an array, and returns an array that contains the results." },
                        { label: "filter", insert: "filter((item) => ${1:Boolean(item)})", detail: "(predicate) => any[]", doc: "Returns the elements of an array that meet the condition specified in a callback function." },
                        { label: "find", insert: "find((item) => ${1:Boolean(item)})", detail: "(predicate) => any | undefined", doc: "Returns the value of the first element in the array where predicate is true." },
                        { label: "forEach", insert: "forEach((item) => {\n    ${1}\n})", detail: "(callbackfn) => void", doc: "Performs the specified action for each element in an array." },
                        { label: "some", insert: "some((item) => ${1:Boolean(item)})", detail: "(predicate) => boolean", doc: "Determines whether the specified callback function returns true for any element of an array." },
                        { label: "every", insert: "every((item) => ${1:Boolean(item)})", detail: "(predicate) => boolean", doc: "Determines whether all the members of an array satisfy the specified test." },
                        { label: "reduce", insert: "reduce((acc, item) => ${1:acc}, ${2:initialValue})", detail: "(callbackfn, initialValue) => any", doc: "Calls the specified callback function for all the elements in an array." },
                        { label: "slice", insert: "slice(${1:0}, ${2:5})", detail: "(start?: number, end?: number) => any[]", doc: "Returns a copy of a section of an array." },
                        { label: "push", insert: "push(${1:item})", detail: "(...items) => number", doc: "Appends new elements to the end of an array." }
                    ];
                    return arrMembers.map((m) => ({
                        label: m.label,
                        kind: m.kind || monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "date") {
                    const dateMethods = [
                        { label: "toISOString", insert: "toISOString()", detail: "() => string", doc: "Return the date in ISO-8601 UTC string format (`YYYY-MM-DDTHH:mm:ss.sssZ`)." },
                        { label: "getFullYear", insert: "getFullYear()", detail: "() => number", doc: "Get the 4-digit year using local time." },
                        { label: "getMonth", insert: "getMonth()", detail: "() => number", doc: "Get the month (0–11) using local time." },
                        { label: "getDate", insert: "getDate()", detail: "() => number", doc: "Get the day-of-the-month (1–31) using local time." },
                        { label: "getHours", insert: "getHours()", detail: "() => number", doc: "Get the hours (0–23) using local time." },
                        { label: "getMinutes", insert: "getMinutes()", detail: "() => number", doc: "Get the minutes (0–59) using local time." },
                        { label: "getSeconds", insert: "getSeconds()", detail: "() => number", doc: "Get the seconds (0–59) using local time." },
                        { label: "getTime", insert: "getTime()", detail: "() => number", doc: "Get the time value in milliseconds since midnight, January 1, 1970 UTC." },
                        { label: "toLocaleDateString", insert: "toLocaleDateString()", detail: "() => string", doc: "Return a date as a string value appropriate to the host environment's current locale." },
                        { label: "toDateString", insert: "toDateString()", detail: "() => string", doc: "Return a date as a human-readable string value." },
                        { label: "slice", insert: "slice(0, 10)", detail: "(start, end) => string", doc: "Extract date substring (e.g. `YYYY-MM-DD`) when stored as ISO string." },
                        { label: "startsWith", insert: 'startsWith("${1:2026}")', detail: "(prefix: string) => boolean", doc: "Check if the ISO date string starts with a year/month prefix." },
                        { label: "includes", insert: 'includes("${1:T}")', detail: "(searchString: string) => boolean", doc: "Check if the ISO date string contains a substring." },
                        { label: "length", insert: "length", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: "Length of the ISO date/time string." },
                        { label: "toString", insert: "toString()", detail: "() => string", doc: "Return string representation of the date." }
                    ];
                    return dateMethods.map((m) => ({
                        label: m.label,
                        kind: m.kind || monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "map") {
                    const mapMethods = [
                        { label: "get", insert: "get(${1:key})", detail: "(key: K) => V | undefined", doc: "Return the element associated with the specified key." },
                        { label: "set", insert: "set(${1:key}, ${2:value})", detail: "(key: K, value: V) => this", doc: "Add or update an element with a specified key and value." },
                        { label: "has", insert: "has(${1:key})", detail: "(key: K) => boolean", doc: "Return true if an element with the specified key exists." },
                        { label: "delete", insert: "delete(${1:key})", detail: "(key: K) => boolean", doc: "Remove the specified element from the Map." },
                        { label: "clear", insert: "clear()", detail: "() => void", doc: "Remove all elements from the Map." },
                        { label: "keys", insert: "keys()", detail: "() => IterableIterator<K>", doc: "Return an iterator of keys in the Map." },
                        { label: "values", insert: "values()", detail: "() => IterableIterator<V>", doc: "Return an iterator of values in the Map." },
                        { label: "entries", insert: "entries()", detail: "() => IterableIterator<[K, V]>", doc: "Return an iterator of `[key, value]` pairs." },
                        { label: "size", insert: "size", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: "The number of elements in the Map." }
                    ];
                    return mapMethods.map((m) => ({
                        label: m.label,
                        kind: m.kind || monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "set") {
                    const setMethods = [
                        { label: "add", insert: "add(${1:value})", detail: "(value: T) => this", doc: "Append a new element with a specified value to the Set." },
                        { label: "has", insert: "has(${1:value})", detail: "(value: T) => boolean", doc: "Return true if an element with the specified value exists in the Set." },
                        { label: "delete", insert: "delete(${1:value})", detail: "(value: T) => boolean", doc: "Remove a specified value from the Set." },
                        { label: "clear", insert: "clear()", detail: "() => void", doc: "Remove all elements from the Set." },
                        { label: "values", insert: "values()", detail: "() => IterableIterator<T>", doc: "Return an iterator of values in the Set." },
                        { label: "size", insert: "size", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: "The number of unique elements in the Set." }
                    ];
                    return setMethods.map((m) => ({
                        label: m.label,
                        kind: m.kind || monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                if (primKind === "json") {
                    const jsonMethods = [
                        { label: "toString", insert: "toString()", detail: "() => string", doc: "Return string representation of the JSON value." }
                    ];
                    return jsonMethods.map((m) => ({
                        label: m.label,
                        kind: monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        detail: m.detail,
                        documentation: m.doc,
                        range: targetRange
                    }));
                }

                // Default: `string` / `text` methods & properties (`String.prototype`)
                const strMembers = [
                    { label: "trim", insert: "trim()", detail: "() => string", doc: "Remove leading and trailing whitespace and line terminator characters from a string." },
                    { label: "trimStart", insert: "trimStart()", detail: "() => string", doc: "Remove leading whitespace characters from a string." },
                    { label: "trimEnd", insert: "trimEnd()", detail: "() => string", doc: "Remove trailing whitespace characters from a string." },
                    { label: "length", insert: "length", kind: monaco.languages.CompletionItemKind.Property, detail: "number", doc: "The length of the string in UTF-16 code units." },
                    { label: "toLowerCase", insert: "toLowerCase()", detail: "() => string", doc: "Convert all alphabetic characters in the string to lowercase." },
                    { label: "toUpperCase", insert: "toUpperCase()", detail: "() => string", doc: "Convert all alphabetic characters in the string to uppercase." },
                    { label: "includes", insert: 'includes("${1:searchString}")', detail: "(searchString: string, position?: number) => boolean", doc: "Return true if `searchString` appears as a substring of this string." },
                    { label: "startsWith", insert: 'startsWith("${1:prefix}")', detail: "(searchString: string, position?: number) => boolean", doc: "Return true if the string begins with `prefix`." },
                    { label: "endsWith", insert: 'endsWith("${1:suffix}")', detail: "(searchString: string, endPosition?: number) => boolean", doc: "Return true if the string ends with `suffix`." },
                    { label: "replace", insert: 'replace("${1:searchValue}", "${2:replaceValue}")', detail: "(searchValue: string | RegExp, replaceValue: string) => string", doc: "Replace the first match of `searchValue` with `replaceValue`." },
                    { label: "replaceAll", insert: 'replaceAll("${1:searchValue}", "${2:replaceValue}")', detail: "(searchValue: string | RegExp, replaceValue: string) => string", doc: "Replace all occurrences of `searchValue` with `replaceValue`." },
                    { label: "split", insert: 'split("${1:,}")', detail: "(separator: string | RegExp, limit?: number) => string[]", doc: "Split a string into substrings using the specified separator and return them as an array." },
                    { label: "slice", insert: "slice(${1:0}, ${2:10})", detail: "(start?: number, end?: number) => string", doc: "Return a section of a string." },
                    { label: "substring", insert: "substring(${1:0}, ${2:10})", detail: "(start: number, end?: number) => string", doc: "Return the substring at the specified location within a String object." },
                    { label: "indexOf", insert: 'indexOf("${1:searchString}")', detail: "(searchString: string, position?: number) => number", doc: "Return the position of the first occurrence of a substring (`-1` if not found)." },
                    { label: "lastIndexOf", insert: 'lastIndexOf("${1:searchString}")', detail: "(searchString: string, position?: number) => number", doc: "Return the last occurrence of a substring in the string." },
                    { label: "padStart", insert: 'padStart(${1:6}, "${2:0}")', detail: "(maxLength: number, fillString?: string) => string", doc: "Pad the current string from the start with a given string to reach `maxLength`." },
                    { label: "padEnd", insert: 'padEnd(${1:6}, "${2: }")', detail: "(maxLength: number, fillString?: string) => string", doc: "Pad the current string from the end with a given string to reach `maxLength`." },
                    { label: "charAt", insert: "charAt(${1:0})", detail: "(pos: number) => string", doc: "Return the character at the specified index." },
                    { label: "match", insert: "match(${1:/pattern/})", detail: "(matcher: string | RegExp) => RegExpMatchArray | null", doc: "Match a string with a regular expression." },
                    { label: "repeat", insert: "repeat(${1:2})", detail: "(count: number) => string", doc: "Return a String value that is made from `count` copies appended together." },
                    { label: "concat", insert: 'concat(${1:other})', detail: "(...strings: string[]) => string", doc: "Return a string that contains the concatenation of two or more strings." }
                ];
                return strMembers.map((m) => ({
                    label: m.label,
                    kind: m.kind || monaco.languages.CompletionItemKind.Method,
                    insertText: m.insert,
                    insertTextRules: m.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                    detail: m.detail,
                    documentation: m.doc,
                    range: targetRange
                }));
            };

            // Case A: Decorator completions after `@`
            if (/@[A-Za-z_]*$/.test(linePrefix)) {
                const attrRange = {
                    ...range,
                    startColumn: Math.max(1, word.startColumn - 1)
                };
                return {
                    suggestions: [
                        {
                            label: "@businessEvent",
                            kind: monaco.languages.CompletionItemKind.Event,
                            insertText: "@businessEvent",
                            documentation: "Marks a Codeunit procedure as a public domain Business Event published to the transactional event outbox.",
                            range: attrRange
                        },
                        {
                            label: "@integrationEvent",
                            kind: monaco.languages.CompletionItemKind.Event,
                            insertText: "@integrationEvent(${1:false}, ${2:false})",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            documentation: "Marks a Codeunit procedure as an external Integration Event.",
                            range: attrRange
                        },
                        {
                            label: "@subscribeEvent",
                            kind: monaco.languages.CompletionItemKind.Event,
                            insertText: '@subscribeEvent(${1:SalesPost}, "${2:onAfterPost}")',
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            documentation: "Subscribes a Codeunit procedure to an event emitted by another Codeunit or Table.",
                            range: attrRange
                        },
                        {
                            label: "@pushdown",
                            kind: monaco.languages.CompletionItemKind.Keyword,
                            insertText: "@pushdown",
                            documentation: "BOS-AMDP: Compiles this procedure directly into an in-database PostgreSQL PL/pgSQL stored procedure for zero-network-hop execution.",
                            range: attrRange
                        },
                        {
                            label: "@transactional",
                            kind: monaco.languages.CompletionItemKind.Keyword,
                            insertText: "@transactional",
                            documentation: "Enforces strict ACID savepoint wrapping around the procedure.",
                            range: attrRange
                        }
                    ]
                };
            }

            // Case B: Member completions after `ctx.<ns>.` or `ctx.`
            const ctxSubMatch = linePrefix.match(/\bctx\.(db|ui|events|crypto|audit|dimension|workflow|security|http|wasm|sidecar)\.[A-Za-z_]*$/);
            if (ctxSubMatch) {
                const ns = ctxSubMatch[1];
                const dynamicDbTables = Array.from(
                    new Set([
                        ...liveWorkspaceCatalog.tables.keys(),
                        ...docCtx.symbols.filter((s) => s.kind === "table" && s.name).map((s) => s.name),
                        ...(docCtx.targetTable ? [docCtx.targetTable] : [])
                    ])
                ).sort().map((tName) => ({
                    label: tName,
                    kind: monaco.languages.CompletionItemKind.Class,
                    insert: tName,
                    doc: `Strongly-typed data repository for active database table \`${tName}\`.`
                }));

                const nsMembers = {
                    db: [
                        ...dynamicDbTables,
                        { label: "getModel", insert: 'getModel("${1:TableName}")', doc: "Return the TableModel repository for a table name." },
                        { label: "query", insert: 'query<${1:any}>("${2:SELECT * FROM table_name WHERE id = \\$1}", [${3:id}])', doc: "Execute parameterized read SQL query." },
                        { label: "execute", insert: 'execute("${1:UPDATE table_name SET status = \\$1 WHERE id = \\$2}", [${2:val}, ${3:id}])', doc: "Execute parameterized DML statement." }
                    ],
                    ui: [
                        { label: "toast", insert: 'toast("${1:Operation completed successfully}", "${2|info,success,warning,error|}")', doc: "Display a non-blocking toast notification in the UI." },
                        { label: "message", insert: 'message("${1:Notification message}", "${2|info,success,warning,error|}")', doc: "Emit a structured UI notification message." },
                        { label: "success", insert: 'success("${1:Completed successfully}", "${2:Success}")', doc: "Display a green success toast notification." },
                        { label: "warning", insert: 'warning("${1:Please review before proceeding}", "${2:Warning}")', doc: "Display an amber warning toast notification." },
                        { label: "error", insert: 'error("${1:Validation failed}", "${2:Error}")', doc: "Display a red error toast notification." },
                        { label: "info", insert: 'info("${1:Information message}", "${2:Info}")', doc: "Display a blue informational toast notification." },
                        { label: "confirm", insert: 'confirm("${1:Are you sure you want to post this document?}")', doc: "Prompt the user with a confirmation dialog." },
                        { label: "openModal", insert: 'openModal("${1:CustomerCard}", { id: ${2:rec.id} })', doc: "Open a target Page in a modal dialog." },
                        { label: "navigate", insert: 'navigate("/pages/${1:SalesOrderList}")', doc: "Navigate the active client view." },
                        { label: "setFieldVisible", insert: 'setFieldVisible("${1:field_name}", ${2:true})', doc: "Dynamically toggle field visibility on the current page." },
                        { label: "setFieldEditable", insert: 'setFieldEditable("${1:field_name}", ${2:false})', doc: "Dynamically toggle field editability on the current page." },
                        { label: "getMessages", insert: "getMessages()", doc: "Return all queued UI toast messages for the current execution." },
                        { label: "getDirectives", insert: "getDirectives()", doc: "Return all queued UI page directives (modals, navigation, field visibility)." },
                        { label: "clearMessages", insert: "clearMessages()", doc: "Clear all queued UI messages and directives." }
                    ],
                    events: [
                        { label: "publish", insert: 'publish("${1:SalesPost}", "${2:OnAfterPostOrder}", ${3:{ id: rec.id }})', doc: "Publish a domain event to registered subscribers and the transactional outbox." }
                    ],
                    crypto: [
                        { label: "sha256", insert: 'sha256(${1:input})', doc: "Compute a hex-encoded SHA-256 digest." },
                        { label: "randomBytes", insert: 'randomBytes(${1:16})', doc: "Generate cryptographically secure random bytes." }
                    ],
                    audit: [
                        { label: "log", insert: 'log("${1:Audit trail message}")', doc: "Append an immutable entry to the system audit log." }
                    ],
                    dimension: [
                        { label: "validateSet", insert: 'validateSet(${1:rec.dimension_set_id})', doc: "Validate dimension combinations and blocked rules." },
                        { label: "getSetId", insert: 'getSetId({ "${1:DEPT}": "${2:SALES}" })', doc: "Resolve or create a deduplicated Dimension Set ID." }
                    ],
                    workflow: [
                        { label: "transition", insert: 'transition("${1:SalesHeader}", ${2:rec.id}, "${3:release}")', doc: "Execute a validated State Machine transition." },
                        { label: "canTransition", insert: 'canTransition("${1:SalesHeader}", ${2:rec.id}, "${3:release}")', doc: "Check if a transition guard permits execution." }
                    ],
                    security: [
                        { label: "checkPermission", insert: 'checkPermission("${1:SalesHeader}", "${2|read,insert,modify,delete,execute|}")', doc: "Verify the current user holds the specified permission." },
                        { label: "hasRole", insert: 'hasRole("${1:FINANCE_MANAGER}")', doc: "Return true if the current user has the role." },
                        { label: "requireRole", insert: 'requireRole("${1:ADMIN}")', doc: "Throw a SecurityError if the current user lacks the role." }
                    ],
                    http: [
                        { label: "get", insert: 'get("${1:https://api.example.com/v1/rates}")', doc: "Perform an outbound HTTP GET request." },
                        { label: "post", insert: 'post("${1:https://api.example.com/v1/webhook}", ${2:payload})', doc: "Perform an outbound HTTP POST request." }
                    ],
                    wasm: [
                        { label: "invoke", insert: 'invoke("${1:pricing_module}", "${2:calculate_margin}", ${3:100}, ${4:0.15})', doc: "Invoke a compiled WebAssembly function inside the Wasmtime host." }
                    ],
                    sidecar: [
                        { label: "call", insert: 'call("${1:ml-forecast}", "${2:/v1/predict}", ${3:{ item_no: rec.no }})', doc: "Call an external microservice sidecar over HTTP/IPC." }
                    ]
                };

                return {
                    suggestions: (nsMembers[ns] || []).map((m) => ({
                        label: m.label,
                        kind: m.kind || monaco.languages.CompletionItemKind.Method,
                        insertText: m.insert,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        documentation: m.doc,
                        range
                    }))
                };
            }

            if (/\bctx\.[A-Za-z_]*$/.test(linePrefix)) {
                const ctxProps = [
                    { label: "db", kind: monaco.languages.CompletionItemKind.Module, doc: "Database query & transactional record engine (`ctx.db.*`)." },
                    { label: "ui", kind: monaco.languages.CompletionItemKind.Module, doc: "Client UI interaction bridge (`ctx.ui.toast`, `success`, `warning`, `error`, `confirm`, `openModal`)." },
                    { label: "events", kind: monaco.languages.CompletionItemKind.Module, doc: "Transactional domain event bus (`ctx.events.publish`)." },
                    { label: "crypto", kind: monaco.languages.CompletionItemKind.Module, doc: "Cryptographic hashing & CSPRNG utilities (`ctx.crypto.sha256`, `randomBytes`)." },
                    { label: "audit", kind: monaco.languages.CompletionItemKind.Module, doc: "System audit trail logging (`ctx.audit.log`)." },
                    { label: "dimension", kind: monaco.languages.CompletionItemKind.Module, doc: "Financial dimension set resolver & validator (`ctx.dimension.*`)." },
                    { label: "workflow", kind: monaco.languages.CompletionItemKind.Module, doc: "State machine workflow engine (`ctx.workflow.transition`)." },
                    { label: "security", kind: monaco.languages.CompletionItemKind.Module, doc: "RBAC & permission verification (`ctx.security.*`)." },
                    { label: "http", kind: monaco.languages.CompletionItemKind.Module, doc: "Sandboxed outbound HTTP client (`ctx.http.*`)." },
                    { label: "wasm", kind: monaco.languages.CompletionItemKind.Module, doc: "Wasmtime native WebAssembly execution bridge (`ctx.wasm.invoke`)." },
                    { label: "sidecar", kind: monaco.languages.CompletionItemKind.Module, doc: "External polyglot sidecar RPC bridge (`ctx.sidecar.call`)." },
                    { label: "decimal", kind: monaco.languages.CompletionItemKind.Method, insert: 'decimal(${1:"0.00"})', doc: "Create an exact 128-bit fixed-point BosDecimal (`add`, `sub`, `mul`, `div`, `round`)." },
                    { label: "toast", kind: monaco.languages.CompletionItemKind.Method, insert: 'toast("${1:Operation completed}", "${2|info,success,warning,error|}")', doc: "Shorthand for `ctx.ui.toast(...)`." },
                    { label: "message", kind: monaco.languages.CompletionItemKind.Method, insert: 'message("${1:Notification}", "${2|info,success,warning,error|}")', doc: "Shorthand for `ctx.ui.message(...)`." },
                    { label: "tenantId", kind: monaco.languages.CompletionItemKind.Property, doc: "Active tenant UUID (`string`)." },
                    { label: "companyId", kind: monaco.languages.CompletionItemKind.Property, doc: "Active company UUID (`string`)." },
                    { label: "userId", kind: monaco.languages.CompletionItemKind.Property, doc: "Authenticated user UUID (`string`)." },
                    { label: "roles", kind: monaco.languages.CompletionItemKind.Property, doc: "Assigned security role codes (`string[]`)." },
                    { label: "postLedger", kind: monaco.languages.CompletionItemKind.Method, insert: 'postLedger("${1:GeneralLedgerEntry}", ${2:entries})', doc: "Post balanced, immutable double-entry ledger lines." },
                    { label: "call", kind: monaco.languages.CompletionItemKind.Method, insert: 'call("${1:SalesPost}", "${2:postOrder}", ${3:rec})', doc: "Invoke a method on another Codeunit." },
                    { label: "log", kind: monaco.languages.CompletionItemKind.Method, insert: 'log("${1:info}", "${2:Message}")', doc: "Write structured diagnostic entry to the runtime log." },
                    { label: "now", kind: monaco.languages.CompletionItemKind.Method, insert: "now()", doc: "Return current UTC ISO-8601 timestamp." }
                ];
                return {
                    suggestions: ctxProps.map((p) => ({
                        label: p.label,
                        kind: p.kind,
                        insertText: p.insert || p.label,
                        insertTextRules: p.insert ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        documentation: p.doc,
                        range
                    }))
                };
            }

            // Case B2: String-literal & declarative field/table/state autocompletion inside method arguments or headers!
            // 1. `rec.testField("|")`, `rec.calcFields("|")`, `rec.validate("|", ...)`, `ctx.ui.setFieldVisible("|")`, `ctx.ui.setFieldEditable("|")`
            const recStringArgMatch = linePrefix.match(
                /\b(?:([A-Za-z_][A-Za-z0-9_]*)\.(testField|calcFields|validate)|ctx\.ui\.(setFieldVisible|setFieldEditable))\(\s*["']([A-Za-z0-9_]*)$/
            );
            if (recStringArgMatch) {
                const [, receiverVar, recHelper, uiHelper] = recStringArgMatch;
                let tblForHelper = docCtx.targetTable;
                if (receiverVar && !["rec", "xRec", "Rec", "this"].includes(receiverVar)) {
                    const inferred = docCtx.localVarTypes?.get(receiverVar);
                    tblForHelper = inferred?.table || resolveKnownTableName(receiverVar, docCtx.symbols, docCtx.targetTable) || docCtx.targetTable;
                }
                const fields = getFieldsForTable(tblForHelper);
                const sortedFields = recHelper === "calcFields"
                    ? [...fields.filter((f) => f.is_flow_field), ...fields.filter((f) => !f.is_flow_field)]
                    : fields;
                if (sortedFields.length > 0) {
                    return {
                        suggestions: sortedFields.map((f, idx) => ({
                            label: f.name,
                            kind: monaco.languages.CompletionItemKind.Field,
                            insertText: f.name,
                            sortText: String(idx).padStart(3, "0"),
                            detail: `${tblForHelper || "Record"}.${f.name}: ${f.data_type}${f.is_flow_field ? " (FlowField)" : ""}`,
                            documentation: `Field \`${f.name}\` (${f.data_type}) for \`${recHelper || uiHelper}\`.`,
                            range
                        }))
                    };
                }
            }

            // 2. Second argument of `testField("status", |)` or `validate("status", |)` -> suggest enum / state / boolean values!
            const recSecondArgMatch = linePrefix.match(
                /\b([A-Za-z_][A-Za-z0-9_]*)\.(testField|validate)\(\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*,\s*([A-Za-z0-9_"']*)$/
            );
            if (recSecondArgMatch && !/\.[A-Za-z_0-9]*$/.test(linePrefix)) {
                const [, receiverVar, , fieldArg, typedVal] = recSecondArgMatch;
                const alreadyQuoted = typedVal.startsWith('"') || typedVal.startsWith("'");
                let tblForHelper = docCtx.targetTable;
                if (receiverVar && !["rec", "xRec", "Rec", "this"].includes(receiverVar)) {
                    const inferred = docCtx.localVarTypes?.get(receiverVar);
                    tblForHelper = inferred?.table || resolveKnownTableName(receiverVar, docCtx.symbols, docCtx.targetTable) || docCtx.targetTable;
                }
                const fields = getFieldsForTable(tblForHelper);
                const targetFieldObj = fields.find((f) => f.name.toLowerCase() === fieldArg.toLowerCase());
                const valSuggestions = [];
                if (targetFieldObj && /bool/i.test(targetFieldObj.data_type)) {
                    valSuggestions.push(
                        { label: "true", kind: monaco.languages.CompletionItemKind.Value, insertText: "true", detail: "Boolean true", range },
                        { label: "false", kind: monaco.languages.CompletionItemKind.Value, insertText: "false", detail: "Boolean false", range }
                    );
                }
                if (targetFieldObj && liveWorkspaceCatalog.enums.has(targetFieldObj.data_type)) {
                    for (const ev of liveWorkspaceCatalog.enums.get(targetFieldObj.data_type).values || []) {
                        valSuggestions.push({
                            label: alreadyQuoted ? ev : `"${ev}"`,
                            kind: monaco.languages.CompletionItemKind.EnumMember,
                            insertText: alreadyQuoted ? ev : `"${ev}"`,
                            detail: `${targetFieldObj.data_type}.${ev}`,
                            range
                        });
                    }
                }
                for (const st of docCtx.localStates) {
                    valSuggestions.push({
                        label: alreadyQuoted ? st : `"${st}"`,
                        kind: monaco.languages.CompletionItemKind.EnumMember,
                        insertText: alreadyQuoted ? st : `"${st}"`,
                        detail: "Workflow State",
                        range
                    });
                }
                if (valSuggestions.length > 0) {
                    return { suggestions: valSuggestions };
                }
            }

            // 3. `ctx.db.getModel("|")`, `ctx.workflow.transition("|")`, `ctx.workflow.canTransition("|")`, `ctx.security.checkPermission("|")`
            if (/\bctx\.(?:db\.getModel|workflow\.(?:transition|canTransition)|security\.checkPermission)\(\s*["'][A-Za-z0-9_]*$/.test(linePrefix)) {
                const tableNames = new Set([
                    ...liveWorkspaceCatalog.tables.keys(),
                    ...docCtx.symbols.filter((s) => s.kind === "table" && s.name).map((s) => s.name),
                    ...(docCtx.targetTable ? [docCtx.targetTable] : [])
                ]);
                return {
                    suggestions: Array.from(tableNames).sort().map((tName) => ({
                        label: tName,
                        kind: monaco.languages.CompletionItemKind.Class,
                        insertText: tName,
                        detail: `Active Database Table \`${tName}\``,
                        range
                    }))
                };
            }

            // 4. Declarative `validate <field>(val)` inside `tablelogic` or `table`
            if (/^\s*validate\s+[A-Za-z0-9_]*$/.test(linePrefix) && docCtx.targetTable) {
                const fields = getFieldsForTable(docCtx.targetTable);
                if (fields.length > 0) {
                    return {
                        suggestions: fields.map((f) => ({
                            label: f.name,
                            kind: monaco.languages.CompletionItemKind.Field,
                            insertText: `${f.name}(val) {\n    \${1}\n}`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: `validate ${docCtx.targetTable}.${f.name} (${f.data_type})`,
                            documentation: `Validate incoming changes to \`${docCtx.targetTable}.${f.name}\`.`,
                            range
                        }))
                    };
                }
            }

            // 5. Declarative `field(|)` inside `page` / `pageextension`, `column(|)` inside `query` / `report`, and `primaryKey(|)` / `key(idx, |)` inside `table`
            const declFieldCallMatch = linePrefix.match(/^\s*(field|column|primaryKey|key)\s*\(([^)]*)$/);
            if (declFieldCallMatch && docCtx.targetTable) {
                const [, kw, insideParens] = declFieldCallMatch;
                const commaCount = (insideParens.match(/,/g) || []).length;
                // For `key(idx_name, col1)`, first param is index name, subsequent params are columns
                if (kw !== "key" || commaCount >= 1) {
                    const fields = getFieldsForTable(docCtx.targetTable);
                    if (fields.length > 0) {
                        return {
                            suggestions: fields.map((f) => {
                                const prettyCaption = f.name
                                    .replace(/_/g, " ")
                                    .replace(/\b\w/g, (c) => c.toUpperCase());
                                return {
                                    label: f.name,
                                    kind: monaco.languages.CompletionItemKind.Field,
                                    insertText: kw === "field" && !linePrefix.includes(")")
                                        ? `${f.name}) { caption: "\${1:${prettyCaption}}"; }`
                                        : f.name,
                                    insertTextRules: kw === "field" ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                                    detail: `${docCtx.targetTable}.${f.name}: ${f.data_type}`,
                                    range
                                };
                            })
                        };
                    }
                }
            }

            // 6. `statemachine` transition `from |` and `to |` -> suggest declared workflow states
            if (/^\s*transition\s+[A-Za-z_][A-Za-z0-9_]*\s+(?:from|from\s+[A-Za-z_][A-Za-z0-9_]*\s+to)\s+[A-Za-z0-9_]*$/.test(linePrefix) && docCtx.localStates.length > 0) {
                return {
                    suggestions: docCtx.localStates.map((st) => ({
                        label: st,
                        kind: monaco.languages.CompletionItemKind.EnumMember,
                        insertText: st,
                        detail: "Workflow State",
                        range
                    }))
                };
            }

            // Case C: `models.<Table>.<method>` or `ctx.db.<Table>.<method>` or `models.<Table>`
            const modelsMethodMatch = linePrefix.match(/\b(?:models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)\.[A-Za-z_]*$/);
            if (modelsMethodMatch) {
                const tblName = modelsMethodMatch[1];
                return {
                    suggestions: getRepoMethodSuggestions(tblName, range)
                };
            }

            if (/\bmodels\.[A-Za-z_]*$/.test(linePrefix)) {
                const tableNames = new Set([
                    ...liveWorkspaceCatalog.tables.keys()
                ]);
                for (const sym of docCtx.symbols) {
                    if (sym.kind === "table" && sym.name) {
                        tableNames.add(sym.name);
                    }
                }
                if (docCtx.targetTable) tableNames.add(docCtx.targetTable);
                return {
                    suggestions: Array.from(tableNames).sort().map((tName) => ({
                        label: tName,
                        kind: monaco.languages.CompletionItemKind.Class,
                        insertText: tName,
                        detail: `TableModel<${tName}Record>`,
                        documentation: `Strongly-typed data repository for active database table \`${tName}\`.`,
                        range
                    }))
                };
            }

            // Case C2: Brace-, bracket-, and property-key-aware completions inside ANY TableModel / ctx.db call!
            const callCtx = analyzeEnclosingModelCallContext(model, position, docCtx);
            if (callCtx && !/\.[A-Za-z_0-9]*$/.test(linePrefix)) {
                const { queryTable, calledMethod, stack, inStringQuote } = callCtx;
                const tblFields = getFieldsForTable(queryTable);
                const topFrame = stack[stack.length - 1];
                const parentFrame = stack.length >= 2 ? stack[stack.length - 2] : null;

                // 0. Inside `select: ["|"]` or `select: [|]` array
                if (topFrame && topFrame.type === "bracket" && topFrame.key === "select") {
                    return {
                        suggestions: tblFields.map((f) => ({
                            label: f.name,
                            kind: monaco.languages.CompletionItemKind.Field,
                            insertText: inStringQuote ? f.name : `"${f.name}"`,
                            detail: `Select ${queryTable}.${f.name} (${f.data_type})`,
                            documentation: `Project column \`${f.name}\` from \`${queryTable}\`.`,
                            range
                        }))
                    };
                }

                if (topFrame && topFrame.type === "brace") {
                    const alreadyDefined = topFrame.keysDefined || new Set();
                    const remainingFields = tblFields.filter((f) => !alreadyDefined.has(f.name));
                    const fieldsToSuggest = remainingFields.length > 0 ? remainingFields : tblFields;

                    // 1. Value-side completion after `fieldName: |` or `op: |` inside an object literal!
                    if (topFrame.afterColon && topFrame.lastPropKey) {
                        const activeKey = topFrame.lastPropKey;
                        // Determine which field is being assigned/filtered (either `activeKey` directly or `topFrame.key` if inside an operator object `{ status: { eq: | } }`)
                        const candidateFieldName = ["eq", "ne", "gt", "gte", "lt", "lte", "contains", "startsWith", "endsWith", "ilike", "in", "notIn", "isNull"].includes(activeKey)
                            ? topFrame.key
                            : activeKey;
                        const fieldMeta = tblFields.find((f) => f.name.toLowerCase() === (candidateFieldName || "").toLowerCase());

                        if (activeKey === "orderBy" || topFrame.key === "orderBy") {
                            return {
                                suggestions: ["asc", "desc"].map((dir) => ({
                                    label: inStringQuote ? dir : `"${dir}"`,
                                    kind: monaco.languages.CompletionItemKind.Value,
                                    insertText: inStringQuote ? dir : `"${dir}"`,
                                    detail: dir === "asc" ? "Ascending order" : "Descending order",
                                    range
                                }))
                            };
                        }

                        const valueItems = [];
                        if (activeKey === "isNull" || (fieldMeta && /bool/i.test(fieldMeta.data_type))) {
                            valueItems.push(
                                { label: "true", kind: monaco.languages.CompletionItemKind.Value, insertText: "true", detail: "Boolean true", range },
                                { label: "false", kind: monaco.languages.CompletionItemKind.Value, insertText: "false", detail: "Boolean false", range }
                            );
                        }
                        if (fieldMeta && liveWorkspaceCatalog.enums.has(fieldMeta.data_type)) {
                            const enumObj = liveWorkspaceCatalog.enums.get(fieldMeta.data_type);
                            for (const ev of enumObj.values || []) {
                                valueItems.push({
                                    label: inStringQuote ? ev : `"${ev}"`,
                                    kind: monaco.languages.CompletionItemKind.EnumMember,
                                    insertText: inStringQuote ? ev : `"${ev}"`,
                                    detail: `${fieldMeta.data_type}.${ev}`,
                                    range
                                });
                            }
                        }
                        if (fieldMeta && /status|state/i.test(fieldMeta.name)) {
                            for (const st of docCtx.localStates) {
                                valueItems.push({
                                    label: inStringQuote ? st : `"${st}"`,
                                    kind: monaco.languages.CompletionItemKind.EnumMember,
                                    insertText: inStringQuote ? st : `"${st}"`,
                                    detail: `Workflow State (${st})`,
                                    range
                                });
                            }
                        }
                        // Also offer operator object `{ contains: ... }` if in a `where` filter right after `fieldName: `
                        if (!inStringQuote && fieldMeta && (topFrame.key === "where" || topFrame.key === "OR[]" || topFrame.key === "AND[]" || ["exists", "count"].includes(calledMethod))) {
                            valueItems.push({
                                label: `{ filter operator }`,
                                kind: monaco.languages.CompletionItemKind.Snippet,
                                insertText: `{ \${1|contains,eq,ne,gt,gte,lt,lte,startsWith,endsWith,in,notIn,isNull|}: \${2:value} }`,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                detail: `Comparison / Text operator filter for ${fieldMeta.name}`,
                                range
                            });
                        }
                        // Offer matching `rec.<field>` if inside a trigger/method with `rec`
                        if (!inStringQuote && docCtx.targetTable) {
                            const recFields = getFieldsForTable(docCtx.targetTable);
                            for (const rf of recFields) {
                                if (!fieldMeta || rf.name === fieldMeta.name || rf.data_type === fieldMeta.data_type) {
                                    valueItems.push({
                                        label: `rec.${rf.name}`,
                                        kind: monaco.languages.CompletionItemKind.Variable,
                                        insertText: `rec.${rf.name}`,
                                        detail: `${docCtx.targetTable}.${rf.name} (${rf.data_type})`,
                                        range
                                    });
                                }
                            }
                        }
                        if (valueItems.length > 0) {
                            return { suggestions: valueItems };
                        }
                    }

                    // 2. Inside field filter operator object: e.g. `where: { title: { | } }` or `OR: [{ title: { | } }]`
                    const isFieldOperatorFrame =
                        topFrame.key &&
                        !["where", "orderBy", "create", "update", "OR[]", "AND[]", "createMany[]"].includes(topFrame.key) &&
                        (!parentFrame || ["where", "OR[]", "AND[]", null].includes(parentFrame.key));

                    if (isFieldOperatorFrame) {
                        const filterOps = [
                            { label: "eq", insert: "eq: ${1:value}", detail: "Equals (=)", doc: "Match records where the field equals the specified value." },
                            { label: "ne", insert: "ne: ${1:value}", detail: "Not Equals (!=)", doc: "Match records where the field does not equal the specified value." },
                            { label: "gt", insert: "gt: ${1:0}", detail: "Greater Than (>)", doc: "Match records where the field is strictly greater than the value." },
                            { label: "gte", insert: "gte: ${1:0}", detail: "Greater Than or Equal (>=)", doc: "Match records where the field is greater than or equal to the value." },
                            { label: "lt", insert: "lt: ${1:100}", detail: "Less Than (<)", doc: "Match records where the field is strictly less than the value." },
                            { label: "lte", insert: "lte: ${1:100}", detail: "Less Than or Equal (<=)", doc: "Match records where the field is less than or equal to the value." },
                            { label: "contains", insert: 'contains: "${1:text}"', detail: "Case-insensitive substring (ILIKE %text%)", doc: "Match records where the text field contains the substring (case-insensitive)." },
                            { label: "startsWith", insert: 'startsWith: "${1:prefix}"', detail: "Prefix match (ILIKE prefix%)", doc: "Match records where the text field starts with the prefix." },
                            { label: "endsWith", insert: 'endsWith: "${1:suffix}"', detail: "Suffix match (ILIKE %suffix)", doc: "Match records where the text field ends with the suffix." },
                            { label: "ilike", insert: 'ilike: "${1:%pattern%}"', detail: "PostgreSQL ILIKE pattern", doc: "Match records using a custom PostgreSQL case-insensitive ILIKE pattern." },
                            { label: "like", insert: 'like: "${1:%pattern%}"', detail: "PostgreSQL LIKE pattern", doc: "Match records using a case-sensitive SQL LIKE pattern." },
                            { label: "in", insert: "in: [${1:val1}, ${2:val2}]", detail: "In Array (IN (...))", doc: "Match records where the field value is in the provided array." },
                            { label: "notIn", insert: "notIn: [${1:val1}]", detail: "Not In Array (NOT IN (...))", doc: "Match records where the field value is not in the provided array." },
                            { label: "isNull", insert: "isNull: ${1|true,false|}", detail: "IS NULL / IS NOT NULL", doc: "Check whether the column value is NULL (`true`) or NOT NULL (`false`)." }
                        ];
                        return {
                            suggestions: filterOps
                                .filter((op) => !alreadyDefined.has(op.label))
                                .map((op) => ({
                                    label: op.label,
                                    kind: monaco.languages.CompletionItemKind.Property,
                                    insertText: op.insert,
                                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                    detail: op.detail,
                                    documentation: op.doc,
                                    range
                                }))
                        };
                    }

                    // 3. Inside `orderBy: { | }`
                    if (topFrame.key === "orderBy") {
                        return {
                            suggestions: fieldsToSuggest.map((f) => ({
                                label: f.name,
                                kind: monaco.languages.CompletionItemKind.Field,
                                insertText: `${f.name}: "\${1|asc,desc|}"`,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                detail: `Order by ${queryTable}.${f.name} (${f.data_type})`,
                                range
                            }))
                        };
                    }

                    // 4. Top-level object of `upsert({ | })` -> suggest `where`, `create`, `update`
                    if (calledMethod === "upsert" && topFrame.key === null) {
                        const pkField = tblFields.find((f) => f.is_primary_key)?.name || tblFields[0]?.name || "id";
                        const sampleField = tblFields.find((f) => !f.is_primary_key)?.name || pkField;
                        const upsertTopProps = [
                            {
                                label: "where",
                                insert: `where: { \${1:${pkField}}: \${2:val} }`,
                                detail: `Conflict key filter for ${queryTable}`,
                                doc: `Specify the unique/primary key column on \`${queryTable}\` to check for conflict.`
                            },
                            {
                                label: "create",
                                insert: `create: { \${1:${sampleField}}: \${2:val} }`,
                                detail: `Insert payload for ${queryTable}`,
                                doc: `Column values to insert if no matching \`${queryTable}\` row exists.`
                            },
                            {
                                label: "update",
                                insert: `update: { \${1:${sampleField}}: \${2:val} }`,
                                detail: `Update patch for ${queryTable}`,
                                doc: `Column values to update when a conflicting \`${queryTable}\` row already exists.`
                            }
                        ];
                        return {
                            suggestions: upsertTopProps
                                .filter((p) => !alreadyDefined.has(p.label))
                                .map((p) => ({
                                    label: p.label,
                                    kind: monaco.languages.CompletionItemKind.Property,
                                    insertText: p.insert,
                                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                    detail: p.detail,
                                    documentation: p.doc,
                                    range
                                }))
                        };
                    }

                    // Helper to generate default snippet placeholder by field type
                    const defaultSnippetValueForField = (f, idx = 1) => {
                        const dt = (f.data_type || "").toLowerCase();
                        if (dt.includes("bool")) return `\${${idx}|true,false|}`;
                        if (dt.includes("int") || dt.includes("decimal") || dt.includes("number")) return `\${${idx}:0}`;
                        if (liveWorkspaceCatalog.enums.has(f.data_type)) {
                            const vals = liveWorkspaceCatalog.enums.get(f.data_type).values || [];
                            if (vals.length > 0) return `"\${${idx}|${vals.join(",")}|}"`;
                        }
                        if (/status|state/i.test(f.name) && docCtx.localStates.length > 0) {
                            return `"\${${idx}|${docCtx.localStates.join(",")}|}"`;
                        }
                        if (dt.includes("string") || dt.includes("text") || dt.includes("code") || dt.includes("uuid") || dt.includes("date")) {
                            return `"\${${idx}:${f.name}}"`;
                        }
                        return `\${${idx}:value}`;
                    };

                    // 5. Inside mutation payload: `create`, `insert`, `createMany`, `update`, `modify`, or `upsert` `{ create: { | }, update: { | } }`
                    const isMutationObject =
                        ["create", "insert", "createMany", "update", "modify"].includes(calledMethod) ||
                        (calledMethod === "upsert" && (topFrame.key === "create" || topFrame.key === "update"));

                    if (isMutationObject) {
                        const writableFields = fieldsToSuggest.filter((f) => !f.is_flow_field);
                        const mutationSuggestions = writableFields.map((f, idx) => ({
                            label: f.name,
                            kind: monaco.languages.CompletionItemKind.Field,
                            insertText: `${f.name}: ${defaultSnippetValueForField(f, 1)}`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: `0_${String(idx).padStart(3, "0")}`,
                            detail: `${queryTable}.${f.name}: ${f.data_type}${f.is_primary_key ? " (PK)" : ""}`,
                            documentation: `Set \`${f.name}\` (${f.data_type}) on \`${queryTable}\`.`,
                            range
                        }));

                        // Multi-field 1-click "Fill all fields" snippet so user never has to type properties one by one!
                        if (writableFields.length > 1) {
                            const allFieldsSnippet = writableFields
                                .map((f, i) => `${f.name}: ${defaultSnippetValueForField(f, i + 1)}`)
                                .join(",\n    ");
                            mutationSuggestions.unshift({
                                label: `Fill all ${queryTable} fields (${writableFields.length} properties)`,
                                kind: monaco.languages.CompletionItemKind.Snippet,
                                insertText: allFieldsSnippet,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                sortText: "0_000",
                                detail: `Populate all ${writableFields.length} columns of ${queryTable}`,
                                documentation: `Automatically insert all writable fields of \`${queryTable}\` with tab-stop placeholders.`,
                                range
                            });
                        }

                        return { suggestions: mutationSuggestions };
                    }

                    // 6. Build filter field suggestions for `where`, `OR: [{ | }]`, `AND: [{ | }]`, `exists({ | })`, `count({ | })`, or shorthand `findMany({ | })`
                    const fieldFilterSuggestions = [];
                    for (const [idx, f] of fieldsToSuggest.entries()) {
                        fieldFilterSuggestions.push({
                            label: f.name,
                            kind: monaco.languages.CompletionItemKind.Field,
                            insertText: `${f.name}: ${defaultSnippetValueForField(f, 1)}`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: `1_${String(idx).padStart(3, "0")}_a`,
                            detail: `${queryTable}.${f.name}: ${f.data_type}`,
                            documentation: `Filter \`${queryTable}\` by \`${f.name}\` (${f.data_type}).`,
                            range
                        });
                        fieldFilterSuggestions.push({
                            label: `${f.name} (operator filter)`,
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: `${f.name}: { \${1|contains,eq,ne,gt,gte,lt,lte,startsWith,endsWith,in,notIn,isNull|}: \${2:value} }`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: `1_${String(idx).padStart(3, "0")}_b`,
                            detail: `Filter ${queryTable}.${f.name} with operator`,
                            documentation: `Apply a comparison or text operator (\`contains\`, \`gt\`, \`lte\`, \`in\`, etc.) on \`${queryTable}.${f.name}\`.`,
                            range
                        });
                    }

                    fieldFilterSuggestions.push(
                        {
                            label: "OR",
                            kind: monaco.languages.CompletionItemKind.Keyword,
                            insertText: "OR: [{ ${1} }, { ${2} }]",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "2_001",
                            detail: "Logical OR filter group",
                            documentation: "Combine multiple filter conditions with logical `OR`.",
                            range
                        },
                        {
                            label: "AND",
                            kind: monaco.languages.CompletionItemKind.Keyword,
                            insertText: "AND: [{ ${1} }, { ${2} }]",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "2_002",
                            detail: "Logical AND filter group",
                            documentation: "Combine multiple filter conditions with logical `AND`.",
                            range
                        }
                    );

                    const isInsideWhereOrLogicalGroup =
                        topFrame.key === "where" ||
                        topFrame.key === "OR[]" ||
                        topFrame.key === "AND[]" ||
                        ["exists", "count"].includes(calledMethod);

                    if (isInsideWhereOrLogicalGroup) {
                        return { suggestions: fieldFilterSuggestions };
                    }

                    // 7. Top-level `QueryOptions` object in `findMany({ | })`, `first({ | })`, `findFirst({ | })`, `last({ | })`, `get(pk, { | })`
                    const sampleField = tblFields[0]?.name || "id";
                    const queryOptionSuggestions = [
                        {
                            label: "where",
                            kind: monaco.languages.CompletionItemKind.Property,
                            insertText: `where: { \${1:${sampleField}}: \${2:val} }`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "0_001",
                            detail: `WhereFilter<${queryTable}Record>`,
                            documentation: `Filter \`${queryTable}\` rows by column equality, comparison operators (\`gt\`, \`lte\`, \`contains\`, \`in\`), or \`OR\`/\`AND\` groups.`,
                            range
                        },
                        {
                            label: "orderBy",
                            kind: monaco.languages.CompletionItemKind.Property,
                            insertText: `orderBy: { \${1:${sampleField}}: "\${2|asc,desc|}" }`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "0_002",
                            detail: `{ [field]: "asc" | "desc" }`,
                            documentation: `Sort \`${queryTable}\` results ascending (\`asc\`) or descending (\`desc\`).`,
                            range
                        },
                        {
                            label: "take",
                            kind: monaco.languages.CompletionItemKind.Property,
                            insertText: "take: ${1:50}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "0_003",
                            detail: "number (LIMIT)",
                            documentation: "Maximum number of records to return (`LIMIT`).",
                            range
                        },
                        {
                            label: "skip",
                            kind: monaco.languages.CompletionItemKind.Property,
                            insertText: "skip: ${1:0}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "0_004",
                            detail: "number (OFFSET)",
                            documentation: "Number of records to skip for pagination (`OFFSET`).",
                            range
                        },
                        {
                            label: "select",
                            kind: monaco.languages.CompletionItemKind.Property,
                            insertText: `select: ["\${1:${sampleField}}"]`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "0_005",
                            detail: `(keyof ${queryTable}Record)[]`,
                            documentation: "Project specific columns from the table.",
                            range
                        },
                        ...fieldFilterSuggestions
                    ].filter((item) => !alreadyDefined.has(item.label));

                    return { suggestions: queryOptionSuggestions };
                }
            }

            // Case D0: Direct Chained Field Access (`rec.<field>.`, `<recordVar>.<field>.`) AND Multi-step Method/Property Chaining (`rec.content.trim().`, `text.split(",").`, `amt.add(10).`, `rec.calcFields("...").`)
            const chainedMethodDotMatch = linePrefix.match(/\.([A-Za-z_][A-Za-z0-9_]*)\s*\([^()]*\)\.[A-Za-z_]*$/);
            if (chainedMethodDotMatch) {
                const lastMethod = chainedMethodDotMatch[1];
                if (["calcFields", "validate"].includes(lastMethod)) {
                    return { suggestions: getRecordMemberSuggestions(docCtx.targetTable, range) };
                }
                if (["add", "sub", "mul", "div", "round", "abs", "neg", "decimal"].includes(lastMethod)) {
                    return { suggestions: getPrimitiveMemberSuggestions("decimal", range) };
                }
                if (["split"].includes(lastMethod)) {
                    return { suggestions: getPrimitiveMemberSuggestions("array", range) };
                }
                if (
                    [
                        "trim", "trimStart", "trimEnd", "toLowerCase", "toUpperCase",
                        "replace", "replaceAll", "slice", "substring", "padStart",
                        "padEnd", "charAt", "repeat", "concat", "toFixed", "toString",
                        "toISOString", "join"
                    ].includes(lastMethod)
                ) {
                    return { suggestions: getPrimitiveMemberSuggestions("string", range) };
                }
                if (["indexOf", "lastIndexOf", "toNumber", "count"].includes(lastMethod)) {
                    return { suggestions: getPrimitiveMemberSuggestions("number", range) };
                }
            }

            const chainedFieldDotMatch = linePrefix.match(/\b([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)\.[A-Za-z_]*$/);
            if (chainedFieldDotMatch) {
                const [, baseVar, propName] = chainedFieldDotMatch;
                if (propName === "length") {
                    return { suggestions: getPrimitiveMemberSuggestions("number", range) };
                }
                if (!["ctx", "models"].includes(baseVar)) {
                    let baseTable = null;
                    if (["rec", "xRec", "Rec", "this"].includes(baseVar)) {
                        baseTable = docCtx.targetTable;
                    } else if (docCtx.localVarTypes?.get(baseVar)?.kind === "record") {
                        baseTable = docCtx.localVarTypes.get(baseVar).table;
                    }
                    if (baseTable) {
                        const fieldPrim = resolveTableFieldPrimitiveKind(baseTable, propName, docCtx.localFields, docCtx.targetTable) || "string";
                        return {
                            suggestions: getPrimitiveMemberSuggestions(fieldPrim, range)
                        };
                    }
                }
            }

            // Case D: `rec.`, `xRec.`, `Rec.`, `this.` -> active table fields & record helpers
            if (/\b(?:rec|xRec|Rec|this)\.[A-Za-z_]*$/.test(linePrefix)) {
                return {
                    suggestions: getRecordMemberSuggestions(docCtx.targetTable, range)
                };
            }

            // Case E: `<EntityOrVar>.<member>` (Enum members, Table repository/fields, or inferred local variable properties!)
            const entityDotMatch = linePrefix.match(/\b([A-Za-z_][A-Za-z0-9_]*)\.[A-Za-z_]*$/);
            if (entityDotMatch) {
                const entityName = entityDotMatch[1];

                // 0. Standard JavaScript built-in globals (`Math.`, `JSON.`, `Object.`, `Array.`, `String.`, `Number.`, `Date.`, `console.`)
                if (entityName === "Math") {
                    return {
                        suggestions: ["abs", "ceil", "floor", "round", "max", "min", "pow", "sqrt", "trunc", "random"].map((fn) => ({
                            label: fn,
                            kind: monaco.languages.CompletionItemKind.Method,
                            insertText: `${fn}(\${1})`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: `Math.${fn}(...)`,
                            range
                        }))
                    };
                }
                if (entityName === "JSON") {
                    return {
                        suggestions: [
                            { label: "stringify", kind: monaco.languages.CompletionItemKind.Method, insertText: "stringify(${1:value})", insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet, detail: "(value) => string", range },
                            { label: "parse", kind: monaco.languages.CompletionItemKind.Method, insertText: "parse(${1:text})", insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet, detail: "(text: string) => any", range }
                        ]
                    };
                }
                if (entityName === "Object") {
                    return {
                        suggestions: ["keys", "values", "entries", "assign", "freeze", "fromEntries"].map((fn) => ({
                            label: fn,
                            kind: monaco.languages.CompletionItemKind.Method,
                            insertText: `${fn}(\${1:obj})`,
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: `Object.${fn}(...)`,
                            range
                        }))
                    };
                }

                // 1. Enum members
                if (liveWorkspaceCatalog.enums.has(entityName)) {
                    const enumObj = liveWorkspaceCatalog.enums.get(entityName);
                    return {
                        suggestions: (enumObj.values || []).map((v) => ({
                            label: v,
                            kind: monaco.languages.CompletionItemKind.EnumMember,
                            insertText: v,
                            detail: `${entityName}.${v}`,
                            range
                        }))
                    };
                }

                // 2. Local variable inferred from buffer (`const text = rec.content || ""`, `const note = models.Note.first(...)`, `var note = models.Note`, `for (const n of notes)`, etc.)
                const inferredVar = docCtx.localVarTypes?.get(entityName);
                if (inferredVar) {
                    if (["string", "decimal", "number", "boolean", "date", "json", "array", "map", "set"].includes(inferredVar.kind)) {
                        return { suggestions: getPrimitiveMemberSuggestions(inferredVar.kind, range) };
                    }
                    if (inferredVar.table) {
                        if (inferredVar.kind === "record") {
                            return { suggestions: getRecordMemberSuggestions(inferredVar.table, range) };
                        }
                        if (inferredVar.kind === "model") {
                            return { suggestions: getRepoMethodSuggestions(inferredVar.table, range) };
                        }
                        if (inferredVar.kind === "record_array") {
                            return { suggestions: getArrayMemberSuggestions(inferredVar.table, range) };
                        }
                        if (inferredVar.kind === "model_or_record") {
                            return {
                                suggestions: [
                                    ...getRecordMemberSuggestions(inferredVar.table, range),
                                    ...getRepoMethodSuggestions(inferredVar.table, range)
                                ]
                            };
                        }
                    }
                }

                // 3. Direct Table name (or case-insensitive variable matching a table name like `note.` for `Note`)
                const matchedTable = resolveKnownTableName(entityName, docCtx.symbols, docCtx.targetTable);
                if (matchedTable) {
                    const isInsideDeclarative = docCtx.currentMetaObject && ["statemachine", "table", "tableextension", "page", "query", "report"].includes(docCtx.currentMetaObject.kind);
                    const fieldItems = getFieldsForTable(matchedTable).map((f) => ({
                        label: f.name,
                        kind: monaco.languages.CompletionItemKind.Field,
                        insertText: f.name,
                        detail: `${matchedTable}.${f.name}: ${f.data_type}${f.is_primary_key ? " (PK)" : ""}${f.is_flow_field ? " (FlowField)" : ""}`,
                        documentation: `Field \`${f.name}\` (${f.data_type}) on table \`${matchedTable}\`.`,
                        range
                    }));
                    if (isInsideDeclarative && fieldItems.length > 0) {
                        return { suggestions: fieldItems };
                    }
                    // Inside procedural code (`tablelogic`, `codeunit`), provide BOTH the table's fields/helpers AND its `TableModel` methods!
                    return {
                        suggestions: [
                            ...getRecordMemberSuggestions(matchedTable, range),
                            ...getRepoMethodSuggestions(matchedTable, range)
                        ]
                    };
                }

                // 4. Fallback for unknown local variables (e.g. untyped `text.`):
                // Suggest standard String/Object primitive methods rather than polluting with `Note` TableModel methods!
                return {
                    suggestions: getPrimitiveMemberSuggestions("string", range)
                };
            }

            // -----------------------------------------------------------------
            // Case F0: Constructor completions right after `new `
            // (e.g. `let test: Date = new ` or `let note: Note = new `)
            // -----------------------------------------------------------------
            const newExprMatch = linePrefix.match(/\bnew\s+([A-Za-z_][A-Za-z0-9_]*)?$/);
            if (newExprMatch) {
                const declaredTypeMatch = linePrefix.match(/(?:const|let|var)\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*([A-Za-z_][A-Za-z0-9_]*)/);
                const expectedType = declaredTypeMatch ? declaredTypeMatch[1] : null;
                const expectedLower = expectedType ? expectedType.toLowerCase() : "";

                const newSuggestions = [];

                // Built-in ES2020 / TypeScript constructors
                const builtinConstructors = [
                    {
                        name: "Date",
                        insertText: "Date($0)",
                        detail: "new Date(value?: string | number | Date) => Date",
                        doc: "Construct a new JavaScript `Date` instance (defaults to current date/time when called with no arguments).",
                        matchTypes: ["date", "datetime", "time"]
                    },
                    {
                        name: "Map",
                        insertText: "Map($0)",
                        detail: "new Map<K, V>() => Map<K, V>",
                        doc: "Construct a new key-value `Map` collection.",
                        matchTypes: ["map"]
                    },
                    {
                        name: "Set",
                        insertText: "Set($0)",
                        detail: "new Set<T>() => Set<T>",
                        doc: "Construct a new unique-value `Set` collection.",
                        matchTypes: ["set"]
                    },
                    {
                        name: "Array",
                        insertText: "Array($0)",
                        detail: "new Array<T>(...items) => T[]",
                        doc: "Construct a new typed array.",
                        matchTypes: ["array"]
                    },
                    {
                        name: "Error",
                        insertText: 'Error("${1:Validation failed}")',
                        detail: "new Error(message?: string) => Error",
                        doc: "Construct an Error object to throw.",
                        matchTypes: ["error"]
                    },
                    {
                        name: "RegExp",
                        insertText: 'RegExp("${1:pattern}", "${2:i}")',
                        detail: "new RegExp(pattern: string, flags?: string) => RegExp",
                        doc: "Construct a regular expression object.",
                        matchTypes: ["regexp"]
                    }
                ];

                for (const bc of builtinConstructors) {
                    const isExactMatch = expectedLower && (bc.name.toLowerCase() === expectedLower || bc.matchTypes.includes(expectedLower));
                    newSuggestions.push({
                        label: bc.name,
                        kind: monaco.languages.CompletionItemKind.Constructor,
                        insertText: bc.insertText,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: bc.detail,
                        documentation: bc.doc,
                        sortText: isExactMatch ? "0_000_" + bc.name : "1_100_" + bc.name,
                        preselect: Boolean(isExactMatch),
                        range
                    });
                }

                // Active Table Record constructors (`new Note({ code: "", content: "" })`)
                const knownTablesForNew = new Set([
                    ...liveWorkspaceCatalog.tables.keys(),
                    ...docCtx.symbols.filter((s) => s.kind === "table" && s.name).map((s) => s.name),
                    ...(docCtx.targetTable ? [docCtx.targetTable] : [])
                ]);
                for (const tName of knownTablesForNew) {
                    const tNameClean = tName.replace(/Record$/, "");
                    const isExactTable = expectedLower && (tName.toLowerCase() === expectedLower || tNameClean.toLowerCase() === expectedLower || `${tNameClean.toLowerCase()}record` === expectedLower);
                    const writableFields = getFieldsForTable(tName).filter(
                        (f) => !f.is_flow_field && !["id", "version", "created_at", "updated_at", "tenant_id"].includes(f.name)
                    );
                    const sampleAssignments = writableFields
                        .slice(0, 3)
                        .map((f, idx) => {
                            const dt = String(f.data_type || "").toLowerCase();
                            const sampleVal = dt.includes("int") || dt.includes("decimal") ? "0" : dt.includes("bool") ? "false" : '""';
                            return `${f.name}: \${${idx + 1}:${sampleVal}}`;
                        })
                        .join(", ");
                    newSuggestions.push({
                        label: tName,
                        kind: monaco.languages.CompletionItemKind.Constructor,
                        insertText: sampleAssignments ? `${tName}({ ${sampleAssignments} })` : `${tName}({ $0 })`,
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `new ${tName}(initData?: Partial<${tName}Record>) => ${tName}Record`,
                        documentation: `Instantiate an in-memory \`${tName}Record\` equipped with \`.testField()\`, \`.calcFields()\`, \`.validate()\`, \`.save()\`, \`.insert()\`, and \`.modify()\`.`,
                        sortText: isExactTable ? "0_000_" + tName : "0_500_" + tName,
                        preselect: Boolean(isExactTable),
                        range
                    });
                }

                return { suggestions: newSuggestions };
            }

            // -----------------------------------------------------------------
            // Case F0b: Smart Initializer Completions right after `= ` in a
            // typed variable declaration (`let test: Date = `, `let n: Note = `)
            // -----------------------------------------------------------------
            const typedInitMatch = linePrefix.match(/(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([A-Za-z_][A-Za-z0-9_<>,\s\[\]]*)\s*=\s*([A-Za-z_]*)$/);
            if (typedInitMatch) {
                const rawDeclaredType = typedInitMatch[2].trim();
                const baseType = rawDeclaredType.replace(/Record$/, "").trim();
                const lowerType = baseType.toLowerCase();
                const initSuggestions = [];

                if (lowerType === "date" || lowerType === "datetime") {
                    initSuggestions.push(
                        {
                            label: "new Date()",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "new Date($0)",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: "Date — Current timestamp",
                            documentation: "Initialize with a new `Date` instance.",
                            sortText: "0_000_a",
                            preselect: true,
                            range
                        },
                        {
                            label: "ctx.now()",
                            kind: monaco.languages.CompletionItemKind.Method,
                            insertText: "new Date(ctx.now())",
                            detail: "Date — Server UTC timestamp",
                            documentation: "Initialize from the server's current UTC ISO timestamp (`ctx.now()`).",
                            sortText: "0_000_b",
                            range
                        }
                    );
                } else if (lowerType === "decimal" || lowerType === "bosdecimal") {
                    initSuggestions.push(
                        {
                            label: 'ctx.decimal("0.00")',
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: 'ctx.decimal("${1:0.00}")',
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: "BosDecimal — Exact fixed-point Decimal",
                            documentation: "Initialize a high-precision fixed-point `Decimal` via `ctx.decimal(...)`.",
                            sortText: "0_000_a",
                            preselect: true,
                            range
                        },
                        {
                            label: 'Decimal("0.00")',
                            kind: monaco.languages.CompletionItemKind.Function,
                            insertText: 'Decimal("${1:0.00}")',
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            detail: "BosDecimal — Global Decimal constructor",
                            documentation: "Initialize an exact fixed-point `Decimal` value.",
                            sortText: "0_000_b",
                            range
                        }
                    );
                } else if (lowerType.startsWith("map")) {
                    initSuggestions.push({
                        label: "new Map()",
                        kind: monaco.languages.CompletionItemKind.Constructor,
                        insertText: "new Map($0)",
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `Initialize ${rawDeclaredType}`,
                        sortText: "0_000_a",
                        preselect: true,
                        range
                    });
                } else if (lowerType.startsWith("set")) {
                    initSuggestions.push({
                        label: "new Set()",
                        kind: monaco.languages.CompletionItemKind.Constructor,
                        insertText: "new Set($0)",
                        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                        detail: `Initialize ${rawDeclaredType}`,
                        sortText: "0_000_a",
                        preselect: true,
                        range
                    });
                } else {
                    const matchedTable = resolveKnownTableName(baseType, docCtx.symbols, docCtx.targetTable);
                    if (matchedTable) {
                        const writableFields = getFieldsForTable(matchedTable).filter(
                            (f) => !f.is_flow_field && !["id", "version", "created_at", "updated_at", "tenant_id"].includes(f.name)
                        );
                        const sampleAssignments = writableFields
                            .slice(0, 3)
                            .map((f, idx) => {
                                const dt = String(f.data_type || "").toLowerCase();
                                const sampleVal = dt.includes("int") || dt.includes("decimal") ? "0" : dt.includes("bool") ? "false" : '""';
                                return `${f.name}: \${${idx + 1}:${sampleVal}}`;
                            })
                            .join(", ");
                        initSuggestions.push(
                            {
                                label: `new ${matchedTable}({ ... })`,
                                kind: monaco.languages.CompletionItemKind.Constructor,
                                insertText: sampleAssignments ? `new ${matchedTable}({ ${sampleAssignments} })` : `new ${matchedTable}({ $0 })`,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                detail: `Instantiate ${matchedTable}Record in memory`,
                                documentation: `Create a new \`${matchedTable}Record\` instance with helper methods (\`.save()\`, \`.insert()\`, \`.validate()\`).`,
                                sortText: "0_000_a",
                                preselect: true,
                                range
                            },
                            {
                                label: `models.${matchedTable}.first({ ... })`,
                                kind: monaco.languages.CompletionItemKind.Method,
                                insertText: `models.${matchedTable}.first({ where: { $1 } })`,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                detail: `Query first ${matchedTable}Record from DB`,
                                sortText: "0_000_b",
                                range
                            },
                            {
                                label: `models.${matchedTable}.create({ ... })`,
                                kind: monaco.languages.CompletionItemKind.Method,
                                insertText: sampleAssignments ? `models.${matchedTable}.create({ ${sampleAssignments} })` : `models.${matchedTable}.create({ $0 })`,
                                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                                detail: `Insert and return new ${matchedTable}Record in DB`,
                                sortText: "0_000_c",
                                range
                            }
                        );
                    }
                }

                if (initSuggestions.length > 0 && typedInitMatch[3] === "") {
                    return { suggestions: initSuggestions };
                }
            }

            // Case F1: TypeScript Type Annotations inside procedural code:
            //   - Variable declaration: `let dt: `, `const count: `, `var items: `
            //   - Parameter type: `procedure foo(x: `, `function bar(a: number, b: `
            //   - Return type: `procedure foo(rec: Note): `
            const tsVarOrParamTypeMatch = linePrefix.match(
                /(?:\b(?:const|let|var)\s+[A-Za-z_][A-Za-z0-9_]*|\b(?:procedure|function)\s+[A-Za-z_][A-Za-z0-9_]*\s*\([^)]*[A-Za-z_][A-Za-z0-9_]*|\)\s*)\s*:\s*[A-Za-z_]*$/
            );
            if (tsVarOrParamTypeMatch) {
                const tsTypes = [
                    { label: "string", insert: "string", detail: "TypeScript primitive string", doc: "Native TypeScript UTF-16 string primitive (`\"...\"`)." },
                    { label: "number", insert: "number", detail: "TypeScript primitive number", doc: "Native TypeScript numeric primitive (`0`, `42`, `3.14`)." },
                    { label: "boolean", insert: "boolean", detail: "TypeScript primitive boolean", doc: "Native TypeScript boolean primitive (`true` | `false`)." },
                    { label: "Decimal", insert: "Decimal", detail: "BOS Exact Fixed-Point Decimal", doc: "Exact fixed-point currency/quantity type (`ctx.decimal(...)`)." },
                    { label: "any", insert: "any", detail: "TypeScript dynamic any type", doc: "Opt-out of static type checking for this binding." },
                    { label: "string[]", insert: "string[]", detail: "Array of strings", doc: "Typed array of `string` elements." },
                    { label: "number[]", insert: "number[]", detail: "Array of numbers", doc: "Typed array of `number` elements." },
                    { label: "Record<string, any>", insert: "Record<string, any>", detail: "Key-value dictionary object", doc: "TypeScript dictionary object with `string` keys." },
                    { label: "void", insert: "void", detail: "No return value", doc: "Procedure return type indicating no value is returned." },
                    { label: "Promise<void>", insert: "Promise<${1:void}>", detail: "Async return type", doc: "Async procedure `Promise<T>` return type." },
                    { label: "Date", insert: "Date", detail: "ISO Date / DateTime", doc: "Date / timestamp value." },
                    { label: "UUID", insert: "UUID", detail: "UUID string alias", doc: "128-bit UUID identifier string." }
                ].map((t, idx) => ({
                    label: t.label,
                    kind: monaco.languages.CompletionItemKind.TypeParameter,
                    insertText: t.insert,
                    insertTextRules: t.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                    sortText: `0_${String(idx).padStart(2, "0")}`,
                    detail: t.detail,
                    documentation: t.doc,
                    range
                }));

                // Also include active Table Record types (`NoteRecord`, `Note`, `Note[]`, `TableModel<NoteRecord>`)
                const knownTables = new Set([
                    ...liveWorkspaceCatalog.tables.keys(),
                    ...docCtx.symbols.filter((s) => s.kind === "table" && s.name).map((s) => s.name),
                    ...(docCtx.targetTable ? [docCtx.targetTable] : [])
                ]);
                for (const tblName of knownTables) {
                    tsTypes.push(
                        {
                            label: `${tblName}Record`,
                            kind: monaco.languages.CompletionItemKind.Interface,
                            insertText: `${tblName}Record`,
                            sortText: `1_${tblName}_a`,
                            detail: `Row record interface for ${tblName}`,
                            documentation: `Typed record row for table \`${tblName}\`.`,
                            range
                        },
                        {
                            label: tblName,
                            kind: monaco.languages.CompletionItemKind.Class,
                            insertText: tblName,
                            sortText: `1_${tblName}_b`,
                            detail: `Table record type (${tblName})`,
                            documentation: `Typed record for table \`${tblName}\`.`,
                            range
                        },
                        {
                            label: `${tblName}Record[]`,
                            kind: monaco.languages.CompletionItemKind.Interface,
                            insertText: `${tblName}Record[]`,
                            sortText: `1_${tblName}_c`,
                            detail: `Array of ${tblName} records`,
                            range
                        }
                    );
                }

                for (const [enumName, enumDef] of liveWorkspaceCatalog.enums.entries()) {
                    tsTypes.push({
                        label: enumName,
                        kind: monaco.languages.CompletionItemKind.Enum,
                        insertText: enumName,
                        sortText: `2_${enumName}`,
                        detail: `Enum (${(enumDef.values || []).join(" | ")})`,
                        range
                    });
                }

                return { suggestions: tsTypes };
            }

            // Case F2: Declarative Table Column Type annotations after `:` (e.g., `customer_status: ` inside `table` / `tableextension`)
            if (/^\s*[A-Za-z_][A-Za-z0-9_]*\s*:\s*[A-Za-z_]*$/.test(linePrefix) && !/^\s*(tableType|pageType|sourceTable|usageCategory|applicationArea|extensible|dataPerCompany)\s*:/.test(linePrefix)) {
                const typeSuggestions = [
                    { label: "UUID notNull", insert: "UUID notNull;", detail: "Primary / Foreign Key UUID" },
                    { label: "String(100)", insert: "String(${1:100});", detail: "Bounded VARCHAR(n)" },
                    { label: "Text", insert: "Text;", detail: "Unbounded TEXT" },
                    { label: "Decimal(18, 2)", insert: "Decimal(${1:18}, ${2:2}) default(${3:0.00});", detail: "Fixed-point Currency / Quantity" },
                    { label: "Integer", insert: "Integer default(${1:0});", detail: "32-bit Integer" },
                    { label: "BigInteger", insert: "BigInteger;", detail: "64-bit Integer" },
                    { label: "Boolean", insert: "Boolean default(${1:false});", detail: "Boolean flag" },
                    { label: "Date", insert: "Date;", detail: "Calendar Date" },
                    { label: "DateTime", insert: "DateTime;", detail: "UTC Timestamp" },
                    { label: "Json", insert: "Json;", detail: "Indexed JSONB payload" }
                ].map((t) => ({
                    label: t.label,
                    kind: monaco.languages.CompletionItemKind.TypeParameter,
                    insertText: t.insert,
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    detail: t.detail,
                    range
                }));

                for (const [enumName, enumDef] of liveWorkspaceCatalog.enums.entries()) {
                    typeSuggestions.push({
                        label: enumName,
                        kind: monaco.languages.CompletionItemKind.Enum,
                        insertText: `${enumName};`,
                        detail: `Enum (${(enumDef.values || []).join(" | ")})`,
                        range
                    });
                }

                return { suggestions: typeSuggestions };
            }

            // Determine whether the cursor is inside a procedural body (`tablelogic`, `codeunit`, or inside a trigger/procedure block)
            const isInsideProceduralBlock = (() => {
                if (docCtx.currentMetaObject && ["tablelogic", "tablelogicextension", "codeunit"].includes(docCtx.currentMetaObject.kind)) {
                    // Check if cursor is after the opening `{` of the meta-object
                    return position.lineNumber > docCtx.currentMetaObject.line;
                }
                return false;
            })();

            // If inside a procedural body (`onInsert`, `onModify`, `validate`, `procedure`), suggest TypeScript keywords, local variables, parameters, `ctx`, `models`, `rec`, and active Tables — NEVER top-level `codeunit` / `pageextension` Meta-Object templates!
            if (isInsideProceduralBlock) {
                const procSuggestions = [
                    { label: "rec", kind: monaco.languages.CompletionItemKind.Variable, insertText: "rec", sortText: "0_01", detail: `${docCtx.targetTable || "Current"}Record (active row)`, documentation: "Current record being inserted, modified, or validated.", range },
                    { label: "xRec", kind: monaco.languages.CompletionItemKind.Variable, insertText: "xRec", sortText: "0_02", detail: `${docCtx.targetTable || "Current"}Record (prior snapshot)`, documentation: "Original database snapshot of the record before modification.", range },
                    { label: "ctx", kind: monaco.languages.CompletionItemKind.Module, insertText: "ctx", sortText: "0_03", detail: "BosExecutionContext", documentation: "BOS execution context (`ctx.db`, `ctx.ui`, `ctx.decimal`, `ctx.events`, `ctx.security`).", range },
                    { label: "models", kind: monaco.languages.CompletionItemKind.Module, insertText: "models", sortText: "0_04", detail: " BosModelsRegistry", documentation: "Strongly-typed repository registry for all database tables (`models.Note.findMany(...)`).", range },
                    { label: "error", kind: monaco.languages.CompletionItemKind.Function, insertText: 'error("${1:Validation failed}")', insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet, sortText: "0_05", detail: "(message: string) => never", documentation: "Abort the current transaction and display a validation error.", range }
                ];

                // Add declared local variables & parameters from current buffer so user can autocomplete their own variables!
                if (docCtx.localVarTypes) {
                    for (const [varName, varMeta] of docCtx.localVarTypes.entries()) {
                        if (["rec", "xRec", "Rec", "ctx", "models"].includes(varName)) continue;
                        procSuggestions.push({
                            label: varName,
                            kind: monaco.languages.CompletionItemKind.Variable,
                            insertText: varName,
                            sortText: `0_10_${varName}`,
                            detail: varMeta.table ? `${varMeta.kind} (${varMeta.table})` : `${varMeta.dataType || varMeta.kind}`,
                            documentation: `Local variable \`${varName}\`.`,
                            range
                        });
                    }
                }

                // Add TypeScript statement & primitive type completions
                const tsConstructs = [
                    { label: "const", kind: monaco.languages.CompletionItemKind.Keyword, insert: "const ${1:name}: ${2:string} = ${3:\"\"};", detail: "Declare typed constant" },
                    { label: "let", kind: monaco.languages.CompletionItemKind.Keyword, insert: "let ${1:name}: ${2:string} = ${3:\"\"};", detail: "Declare typed mutable variable" },
                    { label: "if", kind: monaco.languages.CompletionItemKind.Snippet, insert: "if (${1:condition}) {\n    ${2}\n}", detail: "if statement" },
                    { label: "for...of", kind: monaco.languages.CompletionItemKind.Snippet, insert: "for (const ${1:item} of ${2:items}) {\n    ${3}\n}", detail: "Iterate over array/records" },
                    { label: "string", kind: monaco.languages.CompletionItemKind.TypeParameter, insert: "string", detail: "TypeScript string type" },
                    { label: "number", kind: monaco.languages.CompletionItemKind.TypeParameter, insert: "number", detail: "TypeScript number type" },
                    { label: "boolean", kind: monaco.languages.CompletionItemKind.TypeParameter, insert: "boolean", detail: "TypeScript boolean type" },
                    { label: "Decimal", kind: monaco.languages.CompletionItemKind.TypeParameter, insert: "Decimal", detail: "BOS Exact Fixed-Point Decimal" }
                ];
                for (const item of tsConstructs) {
                    procSuggestions.push({
                        label: item.label,
                        kind: item.kind,
                        insertText: item.insert,
                        insertTextRules: item.insert.includes("$") ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
                        sortText: `1_${item.label}`,
                        detail: item.detail,
                        range
                    });
                }

                // Add active Catalog + Local Tables (`Note`, etc.)
                const allTableNames = new Set([
                    ...liveWorkspaceCatalog.tables.keys(),
                    ...docCtx.symbols.filter((s) => s.kind === "table" && s.name).map((s) => s.name),
                    ...(docCtx.targetTable ? [docCtx.targetTable] : [])
                ]);
                for (const tblName of allTableNames) {
                    procSuggestions.push({
                        label: tblName,
                        kind: monaco.languages.CompletionItemKind.Class,
                        insertText: tblName,
                        sortText: `2_${tblName}`,
                        detail: `TableModel<${tblName}Record>`,
                        documentation: `Direct table repository \`${tblName}\`.`,
                        range
                    });
                }

                // Add `validate` and `procedure` member snippets if at member level of `tablelogic` / `codeunit`
                if (/^\s*[A-Za-z_]*$/.test(linePrefix)) {
                    procSuggestions.push(
                        {
                            label: "validate (Field Validation Trigger)",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "validate ${1:field_name}(val) {\n    if (!val) {\n        error(\"${2:Field is required}\");\n    }\n}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "3_validate",
                            detail: "tablelogic field validator",
                            range
                        },
                        {
                            label: "onInsert (Lifecycle Trigger)",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "onInsert(rec, ctx) {\n    ${1}\n}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "3_onInsert",
                            detail: "Before-insert row trigger",
                            range
                        },
                        {
                            label: "onModify (Lifecycle Trigger)",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "onModify(rec, xRec, ctx) {\n    ${1}\n}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "3_onModify",
                            detail: "Before-modify row trigger",
                            range
                        },
                        {
                            label: "onDelete (Lifecycle Trigger)",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "onDelete(rec, ctx) {\n    ${1}\n}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "3_onDelete",
                            detail: "Before-delete row trigger",
                            range
                        },
                        {
                            label: "procedure (Method)",
                            kind: monaco.languages.CompletionItemKind.Snippet,
                            insertText: "procedure ${1:execute}(${2:rec}: ${3:any}): ${4:void} {\n    ${5}\n}",
                            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                            sortText: "3_procedure",
                            detail: "Define a procedure",
                            range
                        }
                    );
                }

                return { suggestions: procSuggestions };
            }

            // Case G: Top-Level General Meta-Object Templates (`table`, `codeunit`, `page`, etc.)
            const suggestions = [
                // All 14 Meta-Object Snippets
                {
                    label: "table (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "table ${1:EntityName} {",
                        "    tableType: ${2|Master,Document,Ledger,Setup,Supplemental|};",
                        "    extensible: true;",
                        "",
                        "    id: UUID notNull;",
                        "    code: String(30) notNull;",
                        "    name: String(100) notNull;",
                        "    created_at: DateTime;",
                        "",
                        "    primaryKey(id);",
                        "    key(idx_code, code);",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a physical BOS table. Field checks and lifecycle scripts belong in tablelogic.",
                    range
                },
                {
                    label: "tableextension (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "tableextension ${1:CustomerExt} extends ${2:Customer} {",
                        "    ${3:custom_field}: String(${4:50}) default(\"${5:Standard}\");",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Non-invasively add fields or keys to an existing base Table.",
                    range
                },
                {
                    label: "tablelogic (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "tablelogic ${1:CustomerLogic} for ${2:Customer} {",
                        "    onInsert(rec) {",
                        "        ${3:// Validation before insert}",
                        "    }",
                        "    onModify(rec, xRec) {",
                        "        ${4:// Compare rec vs xRec}",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Define row-level triggers (`onInsert`, `onModify`, `onDelete`) for a Table.",
                    range
                },
                {
                    label: "codeunit (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "codeunit ${1:SalesPostService} {",
                        "    @businessEvent",
                        "    procedure ${2:postOrder}(rec: ${3:SalesHeader}): boolean {",
                        "        ctx.ui.toast(\"Posting order \" + rec.id, \"info\");",
                        "        return true;",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a transactional Codeunit with typed procedures and events.",
                    range
                },
                {
                    label: "statemachine (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "statemachine ${1:OrderWorkflow} for ${2:SalesHeader}.status {",
                        "    initial state Draft {",
                        "        allowModify: true;",
                        "        allowDelete: true;",
                        "    }",
                        "    state Released {",
                        "        allowModify: false;",
                        "        allowDelete: false;",
                        "    }",
                        "    transition release from Draft to Released {",
                        "        guard(rec) {",
                        "            return rec.total_amount > 0;",
                        "        }",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a deterministic State Machine governing document status transitions.",
                    range
                },
                {
                    label: "page (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "page ${1:CustomerCard} {",
                        "    pageType: ${2|Card,List,Document,RoleCenter,Worksheet|};",
                        "    sourceTable: ${3:Customer};",
                        "    caption: \"${4:Customer Card}\";",
                        "",
                        "    layout {",
                        "        group(General) {",
                        "            field(name) { caption: \"Name\"; }",
                        "            field(email) { caption: \"Email\"; }",
                        "        }",
                        "    }",
                        "    actions {",
                        "        action(Approve) {",
                        "            caption: \"Approve\";",
                        "        }",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a metadata-driven UI Page bound to a source Table.",
                    range
                },
                {
                    label: "pageextension (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "pageextension ${1:CustomerCardExt} extends ${2:CustomerCard} {",
                        "    layout {",
                        "        field(${3:loyalty_tier}) { caption: \"${4:Loyalty Tier}\"; }",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Extend an existing Page with additional fields or actions.",
                    range
                },
                {
                    label: "query (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "query ${1:TopCustomersQuery} {",
                        "    sourceTable: ${2:Customer};",
                        "    columns {",
                        "        column(id, id);",
                        "        column(name, name);",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a high-speed analytical Query meta-object.",
                    range
                },
                {
                    label: "view (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "view ${1:ActiveCustomersView} {",
                        "    SELECT id, name, email FROM customer WHERE is_blocked = false",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a SQL View meta-object.",
                    range
                },
                {
                    label: "enum (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "enum ${1:OrderStatus} {",
                        "    extensible: true;",
                        "    value(0, Draft);",
                        "    value(1, Released);",
                        "    value(2, Posted);",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare an extensible Enum meta-object.",
                    range
                },
                {
                    label: "interface (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "interface ${1:ITaxProvider} {",
                        "    procedure ${2:calculateTax}(amount: Decimal): Decimal;",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a polymorphic Interface contract for Codeunits.",
                    range
                },
                {
                    label: "permissionset (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "permissionset ${1:SalesManagerRole} {",
                        "    caption: \"${2:Sales Manager}\";",
                        "    tabledata ${3:Customer} = RIMD;",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a Role-Based Access Control (RBAC) Permission Set.",
                    range
                },
                {
                    label: "report (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "report ${1:CustomerBalanceReport} {",
                        "    caption: \"${2:Customer Balance Trial Balance}\";",
                        "    dataset: ${3:Customer};",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare an enterprise analytical/document Report meta-object.",
                    range
                },
                {
                    label: "custommetadata (Meta-Object Snippet)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "custommetadata ${1:TaxEngineSetting} {",
                        "    provider_code: String(30) notNull;",
                        "    endpoint_url: String(250);",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Declare a deployable Custom Metadata configuration type.",
                    range
                },
                // Member / Construct Snippets
                {
                    label: "flowField (Calculated Subquery Field)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: "${1:total_balance}: Decimal(18, 2) flowField(${2|sum,count,average,min,max,lookup|}(${3:LedgerEntry}.${4:amount} where (${5:parent_id} == this.id)));",
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Add a declarative FlowField evaluated via zero-N+1 SQL subquery.",
                    range
                },
                {
                    label: "validate (Field Validation Trigger)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "validate ${1:amount}(val) {",
                        "    if (${2:val < 0}) {",
                        "        error(\"${3:Value cannot be negative}\");",
                        "    }",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Field validation inside a tablelogic companion: validate field(val) { }.",
                    range
                },
                {
                    label: "procedure (Codeunit Method)",
                    kind: monaco.languages.CompletionItemKind.Snippet,
                    insertText: [
                        "procedure ${1:execute}(${2:rec}: ${3:any}): ${4:void} {",
                        "    ${5}",
                        "}"
                    ].join("\n"),
                    insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                    documentation: "Define a procedure inside a Codeunit.",
                    range
                }
            ];

            // Add live Catalog Tables to general completions
            for (const [tblName, tblObj] of liveWorkspaceCatalog.tables.entries()) {
                suggestions.push({
                    label: tblName,
                    kind: monaco.languages.CompletionItemKind.Class,
                    insertText: tblName,
                    detail: `Table (${(tblObj.fields || []).length} fields)`,
                    documentation: `Catalog Table \`${tblName}\``,
                    range
                });
            }

            // Add live Catalog Enums
            for (const [enumName, enumObj] of liveWorkspaceCatalog.enums.entries()) {
                suggestions.push({
                    label: enumName,
                    kind: monaco.languages.CompletionItemKind.Enum,
                    insertText: enumName,
                    detail: `Enum [${(enumObj.values || []).join(", ")}]`,
                    range
                });
            }

            // Add local States if inside a StateMachine
            for (const st of docCtx.localStates) {
                suggestions.push({
                    label: st,
                    kind: monaco.languages.CompletionItemKind.EnumMember,
                    insertText: st,
                    detail: "Workflow State",
                    range
                });
            }

            return { suggestions };
    };

    track(monaco.languages.registerCompletionItemProvider("bos-script", {
        triggerCharacters: [".", "@", ":", "(", "{", ",", " "],
        provideCompletionItems: (model, position) => dedupeCompletionResult(rawProvideCompletionItems(model, position))
    }));

    // 4. Hover Provider (Rich Markdown Documentation for Keywords, Types, & Live Catalog)
    track(monaco.languages.registerHoverProvider("bos-script", {
        provideHover: (model, position) => {
            const wordInfo = model.getWordAtPosition(position);
            if (!wordInfo) return null;
            const word = wordInfo.word;
            const range = {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: wordInfo.startColumn,
                endColumn: wordInfo.endColumn
            };

            const docCtx = parseDocumentSymbolsAndContext(model.getValue());
            const lineContent = model.getLineContent(position.lineNumber);
            const prefixBeforeWord = lineContent.slice(0, wordInfo.startColumn - 1);

            // 1. Check member hover: `<receiver>.<word>` (e.g. `rec.title`, `note.status`, `models.Note.findMany`)
            const receiverDotMatch = prefixBeforeWord.match(/\b(?:(models|ctx\.db)\.([A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*))\.\s*$/);
            if (receiverDotMatch) {
                const [, modelsNs, explicitModelTbl, singleReceiver] = receiverDotMatch;
                let hoveredTable = explicitModelTbl || null;
                if (!hoveredTable && singleReceiver) {
                    if (["rec", "xRec", "Rec", "this"].includes(singleReceiver)) {
                        hoveredTable = docCtx.targetTable;
                    } else if (docCtx.localVarTypes?.has(singleReceiver)) {
                        hoveredTable = docCtx.localVarTypes.get(singleReceiver).table;
                    } else {
                        hoveredTable = resolveKnownTableName(singleReceiver, docCtx.symbols, docCtx.targetTable);
                    }
                }
                if (hoveredTable) {
                    const catTbl = liveWorkspaceCatalog.tables.get(hoveredTable);
                    const allFields = [
                        ...((catTbl && Array.isArray(catTbl.fields)) ? catTbl.fields : []),
                        ...(hoveredTable === docCtx.targetTable ? docCtx.localFields : [])
                    ];
                    const matchedField = allFields.find((f) => (f.name || f.column_name) === word);
                    if (matchedField) {
                        const fType = matchedField.data_type || matchedField.type || "any";
                        const badges = [
                            matchedField.is_primary_key ? "`Primary Key`" : "",
                            matchedField.is_flow_field ? "`FlowField`" : ""
                        ].filter(Boolean).join(" · ");
                        return {
                            range,
                            contents: [
                                { value: `### Field \`${hoveredTable}.${word}: ${fType}\`` },
                                { value: `${badges ? badges + "\n\n" : ""}Column \`${word}\` on table \`${hoveredTable}\`.` }
                            ]
                        };
                    }
                }
                if (modelsNs && !explicitModelTbl && liveWorkspaceCatalog.tables.has(word)) {
                    const tbl = liveWorkspaceCatalog.tables.get(word);
                    const fields = Array.isArray(tbl.fields) ? tbl.fields : [];
                    return {
                        range,
                        contents: [
                            { value: `### Repository \`models.${word}: TableModel<${word}Record>\`` },
                            { value: `Active database repository with **${fields.length}** columns.` }
                        ]
                    };
                }
            }

            // 2. Check if hovering over an inferred local variable (`note`, `notes`, `rec`, `xRec`)
            if (["rec", "xRec", "Rec"].includes(word) && docCtx.targetTable) {
                const catTbl = liveWorkspaceCatalog.tables.get(docCtx.targetTable);
                const fields = [
                    ...((catTbl && Array.isArray(catTbl.fields)) ? catTbl.fields : []),
                    ...docCtx.localFields
                ];
                const fieldPreview = fields.slice(0, 12).map((f) => `- \`${f.name || f.column_name}\`: **${f.data_type || f.type || "any"}**`).join("\n");
                return {
                    range,
                    contents: [
                        { value: `### Record \`${word}: ${docCtx.targetTable}Record\`` },
                        { value: fieldPreview || `Current record instance of \`${docCtx.targetTable}\`.` }
                    ]
                };
            }
            if (docCtx.localVarTypes?.has(word)) {
                const info = docCtx.localVarTypes.get(word);
                const typeLabel = info.kind === "model"
                    ? `TableModel<${info.table}Record>`
                    : info.kind === "record_array"
                        ? `${info.table}Record[]`
                        : `${info.table}Record`;
                return {
                    range,
                    contents: [
                        { value: `### Variable \`${word}: ${typeLabel}\`` },
                        { value: `Bound to active database table \`${info.table}\`.` }
                    ]
                };
            }

            if (HOVER_DOCS[word]) {
                return {
                    range,
                    contents: [
                        { value: `### ${HOVER_DOCS[word].title}` },
                        { value: HOVER_DOCS[word].body }
                    ]
                };
            }

            if (liveWorkspaceCatalog.tables.has(word)) {
                const tbl = liveWorkspaceCatalog.tables.get(word);
                const fields = Array.isArray(tbl.fields) ? tbl.fields : [];
                const fieldLines = fields
                    .slice(0, 15)
                    .map((f) => `- \`${f.name || f.column_name}\`: **${f.data_type || f.type || "any"}**${f.is_primary_key ? " *(PK)*" : ""}`)
                    .join("\n");
                return {
                    range,
                    contents: [
                        { value: `### Table \`${word}\`` },
                        {
                            value: fields.length > 0
                                ? `**Registered Schema Fields (${fields.length}):**\n${fieldLines}${fields.length > 15 ? "\n- *...more*" : ""}`
                                : "Registered BOS Table meta-object. Access via `models." + word + "`."
                        }
                    ]
                };
            }

            if (liveWorkspaceCatalog.enums.has(word)) {
                const en = liveWorkspaceCatalog.enums.get(word);
                return {
                    range,
                    contents: [
                        { value: `### Enum \`${word}\`` },
                        { value: `Values: ${(en.values || []).map((v) => `\`${v}\``).join(" | ") || "*(extensible)*"}` }
                    ]
                };
            }

            // Check if hovering over a field of the current document's table
            const localField = docCtx.localFields.find((f) => f.name === word);
            if (localField) {
                return {
                    range,
                    contents: [
                        { value: `### Field \`${localField.name}: ${localField.data_type}\`` },
                        { value: `Defined on \`${docCtx.targetTable || "current object"}\` (line ${localField.line}).` }
                    ]
                };
            }

            return null;
        }
    }));

    // 5. Signature Help Provider
    track(monaco.languages.registerSignatureHelpProvider("bos-script", {
        signatureHelpTriggerCharacters: ["(", ","],
        provideSignatureHelp: (model, position) => {
            const lineUntilCursor = model.getLineContent(position.lineNumber).slice(0, position.column - 1);
            const callMatch = lineUntilCursor.match(/([A-Za-z_][A-Za-z0-9_.]*)\s*\(([^()]*)$/);
            if (!callMatch) return null;

            const fnName = callMatch[1];
            const argsText = callMatch[2];
            const activeParam = (argsText.match(/,/g) || []).length;

            const signatures = {
                Decimal: {
                    label: "Decimal(precision: number, scale: number)",
                    doc: "Fixed-point exact decimal type (e.g., Decimal(18, 2) for currency).",
                    params: [
                        { label: "precision: number", documentation: "Total number of significant digits (e.g., 18)." },
                        { label: "scale: number", documentation: "Number of digits to the right of the decimal point (e.g., 2)." }
                    ]
                },
                String: {
                    label: "String(maxLength: number)",
                    doc: "Bounded UTF-8 text column.",
                    params: [
                        { label: "maxLength: number", documentation: "Maximum character length (1..10485760)." }
                    ]
                },
                primaryKey: {
                    label: "primaryKey(column1: Field, ...columns: Field[])",
                    doc: "Declares the single or composite primary key for this table.",
                    params: [
                        { label: "column1: Field", documentation: "First primary key field name." },
                        { label: "...columns: Field[]", documentation: "Additional composite key fields." }
                    ]
                },
                key: {
                    label: "key(indexName: Ident, column1: Field, ...columns: Field[])",
                    doc: "Declares a secondary B-Tree database index.",
                    params: [
                        { label: "indexName: Ident", documentation: "Name of the secondary index (e.g., idx_customer_email)." },
                        { label: "column1: Field", documentation: "First indexed column." },
                        { label: "...columns: Field[]", documentation: "Additional indexed columns." }
                    ]
                },
                "ctx.ui.toast": {
                    label: "ctx.ui.toast(message: string, severity?: 'info' | 'success' | 'warning' | 'error')",
                    doc: "Displays a toast notification in the active user session.",
                    params: [
                        { label: "message: string", documentation: "Notification message text." },
                        { label: "severity?: string", documentation: "Visual style: 'info', 'success', 'warning', or 'error'." }
                    ]
                },
                "ctx.events.publish": {
                    label: "ctx.events.publish(topic: string, payload: Record<string, any>)",
                    doc: "Publishes an event to the transactional outbox.",
                    params: [
                        { label: "topic: string", documentation: "Event routing topic (e.g., 'sales.order.posted')." },
                        { label: "payload: Record<string, any>", documentation: "Serializable event payload." }
                    ]
                },
                "ctx.audit.log": {
                    label: "ctx.audit.log(message: string): void",
                    doc: "Appends an immutable entry to the system audit log.",
                    params: [
                        { label: "message: string", documentation: "Audit trail message describing the action." }
                    ]
                },
                "ctx.decimal": {
                    label: "ctx.decimal(value: string | number): BosDecimal",
                    doc: "Creates an exact 128-bit fixed-point decimal with `.add()`, `.sub()`, `.mul()`, `.div()`, and `.round()`.",
                    params: [
                        { label: "value: string | number", documentation: "Initial numeric or string decimal representation." }
                    ]
                },
                "ctx.db.getModel": {
                    label: "ctx.db.getModel<T>(tableName: string): TableModel<T>",
                    doc: "Returns the typed TableModel repository for an active database table.",
                    params: [
                        { label: "tableName: string", documentation: "Active table name (e.g., 'Note')." }
                    ]
                },
                "ctx.db.query": {
                    label: "ctx.db.query<T>(sql: string, params?: any[]): T[]",
                    doc: "Executes a parameterized SQL SELECT query.",
                    params: [
                        { label: "sql: string", documentation: "Parameterized SQL query ($1, $2...)." },
                        { label: "params?: any[]", documentation: "Bound parameter array." }
                    ]
                },
                "ctx.db.execute": {
                    label: "ctx.db.execute(sql: string, params?: any[]): number",
                    doc: "Executes a parameterized DML statement.",
                    params: [
                        { label: "sql: string", documentation: "Parameterized SQL DML statement." },
                        { label: "params?: any[]", documentation: "Bound parameter array." }
                    ]
                },
                "ctx.workflow.transition": {
                    label: "ctx.workflow.transition(tableName: string, recordId: string, transitionName: string): any",
                    doc: "Executes a state machine transition on a record.",
                    params: [
                        { label: "tableName: string", documentation: "Target table name." },
                        { label: "recordId: string", documentation: "Primary key of the record." },
                        { label: "transitionName: string", documentation: "Name of the transition to execute." }
                    ]
                },
                "ctx.security.checkPermission": {
                    label: "ctx.security.checkPermission(objectName: string, permission: 'read' | 'insert' | 'modify' | 'delete' | 'execute'): boolean",
                    doc: "Verifies whether the current user holds the specified permission.",
                    params: [
                        { label: "objectName: string", documentation: "Meta-object or table name." },
                        { label: "permission: string", documentation: "'read' | 'insert' | 'modify' | 'delete' | 'execute'." }
                    ]
                },
                error: {
                    label: "error(message: string): never",
                    doc: "Aborts the current trigger or codeunit execution and rolls back the active transaction.",
                    params: [
                        { label: "message: string", documentation: "User-facing validation error message." }
                    ]
                }
            };

            const methodSuffix = fnName.includes(".") ? fnName.split(".").pop() : null;
            const modelMethodSignatures = {
                get: {
                    label: "get(pk: string | number, options?: { select?: string[] }): TRecord | null",
                    doc: "Fetch a single record by primary key or return null.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key value of the record." },
                        { label: "options?: { select?: string[] }", documentation: "Optional column projection." }
                    ]
                },
                getOrThrow: {
                    label: "getOrThrow(pk: string | number, options?: { select?: string[] }): TRecord",
                    doc: "Fetch a single record by primary key or throw an error if not found.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key value of the record." },
                        { label: "options?: { select?: string[] }", documentation: "Optional column projection." }
                    ]
                },
                findById: {
                    label: "findById(pk: string | number, options?: { select?: string[] }): TRecord | null",
                    doc: "Fetch a single record by primary key.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key value." },
                        { label: "options?: { select?: string[] }", documentation: "Optional column projection." }
                    ]
                },
                first: {
                    label: "first(query?: { where?: WhereFilter<T>, orderBy?: OrderBy<T>, select?: (keyof T)[] }): TRecord | null",
                    doc: "Find the first record matching the filter criteria.",
                    params: [
                        { label: "query?: QueryOptions<T>", documentation: "Filter (`where`), sort (`orderBy`), and projection (`select`) options." }
                    ]
                },
                findFirst: {
                    label: "findFirst(query?: { where?: WhereFilter<T>, orderBy?: OrderBy<T>, select?: (keyof T)[] }): TRecord | null",
                    doc: "Find the first record matching the filter criteria.",
                    params: [
                        { label: "query?: QueryOptions<T>", documentation: "Filter (`where`), sort (`orderBy`), and projection (`select`) options." }
                    ]
                },
                last: {
                    label: "last(query?: { where?: WhereFilter<T>, orderBy?: OrderBy<T>, select?: (keyof T)[] }): TRecord | null",
                    doc: "Find the last record matching the filter criteria.",
                    params: [
                        { label: "query?: QueryOptions<T>", documentation: "Filter (`where`), sort (`orderBy`), and projection (`select`) options." }
                    ]
                },
                findMany: {
                    label: "findMany(query?: { where?: WhereFilter<T>, orderBy?: OrderBy<T>, take?: number, skip?: number, select?: (keyof T)[] }): TRecord[]",
                    doc: "Query an array of records matching filter, sort, and pagination criteria.",
                    params: [
                        { label: "query?: QueryOptions<T>", documentation: "Filter (`where` with `eq`, `contains`, `gt`, `in`, `OR`, `AND`), `orderBy`, `take`, `skip`, `select`." }
                    ]
                },
                create: {
                    label: "create(data: Partial<TRecord>): TRecord",
                    doc: "Insert a new record into the table (runs field validations & onInsert).",
                    params: [
                        { label: "data: Partial<TRecord>", documentation: "Column key-value pairs for the new record." }
                    ]
                },
                insert: {
                    label: "insert(data: Partial<TRecord>): TRecord",
                    doc: "Insert a new record into the table (runs field validations & onInsert).",
                    params: [
                        { label: "data: Partial<TRecord>", documentation: "Column key-value pairs for the new record." }
                    ]
                },
                createMany: {
                    label: "createMany(records: Partial<TRecord>[]): TRecord[]",
                    doc: "Insert multiple records in batch.",
                    params: [
                        { label: "records: Partial<TRecord>[]", documentation: "Array of record objects to insert." }
                    ]
                },
                update: {
                    label: "update(pk: string | number, patch: Partial<TRecord>): TRecord | null",
                    doc: "Update an existing record by primary key.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key of the record to update." },
                        { label: "patch: Partial<TRecord>", documentation: "Fields to modify on the record." }
                    ]
                },
                modify: {
                    label: "modify(recordOrPk: Partial<TRecord> | string, patch?: Partial<TRecord>): TRecord | null",
                    doc: "Modify an existing record (supports `modify(rec)` or `modify(pk, patch)`).",
                    params: [
                        { label: "recordOrPk: Partial<TRecord> | string", documentation: "Record object containing primary key or primary key value." },
                        { label: "patch?: Partial<TRecord>", documentation: "Optional field patch when first argument is primary key." }
                    ]
                },
                upsert: {
                    label: "upsert(options: { where: Partial<TRecord>, create: Partial<TRecord>, update?: Partial<TRecord> }): TRecord",
                    doc: "Insert a new record or atomically update on conflict.",
                    params: [
                        { label: "options: UpsertOptions<TRecord>", documentation: "`{ where, create, update }` specification." }
                    ]
                },
                delete: {
                    label: "delete(pk: string | number): boolean",
                    doc: "Delete a record by primary key.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key of the record to delete." }
                    ]
                },
                exists: {
                    label: "exists(where?: WhereFilter<TRecord>): boolean",
                    doc: "Return true if at least one record matches the filter.",
                    params: [
                        { label: "where?: WhereFilter<TRecord>", documentation: "Field filter conditions." }
                    ]
                },
                count: {
                    label: "count(where?: WhereFilter<TRecord>): number",
                    doc: "Count records matching the optional filter.",
                    params: [
                        { label: "where?: WhereFilter<TRecord>", documentation: "Field filter conditions." }
                    ]
                },
                lock: {
                    label: "lock(pk: string | number): TRecord | null",
                    doc: "Acquire a row-level `FOR UPDATE` lock on a record.",
                    params: [
                        { label: "pk: string | number", documentation: "Primary key of the record to lock." }
                    ]
                },
                testField: {
                    label: "testField(fieldName: keyof TRecord, expectedValue?: any): boolean",
                    doc: "Assert that a field on this record is non-blank (or equals `expectedValue`).",
                    params: [
                        { label: "fieldName: keyof TRecord", documentation: "Name of the field on the record." },
                        { label: "expectedValue?: any", documentation: "Optional exact value the field must equal." }
                    ]
                },
                calcFields: {
                    label: "calcFields(...fieldNames: (keyof TRecord)[]): this",
                    doc: "Evaluate declarative FlowFields on this record via SQL subqueries.",
                    params: [
                        { label: "...fieldNames: (keyof TRecord)[]", documentation: "One or more FlowField names to calculate." }
                    ]
                },
                validate: {
                    label: "validate(fieldName: keyof TRecord, newValue?: any): this",
                    doc: "Assign a field value and trigger its field-level `validate` handler.",
                    params: [
                        { label: "fieldName: keyof TRecord", documentation: "Name of the field to validate." },
                        { label: "newValue?: any", documentation: "New value to assign before validation." }
                    ]
                }
            };

            const sig = signatures[fnName] || (methodSuffix ? modelMethodSignatures[methodSuffix] : null);
            if (!sig) return null;

            return {
                value: {
                    signatures: [
                        {
                            label: sig.label,
                            documentation: sig.doc,
                            parameters: sig.params
                        }
                    ],
                    activeSignature: 0,
                    activeParameter: Math.min(activeParam, sig.params.length - 1)
                },
                dispose: () => {}
            };
        }
    }));

    // 6. Document Symbol Provider (Outline / Breadcrumbs / Go-to-Symbol)
    track(monaco.languages.registerDocumentSymbolProvider("bos-script", {
        provideDocumentSymbols: (model) => {
            const docCtx = parseDocumentSymbolsAndContext(model.getValue());
            const lineCount = model.getLineCount();

            const mapKind = (k) => {
                switch (k) {
                    case "table":
                    case "tableextension":
                        return monaco.languages.SymbolKind.Struct;
                    case "codeunit":
                    case "tablelogic":
                        return monaco.languages.SymbolKind.Class;
                    case "interface":
                        return monaco.languages.SymbolKind.Interface;
                    case "enum":
                    case "enumextension":
                        return monaco.languages.SymbolKind.Enum;
                    case "statemachine":
                        return monaco.languages.SymbolKind.Namespace;
                    case "field":
                        return monaco.languages.SymbolKind.Field;
                    case "method":
                        return monaco.languages.SymbolKind.Method;
                    case "event":
                    case "transition":
                        return monaco.languages.SymbolKind.Event;
                    case "key":
                        return monaco.languages.SymbolKind.Key;
                    default:
                        return monaco.languages.SymbolKind.Module;
                }
            };

            return docCtx.symbols.map((sym, idx) => {
                const nextSymLine = idx + 1 < docCtx.symbols.length ? docCtx.symbols[idx + 1].line - 1 : lineCount;
                const range = {
                    startLineNumber: sym.line,
                    startColumn: 1,
                    endLineNumber: Math.max(sym.line, nextSymLine),
                    endColumn: model.getLineMaxColumn(Math.max(sym.line, nextSymLine))
                };
                return {
                    name: sym.name,
                    detail: sym.related ? `${sym.kind} -> ${sym.related}` : sym.kind,
                    kind: mapKind(sym.kind),
                    range,
                    selectionRange: {
                        startLineNumber: sym.line,
                        startColumn: 1,
                        endLineNumber: sym.line,
                        endColumn: model.getLineMaxColumn(sym.line)
                    },
                    children: (sym.children || []).map((ch) => ({
                        name: ch.name,
                        detail: ch.detail || "",
                        kind: mapKind(ch.kind),
                        range: {
                            startLineNumber: ch.line,
                            startColumn: 1,
                            endLineNumber: ch.line,
                            endColumn: model.getLineMaxColumn(ch.line)
                        },
                        selectionRange: {
                            startLineNumber: ch.line,
                            startColumn: 1,
                            endLineNumber: ch.line,
                            endColumn: model.getLineMaxColumn(ch.line)
                        }
                    }))
                };
            });
        }
    }));

    // 7. Prettier formatting (Shift+Alt+F, format on paste, format selection)
    const fullFormatEdit = async (model, formatted) => [
        {
            range: model.getFullModelRange(),
            text: formatted
        }
    ];
    track(monaco.languages.registerDocumentFormattingEditProvider("bos-script", {
        provideDocumentFormattingEdits: async (model) => fullFormatEdit(model, await formatBosScriptSource(model.getValue()))
    }));
    if (typeof monaco.languages.registerDocumentRangeFormattingEditProvider === "function") {
        track(monaco.languages.registerDocumentRangeFormattingEditProvider("bos-script", {
            provideDocumentRangeFormattingEdits: async (model, range) => [
                {
                    range,
                    text: await formatRangeText(model.getValueInRange(range))
                }
            ]
        }));
    }
    for (const language of ["typescript", "javascript"]) {
        const parser = language === "javascript" ? "babel" : "typescript";
        track(monaco.languages.registerDocumentFormattingEditProvider(language, {
            provideDocumentFormattingEdits: async (model) => fullFormatEdit(model, await formatTypeScriptSource(model.getValue(), parser))
        }));
        if (typeof monaco.languages.registerDocumentRangeFormattingEditProvider === "function") {
            track(monaco.languages.registerDocumentRangeFormattingEditProvider(language, {
                provideDocumentRangeFormattingEdits: async (model, range) => [
                    {
                        range,
                        text: await formatRangeText(model.getValueInRange(range), parser)
                    }
                ]
            }));
        }
    }

    // 8. Code Action Provider (Quick Fixes for Compiler Diagnostics)
    track(monaco.languages.registerCodeActionProvider("bos-script", {
        provideCodeActions: (model, _range, context) => {
            const actions = [];
            for (const marker of context.markers || []) {
                const msg = marker.message || "";
                const code = marker.code || "";

                // Quick Fix 1: Missing primaryKey (BOS2002)
                if (code === "BOS2002" || msg.includes("primaryKey")) {
                    const insertLine = Math.min(model.getLineCount(), marker.startLineNumber + 1);
                    actions.push({
                        title: "Add 'id: UUID notNull' and 'primaryKey(id)'",
                        diagnostics: [marker],
                        kind: "quickfix",
                        isPreferred: true,
                        edit: {
                            edits: [
                                {
                                    resource: model.uri,
                                    textEdit: {
                                        range: {
                                            startLineNumber: insertLine,
                                            startColumn: 1,
                                            endLineNumber: insertLine,
                                            endColumn: 1
                                        },
                                        text: "    id: UUID notNull;\n    primaryKey(id);\n"
                                    },
                                    versionId: model.getVersionId()
                                }
                            ]
                        }
                    });
                }

                // Quick Fix 2: Document table missing status field (BOS2005)
                if (code === "BOS2005" || (msg.includes("Document") && msg.includes("status"))) {
                    const insertLine = Math.min(model.getLineCount(), marker.startLineNumber + 1);
                    actions.push({
                        title: "Add 'status: String(30) default(\"Draft\");' field",
                        diagnostics: [marker],
                        kind: "quickfix",
                        isPreferred: true,
                        edit: {
                            edits: [
                                {
                                    resource: model.uri,
                                    textEdit: {
                                        range: {
                                            startLineNumber: insertLine,
                                            startColumn: 1,
                                            endLineNumber: insertLine,
                                            endColumn: 1
                                        },
                                        text: "    status: String(30) default(\"Draft\");\n"
                                    },
                                    versionId: model.getVersionId()
                                }
                            ]
                        }
                    });
                }

                // Quick Fix 3: Unknown field typo (BOS2014) -> replace with suggested column from hint
                if (code === "BOS2014" || msg.includes("does not exist on table")) {
                    const didYouMeanMatch = msg.match(/Did you mean `([A-Za-z_][A-Za-z0-9_]*)`\?/);
                    if (didYouMeanMatch) {
                        const suggestedCol = didYouMeanMatch[1];
                        actions.push({
                            title: `Replace with '${suggestedCol}'`,
                            diagnostics: [marker],
                            kind: "quickfix",
                            isPreferred: true,
                            edit: {
                                edits: [
                                    {
                                        resource: model.uri,
                                        textEdit: {
                                            range: {
                                                startLineNumber: marker.startLineNumber,
                                                startColumn: marker.startColumn,
                                                endLineNumber: marker.endLineNumber,
                                                endColumn: marker.endColumn
                                            },
                                            text: suggestedCol
                                        },
                                        versionId: model.getVersionId()
                                    }
                                ]
                            }
                        });
                    }
                }

                // Quick Fix 4: Missing semicolon
                if (msg.toLowerCase().includes("semicolon")) {
                    const targetLine = marker.startLineNumber;
                    const lineContent = model.getLineContent(targetLine);
                    const endCol = lineContent.length + 1;
                    actions.push({
                        title: "Insert missing semicolon ';'",
                        diagnostics: [marker],
                        kind: "quickfix",
                        isPreferred: true,
                        edit: {
                            edits: [
                                {
                                    resource: model.uri,
                                    textEdit: {
                                        range: {
                                            startLineNumber: targetLine,
                                            startColumn: endCol,
                                            endLineNumber: targetLine,
                                            endColumn: endCol
                                        },
                                        text: ";"
                                    },
                                    versionId: model.getVersionId()
                                }
                            ]
                        }
                    });
                }
            }
            return {
                actions,
                dispose: () => {}
            };
        }
    }));

    // 9. Folding Range Provider
    track(monaco.languages.registerFoldingRangeProvider("bos-script", {
        provideFoldingRanges: (model) => {
            const ranges = [];
            const stack = [];
            const lineCount = model.getLineCount();

            for (let i = 1; i <= lineCount; i++) {
                const text = model.getLineContent(i).trim();
                if (text.startsWith("//")) continue;
                for (let c = 0; c < text.length; c++) {
                    if (text[c] === "{") {
                        stack.push(i);
                    } else if (text[c] === "}" && stack.length > 0) {
                        const start = stack.pop();
                        if (i > start) {
                            ranges.push({
                                start,
                                end: i,
                                kind: monaco.languages.FoldingRangeKind.Region
                            });
                        }
                    }
                }
            }
            return ranges;
        }
    }));

    // 10. Definition Provider (F12 / Ctrl+Click Go-To-Definition for Local Symbols & Active DB Catalog Objects)
    if (typeof monaco.languages.registerDefinitionProvider === "function") {
        track(monaco.languages.registerDefinitionProvider("bos-script", {
            provideDefinition: (model, position) => {
                const word = model.getWordAtPosition(position);
                if (!word || !word.word) return null;
                const token = word.word;
                const docCtx = parseDocumentSymbolsAndContext(model.getValue());

                // 1. Check local meta-object or child symbol in the current buffer
                for (const sym of docCtx.symbols) {
                    if (sym.name === token) {
                        return {
                            uri: model.uri,
                            range: {
                                startLineNumber: sym.line,
                                startColumn: 1,
                                endLineNumber: sym.line,
                                endColumn: model.getLineMaxColumn(sym.line)
                            }
                        };
                    }
                    for (const ch of sym.children || []) {
                        if (ch.name === token) {
                            return {
                                uri: model.uri,
                                range: {
                                    startLineNumber: ch.line,
                                    startColumn: 1,
                                    endLineNumber: ch.line,
                                    endColumn: model.getLineMaxColumn(ch.line)
                                }
                            };
                        }
                    }
                }

                // 2. Check active database catalog objects (Table, Enum, Codeunit, Interface)
                if (liveWorkspaceCatalog.tables.has(token)) {
                    if (openObjectCallback) openObjectCallback(token, "code", "table");
                    return {
                        uri: model.uri,
                        range: {
                            startLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endLineNumber: position.lineNumber,
                            endColumn: word.endColumn
                        }
                    };
                }
                if (liveWorkspaceCatalog.enums.has(token)) {
                    if (openObjectCallback) openObjectCallback(token, "code", "enum");
                    return {
                        uri: model.uri,
                        range: {
                            startLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endLineNumber: position.lineNumber,
                            endColumn: word.endColumn
                        }
                    };
                }
                if (liveWorkspaceCatalog.codeunits.has(token)) {
                    if (openObjectCallback) openObjectCallback(token, "code", "codeunit");
                    return {
                        uri: model.uri,
                        range: {
                            startLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endLineNumber: position.lineNumber,
                            endColumn: word.endColumn
                        }
                    };
                }
                if (liveWorkspaceCatalog.interfaces.has(token)) {
                    if (openObjectCallback) openObjectCallback(token, "code", "interface");
                    return {
                        uri: model.uri,
                        range: {
                            startLineNumber: position.lineNumber,
                            startColumn: word.startColumn,
                            endLineNumber: position.lineNumber,
                            endColumn: word.endColumn
                        }
                    };
                }

                return null;
            }
        }));
    }
}

export { formatBosScriptSource, formatRangeText, formatTypeScriptSource } from "./bosPrettier.js";
