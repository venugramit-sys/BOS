import test from "node:test";
import assert from "node:assert/strict";

import {
    registerBosScriptLanguage,
    updateBosScriptCatalogContext,
    registerTableSchemaForIntelliSense,
    hydrateCatalogFromAmbientDts,
    parseDocumentSymbolsAndContext,
    formatBosScriptSource
} from "../../ui/src/lib/components/studio/code-editor/monacoBosScript.js";

import {
    setupMonacoEnvironment,
    transpileBosToShadowTypeScript,
    parseCompilerErrorPosition
} from "../../ui/src/lib/components/studio/code-editor/monacoLspProvider.js";

function createMockMonaco() {
    const providers = {
        activeCompletionProviders: []
    };
    const themes = {};

    return {
        providers,
        themes,
        MarkerSeverity: { Error: 8, Warning: 4, Info: 2, Hint: 1 },
        editor: {
            defineTheme: (name, def) => {
                themes[name] = def;
            },
            setModelMarkers: () => {}
        },
        languages: {
            CompletionItemKind: {
                Method: 0,
                Function: 1,
                Constructor: 2,
                Field: 3,
                Variable: 4,
                Class: 5,
                Struct: 6,
                Interface: 7,
                Module: 8,
                Property: 9,
                Event: 10,
                Operator: 11,
                Unit: 12,
                Value: 13,
                Constant: 14,
                Enum: 15,
                EnumMember: 16,
                Keyword: 17,
                Text: 18,
                Color: 19,
                File: 20,
                Reference: 21,
                Customcolor: 22,
                Folder: 23,
                TypeParameter: 24,
                Snippet: 25
            },
            CompletionItemInsertTextRule: {
                InsertAsSnippet: 4
            },
            SymbolKind: {
                File: 0,
                Module: 1,
                Namespace: 2,
                Package: 3,
                Class: 4,
                Method: 5,
                Property: 6,
                Field: 7,
                Constructor: 8,
                Enum: 9,
                Interface: 10,
                Function: 11,
                Variable: 12,
                Constant: 13,
                String: 14,
                Number: 15,
                Boolean: 16,
                Array: 17,
                Object: 18,
                Key: 19,
                Null: 20,
                EnumMember: 21,
                Struct: 22,
                Event: 23
            },
            FoldingRangeKind: {
                Comment: "comment",
                Imports: "imports",
                Region: "region"
            },
            register: (lang) => {
                providers.language = lang;
            },
            setMonarchTokensProvider: (id, def) => {
                providers.monarch = { id, def };
                return { dispose: () => {} };
            },
            setLanguageConfiguration: (id, conf) => {
                providers.configuration = { id, conf };
                return { dispose: () => {} };
            },
            registerCompletionItemProvider: (id, prov) => {
                providers.completion = prov;
                providers.activeCompletionProviders.push(prov);
                return {
                    dispose: () => {
                        const idx = providers.activeCompletionProviders.indexOf(prov);
                        if (idx !== -1) providers.activeCompletionProviders.splice(idx, 1);
                    }
                };
            },
            registerHoverProvider: (id, prov) => {
                providers.hover = prov;
                return { dispose: () => {} };
            },
            registerSignatureHelpProvider: (id, prov) => {
                providers.signatureHelp = prov;
                return { dispose: () => {} };
            },
            registerDocumentSymbolProvider: (id, prov) => {
                providers.documentSymbol = prov;
                return { dispose: () => {} };
            },
            registerDocumentFormattingEditProvider: (id, prov) => {
                if (id === "bos-script") providers.formatting = prov;
                return { dispose: () => {} };
            },
            registerDocumentRangeFormattingEditProvider: (id, prov) => {
                if (id === "bos-script") providers.rangeFormatting = prov;
                return { dispose: () => {} };
            },
            registerCodeActionProvider: (id, prov) => {
                providers.codeAction = prov;
                return { dispose: () => {} };
            },
            registerFoldingRangeProvider: (id, prov) => {
                providers.folding = prov;
                return { dispose: () => {} };
            },
            registerDefinitionProvider: (id, prov) => {
                providers.definition = prov;
                return { dispose: () => {} };
            }
        }
    };
}

function createMockModel(text) {
    const lines = text.split("\n");
    return {
        uri: "inmemory://model/test.bos",
        getValue: () => text,
        getLineCount: () => lines.length,
        getLineContent: (lineNumber) => lines[lineNumber - 1] || "",
        getLineMaxColumn: (lineNumber) => (lines[lineNumber - 1] || "").length + 1,
        getFullModelRange: () => ({
            startLineNumber: 1,
            startColumn: 1,
            endLineNumber: lines.length,
            endColumn: (lines[lines.length - 1] || "").length + 1
        }),
        getVersionId: () => 1,
        getWordUntilPosition: (pos) => {
            const line = lines[pos.lineNumber - 1] || "";
            const prefix = line.slice(0, pos.column - 1);
            const match = prefix.match(/([A-Za-z0-9_]+)$/);
            const word = match ? match[1] : "";
            return {
                word,
                startColumn: pos.column - word.length,
                endColumn: pos.column
            };
        },
        getWordAtPosition: (pos) => {
            const line = lines[pos.lineNumber - 1] || "";
            const left = line.slice(0, pos.column - 1).match(/([A-Za-z0-9_]+)$/);
            const right = line.slice(pos.column - 1).match(/^([A-Za-z0-9_]+)/);
            const word = (left ? left[1] : "") + (right ? right[1] : "");
            if (!word) return null;
            const startColumn = pos.column - (left ? left[1].length : 0);
            return {
                word,
                startColumn,
                endColumn: startColumn + word.length
            };
        }
    };
}

test("registers BOS-Script language, themes, and all 7 IntelliSense providers", () => {
    const mockMonaco = createMockMonaco();
    setupMonacoEnvironment(mockMonaco);
    registerBosScriptLanguage(mockMonaco);

    assert.ok(mockMonaco.providers.monarch, "Monarch tokenizer should be registered");
    assert.ok(mockMonaco.providers.completion, "CompletionItemProvider should be registered");
    assert.ok(mockMonaco.providers.hover, "HoverProvider should be registered");
    assert.ok(mockMonaco.providers.signatureHelp, "SignatureHelpProvider should be registered");
    assert.ok(mockMonaco.providers.documentSymbol, "DocumentSymbolProvider should be registered");
    assert.ok(mockMonaco.providers.formatting, "DocumentFormattingEditProvider should be registered");
    assert.ok(mockMonaco.providers.codeAction, "CodeActionProvider should be registered");
    assert.ok(mockMonaco.providers.folding, "FoldingRangeProvider should be registered");
    assert.ok(mockMonaco.providers.definition, "DefinitionProvider should be registered");
    assert.ok(mockMonaco.themes["bos-dark"], "bos-dark theme should be registered");
    assert.ok(mockMonaco.themes["bos-light"], "bos-light theme should be registered");

    const metaKeywords = mockMonaco.providers.monarch.def.metaObjectKeywords;
    for (const kw of [
        "table", "tableextension", "tablelogic", "tablelogicextension",
        "statemachine", "statemachineextension", "codeunit", "interface",
        "enum", "enumextension", "permissionset", "view", "query",
        "page", "pageextension", "report", "custommetadata"
    ]) {
        assert.ok(metaKeywords.includes(kw), `Monarch grammar must include '${kw}'`);
    }
});

test("provides context-aware completions for ctx, models, rec, decorators, and live catalog", () => {
    const mockMonaco = createMockMonaco();
    registerBosScriptLanguage(mockMonaco);

    updateBosScriptCatalogContext({
        tables: [{ name: "Customer", fields: [{ name: "credit_limit", data_type: "Decimal(18,2)" }] }],
        enums: [{ name: "ApprovalStatus", values: ["Draft", "Approved", "Rejected"] }]
    });
    registerTableSchemaForIntelliSense("SalesHeader", [
        { name: "order_no", data_type: "String(20)", is_primary_key: true },
        { name: "total_amount", data_type: "Decimal(18,2)", is_flow_field: true }
    ]);

    // 1. Test `ctx.` completion (and verify zero duplicate providers or labels across repeated registrations)
    registerBosScriptLanguage(mockMonaco);
    setupMonacoEnvironment(mockMonaco);
    assert.equal(mockMonaco.providers.activeCompletionProviders.length, 1, "Must only keep 1 active CompletionItemProvider across repeated HMR/registrations");

    const ctxModel = createMockModel("codeunit Test {\n    procedure run() {\n        ctx.\n    }\n}");
    const ctxRes = mockMonaco.providers.completion.provideCompletionItems(ctxModel, { lineNumber: 3, column: 13 });
    const ctxLabels = ctxRes.suggestions.map((s) => s.label);
    assert.equal(new Set(ctxLabels).size, ctxLabels.length, "ctx. completions must contain zero duplicate labels");
    assert.ok(ctxLabels.includes("db"));
    assert.ok(ctxLabels.includes("ui"));
    assert.ok(ctxLabels.includes("events"));
    assert.ok(ctxLabels.includes("crypto"));
    assert.ok(ctxLabels.includes("audit"));
    assert.ok(!ctxLabels.includes("numberSeries"));

    // 2. Test `ctx.ui.` completion
    const uiModel = createMockModel("codeunit Test {\n    procedure run() {\n        ctx.ui.\n    }\n}");
    const uiRes = mockMonaco.providers.completion.provideCompletionItems(uiModel, { lineNumber: 3, column: 16 });
    const uiLabels = uiRes.suggestions.map((s) => s.label);
    assert.equal(new Set(uiLabels).size, uiLabels.length, "ctx.ui. completions must contain zero duplicate labels");
    assert.ok(uiLabels.includes("toast"));
    assert.ok(uiLabels.includes("confirm"));
    assert.ok(uiLabels.includes("message"));
    assert.ok(uiLabels.includes("success"));
    assert.ok(uiLabels.includes("warning"));
    assert.ok(uiLabels.includes("error"));

    // 3. Test `models.` completion (ONLY includes live catalog Customer & SalesHeader, no non-existent tables)
    const modelsModel = createMockModel("codeunit Test {\n    procedure run() {\n        models.\n    }\n}");
    const modelsRes = mockMonaco.providers.completion.provideCompletionItems(modelsModel, { lineNumber: 3, column: 16 });
    const modelLabels = modelsRes.suggestions.map((s) => s.label);
    assert.deepEqual(modelLabels, ["Customer", "SalesHeader"]);
    assert.ok(!modelLabels.includes("CustomerLedgerEntry"));
    assert.ok(!modelLabels.includes("GeneralLedgerEntry"));
    assert.ok(!modelLabels.includes("Vendor"));

    // 4. Test `rec.` completion inside a table with local unsaved fields + catalog fields
    const recSource = [
        "table SalesHeader {",
        "    custom_discount: Decimal(18, 2);",
        "    onInsert(rec) {",
        "        rec.",
        "    }",
        "}"
    ].join("\n");
    const recModel = createMockModel(recSource);
    const recRes = mockMonaco.providers.completion.provideCompletionItems(recModel, { lineNumber: 4, column: 13 });
    const recLabels = recRes.suggestions.map((s) => s.label);
    assert.ok(recLabels.includes("custom_discount"), "Should include local unsaved field custom_discount");
    assert.ok(recLabels.includes("order_no"), "Should include catalog field order_no");
    assert.ok(recLabels.includes("total_amount"), "Should include catalog FlowField total_amount");

    // 5. Test `@` decorator completion
    const decModel = createMockModel("codeunit Test {\n    @b\n}");
    const decRes = mockMonaco.providers.completion.provideCompletionItems(decModel, { lineNumber: 2, column: 7 });
    const decLabels = decRes.suggestions.map((s) => s.label);
    assert.ok(decLabels.includes("@businessEvent"));
    assert.ok(decLabels.includes("@subscribeEvent"));
    assert.ok(decLabels.includes("@pushdown"));
});

test("provides hover documentation, signature help, document symbols, formatting, and quick-fix code actions", async () => {
    const mockMonaco = createMockMonaco();
    registerBosScriptLanguage(mockMonaco);

    const sampleSource = [
        "table InvoiceHeader {",
        "tableType: Document;",
        "id: UUID notNull;",
        "amount: Decimal(18, 2);",
        "onInsert(rec) {",
        "ctx.ui.toast(\"Created\", \"info\");",
        "}",
        "}"
    ].join("\n");
    const model = createMockModel(sampleSource);

    // 1. Hover on `table`
    const hoverRes = mockMonaco.providers.hover.provideHover(model, { lineNumber: 1, column: 3 });
    assert.ok(hoverRes, "Hover on 'table' should return documentation");
    assert.ok(hoverRes.contents[0].value.includes("table"));

    // 2. Signature help on `Decimal(18, `
    const sigModel = createMockModel("table T {\n    amt: Decimal(18, \n}");
    const sigRes = mockMonaco.providers.signatureHelp.provideSignatureHelp(sigModel, { lineNumber: 2, column: 22 });
    assert.ok(sigRes, "SignatureHelp on Decimal(18, should return parameter hints");
    assert.equal(sigRes.value.activeParameter, 1);

    // 3. Document Symbols
    const syms = mockMonaco.providers.documentSymbol.provideDocumentSymbols(model);
    assert.equal(syms.length, 1);
    assert.equal(syms[0].name, "InvoiceHeader");
    assert.ok(syms[0].children.length >= 3);

    // 4. Formatter
    const formatted = await formatBosScriptSource(sampleSource);
    assert.ok(formatted.includes("    tableType: Document;"), "Should indent body by 4 spaces");
    assert.ok(formatted.includes("        ctx.ui.toast"), "Should indent nested trigger body by 8 spaces");

    // 5. Code Actions (Quick Fix for BOS2002 missing primaryKey)
    const actionsRes = mockMonaco.providers.codeAction.provideCodeActions(
        model,
        null,
        {
            markers: [
                {
                    code: "BOS2002",
                    message: "Table 'InvoiceHeader' does not declare a `primaryKey(...)`",
                    startLineNumber: 1,
                    startColumn: 1
                }
            ]
        }
    );
    assert.ok(actionsRes.actions.length >= 1, "Should provide quick-fix action for BOS2002");
    assert.ok(actionsRes.actions[0].title.includes("primaryKey"));
});

test("transpiles BOS-Script codeunit line-for-line into Shadow TypeScript and parses compiler error positions", () => {
    const bosCodeunit = [
        "codeunit SalesPostingService {",
        "    @businessEvent",
        "    async procedure postOrder(rec: SalesHeader): Promise<boolean> {",
        "        const ok = true;",
        "        return ok;",
        "    }",
        "}"
    ].join("\n");

    const shadowTs = transpileBosToShadowTypeScript(bosCodeunit);
    assert.ok(shadowTs, "Should transpile codeunit to shadow TypeScript");
    const bosLines = bosCodeunit.split("\n");
    const tsLines = shadowTs.split("\n");
    assert.equal(tsLines.length, bosLines.length, "Shadow TypeScript must preserve exact line count");
    assert.ok(tsLines[0].includes("class SalesPostingService"));
    assert.ok(tsLines[1].trim().startsWith("// @businessEvent"));
    assert.ok(tsLines[2].includes("async postOrder(rec: SalesHeader): Promise<boolean>"));

    // Verify error position parser handles both human and Rust Debug formats
    const posHuman = parseCompilerErrorPosition("Parser error at line 14, column 9: Unexpected token");
    assert.equal(posHuman.startLineNumber, 14);
    assert.equal(posHuman.startColumn, 9);

    const posDebug = parseCompilerErrorPosition('UnexpectedToken { expected: "Semicolon", line: 7, col: 22 }');
    assert.equal(posDebug.startLineNumber, 7);
    assert.equal(posDebug.startColumn, 22);
});

test("infers local model/record variables, hydrates from ambient.d.ts, and autocompletes model properties & filter queries without snippet leakage", () => {
    const mockMonaco = createMockMonaco();
    registerBosScriptLanguage(mockMonaco);

    updateBosScriptCatalogContext({
        tables: [{ name: "Note", fields: [] }],
        replace: true
    });

    hydrateCatalogFromAmbientDts(`
declare interface NoteRecord {
    /** id (UUID) */
    id: UUID;
    /** title (VARCHAR(200)) */
    title: string;
    /** content (TEXT) */
    content?: string;
    /** status (VARCHAR(30)) */
    status?: string;
    testField(field: string, expectedValue?: any): void;
}
`);

    // 1. `var note` / `note.` inside `tablelogic NoteLogic for Note` should show Note's fields & methods, NEVER `pageextension (Meta-Object Snippet)`
    const noteLogicSrc = [
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec) {",
        "        var note",
        "        note.",
        "    }",
        "}"
    ].join("\n");
    const noteModel = createMockModel(noteLogicSrc);
    const noteDotRes = mockMonaco.providers.completion.provideCompletionItems(noteModel, { lineNumber: 4, column: 14 });
    const noteDotLabels = noteDotRes.suggestions.map((s) => s.label);
    assert.ok(noteDotLabels.includes("title"), "note. should suggest Note field 'title'");
    assert.ok(noteDotLabels.includes("content"), "note. should suggest Note field 'content'");
    assert.ok(noteDotLabels.includes("findMany"), "note. should suggest TableModel method 'findMany'");
    assert.ok(!noteDotLabels.includes("pageextension (Meta-Object Snippet)"), "note. must NEVER leak top-level Meta-Object Snippets");

    // 2. `const found = models.Note.first(...)` -> `found.` should suggest NoteRecord fields & record helpers
    const recordVarSrc = [
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec) {",
        "        const found = models.Note.first({ where: { status: \"Draft\" } });",
        "        found.",
        "    }",
        "}"
    ].join("\n");
    const foundRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(recordVarSrc), { lineNumber: 4, column: 15 });
    const foundLabels = foundRes.suggestions.map((s) => s.label);
    assert.ok(foundLabels.includes("title"));
    assert.ok(foundLabels.includes("status"));
    assert.ok(foundLabels.includes("testField"));

    // 3. Filter completions inside `models.Note.findMany({ where: { ` and `where: { title: { `
    const whereFieldSrc = [
        "codeunit NoteSearch {",
        "    procedure search() {",
        "        const list = models.Note.findMany({ where: { ",
        "    }",
        "}"
    ].join("\n");
    const whereFieldRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(whereFieldSrc), { lineNumber: 3, column: 54 });
    const whereFieldLabels = whereFieldRes.suggestions.map((s) => s.label);
    assert.ok(whereFieldLabels.includes("title"), "where: { should suggest table field 'title'");
    assert.ok(whereFieldLabels.includes("status"), "where: { should suggest table field 'status'");
    assert.ok(whereFieldLabels.includes("OR"), "where: { should suggest logical 'OR'");

    const whereOpSrc = [
        "codeunit NoteSearch {",
        "    procedure search() {",
        "        const list = models.Note.findMany({ where: { title: { ",
        "    }",
        "}"
    ].join("\n");
    const whereOpRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(whereOpSrc), { lineNumber: 3, column: 63 });
    const whereOpLabels = whereOpRes.suggestions.map((s) => s.label);
    assert.ok(whereOpLabels.includes("contains"), "Operator filter should suggest 'contains'");
    assert.ok(whereOpLabels.includes("eq"), "Operator filter should suggest 'eq'");
    assert.ok(whereOpLabels.includes("gt"), "Operator filter should suggest 'gt'");
    assert.ok(whereOpLabels.includes("in"), "Operator filter should suggest 'in'");

    // 4. After closing a nested operator brace `where: { title: { contains: "x" }, `, remaining fields (`content`, `status`) must still autocomplete!
    const afterNestedOpSrc = [
        "codeunit NoteSearch {",
        "    procedure search() {",
        '        const list = models.Note.findMany({ where: { title: { contains: "urgent" }, ',
        "    }",
        "}"
    ].join("\n");
    const afterNestedRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(afterNestedOpSrc), { lineNumber: 3, column: 85 });
    const afterNestedLabels = afterNestedRes.suggestions.map((s) => s.label);
    assert.ok(afterNestedLabels.includes("content"), "Should suggest remaining field 'content' after closed operator brace");
    assert.ok(afterNestedLabels.includes("status"), "Should suggest remaining field 'status' after closed operator brace");
    assert.ok(!afterNestedLabels.includes("title"), "Should deduplicate already-defined field 'title'");

    // 5. Inside `OR: [{ ` logical array -> should suggest table fields
    const orArraySrc = [
        "codeunit NoteSearch {",
        "    procedure search() {",
        "        const list = models.Note.findMany({ where: { OR: [{ ",
        "    }",
        "}"
    ].join("\n");
    const orArrayRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(orArraySrc), { lineNumber: 3, column: 61 });
    const orArrayLabels = orArrayRes.suggestions.map((s) => s.label);
    assert.ok(orArrayLabels.includes("title"), "Inside OR: [{ should suggest 'title'");
    assert.ok(orArrayLabels.includes("status"), "Inside OR: [{ should suggest 'status'");

    // 6. Inside `select: ["` array -> should suggest table column names
    const selectArrSrc = [
        "codeunit NoteSearch {",
        "    procedure search() {",
        '        const list = models.Note.findMany({ select: ["',
        "    }",
        "}"
    ].join("\n");
    const selectArrRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(selectArrSrc), { lineNumber: 3, column: 55 });
    const selectArrLabels = selectArrRes.suggestions.map((s) => s.label);
    assert.ok(selectArrLabels.includes("title"), "Inside select: [\" should suggest 'title'");
    assert.ok(selectArrLabels.includes("content"), "Inside select: [\" should suggest 'content'");

    // 7. Inside `upsert({ ` -> depth 1 suggests `where`, `create`, `update`; depth 2 inside `create: { ` suggests fields + Fill-all snippet
    const upsertTopSrc = [
        "codeunit NoteUpsert {",
        "    procedure sync() {",
        "        models.Note.upsert({ ",
        "    }",
        "}"
    ].join("\n");
    const upsertTopRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(upsertTopSrc), { lineNumber: 3, column: 30 });
    const upsertTopLabels = upsertTopRes.suggestions.map((s) => s.label);
    assert.ok(upsertTopLabels.includes("where"), "upsert top-level should suggest 'where'");
    assert.ok(upsertTopLabels.includes("create"), "upsert top-level should suggest 'create'");
    assert.ok(upsertTopLabels.includes("update"), "upsert top-level should suggest 'update'");

    const upsertCreateSrc = [
        "codeunit NoteUpsert {",
        "    procedure sync() {",
        '        models.Note.upsert({ where: { id: "1" }, create: { ',
        "    }",
        "}"
    ].join("\n");
    const upsertCreateRes = mockMonaco.providers.completion.provideCompletionItems(createMockModel(upsertCreateSrc), { lineNumber: 3, column: 60 });
    const upsertCreateLabels = upsertCreateRes.suggestions.map((s) => s.label);
    assert.ok(upsertCreateLabels.some((l) => l.startsWith("Fill all Note fields")), "Mutation object should offer 1-click Fill all fields snippet");
    assert.ok(upsertCreateLabels.includes("title"), "upsert create: { should suggest 'title'");

    // 8. String-literal field completion inside `rec.testField("` and declarative `validate ` inside `tablelogic`
    const testFieldStrSrc = [
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec) {",
        '        rec.testField("',
        "    }",
        "    validate ",
        "}"
    ].join("\n");
    const tfModel = createMockModel(testFieldStrSrc);
    const tfRes = mockMonaco.providers.completion.provideCompletionItems(tfModel, { lineNumber: 3, column: 24 });
    const tfLabels = tfRes.suggestions.map((s) => s.label);
    assert.ok(tfLabels.includes("title"), 'rec.testField(" should suggest field "title"');
    assert.ok(tfLabels.includes("status"), 'rec.testField(" should suggest field "status"');

    const valDeclRes = mockMonaco.providers.completion.provideCompletionItems(tfModel, { lineNumber: 5, column: 14 });
    const valDeclLabels = valDeclRes.suggestions.map((s) => s.label);
    assert.ok(valDeclLabels.includes("title"), "validate <field> in tablelogic should suggest field 'title'");

    // 9. Quick-Fix for BOS2014 unknown field typo
    const quickFixRes = mockMonaco.providers.codeAction.provideCodeActions(
        tfModel,
        null,
        {
            markers: [
                {
                    code: "BOS2014",
                    message: "Field `titl` does not exist on table `Note`. — 💡 Did you mean `title`?",
                    startLineNumber: 3,
                    startColumn: 13,
                    endLineNumber: 3,
                    endColumn: 17
                }
            ]
        }
    );
    assert.ok(quickFixRes.actions.some((a) => a.title === "Replace with 'title'"), "Should provide 1-click Quick Fix for BOS2014 field typo");

    // 10. Primitive string / BosDecimal / field-access variable inference (`const text = rec.content || ""` -> `text.` suggests `trim`, NOT `calcFields`/`findMany`)
    const primVarSrc = [
        "table Note {",
        "    code: String(20) { notNull = true; };",
        "    content: String(100);",
        "    amount: Decimal(18, 2);",
        "    primaryKey(code);",
        "}",
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec, ctx) {",
        '        const text = rec.content || "";',
        "        rec.content = text.",
        "        const amt = rec.amount;",
        "        amt.",
        "        rec.content.",
        "    }",
        "}"
    ].join("\n");
    const primModel = createMockModel(primVarSrc);

    const textDotRes = mockMonaco.providers.completion.provideCompletionItems(primModel, { lineNumber: 10, column: 28 });
    const textDotLabels = textDotRes.suggestions.map((s) => s.label);
    assert.ok(textDotLabels.includes("trim"), "`text.` (from `rec.content || \"\"`) must suggest String `trim`");
    assert.ok(textDotLabels.includes("toLowerCase"), "`text.` must suggest String `toLowerCase`");
    assert.ok(textDotLabels.includes("includes"), "`text.` must suggest String `includes`");
    assert.ok(!textDotLabels.includes("calcFields"), "`text.` must NOT suggest Note record helper `calcFields`");
    assert.ok(!textDotLabels.includes("findMany"), "`text.` must NOT suggest Note TableModel method `findMany`");
    assert.ok(!textDotLabels.includes("code"), "`text.` must NOT suggest Note column `code`");

    const amtDotRes = mockMonaco.providers.completion.provideCompletionItems(primModel, { lineNumber: 12, column: 13 });
    const amtDotLabels = amtDotRes.suggestions.map((s) => s.label);
    assert.ok(amtDotLabels.includes("add"), "`amt.` (from `Decimal` field) must suggest BosDecimal `add`");
    assert.ok(amtDotLabels.includes("round"), "`amt.` must suggest BosDecimal `round`");
    assert.ok(!amtDotLabels.includes("findMany"), "`amt.` must NOT suggest TableModel `findMany`");

    const chainedFieldDotRes = mockMonaco.providers.completion.provideCompletionItems(primModel, { lineNumber: 13, column: 21 });
    const chainedFieldDotLabels = chainedFieldDotRes.suggestions.map((s) => s.label);
    assert.ok(chainedFieldDotLabels.includes("trim"), "`rec.content.` must suggest String `trim`");
    assert.ok(!chainedFieldDotLabels.includes("calcFields"), "`rec.content.` must NOT suggest `calcFields`");

    // 11. TypeScript local variable type annotations (`let dt: `) & procedural body completions must suggest TS primitives (`string`, `number`, `boolean`, `Decimal`) and NEVER top-level `codeunit` Meta-Object snippets!
    const tsTypedDeclSrc = [
        "table Note {",
        "    code: String(20) { notNull = true; };",
        "    content: String(100);",
        "    primaryKey(code);",
        "}",
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec, ctx) {",
        '        const text = rec.content || "";',
        "        let dt: ",
        "        let dt2: string;",
        "        dt2.",
        "        ",
        "    }",
        "}"
    ].join("\n");
    const tsTypedModel = createMockModel(tsTypedDeclSrc);

    const letColonRes = mockMonaco.providers.completion.provideCompletionItems(tsTypedModel, { lineNumber: 9, column: 17 });
    const letColonLabels = letColonRes.suggestions.map((s) => s.label);
    assert.ok(letColonLabels.includes("string"), "`let dt: ` must suggest native TS `string`");
    assert.ok(letColonLabels.includes("number"), "`let dt: ` must suggest native TS `number`");
    assert.ok(letColonLabels.includes("boolean"), "`let dt: ` must suggest native TS `boolean`");
    assert.ok(letColonLabels.includes("Decimal"), "`let dt: ` must suggest `Decimal`");
    assert.ok(letColonLabels.includes("NoteRecord"), "`let dt: ` must suggest `NoteRecord`");
    assert.ok(
        !letColonLabels.some((l) => l.includes("Meta-Object Snippet")),
        "`let dt: ` must NEVER suggest `codeunit (Meta-Object Snippet)` or other top-level Meta-Object snippets!"
    );

    const dt2DotRes = mockMonaco.providers.completion.provideCompletionItems(tsTypedModel, { lineNumber: 11, column: 13 });
    const dt2DotLabels = dt2DotRes.suggestions.map((s) => s.label);
    assert.ok(dt2DotLabels.includes("trim"), "`let dt2: string; dt2.` must suggest String `trim`");
    assert.ok(!dt2DotLabels.includes("findMany"), "`dt2.` must NOT suggest TableModel `findMany`");

    const procBodyRes = mockMonaco.providers.completion.provideCompletionItems(tsTypedModel, { lineNumber: 12, column: 9 });
    const procBodyLabels = procBodyRes.suggestions.map((s) => s.label);
    assert.ok(procBodyLabels.includes("rec"), "Procedural body must suggest `rec`");
    assert.ok(procBodyLabels.includes("ctx"), "Procedural body must suggest `ctx`");
    assert.ok(procBodyLabels.includes("text"), "Procedural body must suggest declared local variable `text`");
    assert.ok(
        !procBodyLabels.some((l) => l.includes("Meta-Object Snippet")),
        "Inside `tablelogic` procedural body must NEVER suggest top-level `codeunit (Meta-Object Snippet)`"
    );

    // 12. Shadow TypeScript transpiler comments out declarative `table` blocks line-for-line so combined `table + tablelogic` buffers get clean TS validation
    const combinedShadowTs = transpileBosToShadowTypeScript(tsTypedDeclSrc);
    assert.ok(combinedShadowTs.startsWith("// table Note {"), "Declarative `table` header must be commented out in shadow TS");
    assert.ok(combinedShadowTs.includes("class NoteLogic {"), "`tablelogic` header must become `class NoteLogic {` on exact line");
    assert.ok(combinedShadowTs.includes("onInsert(rec: NoteRecord, ctx: any)"), "`onInsert(rec, ctx)` must be typed in shadow TS");
    assert.equal(combinedShadowTs.split("\n").length, tsTypedDeclSrc.split("\n").length, "Line count must be 100% preserved");

    // 13. Constructor completions after `new ` (`let test: Date = new `, `let test: Note = new `) must suggest `Date` / `Note` first and NEVER `rec`, `xRec`, `ctx`, `const`, `for...of`
    const newExprSrc = [
        "table Note {",
        "    code: String(20) { notNull = true; };",
        "    content: String(100);",
        "    primaryKey(code);",
        "}",
        "tablelogic NoteLogic for Note {",
        "    onInsert(rec, ctx) {",
        "        let testDate: Date = new ",
        "        let testNote: Note = new ",
        "        testDate.",
        "        testNote.",
        "    }",
        "}"
    ].join("\n");
    const newExprModel = createMockModel(newExprSrc);

    const newDateRes = mockMonaco.providers.completion.provideCompletionItems(newExprModel, { lineNumber: 8, column: 34 });
    const newDateLabels = newDateRes.suggestions.map((s) => s.label);
    assert.ok(newDateLabels.includes("Date"), "`let testDate: Date = new ` must suggest `Date` constructor");
    assert.ok(!newDateLabels.includes("rec"), "`new ` must NOT suggest `rec`");
    assert.ok(!newDateLabels.includes("xRec"), "`new ` must NOT suggest `xRec`");
    assert.ok(!newDateLabels.includes("for...of"), "`new ` must NOT suggest `for...of`");
    const dateItem = newDateRes.suggestions.find((s) => s.label === "Date");
    assert.equal(dateItem.preselect, true, "`Date` must be preselected when variable is typed `: Date`");

    const newNoteRes = mockMonaco.providers.completion.provideCompletionItems(newExprModel, { lineNumber: 9, column: 34 });
    const newNoteLabels = newNoteRes.suggestions.map((s) => s.label);
    assert.ok(newNoteLabels.includes("Note"), "`let testNote: Note = new ` must suggest `Note` constructor");
    assert.ok(!newNoteLabels.includes("rec"), "`new ` must NOT suggest `rec`");
    const noteCtorItem = newNoteRes.suggestions.find((s) => s.label === "Note");
    assert.equal(noteCtorItem.preselect, true, "`Note` must be preselected when variable is typed `: Note`");

    const testDateDotRes = mockMonaco.providers.completion.provideCompletionItems(newExprModel, { lineNumber: 10, column: 18 });
    const testDateDotLabels = testDateDotRes.suggestions.map((s) => s.label);
    assert.ok(testDateDotLabels.includes("toISOString"), "`testDate.` (`: Date`) must suggest `toISOString`");
    assert.ok(testDateDotLabels.includes("getFullYear"), "`testDate.` (`: Date`) must suggest `getFullYear`");

    const testNoteDotRes = mockMonaco.providers.completion.provideCompletionItems(newExprModel, { lineNumber: 11, column: 18 });
    const testNoteDotLabels = testNoteDotRes.suggestions.map((s) => s.label);
    assert.ok(testNoteDotLabels.includes("code"), "`testNote.` (`: Note`) must suggest `code` field");
    assert.ok(testNoteDotLabels.includes("content"), "`testNote.` (`: Note`) must suggest `content` field");
    assert.ok(testNoteDotLabels.includes("save"), "`testNote.` (`: Note`) must suggest `.save()` record helper");
});
