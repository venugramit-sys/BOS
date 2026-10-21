/**
 * Monaco Editor LSP, Live Compiler Diagnostics & Virtual TypeScript Shadow Validator
 * Bridges Monaco Editor with:
 *   1. `/api/meta/ambient.d.ts` (live catalog-driven TypeScript declarations)
 *   2. `/api/script/compile` (real-time Rust `bosc` lexer, parser, & semantic diagnostics)
 *   3. Monaco's TypeScript Worker via a line-preserving Virtual Shadow `.ts` Model for `codeunit` & `tablelogic`
 */

import { registerBosScriptLanguage, hydrateCatalogFromAmbientDts } from "./monacoBosScript.js";

let ambientLoaded = false;
let lastAmbientDts = "";
let shadowModelCounter = 0;
const themedInstances = new WeakSet();

/**
 * Configure BOS custom themes (`bos-dark` and `bos-light`), register the `bos-script` language,
 * and initialize ambient `.d.ts` declarations on the provided Monaco instance.
 */
export function setupMonacoEnvironment(monaco) {
    if (!monaco) return null;

    registerBosScriptLanguage(monaco);

    if (!themedInstances.has(monaco) && monaco.editor?.defineTheme) {
        themedInstances.add(monaco);
        monaco.editor.defineTheme("bos-dark", {
            base: "vs-dark",
            inherit: true,
            rules: [
                { token: "keyword.declaration.bos", foreground: "38BDF8", fontStyle: "bold" },
                { token: "keyword.bos", foreground: "C084FC" },
                { token: "type.bos", foreground: "34D399" },
                { token: "annotation.bos", foreground: "FBBF24", fontStyle: "italic" },
                { token: "predefined.bos", foreground: "60A5FA" },
                { token: "variable.predefined.bos", foreground: "F472B6" },
                { token: "string.bos", foreground: "A7F3D0" },
                { token: "number.bos", foreground: "FCD34D" },
                { token: "comment.bos", foreground: "64748B", fontStyle: "italic" }
            ],
            colors: {
                "editor.background": "#090D16",
                "editor.lineHighlightBackground": "#1E293B50",
                "editorLineNumber.foreground": "#475569",
                "editorLineNumber.activeForeground": "#E2E8F0",
                "editorIndentGuide.background": "#1E293B",
                "editorSuggestWidget.background": "#0F172A",
                "editorSuggestWidget.border": "#334155",
                "editorSuggestWidget.selectedBackground": "#1E293B",
                "editorHoverWidget.background": "#0F172A",
                "editorHoverWidget.border": "#334155"
            }
        });

        monaco.editor.defineTheme("bos-light", {
            base: "vs",
            inherit: true,
            rules: [
                { token: "keyword.declaration.bos", foreground: "0284C7", fontStyle: "bold" },
                { token: "keyword.bos", foreground: "7C3AED" },
                { token: "type.bos", foreground: "059669" },
                { token: "annotation.bos", foreground: "D97706", fontStyle: "italic" },
                { token: "predefined.bos", foreground: "2563EB" },
                { token: "variable.predefined.bos", foreground: "DB2777" },
                { token: "comment.bos", foreground: "64748B", fontStyle: "italic" }
            ],
            colors: {
                "editor.background": "#FFFFFF",
                "editor.lineHighlightBackground": "#F1F5F9"
            }
        });
    }

    syncAmbientDts(monaco);
    return monaco;
}

/**
 * Fetch and inject live catalog-aware ambient TypeScript declarations from `/api/meta/ambient.d.ts`
 */
export async function syncAmbientDts(monaco, forceRefresh = false) {
    if (!monaco || !monaco.languages || !monaco.languages.typescript) return false;
    if (ambientLoaded && !forceRefresh) return true;

    try {
        const tsDefaults = monaco.languages.typescript.typescriptDefaults;

        tsDefaults.setCompilerOptions({
            target: monaco.languages.typescript.ScriptTarget.ES2020,
            allowNonTsExtensions: true,
            moduleResolution: monaco.languages.typescript.ModuleResolutionKind.NodeJs,
            module: monaco.languages.typescript.ModuleKind.CommonJS,
            noEmit: true,
            strict: false,
            noImplicitAny: false,
            strictNullChecks: false,
            allowJs: true
        });

        tsDefaults.setDiagnosticsOptions({
            noSemanticValidation: false,
            noSyntaxValidation: false
        });

        const res = await fetch("/api/meta/ambient.d.ts");
        if (res.ok) {
            const dtsText = await res.text();
            lastAmbientDts = dtsText;
            hydrateCatalogFromAmbientDts(dtsText);
            tsDefaults.setExtraLibs([
                {
                    content: dtsText,
                    filePath: "ts:filename/bos-ambient.d.ts"
                }
            ]);
            ambientLoaded = true;
            return true;
        }
    } catch (e) {
        console.warn("Failed to load ambient.d.ts from backend, using fallback declarations:", e);
    }

    // Fallback ambient declarations if backend is offline
    if (!lastAmbientDts) {
        const fallbackDts = `
declare type UUID = string;
declare type Decimal = number;
declare type Integer = number;
declare type BigInteger = number;
declare type Json = Record<string, any>;
declare const ctx: any;
declare const models: Record<string, any>;
declare const rec: Record<string, any>;
declare const xRec: Record<string, any>;
declare const Rec: Record<string, any>;
declare function error(message: string): never;
declare function calcFields(...fields: string[]): void;
`;
        monaco.languages.typescript.typescriptDefaults.setExtraLibs([
            {
                content: fallbackDts,
                filePath: "ts:filename/bos-ambient.d.ts"
            }
        ]);
    }
    return false;
}

/**
 * Transpiles a `codeunit` or `tablelogic` BOS-Script buffer line-for-line into valid TypeScript
 * so Monaco's TypeScript language service can perform syntax & type checking on embedded TS bodies
 * while preserving exact line numbers.
 */
export function transpileBosToShadowTypeScript(source = "") {
    const lines = source.split("\n");
    let isCodeunitOrLogic = false;
    let targetRecordType = "any";

    for (const line of lines) {
        const headerMatch = line.match(
            /^\s*(codeunit|tablelogic|tablelogicextension)\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+(?:for|extends)\s+([A-Za-z_][A-Za-z0-9_]*))?/
        );
        if (headerMatch) {
            isCodeunitOrLogic = true;
            if (headerMatch[3]) {
                targetRecordType = `${headerMatch[3]}Record`;
            }
            break;
        }
    }
    if (!isCodeunitOrLogic) return null;

    let inDeclarativeBlock = false;
    let declBraceDepth = 0;
    const scan = { mode: "code", holes: [] };

    function codeBraceDelta(line) {
        let delta = 0;
        for (let i = 0; i < line.length; i++) {
            const c = line[i];
            const n = line[i + 1];
            if (scan.mode === "line") break;
            if (scan.mode === "block") {
                if (c === "*" && n === "/") {
                    scan.mode = "code";
                    i += 1;
                }
                continue;
            }
            if (scan.mode === "sq" || scan.mode === "dq") {
                const quote = scan.mode === "sq" ? "'" : '"';
                if (c === "\\") {
                    i += 1;
                    continue;
                }
                if (c === quote || c === "\n") scan.mode = "code";
                continue;
            }
            if (scan.mode === "tpl") {
                if (c === "\\") {
                    i += 1;
                    continue;
                }
                if (c === "`") {
                    scan.mode = "code";
                    continue;
                }
                if (c === "$" && n === "{") {
                    scan.holes.push(1);
                    scan.mode = "code";
                    i += 1;
                }
                continue;
            }
            if (c === "/" && n === "/") {
                scan.mode = "line";
                break;
            }
            if (c === "/" && n === "*") {
                scan.mode = "block";
                i += 1;
                continue;
            }
            if (c === "'") {
                scan.mode = "sq";
                continue;
            }
            if (c === '"') {
                scan.mode = "dq";
                continue;
            }
            if (c === "`") {
                scan.mode = "tpl";
                continue;
            }
            if (scan.holes.length && c === "{") {
                scan.holes[scan.holes.length - 1] += 1;
                continue;
            }
            if (scan.holes.length && c === "}") {
                scan.holes[scan.holes.length - 1] -= 1;
                if (scan.holes[scan.holes.length - 1] === 0) {
                    scan.holes.pop();
                    scan.mode = "tpl";
                }
                continue;
            }
            if (c === "{") delta += 1;
            else if (c === "}") delta -= 1;
        }
        if (scan.mode === "line") scan.mode = "code";
        return delta;
    }

    const tsLines = lines.map((line) => {
        // 0. If the buffer contains a declarative `table`, `tableextension`, `statemachine`, `enum`, or `page` block
        // above or alongside `tablelogic` / `codeunit`, comment out its lines 1-for-1 so line numbers stay 100% identical!
        const declHeader = line.match(
            /^\s*(table|tableextension|statemachine|statemachineextension|enum|enumextension|page|pageextension|query|view|report|permissionset|custommetadata)\s+[A-Za-z_][A-Za-z0-9_]*\b/
        );
        if (declHeader && !inDeclarativeBlock) {
            inDeclarativeBlock = true;
            declBraceDepth = 0;
        }
        if (inDeclarativeBlock) {
            declBraceDepth += codeBraceDelta(line);
            if (declBraceDepth <= 0 && line.includes("}")) {
                inDeclarativeBlock = false;
            }
            return `// ${line}`;
        }

        // 1. Replace meta-object header with a TypeScript class header on the exact same line
        if (/^\s*(codeunit|tablelogic|tablelogicextension)\s+([A-Za-z_][A-Za-z0-9_]*)/.test(line)) {
            return line.replace(
                /^(\s*)(codeunit|tablelogic|tablelogicextension)\s+([A-Za-z_][A-Za-z0-9_]*)(?:[^{]*)(\{?)/,
                "$1class $3 $4"
            );
        }

        // 2. Comment out BOS decorators (@businessEvent, @subscribeEvent, @pushdown, etc.) on the same line
        if (/^\s*@[A-Za-z_]/.test(line)) {
            return line.replace(/^(\s*)(@.*)$/, "$1// $2");
        }

        // 3. Convert `async procedure foo(...)` or `procedure foo(...)` or `function foo(...)` to class methods
        if (/^\s*(async\s+)?(procedure|function)\s+[A-Za-z_]/.test(line)) {
            return line.replace(
                /^(\s*)(async\s+)?(?:procedure|function)\s+([A-Za-z_][A-Za-z0-9_]*)/,
                "$1$2$3"
            );
        }

        // 4. Convert field validation `validate <fieldName>(val)` into valid TS method `__validate_<fieldName>(val: any)`
        if (/^\s*validate\s+[A-Za-z_][A-Za-z0-9_]*\s*\(/.test(line)) {
            return line.replace(
                /^(\s*)validate\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*,\s*([A-Za-z_][A-Za-z0-9_]*))?\s*\)/,
                (_, indent, fName, valParam, ctxParam) =>
                    ctxParam
                        ? `${indent}__validate_${fName}(${valParam}: any, ${ctxParam}: any)`
                        : `${indent}__validate_${fName}(${valParam}: any)`
            );
        }

        // 5. Convert lifecycle triggers `onInsert(rec)` / `onInsert(rec, ctx)` / `onModify(rec, xRec, ctx)` -> typed TS methods!
        if (/^\s*(onInsert|onModify|onDelete|onRename)\s*\(/.test(line)) {
            return line
                .replace(
                    /\b(onInsert|onDelete|onRename)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*,\s*([A-Za-z_][A-Za-z0-9_]*))?\s*\)/,
                    (_, trig, recParam, ctxParam) =>
                        ctxParam
                            ? `${trig}(${recParam}: ${targetRecordType}, ${ctxParam}: any)`
                            : `${trig}(${recParam}: ${targetRecordType})`
                )
                .replace(
                    /\bonModify\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*,\s*([A-Za-z_][A-Za-z0-9_]*))?\s*\)/,
                    (_, recParam, xRecParam, ctxParam) =>
                        ctxParam
                            ? `onModify(${recParam}: ${targetRecordType}, ${xRecParam}: ${targetRecordType}, ${ctxParam}: any)`
                            : `onModify(${recParam}: ${targetRecordType}, ${xRecParam}: ${targetRecordType})`
                );
        }

        return line;
    });

    return tsLines.join("\n");
}

/**
 * Run Monaco's TypeScript worker on the shadow `.ts` model to collect embedded TypeScript diagnostics.
 */
async function collectShadowTypeScriptDiagnostics(monaco, shadowTsCode, shadowModelRef) {
    if (!shadowTsCode || !monaco?.languages?.typescript) return [];

    try {
        if (!shadowModelRef.model || shadowModelRef.model.isDisposed()) {
            const uri = monaco.Uri.parse(`inmemory://bos-shadow/unit_${++shadowModelCounter}.ts`);
            shadowModelRef.model = monaco.editor.createModel(shadowTsCode, "typescript", uri);
        } else {
            shadowModelRef.model.setValue(shadowTsCode);
        }

        const getWorker = await monaco.languages.typescript.getTypeScriptWorker();
        const worker = await getWorker(shadowModelRef.model.uri);
        const uriStr = shadowModelRef.model.uri.toString();

        const [syntactic, semantic] = await Promise.all([
            worker.getSyntacticDiagnostics(uriStr),
            worker.getSemanticDiagnostics(uriStr)
        ]);

        const allTsDiags = [...(syntactic || []), ...(semantic || [])];
        const converted = [];

        for (const d of allTsDiags) {
            if (typeof d.start !== "number") continue;
            // Ignore benign TS complaints about unused declarations or missing return statements in stubs
            if ([6133, 2355, 7006, 2564].includes(d.code)) continue;

            const startPos = shadowModelRef.model.getPositionAt(d.start);
            const endPos = shadowModelRef.model.getPositionAt(d.start + (d.length || 1));
            const msgText =
                typeof d.messageText === "string"
                    ? d.messageText
                    : d.messageText?.messageText || "TypeScript error";

            converted.push({
                severity: d.category === 0 ? "warning" : "error",
                code: `TS${d.code || ""}`,
                message: msgText,
                line: startPos.lineNumber,
                column: startPos.column,
                end_line: endPos.lineNumber,
                end_column: endPos.column,
                hint: null,
                source: "TypeScript"
            });
        }

        return converted;
    } catch {
        return [];
    }
}

/**
 * Set up live debounced compilation & diagnostics markers on a Monaco editor instance.
 * @param {object} monaco - Monaco namespace
 * @param {object} editor - Monaco IStandaloneCodeEditor instance
 * @param {string} packageName - Target package name
 * @param {function} onValidateCallback - Called with `{ isCompiling, isValid, error, ddl, ir, diagnostics }`
 * @returns {object} Controller with `{ triggerValidation, dispose }`
 */
export function setupLiveDiagnosticMarkers(monaco, editor, packageName = "custom", onValidateCallback = null) {
    let debounceTimer = null;
    let currentAbortController = null;
    const shadowModelRef = { model: null };

    const validateBosScript = async () => {
        const model = editor.getModel();
        if (!model || model.isDisposed()) return;

        const script = model.getValue();
        if (!script.trim()) {
            monaco.editor.setModelMarkers(model, "bosc-compiler", []);
            if (onValidateCallback) {
                onValidateCallback({
                    isCompiling: false,
                    isValid: true,
                    error: null,
                    ddl: "",
                    ir: null,
                    diagnostics: []
                });
            }
            return;
        }

        if (currentAbortController) {
            currentAbortController.abort();
        }
        currentAbortController = new AbortController();

        if (onValidateCallback) {
            onValidateCallback({ isCompiling: true });
        }

        try {
            // Run Rust `bosc` compiler check and (if applicable) Shadow TypeScript check in parallel
            const shadowTs = transpileBosToShadowTypeScript(script);
            const [res, shadowTsDiags] = await Promise.all([
                fetch("/api/script/compile", {
                    method: "POST",
                    headers: { "Content-Type": "application/json" },
                    body: JSON.stringify({
                        script,
                        package_name: packageName,
                        apply: false
                    }),
                    signal: currentAbortController.signal
                }),
                collectShadowTypeScriptDiagnostics(monaco, shadowTs, shadowModelRef)
            ]);

            const data = await res.json();
            const rawDiagnostics = Array.isArray(data.diagnostics) ? [...data.diagnostics] : [];

            // Fallback if backend returned an error string without structured diagnostics
            if ((data.status === "error" || !res.ok) && rawDiagnostics.length === 0) {
                const errText = data.error || data.message || "Compilation error";
                const parsedPos = parseCompilerErrorPosition(errText, model);
                rawDiagnostics.push({
                    severity: "error",
                    code: "BOS1000",
                    message: errText,
                    line: parsedPos.startLineNumber,
                    column: parsedPos.startColumn,
                    end_line: parsedPos.endLineNumber,
                    end_column: parsedPos.endColumn,
                    hint: null,
                    source: "bosc"
                });
            }

            // Merge shadow TypeScript diagnostics if they don't duplicate an existing bosc diagnostic on the same line
            const boscLines = new Set(rawDiagnostics.map((d) => d.line));
            for (const tsDiag of shadowTsDiags) {
                if (!boscLines.has(tsDiag.line)) {
                    rawDiagnostics.push(tsDiag);
                }
            }

            // Convert combined diagnostics into Monaco Editor Markers
            const lineCount = model.getLineCount();
            const markers = rawDiagnostics.map((d) => {
                const startLine = Math.min(Math.max(1, Number(d.line) || 1), lineCount);
                const endLine = Math.min(Math.max(startLine, Number(d.end_line) || startLine), lineCount);
                const maxCol = model.getLineMaxColumn(endLine);
                const startCol = Math.min(Math.max(1, Number(d.column) || 1), maxCol);
                const endCol =
                    d.end_column && Number(d.end_column) > startCol
                        ? Math.min(Number(d.end_column), maxCol)
                        : maxCol;

                let severity = monaco.MarkerSeverity.Error;
                if (d.severity === "warning") severity = monaco.MarkerSeverity.Warning;
                else if (d.severity === "info") severity = monaco.MarkerSeverity.Info;

                const fullMsg = d.hint ? `${d.message}\n💡 Hint: ${d.hint}` : d.message;

                return {
                    severity,
                    message: fullMsg,
                    code: d.code || "BOS",
                    startLineNumber: startLine,
                    startColumn: startCol,
                    endLineNumber: endLine,
                    endColumn: Math.max(startCol + 1, endCol),
                    source: d.source || "bosc-compiler"
                };
            });

            monaco.editor.setModelMarkers(model, "bosc-compiler", markers);

            const hasErrors = rawDiagnostics.some((d) => d.severity === "error") || data.status === "error" || !res.ok;
            const primaryError =
                rawDiagnostics.find((d) => d.severity === "error")?.message ||
                (hasErrors ? data.error || "Compilation failed" : null);

            if (onValidateCallback) {
                onValidateCallback({
                    isCompiling: false,
                    isValid: !hasErrors,
                    error: primaryError,
                    ddl: data.compiled_ddl || "",
                    ir: data.ir || null,
                    diagnostics: rawDiagnostics
                });
            }
        } catch (err) {
            if (err.name === "AbortError") return;
            if (onValidateCallback) {
                onValidateCallback({
                    isCompiling: false,
                    isValid: false,
                    error: err.message,
                    ddl: "",
                    ir: null,
                    diagnostics: []
                });
            }
        }
    };

    const disposable = editor.onDidChangeModelContent(() => {
        if (debounceTimer) clearTimeout(debounceTimer);
        debounceTimer = setTimeout(validateBosScript, 320);
    });

    // Run initial validation
    setTimeout(validateBosScript, 120);

    return {
        triggerValidation: validateBosScript,
        dispose: () => {
            if (debounceTimer) clearTimeout(debounceTimer);
            if (currentAbortController) currentAbortController.abort();
            if (shadowModelRef.model && !shadowModelRef.model.isDisposed()) {
                shadowModelRef.model.dispose();
            }
            disposable.dispose();
        }
    };
}

export function getLastAmbientDts() {
    return lastAmbientDts;
}

/**
 * Extract line/column numbers from bosc compiler error strings (supports both human and Rust Debug formats)
 */
export function parseCompilerErrorPosition(errorMsg = "", model = null) {
    // 1. Match "line 5, column 12" or "line 5, col 12"
    const humanMatch = errorMsg.match(/line\s+(\d+)(?:,\s*(?:column|col)\s+(\d+))?/i);
    // 2. Match Rust Debug format "line: 5, col: 12"
    const debugMatch = errorMsg.match(/line:\s*(\d+)(?:,\s*col:\s*(\d+))?/i);

    const match = humanMatch || debugMatch;
    const totalLines = model ? model.getLineCount() : 9999;

    if (match) {
        const line = Math.min(Math.max(1, parseInt(match[1], 10)), totalLines);
        const col = match[2] ? Math.max(1, parseInt(match[2], 10)) : 1;
        const maxCol = model ? model.getLineMaxColumn(line) : col + 20;

        return {
            startLineNumber: line,
            startColumn: col,
            endLineNumber: line,
            endColumn: Math.max(col + 1, maxCol)
        };
    }

    return {
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: 1,
        endColumn: model ? model.getLineMaxColumn(1) : 80
    };
}
