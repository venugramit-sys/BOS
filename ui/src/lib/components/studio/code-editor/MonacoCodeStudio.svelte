<script>
  /**
   * MonacoCodeStudio.svelte (Svelte 5 Runes)
   * Full-fledged Enterprise IDE Code Studio for BOS Meta-Objects, BOS-Script, and TypeScript.
   * Features:
   * - Context-aware IntelliSense (all 14 Meta-Objects, live Catalog Tables/Fields/Enums, ctx, models, Rec/xRec)
   * - Real-time Grammar, Semantic & Embedded TypeScript checking (Rust `bosc` + Monaco TypeScript Shadow Worker)
   * - Interactive Problems & Diagnostics Panel with click-to-jump and Quick Fixes (Ctrl+.)
   * - Live Symbol Outline Navigation, Code Formatting (Shift+Alt+F), and Snippet Insertion
   * - Side-by-side PostgreSQL 18.4 DDL, Intermediate Representation (IR), and Live Ambient .d.ts inspection
   */
  import { onMount, onDestroy } from "svelte";
  import * as monaco from "monaco-editor";
  import editorWorker from "monaco-editor/editor/editor.worker.js?worker";
  import tsWorker from "monaco-editor/language/typescript/ts.worker.js?worker";
  import jsonWorker from "monaco-editor/language/json/json.worker.js?worker";
  import {
    Play,
    Terminal,
    Sparkles,
    AlertTriangle,
    AlertCircle,
    CheckCircle2,
    Code2,
    Search,
    Wand2,
    ListTree,
    ChevronDown,
    ChevronUp,
    RefreshCw,
    WrapText,
    Map as MapIcon,
    Lightbulb,
  } from "lucide-svelte";
  import { Button, Tabs, Toolbar, Banner } from "$lib/bos-ui/index.js";
  import {
    setupMonacoEnvironment,
    setupLiveDiagnosticMarkers,
    syncAmbientDts,
    getLastAmbientDts,
  } from "./monacoLspProvider.js";
  import {
    updateBosScriptCatalogContext,
    registerTableSchemaForIntelliSense,
    parseDocumentSymbolsAndContext,
    formatBosScriptSource,
    formatTypeScriptSource,
    setBosScriptOpenObjectHandler,
  } from "./monacoBosScript.js";
  import { studioStore } from "../stores/studioStore.svelte.js";
  import { StudioApi } from "../stores/studioApi.js";
  import CodeunitRunnerPanel from "./CodeunitRunnerPanel.svelte";

  let {
    content = $bindable(""),
    objectName = "",
    objectType = "table",
    packageName = "bos.app",
    theme = "dark",
    focusLine = 0,
    onApplySuccess = null,
    onDesign = null,
    onChange = null,
  } = $props();

  let editorContainer = $state(null);
  let editorInstance = null;
  let validatorController = null;
  let isInternalEdit = false;
  let isFormattingSilent = false;

  let activeTab = $state("source"); // 'source' | 'ddl' | 'ir' | 'dts'
  let isRunnerOpen = $state(false);
  let isProblemsOpen = $state(false);
  let selectedLanguage = $state("bos-script"); // 'bos-script' | 'typescript'
  let wordWrapEnabled = $state(false);
  let minimapEnabled = $state(true);

  // Cursor & Outline state
  let cursorLine = $state(1);
  let cursorColumn = $state(1);
  let selectionLength = $state(0);
  let documentSymbols = $state([]);
  let selectedOutlineLine = $state("");
  let selectedSnippetId = $state("");

  // Compilation & Diagnostics state
  let isCompiling = $state(false);
  let isApplying = $state(false);
  let actionNotice = $state(null);
  let compileStatus = $state("idle"); // 'idle' | 'compiling' | 'success' | 'error'
  let compileMessage = $state("");
  let compiledDdl = $state("");
  let compiledIr = $state(null);
  let diagnostics = $state([]);
  let ambientDtsText = $state("");
  let ambientTableCount = $state(0);

  let errorCount = $derived(diagnostics.filter((d) => d.severity === "error").length);
  let warningCount = $derived(diagnostics.filter((d) => d.severity === "warning").length);

  const QUICK_SNIPPETS = [
    {
      id: "page-group",
      label: "Page Group (Card Section)",
      code: '        group(Details) {\n            caption = "Details";\n            field(code) { caption = "Code"; }\n            field(name) { caption = "Name"; }\n        }\n',
    },
    {
      id: "page-field",
      label: "Page Field",
      code: '            field(status) {\n                caption = "Status";\n                editable = true;\n            }\n',
    },
    {
      id: "page-repeater",
      label: "Page Repeater (Grid Lines)",
      code: '        repeater(Lines) {\n            caption = "Lines";\n            field(line_no) { caption = "Line No"; }\n            field(description) { caption = "Description"; }\n            field(amount) { caption = "Amount"; }\n        }\n',
    },
    {
      id: "page-part",
      label: "Page Part / Factbox",
      code: '        part(FactBox, CustomerFactbox) {\n            caption = "Customer Details";\n            subFormLink = [customer_id == this.id];\n        }\n',
    },
    {
      id: "page-action",
      label: "Page Action",
      code: '        action(Post) {\n            caption = "Post Document";\n            trigger onAction() {\n                ctx.ui.toast("Posted", "success");\n            }\n        }\n',
    },
    {
      id: "ext-addlast",
      label: "Page Extension addLast",
      code: '        addLast(Content) {\n            group(CustomDetails) {\n                caption = "Custom Details";\n                field(custom_tag) { caption = "Custom Tag"; }\n            }\n        }\n',
    },
    {
      id: "ext-addafter",
      label: "Page Extension addAfter",
      code: '        addAfter(General) {\n            group(Audit) {\n                caption = "Audit Trail";\n                field(audit_note) { caption = "Audit Note"; }\n            }\n        }\n',
    },
    {
      id: "ext-modify",
      label: "Page Extension modify",
      code: '        modify(name) {\n            caption = "Legal Name";\n            editable = false;\n        }\n',
    },
    {
      id: "field",
      label: "Table Field (Column)",
      code: '    new_field: String(100) notNull default("");\n',
    },
    {
      id: "flowfield",
      label: "FlowField (Calculated Subquery)",
      code: "    total_amount: Decimal(18, 2) flowField(sum(SalesLine.line_amount where (document_id == this.id)));\n",
    },
    {
      id: "validate",
      label: "Logic companion field validation",
      code: '    validate amount(val) {\n        if (val < 0) {\n            error("Amount cannot be negative");\n        }\n    }\n',
    },
    {
      id: "triggers",
      label: "Logic companion triggers (onInsert / onModify)",
      code: '    onInsert(rec, ctx) {\n        ctx.ui.toast("Inserting record", "info");\n    }\n\n    onModify(rec, xRec, ctx) {\n        // Compare rec vs xRec\n    }\n',
    },
    {
      id: "procedure",
      label: "Codeunit Procedure",
      code: '    procedure processDocument(rec: any): boolean {\n        ctx.ui.toast("Processing " + rec.id, "info");\n        return true;\n    }\n',
    },
    {
      id: "businessevent",
      label: "@businessEvent Procedure",
      code: '    @businessEvent\n    procedure onAfterPost(rec: any) {\n        ctx.events.publish("document.posted", { id: rec.id });\n    }\n',
    },
    {
      id: "pushdown",
      label: "@pushdown AMDP Procedure (PL/pgSQL)",
      code: "    @pushdown\n    procedure recalculateTotals(customerId: UUID) {\n        UPDATE customer SET updated_at = NOW() WHERE id = customerId;\n    }\n",
    },
    {
      id: "transition",
      label: "State Machine Transition",
      code: "    transition release from Draft to Released {\n        guard(rec) {\n            return true;\n        }\n    }\n",
    },
  ];

  function configureWorkerEnvironment() {
    if (typeof self !== "undefined" && !self.MonacoEnvironment) {
      self.MonacoEnvironment = {
        getWorker(_, label) {
          if (label === "typescript" || label === "javascript") {
            return new tsWorker();
          }
          if (label === "json") {
            return new jsonWorker();
          }
          return new editorWorker();
        },
      };
    }
  }

  async function hydrateCatalogIntelliSense() {
    try {
      const cat = studioStore.catalog || [];
      const tables = cat
        .filter((o) => o.object_type === "table")
        .map((o) => ({ name: o.name || o.object_name, fields: o.fields || [] }));
      const enums = cat
        .filter((o) => o.object_type === "enum")
        .map((o) => ({ name: o.name || o.object_name, values: o.values || [] }));
      const codeunits = cat
        .filter((o) => o.object_type === "codeunit")
        .map((o) => ({ name: o.name || o.object_name, methods: o.methods || [] }));
      const interfaces = cat
        .filter((o) => o.object_type === "interface")
        .map((o) => ({ name: o.name || o.object_name, methods: o.methods || [] }));

      updateBosScriptCatalogContext({ tables, enums, codeunits, interfaces, replace: true });

      // Proactively fetch schema columns for all active catalog tables + target table for instant field & filter IntelliSense
      const docCtx = parseDocumentSymbolsAndContext(content || "");
      const targetTbl = docCtx.targetTable || (objectType === "table" ? objectName : null);
      const tableNamesToHydrate = new Set([
        ...tables.map((t) => t.name).filter(Boolean),
        ...(targetTbl ? [targetTbl] : []),
      ]);

      await Promise.all(
        Array.from(tableNamesToHydrate).map(async (tblName) => {
          try {
            const schema = await StudioApi.getTableSchema(tblName);
            if (schema && Array.isArray(schema.fields || schema.columns)) {
              registerTableSchemaForIntelliSense(tblName, schema.fields || schema.columns);
            }
          } catch {
            // Table may be a new unsaved draft
          }
        })
      );
    } catch {
      // Non-fatal
    }
  }

  function refreshOutline(sourceText) {
    const parsed = parseDocumentSymbolsAndContext(sourceText || "");
    // Live cross-tab schema sync: when editing a `table` or `tableextension`, immediately register its fields
    // into `liveWorkspaceCatalog` so switching to a `tablelogic` or `codeunit` tab has instant field completions!
    if (
      parsed.targetTable &&
      parsed.localFields.length > 0 &&
      parsed.currentMetaObject &&
      ["table", "tableextension"].includes(parsed.currentMetaObject.kind)
    ) {
      registerTableSchemaForIntelliSense(parsed.targetTable, parsed.localFields);
    }

    const flat = [];
    for (const sym of parsed.symbols) {
      flat.push({
        label: `${sym.kind}: ${sym.name}`,
        line: sym.line,
      });
      for (const ch of sym.children || []) {
        flat.push({
          label: `  ↳ ${ch.name} (${ch.detail || ch.kind})`,
          line: ch.line,
        });
      }
    }
    documentSymbols = flat;
  }

  onMount(() => {
    configureWorkerEnvironment();
    setupMonacoEnvironment(monaco);
    setBosScriptOpenObjectHandler((name, mode, type) => {
      studioStore.openObject(name, mode, type);
    });

    const selectedTheme = theme === "dark" ? "bos-dark" : "bos-light";
    const initialValue = content || getBoilerplate(objectType, objectName);
    if (!content) {
      content = initialValue;
    }

    editorInstance = monaco.editor.create(editorContainer, {
      value: initialValue,
      language: selectedLanguage,
      theme: selectedTheme,
      automaticLayout: true,
      minimap: { enabled: minimapEnabled },
      wordWrap: wordWrapEnabled ? "on" : "off",
      fontFamily: "'Fira Code', 'JetBrains Mono', Consolas, monospace",
      fontSize: 13,
      lineHeight: 21,
      renderWhitespace: "selection",
      scrollBeyondLastLine: false,
      tabSize: 4,
      insertSpaces: true,
      bracketPairColorization: { enabled: true },
      guides: {
        bracketPairs: true,
        indentation: true,
      },
      wordBasedSuggestions: "off",
      suggest: {
        showWords: false,
        showSnippets: true,
        showMethods: true,
        showFields: true,
        showClasses: true,
        preview: true,
      },
      quickSuggestions: {
        other: true,
        comments: false,
        strings: true,
      },
      parameterHints: {
        enabled: true,
      },
      formatOnPaste: true,
      lightbulb: {
        enabled: "on",
      },
    });

    refreshOutline(initialValue);
    void hydrateCatalogIntelliSense();
    // Ensure initial loaded script is cleanly formatted without marking the tab dirty
    void handleFormatDocument({ silent: true });

    // Sync ambient .d.ts and record table count
    syncAmbientDts(monaco).then(() => {
      ambientDtsText = getLastAmbientDts();
      const matches = ambientDtsText.match(/declare interface \w+Record/g);
      ambientTableCount = matches ? matches.length : 0;
    });

    // Set up live compiler & shadow TypeScript diagnostics
    validatorController = setupLiveDiagnosticMarkers(
      monaco,
      editorInstance,
      packageName,
      (stateUpdate) => {
        if (stateUpdate.isCompiling) {
          isCompiling = true;
          compileStatus = "compiling";
          return;
        }
        isCompiling = false;
        diagnostics = Array.isArray(stateUpdate.diagnostics) ? stateUpdate.diagnostics : [];
        compiledDdl = stateUpdate.ddl ?? compiledDdl;
        compiledIr = stateUpdate.ir ?? compiledIr;

        if (stateUpdate.isValid) {
          compileStatus = "success";
          compileMessage =
            warningCount > 0
              ? `Compiled with ${warningCount} warning(s).`
              : "Zero compilation errors. AST & PostgreSQL DDL synchronized.";
        } else {
          compileStatus = "error";
          compileMessage = stateUpdate.error || "Compilation failed with syntax or semantic errors.";
          isProblemsOpen = true;
        }
      }
    );

    // Content change listener
    editorInstance.onDidChangeModelContent(() => {
      const nextVal = editorInstance.getValue();
      const suppressDirty = isFormattingSilent;
      isInternalEdit = true;
      content = nextVal;
      if (onChange && !suppressDirty) onChange(nextVal);
      refreshOutline(nextVal);
      queueMicrotask(() => {
        isInternalEdit = false;
      });
    });

    // Cursor & selection listener
    editorInstance.onDidChangeCursorPosition((e) => {
      cursorLine = e.position.lineNumber;
      cursorColumn = e.position.column;
    });

    editorInstance.onDidChangeCursorSelection((e) => {
      const model = editorInstance.getModel();
      if (model) {
        selectionLength = model.getValueInRange(e.selection).length;
      }
    });

    // Keyboard shortcuts: F5 and Ctrl+S save. Compile only checks.
    editorInstance.addCommand(monaco.KeyCode.F5, () => {
      handleApplyToDatabase();
    });

    editorInstance.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
      handleApplyToDatabase();
    });

    editorInstance.addCommand(
      monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF,
      () => {
        handleFormatDocument();
      }
    );

    if (focusLine > 0) {
      jumpToLine(focusLine);
    }
  });

  onDestroy(() => {
    if (validatorController) {
      validatorController.dispose();
      validatorController = null;
    }
    if (editorInstance) {
      editorInstance.dispose();
      editorInstance = null;
    }
  });

  // Re-hydrate catalog IntelliSense whenever the studio catalog finishes loading or changes
  $effect(() => {
    const _catLen = studioStore.catalog?.length || 0;
    if (editorInstance) {
      void hydrateCatalogIntelliSense();
    }
  });

  // Sync external `content` updates (e.g. when async `loadObjectDetails` finishes or TableDesigner updates sourceCode)
  $effect(() => {
    const incoming = content;
    if (editorInstance && !isInternalEdit && typeof incoming === "string" && incoming.trim().length > 0) {
      const currentVal = editorInstance.getValue();
      if (incoming !== currentVal) {
        const pos = editorInstance.getPosition();
        isFormattingSilent = true;
        editorInstance.setValue(incoming);
        isFormattingSilent = false;
        if (pos) editorInstance.setPosition(pos);
        refreshOutline(incoming);
        void handleFormatDocument({ silent: true });
        if (validatorController) {
          validatorController.triggerValidation();
        }
      }
    }
  });

  // Theme reactive effect
  $effect(() => {
    if (editorInstance) {
      monaco.editor.setTheme(theme === "dark" ? "bos-dark" : "bos-light");
    }
  });

  // Focus line reactive effect
  $effect(() => {
    if (editorInstance && focusLine > 0) {
      jumpToLine(focusLine);
    }
  });

  function getDefaultCatalogTable(fallback = "Record", pkg = packageName) {
    const cat = studioStore.catalog || [];
    const pkgTbl = cat.find((o) => o.object_type === "table" && (o.package_name === pkg || o.package === pkg));
    if (pkgTbl) return pkgTbl.name || pkgTbl.object_name;
    const firstTbl = cat.find((o) => o.object_type === "table");
    return firstTbl?.name || firstTbl?.object_name || fallback;
  }

  function getDefaultCatalogPage(fallback = "CustomerCard", pkg = packageName) {
    const cat = studioStore.catalog || [];
    const pkgPage = cat.find((o) => o.object_type === "page" && (o.package_name === pkg || o.package === pkg));
    if (pkgPage) return pkgPage.name || pkgPage.object_name;
    const firstPage = cat.find((o) => o.object_type === "page");
    return firstPage?.name || firstPage?.object_name || fallback;
  }

  function getDefaultCatalogEnum(fallback = "OrderStatus", pkg = packageName) {
    const cat = studioStore.catalog || [];
    const pkgEnum = cat.find((o) => o.object_type === "enum" && (o.package_name === pkg || o.package === pkg));
    if (pkgEnum) return pkgEnum.name || pkgEnum.object_name;
    const firstEnum = cat.find((o) => o.object_type === "enum");
    return firstEnum?.name || firstEnum?.object_name || fallback;
  }

  function getBoilerplate(type, name) {
    const safeName = (name || "NewObject").replace(/[^A-Za-z0-9_]/g, "");
    const activeTbl = getDefaultCatalogTable(safeName);
    const activeTableSql = activeTbl.toLowerCase();
    switch (type) {
      case "codeunit":
        return `codeunit ${safeName} {\n    @businessEvent\n    procedure execute(rec: any): boolean {\n        // Transactional business logic running inside QuickJS / PL/pgSQL\n        ctx.ui.toast("Executed ${safeName}", "info");\n        return true;\n    }\n}\n`;
      case "statemachine":
      case "statemachineextension":
        return `statemachine ${safeName} for ${activeTbl}.status {\n    initial state Draft {\n        allowModify: true;\n        allowDelete: true;\n    }\n    state PendingApproval {\n        allowModify: false;\n        allowDelete: false;\n    }\n    state Released {\n        allowModify: false;\n        allowDelete: false;\n    }\n\n    transition submit from Draft to PendingApproval {}\n    transition approve from PendingApproval to Released {}\n}\n`;
      case "page":
        return `page ${safeName} for ${activeTbl} {\n    pageType = Card;\n    caption = "${safeName}";\n\n    layout {\n        area(Content) {\n            group(General) {\n                caption = "General";\n                field(code) { caption = "Code"; }\n                field(name) { caption = "Name"; }\n            }\n        }\n    }\n    actions {\n        area(Processing) {\n            action(Approve) {\n                caption = "Approve";\n            }\n        }\n    }\n}\n`;
      case "pageextension":
        const targetPage = getDefaultCatalogPage("VendorCard");
        return `pageextension ${safeName} extends ${targetPage} {\n    layout {\n        addLast(Content) {\n            group(Audit) {\n                caption = "Audit and Compliance";\n                field(audit_status) { caption = "Audit Status"; }\n            }\n        }\n    }\n    actions {\n        addLast(Processing) {\n            action(AuditCheck) {\n                caption = "Run Audit Check";\n            }\n        }\n    }\n}\n`;
      case "query":
        return `query ${safeName} {\n    sourceTable: ${activeTbl};\n    columns {\n        column(id, id);\n        column(name, name);\n    }\n}\n`;
      case "view":
        return `view ${safeName} {\n    SELECT id FROM ${activeTableSql}\n}\n`;
      case "enum":
        return `enum ${safeName} {\n    extensible: true;\n    value(0, Draft);\n    value(1, Active);\n    value(2, Archived);\n}\n`;
      case "enumextension":
        const targetEnum = getDefaultCatalogEnum("OrderStatus");
        return `enumextension ${safeName} extends ${targetEnum} {\n    value(10, CustomPending);\n    value(11, CustomVerified);\n}\n`;
      case "interface":
        return `interface ${safeName} {\n    procedure calculate(amount: Decimal): Decimal;\n}\n`;
      case "permissionset":
        return `permissionset ${safeName} {\n    caption: "${safeName}";\n    tabledata ${activeTbl} = RIMD;\n}\n`;
      case "report":
        return `report ${safeName} {\n    caption: "${safeName}";\n    dataset: ${activeTbl};\n}\n`;
      case "custommetadata":
        return `custommetadata ${safeName} {\n    setting_key: String(50) notNull;\n    setting_value: String(250);\n}\n`;
      case "tableextension":
        return `tableextension ${safeName} extends ${activeTbl} {\n    custom_tier: String(30) default("Standard");\n}\n`;
      case "tablelogic":
      case "tablelogicextension":
        return `tablelogic ${safeName} for ${activeTbl} {\n    onInsert(rec) {\n        ctx.ui.toast("${activeTbl} created", "info");\n    }\n}\n`;
      default:
        return `table ${safeName} {\n    tableType: Master;\n    extensible: true;\n\n    id: UUID notNull;\n    code: String(30) notNull;\n    name: String(100) notNull;\n    status: String(20) default("Active");\n\n    primaryKey(id);\n    key(idx_code, code);\n}\n`;
    }
  }

  async function triggerCompileImmediately() {
    await handleFormatDocument();
    if (validatorController) {
      validatorController.triggerValidation();
    }
    actionNotice = {
      tone: "info",
      title: "Checked, not saved",
      message: "Compile only checks this script. The event stays in the editor until you Apply.",
    };
  }

  function serverMessage(data, fallback) {
    if (typeof data?.message === "string" && data.message.trim()) return data.message;
    if (typeof data?.error === "string" && data.error.trim()) return data.error;
    return fallback;
  }

  async function handleFormatDocument(options = {}) {
    if (!editorInstance) return "";
    const model = editorInstance.getModel();
    if (!model) return "";
    const language = model.getLanguageId();
    const original = model.getValue();
    const formatted = language === "bos-script"
      ? await formatBosScriptSource(original)
      : await formatTypeScriptSource(original, language === "javascript" ? "babel" : "typescript");
    if (formatted === original) return original;
    const position = editorInstance.getPosition();
    if (options.silent) isFormattingSilent = true;
    try {
      editorInstance.executeEdits("prettier", [
        {
          range: model.getFullModelRange(),
          text: formatted,
          forceMoveMarkers: true,
        },
      ]);
      if (position) editorInstance.setPosition(position);
      editorInstance.pushUndoStop();
    } finally {
      if (options.silent) isFormattingSilent = false;
    }
    return formatted;
  }

  function handleFindReplace() {
    if (!editorInstance) return;
    editorInstance.focus();
    editorInstance.trigger("toolbar", "actions.find", null);
  }

  function handleLanguageSwitch(newLang) {
    selectedLanguage = newLang;
    if (editorInstance) {
      const model = editorInstance.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, newLang);
      }
    }
  }

  function toggleWordWrap() {
    wordWrapEnabled = !wordWrapEnabled;
    if (editorInstance) {
      editorInstance.updateOptions({ wordWrap: wordWrapEnabled ? "on" : "off" });
    }
  }

  function toggleMinimap() {
    minimapEnabled = !minimapEnabled;
    if (editorInstance) {
      editorInstance.updateOptions({ minimap: { enabled: minimapEnabled } });
    }
  }

  function handleInsertSnippet(snippetId) {
    if (!snippetId || !editorInstance) return;
    const snip = QUICK_SNIPPETS.find((s) => s.id === snippetId);
    selectedSnippetId = "";
    if (!snip) return;

    const pos = editorInstance.getPosition() || { lineNumber: 2, column: 1 };
    editorInstance.executeEdits("snippet-insert", [
      {
        range: new monaco.Range(pos.lineNumber, pos.column, pos.lineNumber, pos.column),
        text: snip.code,
        forceMoveMarkers: true,
      },
    ]);
    editorInstance.focus();
  }

  async function handleRefreshAmbientDts() {
    await syncAmbientDts(monaco, true);
    ambientDtsText = getLastAmbientDts();
    const matches = ambientDtsText.match(/declare interface \w+Record/g);
    ambientTableCount = matches ? matches.length : 0;
    if (validatorController) validatorController.triggerValidation();
  }

  async function handleApplyToDatabase() {
    if (!editorInstance) return;
    await handleFormatDocument();
    const script = editorInstance.getValue();
    if (!script.trim()) return;

    isApplying = true;
    try {
      const res = await fetch("/api/script/apply", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          script,
          package_name: packageName,
          apply: true,
        }),
      });

      const data = await res.json();
      const message = serverMessage(data, "Apply did not save this script.");
      const savedAnyway = message.startsWith("The script was saved");
      if ((res.ok && (data.status === "success" || data.success)) || savedAnyway) {
        compileStatus = savedAnyway ? "error" : "success";
        compileMessage = message;
        compiledDdl = data.compiled_ddl || compiledDdl;
        compiledIr = data.ir || data.compiled_ir || compiledIr;
        diagnostics = Array.isArray(data.diagnostics) ? data.diagnostics : [];
        actionNotice = {
          tone: savedAnyway ? "warning" : "success",
          title: savedAnyway ? "Saved, with a database warning" : "Saved",
          message: savedAnyway
            ? message
            : "This script is stored. Close the object and open it again, and the same event logic is still there.",
        };
        // Refresh ambient .d.ts and catalog so newly created tables/enums/codeunits appear in IntelliSense immediately
        await studioStore.loadCatalog();
        await handleRefreshAmbientDts();
        if (onApplySuccess) onApplySuccess(data);
      } else {
        compileStatus = "error";
        compileMessage = message;
        actionNotice = { tone: "danger", title: "Not saved", message };
        if (Array.isArray(data.diagnostics) && data.diagnostics.length > 0) {
          diagnostics = data.diagnostics;
        }
        isProblemsOpen = true;
      }
    } catch (err) {
      compileStatus = "error";
      compileMessage = err.message || "Network error applying migration";
      actionNotice = {
        tone: "danger",
        title: "Not saved",
        message: compileMessage,
      };
      isProblemsOpen = true;
    } finally {
      isApplying = false;
    }
  }

  export function jumpToLine(lineNumber, column = 1) {
    if (!editorInstance || !lineNumber) return;
    activeTab = "source";
    const model = editorInstance.getModel();
    const safeLine = model ? Math.min(Math.max(1, lineNumber), model.getLineCount()) : lineNumber;
    const lineText = model?.getLineContent(safeLine) || "";
    const brace = lineText.indexOf("{");
    const targetCol = column > 1 ? column : brace >= 0 ? brace + 2 : 1;

    editorInstance.setPosition({ lineNumber: safeLine, column: targetCol });
    editorInstance.revealLineInCenter(safeLine);
    editorInstance.focus();
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden">
  <!-- Primary IDE Toolbar -->
  <Toolbar label="Code studio">
    {#if onDesign}
      <Button size="xs" variant="secondary" onclick={onDesign}>Design</Button>
    {/if}
    <Tabs
      bind:value={activeTab}
      variant="segmented"
      items={[
        { id: "source", label: "Source (.bos)" },
        { id: "ddl", label: "SQL DDL" },
        { id: "ir", label: "AST / IR" },
        { id: "dts", label: "Ambient .d.ts" },
      ]}
    />

    <!-- Symbol Outline Quick-Jumper -->
    {#if documentSymbols.length > 0 && activeTab === "source"}
      <div class="relative flex items-center">
        <ListTree class="w-3.5 h-3.5 text-slate-400 absolute left-2 pointer-events-none" />
        <select
          aria-label="Symbol Outline"
          bind:value={selectedOutlineLine}
          onchange={(e) => {
            const ln = parseInt(e.currentTarget.value, 10);
            selectedOutlineLine = "";
            if (!isNaN(ln)) jumpToLine(ln, 1);
          }}
          class="h-7 pl-7 pr-6 text-xs rounded-md bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-sky-500"
        >
          <option value="">Outline ({documentSymbols.length})</option>
          {#each documentSymbols as sym}
            <option value={sym.line}>{sym.label} (Ln {sym.line})</option>
          {/each}
        </select>
      </div>
    {/if}

    <!-- Insert Snippet Dropdown -->
    {#if activeTab === "source"}
      <select
        aria-label="Insert BOS Snippet"
        bind:value={selectedSnippetId}
        onchange={(e) => handleInsertSnippet(e.currentTarget.value)}
        class="h-7 px-2 text-xs rounded-md bg-slate-100 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-200 focus:outline-none focus:ring-1 focus:ring-sky-500"
      >
        <option value="">+ Snippet</option>
        {#each QUICK_SNIPPETS as snip}
          <option value={snip.id}>{snip.label}</option>
        {/each}
      </select>

      <Button size="xs" variant="ghost" icon={Wand2} title="Format with Prettier (Shift+Alt+F)" onclick={handleFormatDocument}>
        Format
      </Button>
      <Button size="xs" variant="ghost" icon={Search} title="Find & Replace (Ctrl+F)" onclick={handleFindReplace}>
        Find
      </Button>
    {/if}

    <span class="flex-1"></span>

    <!-- Live Diagnostics Status Badge -->
    <button
      type="button"
      onclick={() => (isProblemsOpen = !isProblemsOpen)}
      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium border transition-colors {errorCount > 0
        ? 'bg-rose-500/10 border-rose-500/30 text-rose-600 dark:text-rose-400'
        : warningCount > 0
          ? 'bg-amber-500/10 border-amber-500/30 text-amber-600 dark:text-amber-400'
          : 'bg-emerald-500/10 border-emerald-500/30 text-emerald-600 dark:text-emerald-400'}"
      title="Toggle Problems & Diagnostics Panel"
    >
      {#if isCompiling}
        <RefreshCw class="w-3.5 h-3.5 animate-spin text-sky-400" />
        <span>Checking...</span>
      {:else if errorCount > 0}
        <AlertCircle class="w-3.5 h-3.5" />
        <span>{errorCount} {errorCount === 1 ? "Error" : "Errors"}</span>
        {#if warningCount > 0}
          <span class="opacity-75">· {warningCount} Warn</span>
        {/if}
      {:else if warningCount > 0}
        <AlertTriangle class="w-3.5 h-3.5" />
        <span>{warningCount} {warningCount === 1 ? "Warning" : "Warnings"}</span>
      {:else}
        <CheckCircle2 class="w-3.5 h-3.5" />
        <span>0 Problems</span>
      {/if}
    </button>

    {#if objectType === "codeunit"}
      <Button size="xs" variant="secondary" icon={Terminal} onclick={() => (isRunnerOpen = !isRunnerOpen)}>
        {isRunnerOpen ? "Hide console" : "Run console"}
      </Button>
    {/if}
    <Button
      size="xs"
      variant="secondary"
      icon={Sparkles}
      loading={isCompiling}
      tooltip="Checks the script. Does not save it."
      onclick={triggerCompileImmediately}
    >
      Compile
    </Button>
    <Button
      size="xs"
      icon={Play}
      loading={isApplying}
      shortcut="F5"
      showShortcut
      tooltip="Saves the script and updates the database. Ctrl+S does the same."
      onclick={handleApplyToDatabase}
    >
      {isApplying ? "Saving..." : "Apply"}
    </Button>
  </Toolbar>

  {#if actionNotice}
    <div class="px-3 pt-2">
      <Banner
        tone={actionNotice.tone}
        title={actionNotice.title}
        dismissible
        onDismiss={() => (actionNotice = null)}
      >
        {actionNotice.message}
      </Banner>
    </div>
  {/if}

  <!-- Main Editor & Inspection Viewport -->
  <div class="flex-1 relative min-h-0 flex flex-col">
    <div class="flex-1 relative min-h-0">
      <!-- Monaco Editor Container -->
      <div
        bind:this={editorContainer}
        class="w-full h-full {activeTab === 'source' ? 'block' : 'hidden'}"
      ></div>

      <!-- PostgreSQL DDL Tab -->
      {#if activeTab === "ddl"}
        <div class="w-full h-full p-4 overflow-auto bg-slate-950 text-slate-200 font-mono text-xs">
          <div class="flex items-center justify-between pb-2 mb-3 border-b border-slate-800 text-slate-400">
            <span>Target Database: PostgreSQL 18.4 (Compiled via bosc-codegen-db)</span>
            <span class="text-[10px] text-slate-500">Idempotent Atomic DDL Migration</span>
          </div>
          {#if compiledDdl}
            <pre class="whitespace-pre-wrap text-emerald-400 leading-relaxed">{compiledDdl}</pre>
          {:else}
            <p class="text-slate-500 italic">No SQL DDL emitted for this object or waiting for valid compilation.</p>
          {/if}
        </div>
      {/if}

      <!-- Compiled IR AST Tab -->
      {#if activeTab === "ir"}
        <div class="w-full h-full p-4 overflow-auto bg-slate-950 text-slate-200 font-mono text-xs">
          <div class="flex items-center justify-between pb-2 mb-3 border-b border-slate-800 text-slate-400">
            <span>Intermediate Representation (JSONB definition_ir)</span>
            <span class="text-[10px] text-slate-500">bosc-ir schema</span>
          </div>
          {#if compiledIr}
            <pre class="whitespace-pre-wrap text-sky-400 leading-relaxed">{JSON.stringify(compiledIr, null, 2)}</pre>
          {:else}
            <p class="text-slate-500 italic">No IR AST generated yet.</p>
          {/if}
        </div>
      {/if}

      <!-- Live Ambient TypeScript Declarations (.d.ts) Tab -->
      {#if activeTab === "dts"}
        <div class="w-full h-full p-4 overflow-auto bg-slate-950 text-slate-200 font-mono text-xs">
          <div class="flex items-center justify-between pb-2 mb-3 border-b border-slate-800 text-slate-400">
            <span>Live Catalog Ambient TypeScript Declarations (`/api/meta/ambient.d.ts`)</span>
            <Button size="xs" variant="secondary" icon={RefreshCw} onclick={handleRefreshAmbientDts}>
              Refresh .d.ts
            </Button>
          </div>
          {#if ambientDtsText}
            <pre class="whitespace-pre-wrap text-amber-300 leading-relaxed">{ambientDtsText}</pre>
          {:else}
            <p class="text-slate-500 italic">Loading ambient TypeScript declarations...</p>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Interactive Problems & Diagnostics Panel -->
    {#if isProblemsOpen}
      <div class="h-44 border-t border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900/95 flex flex-col">
        <div class="flex items-center justify-between px-3 py-1.5 border-b border-slate-200 dark:border-slate-800 text-xs font-medium text-slate-600 dark:text-slate-300">
          <div class="flex items-center gap-3">
            <span>Problems & Grammar Diagnostics ({diagnostics.length})</span>
            <span class="text-[11px] text-slate-400">{compileMessage}</span>
          </div>
          <button
            type="button"
            onclick={() => (isProblemsOpen = false)}
            class="p-0.5 rounded hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-400"
            title="Close Problems Panel"
          >
            <ChevronDown class="w-4 h-4" />
          </button>
        </div>

        <div class="flex-1 overflow-y-auto divide-y divide-slate-200/60 dark:divide-slate-800/60 text-xs">
          {#if diagnostics.length === 0}
            <div class="p-4 flex items-center gap-2 text-emerald-600 dark:text-emerald-400">
              <CheckCircle2 class="w-4 h-4 shrink-0" />
              <span>No syntax, grammar, or semantic problems detected in `{objectName || "document"}`.</span>
            </div>
          {:else}
            {#each diagnostics as diag}
              <button
                type="button"
                onclick={() => jumpToLine(diag.line || 1, diag.column || 1)}
                class="w-full text-left px-3 py-2 hover:bg-slate-200/60 dark:hover:bg-slate-800/60 flex items-start gap-2.5 transition-colors"
              >
                {#if diag.severity === "error"}
                  <AlertCircle class="w-4 h-4 text-rose-500 shrink-0 mt-0.5" />
                {:else}
                  <AlertTriangle class="w-4 h-4 text-amber-500 shrink-0 mt-0.5" />
                {/if}
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <span class="font-mono text-[10px] px-1.5 py-0.5 rounded bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300">
                      {diag.code || "BOS"}
                    </span>
                    <span class="text-slate-800 dark:text-slate-100 font-medium break-words">
                      {diag.message}
                    </span>
                  </div>
                  {#if diag.hint}
                    <div class="mt-1 flex items-center gap-1 text-[11px] text-sky-600 dark:text-sky-400">
                      <Lightbulb class="w-3 h-3 shrink-0" />
                      <span>{diag.hint}</span>
                    </div>
                  {/if}
                </div>
                <span class="font-mono text-[11px] text-slate-400 shrink-0">
                  Ln {diag.line || 1}, Col {diag.column || 1}
                </span>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </div>

  <!-- Embedded QuickJS Live Runner -->
  <CodeunitRunnerPanel
    codeunitName={objectName}
    sourceCode={content}
    bind:isOpen={isRunnerOpen}
    onJumpToLine={(ln) => jumpToLine(ln, 1)}
  />

  <!-- IDE Status Footer Bar -->
  <div class="h-6 px-3 border-t border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-900 flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400 select-none">
    <div class="flex items-center gap-3">
      <button
        type="button"
        onclick={() => (isProblemsOpen = !isProblemsOpen)}
        class="flex items-center gap-1 hover:text-slate-800 dark:hover:text-slate-200"
      >
        {#if isProblemsOpen}
          <ChevronDown class="w-3 h-3" />
        {:else}
          <ChevronUp class="w-3 h-3" />
        {/if}
        <span>{errorCount} Errors, {warningCount} Warnings</span>
      </button>
      <span>·</span>
      <span>pkg: <strong class="text-slate-700 dark:text-slate-300">{packageName}</strong></span>
      <span>·</span>
      <button
        type="button"
        onclick={handleRefreshAmbientDts}
        class="hover:text-sky-500 transition-colors"
        title="Click to reload live Ambient .d.ts from database catalog"
      >
        IntelliSense: {ambientTableCount > 0 ? `${ambientTableCount} Catalog Models` : "Active"}
      </button>
    </div>

    <div class="flex items-center gap-3">
      <span class="font-mono">
        Ln {cursorLine}, Col {cursorColumn}{selectionLength > 0 ? ` (${selectionLength} selected)` : ""}
      </span>
      <span>·</span>
      <button
        type="button"
        onclick={toggleWordWrap}
        class="flex items-center gap-1 hover:text-slate-800 dark:hover:text-slate-200 {wordWrapEnabled ? 'text-sky-500 font-medium' : ''}"
        title="Toggle Word Wrap"
      >
        <WrapText class="w-3 h-3" />
        <span>Wrap</span>
      </button>
      <button
        type="button"
        onclick={toggleMinimap}
        class="flex items-center gap-1 hover:text-slate-800 dark:hover:text-slate-200 {minimapEnabled ? 'text-sky-500 font-medium' : ''}"
        title="Toggle Minimap"
      >
        <MapIcon class="w-3 h-3" />
        <span>Minimap</span>
      </button>
      <span>·</span>
      <div class="flex items-center gap-1">
        <Code2 class="w-3 h-3 text-sky-500" />
        <select
          aria-label="Language Mode"
          value={selectedLanguage}
          onchange={(e) => handleLanguageSwitch(e.currentTarget.value)}
          class="bg-transparent text-[11px] font-medium text-slate-700 dark:text-slate-200 focus:outline-none cursor-pointer"
        >
          <option value="bos-script">BOS-Script (+TS Shadow)</option>
          <option value="typescript">TypeScript</option>
        </select>
      </div>
    </div>
  </div>
</div>
