<script>
  /**
   * PageDesigner.svelte (Svelte 5 Runes)
   * Microsoft Access-Style WYSIWYG Form / Page Designer
   * Supports all 8 BOS Page Layout Types (List, Record, Document, Board, Workspace, Split, Timeline, Wizard)
   * and all Toolbox Control Types (fasttab, grid, subform, text, decimal, date, lookup, toggle, tile, step, factbox, action).
   */
  import { onMount } from "svelte";
  import { FileCode, Sparkles, LayoutGrid, Sliders, Play } from "lucide-svelte";
  import { Button, Tabs, Toolbar, Spinner, Select, Banner } from "$lib/bos-ui/index.js";
  import PageToolbox from "./PageToolbox.svelte";
  import PageCanvas from "./PageCanvas.svelte";
  import PropertySheet from "../property-sheet/PropertySheet.svelte";
  import { StudioApi } from "../stores/studioApi.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  const PAGE_LAYOUT_TYPES = [
    { value: "List", label: "List (Repeater / Grid)" },
    { value: "Record", label: "Record (Card)" },
    { value: "Document", label: "Document (Header + Lines)" },
    { value: "Board", label: "Board (Kanban / Arrange)" },
    { value: "Workspace", label: "Workspace (Role Home + Cues)" },
    { value: "Split", label: "Split (Master List + Detail)" },
    { value: "Timeline", label: "Timeline (Chronological Feed)" },
    { value: "Wizard", label: "Wizard (Guided Steps)" },
    { value: "Dialog", label: "Dialog (Centered Card / Portal)" },
  ];

  let {
    pageName = "",
    targetTable = "",
    sourceCode = "",
    packageName = "bos.app",
    onSourceChange = null,
    onJumpToCode = null,
    mode = $bindable("design"), // 'design' | 'preview'
  } = $props();

  let isToolboxOpen = $state(true);
  let isPropertySheetOpen = $state(true);
  let isLoading = $state(false);
  let isApplying = $state(false);
  let statusBanner = $state(null);
  let hydrated = $state(false);

  let availableTables = $derived(
    (studioStore.catalog || []).filter((o) => o.object_type === "table")
  );
  let resolvedTable = $state("");
  let layoutType = $state("Record");

  let elements = $state([]);
  let selectedElement = $state(null);

  function normalizeLayoutName(raw = "") {
    const val = String(raw || "").trim().toLowerCase();
    if (val === "list") return "List";
    if (val === "card" || val === "record") return "Record";
    if (val === "document") return "Document";
    if (val === "board" || val === "kanban") return "Board";
    if (val === "workspace" || val === "home" || val === "rolecenter") return "Workspace";
    if (val === "split") return "Split";
    if (val === "timeline") return "Timeline";
    if (val === "wizard" || val === "steps") return "Wizard";
    if (val === "dialog" || val === "portal" || val === "login" || val === "modal") return "Dialog";
    return "Record";
  }

  function controlsToElements(controls = [], actions = []) {
    const parsed = [];
    let seq = 0;

    function walk(node) {
      if (!node) return;
      const kind = String(node.kind || node.control_type || "").toLowerCase();
      const name = node.name || node.id || `node_${seq++}`;
      const caption = node.caption || node.properties?.caption || name;
      const props = node.properties || {};

      if (kind === "area") {
        if (Array.isArray(node.children)) {
          for (const child of node.children) walk(child);
        }
      } else if (kind === "group" || kind === "section") {
        parsed.push({
          id: `el_grp_${name}_${seq++}`,
          name,
          type: "fasttab",
          label: caption,
          caption,
        });
        if (Array.isArray(node.children)) {
          for (const child of node.children) walk(child);
        }
      } else if (kind === "grid") {
        parsed.push({
          id: `el_grid_${name}_${seq++}`,
          name,
          type: "grid",
          label: caption,
          caption,
        });
        if (Array.isArray(node.children)) {
          for (const child of node.children) walk(child);
        }
      } else if (kind === "part" || kind === "subform") {
        const isFactBox = /fact|side/i.test(name) || /fact|side/i.test(props.system_part || "");
        parsed.push({
          id: `el_part_${name}_${seq++}`,
          name,
          type: isFactBox ? "factbox" : "subform",
          label: caption,
          caption,
          sourceField: node.source_expr || props.page || "",
          editable: props.editable !== false,
          visible: props.visible !== false,
        });
      } else if (kind === "tile" || kind === "cue") {
        parsed.push({
          id: `el_tile_${name}_${seq++}`,
          name,
          type: "tile",
          label: caption,
          caption,
          sourceField: node.source_expr || name,
        });
      } else if (kind === "step") {
        parsed.push({
          id: `el_step_${name}_${seq++}`,
          name,
          type: "step",
          label: caption,
          caption,
        });
      } else if (kind === "field") {
        const style = String(props.style || "").toLowerCase();
        let ctrlType = "text";
        if (style === "currency" || style === "decimal" || style === "number") ctrlType = "decimal";
        else if (style === "date" || style === "datetime") ctrlType = "date";
        else if (style === "lookup") ctrlType = "lookup";
        else if (style === "switch" || style === "toggle" || style === "checkbox") ctrlType = "toggle";
        else if (style === "password") ctrlType = "password";

        parsed.push({
          id: `el_${name}_${seq++}`,
          name,
          type: ctrlType,
          label: caption,
          caption,
          sourceField: node.source_expr || name,
          editable: props.editable !== false,
          visible: props.visible !== false,
        });
      } else if (kind === "action" || kind === "command") {
        parsed.push({
          id: `el_act_${name}_${seq++}`,
          name,
          type: "action",
          label: caption,
          caption,
          script: node.script || "",
        });
      }
    }

    if (Array.isArray(controls)) {
      for (const ctrl of controls) walk(ctrl);
    }

    if (Array.isArray(actions)) {
      for (const act of actions) {
        const aName = act.name || `act_${seq++}`;
        const aCap = act.caption || aName;
        parsed.push({
          id: `el_act_${aName}_${seq++}`,
          name: aName,
          type: "action",
          label: aCap,
          caption: aCap,
          script: act.script || "",
        });
      }
    }

    return parsed;
  }

  function parsePageFromSource(src = "") {
    if (!src || !/\b(page|pageextension)\s+[A-Za-z_]/.test(src)) return null;
    const headerForMatch = src.match(/\bpage\s+[A-Za-z_][A-Za-z0-9_]*\s+for\s+([A-Za-z_][A-Za-z0-9_]*)/);
    const propSrcMatch = src.match(/\b(?:sourceTable|source_table|source)\s*[:=]\s*(?:table\.|view\.|query\.)?([A-Za-z_][A-Za-z0-9_]*)/);
    const parsedTable = headerForMatch?.[1] || propSrcMatch?.[1] || "";

    const layoutMatch = src.match(/\b(?:pageType|page_type|layout|type)\s*[:=]\s*"?([A-Za-z_][A-Za-z0-9_]*)"?/);
    let parsedLayout = layoutMatch?.[1] ? normalizeLayoutName(layoutMatch[1]) : "Record";
    if (parsedLayout === "Record" && (pageName === "LoginPage" || /login|signin/i.test(src))) {
      parsedLayout = "Dialog";
    }

    const parsedElements = [];
    const lines = src.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trim();
      const grpMatch = line.match(/^(?:group|section)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)/);
      if (grpMatch) {
        let cap = grpMatch[1];
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        if (capMatch) {
          cap = capMatch[1];
        } else {
          for (let j = i + 1; j < Math.min(lines.length, i + 5); j++) {
            const nextL = lines[j].trim();
            if (nextL.startsWith("group") || nextL.startsWith("field")) break;
            const m = nextL.match(/caption\s*[:=]\s*"([^"]*)"/);
            if (m) { cap = m[1]; break; }
          }
        }
        parsedElements.push({
          id: `el_grp_${grpMatch[1]}_${i}`,
          name: grpMatch[1],
          type: "fasttab",
          label: cap,
          caption: cap,
        });
        continue;
      }
      const gridMatch = line.match(/^grid\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)/);
      if (gridMatch) {
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        parsedElements.push({
          id: `el_grid_${gridMatch[1]}_${i}`,
          name: gridMatch[1],
          type: "grid",
          label: capMatch?.[1] || gridMatch[1],
          caption: capMatch?.[1] || gridMatch[1],
        });
        continue;
      }
      const partMatch = line.match(/^part\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*[;,]\s*([A-Za-z_][A-Za-z0-9_]*))?\s*\)/);
      if (partMatch) {
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        const isFactBox = /fact|side/i.test(partMatch[1]) || /fact/i.test(partMatch[2] || "");
        parsedElements.push({
          id: `el_part_${partMatch[1]}_${i}`,
          name: partMatch[1],
          type: isFactBox ? "factbox" : "subform",
          label: capMatch?.[1] || partMatch[1],
          caption: capMatch?.[1] || partMatch[1],
          sourceField: partMatch[2] || "",
          editable: true,
          visible: true,
        });
        continue;
      }
      const tileMatch = line.match(/^tile\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)/);
      if (tileMatch) {
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        parsedElements.push({
          id: `el_tile_${tileMatch[1]}_${i}`,
          name: tileMatch[1],
          type: "tile",
          label: capMatch?.[1] || tileMatch[1],
          caption: capMatch?.[1] || tileMatch[1],
          sourceField: tileMatch[1],
        });
        continue;
      }
      const stepMatch = line.match(/^step\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)/);
      if (stepMatch) {
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        parsedElements.push({
          id: `el_step_${stepMatch[1]}_${i}`,
          name: stepMatch[1],
          type: "step",
          label: capMatch?.[1] || stepMatch[1],
          caption: capMatch?.[1] || stepMatch[1],
        });
        continue;
      }
      const fldMatch = line.match(/^field\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)/);
      if (fldMatch) {
        const name = fldMatch[1];
        let cap = name;
        let style = "";
        let editable = true;
        let visible = true;

        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        if (capMatch) cap = capMatch[1];
        const styleMatch = line.match(/style\s*[:=]\s*"?([A-Za-z_]+)"?/);
        if (styleMatch) style = styleMatch[1].toLowerCase();
        if (/editable\s*[:=]\s*false/i.test(line)) editable = false;
        if (/visible\s*[:=]\s*false/i.test(line)) visible = false;

        if (!line.includes("}") || line.endsWith("{")) {
          for (let j = i + 1; j < Math.min(lines.length, i + 8); j++) {
            const nextL = lines[j].trim();
            if (nextL.startsWith("field") || nextL.startsWith("group") || nextL.startsWith("action")) break;
            const cm = nextL.match(/caption\s*[:=]\s*"([^"]*)"/);
            if (cm) cap = cm[1];
            const sm = nextL.match(/style\s*[:=]\s*"?([A-Za-z_]+)"?/);
            if (sm) style = sm[1].toLowerCase();
            if (/editable\s*[:=]\s*false/i.test(nextL)) editable = false;
            if (/visible\s*[:=]\s*false/i.test(nextL)) visible = false;
            if (nextL.includes("}")) break;
          }
        }

        let ctrlType = "text";
        if (style === "currency" || style === "number" || style === "decimal") ctrlType = "decimal";
        else if (style === "date" || style === "datetime") ctrlType = "date";
        else if (style === "lookup") ctrlType = "lookup";
        else if (style === "switch" || style === "checkbox" || style === "toggle") ctrlType = "toggle";
        else if (style === "password") ctrlType = "password";

        parsedElements.push({
          id: `el_${name}_${i}`,
          name,
          type: ctrlType,
          label: cap,
          caption: cap,
          sourceField: name,
          editable,
          visible,
        });
        continue;
      }
      const actMatch = line.match(/^(?:action|command)\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)/);
      if (actMatch) {
        const name = actMatch[1];
        let cap = name;
        const capMatch = line.match(/caption\s*[:=]\s*"([^"]*)"/);
        if (capMatch) {
          cap = capMatch[1];
        } else {
          for (let j = i + 1; j < Math.min(lines.length, i + 5); j++) {
            const nextL = lines[j].trim();
            if (nextL.startsWith("action") || nextL.startsWith("field")) break;
            const cm = nextL.match(/caption\s*[:=]\s*"([^"]*)"/);
            if (cm) { cap = cm[1]; break; }
            if (nextL.includes("}")) break;
          }
        }
        parsedElements.push({
          id: `el_act_${name}_${i}`,
          name,
          type: "action",
          label: cap,
          caption: cap,
          sourceField: "",
          editable: true,
          visible: true,
        });
      }
    }
    return {
      targetTable: parsedTable,
      layoutType: parsedLayout,
      elements: parsedElements,
    };
  }

  let loadedPage = $state("");

  $effect(() => {
    const target = pageName;
    if (target && target !== loadedPage) {
      loadedPage = target;
      loadPageLayout(target);
    }
  });

  async function loadPageLayout(targetPage) {
    isLoading = true;
    try {
      // 1. Try to load registered page meta-object definition from catalog
      if (targetPage) {
        try {
          const metaObj = await StudioApi.getMetaObject(targetPage);
          if (targetPage !== loadedPage) return;
          if (metaObj && metaObj.definition) {
            const def = metaObj.definition;
            if (def.source_table || def.source?.name) resolvedTable = def.source_table || def.source?.name;
            if (def.page_type || def.layout) layoutType = normalizeLayoutName(def.page_type || def.layout);

            // A) Check modern def.controls
            if (Array.isArray(def.controls) && def.controls.length > 0) {
              const converted = controlsToElements(def.controls, def.actions);
              if (converted.length > 0) {
                elements = converted;
                selectedElement = elements[0] || null;
                hydrated = true;
                return;
              }
            }

            // B) Check legacy def.elements
            if (Array.isArray(def.elements) && def.elements.length > 0) {
              elements = def.elements;
              selectedElement = elements[0] || null;
              hydrated = true;
              return;
            }
          }
        } catch {
          // not found or new page draft
        }
      }

      // 2. Parse existing `.bos` sourceCode if present
      if (sourceCode) {
        const parsed = parsePageFromSource(sourceCode);
        if (parsed && parsed.elements.length > 0) {
          if (!resolvedTable) resolvedTable = parsed.targetTable || targetTable || "";
          layoutType = parsed.layoutType || layoutType || "Record";
          elements = parsed.elements;
          selectedElement = elements[0] || null;
          hydrated = true;
          return;
        }
      }

      if (!resolvedTable) {
        resolvedTable = targetTable || availableTables[0]?.name || availableTables[0]?.object_name || "";
      }

      // 3. Generate form controls dynamically from target table schema
      if (resolvedTable) {
        try {
          const schema = await StudioApi.getEntitySchema(resolvedTable);
          if (targetPage !== loadedPage) return;
          if (schema && schema.columns && schema.columns.length > 0) {
            const generated = [
              {
                id: "el_grp_general",
                name: "General",
                type: "fasttab",
                label: "General",
                caption: "General",
              },
            ];

            for (const col of schema.columns) {
              if (col.name.startsWith("_")) continue;

              let ctrlType = "text";
              const dt = (col.data_type || "").toLowerCase();
              if (
                dt.includes("int") ||
                dt.includes("decimal") ||
                dt.includes("numeric") ||
                dt.includes("float")
              ) {
                ctrlType = "decimal";
              } else if (dt.includes("date") || dt.includes("time")) {
                ctrlType = "date";
              } else if (dt.includes("bool")) {
                ctrlType = "toggle";
              }

              generated.push({
                id: `el_${col.name}`,
                name: col.name,
                type: ctrlType,
                label: col.caption || col.name,
                caption: col.caption || col.name,
                sourceField: col.name,
                editable: !col.is_primary_key,
                visible: true,
              });
            }

            elements = generated;
            selectedElement = elements[1] || elements[0] || null;
            hydrated = true;
            return;
          }
        } catch {
          // table schema lookup failed or new table
        }
      }

      // 4. Clean default canvas if no target table or schema
      elements = [
        {
          id: "el_grp_1",
          name: "General",
          type: "fasttab",
          label: "General",
          caption: "General",
        },
      ];
      selectedElement = elements[0];
      hydrated = true;
    } finally {
      isLoading = false;
    }
  }

  function emitSource() {
    if (!elements.length) return "";
    const emitted = generatePageScript();
    onSourceChange?.(emitted);
    return emitted;
  }

  function handleAddElement(type, label) {
    const newEl = {
      id: `el_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      name: (label || type).replace(/[^A-Za-z0-9_]/g, ""),
      type,
      label,
      caption: label,
      sourceField: type === "subform" ? "SalesLine" : type === "factbox" ? "CustomerFact" : "",
      editable: true,
      visible: true,
    };
    elements = [...elements, newEl];
    selectedElement = newEl;
    emitSource();
  }

  function generatePageScript() {
    const tbl = resolvedTable || availableTables[0]?.name || availableTables[0]?.object_name || "Record";
    const safePage = (pageName || "NewCard").replace(/[^A-Za-z0-9_]/g, "");

    const groups = [];
    let currentGroup = { name: "General", caption: "General", items: [] };
    groups.push(currentGroup);
    const actions = [];

    for (const el of elements) {
      if (el.type === "fasttab") {
        const grpName = (el.name || el.caption || el.label || "General").replace(/[^A-Za-z0-9_]/g, "") || "General";
        const grpCap = el.caption || el.label || grpName;
        currentGroup = { name: grpName, caption: grpCap, items: [] };
        groups.push(currentGroup);
      } else if (el.type === "action") {
        const actId = (el.name || el.caption || el.label || "Action").replace(/[^A-Za-z0-9_]/g, "") || "Action";
        actions.push({
          id: actId,
          caption: el.caption || el.label || actId,
          script: el.script || "",
        });
      } else {
        currentGroup.items.push(el);
      }
    }

    const nonEmptyGroups = groups.filter((g) => g.items.length > 0);
    const finalGroups = nonEmptyGroups.length > 0 ? nonEmptyGroups : groups;

    let script = `page ${safePage} for ${tbl} {\n    pageType = ${layoutType};\n    caption = "${pageName || safePage}";\n\n    layout {\n        area(Content) {\n`;

    for (const grp of finalGroups) {
      script += `            group(${grp.name}) {\n`;
      if (grp.caption) {
        script += `                caption = "${grp.caption}";\n`;
      }
      for (const el of grp.items) {
        if (el.type === "subform" || el.type === "factbox") {
          const partId = (el.caption || el.label || "Part").replace(/[^A-Za-z0-9_]/g, "") || "Part";
          const target = (el.sourceField || (el.type === "factbox" ? "CustomerFact" : "SalesLine")).replace(/[^A-Za-z0-9_]/g, "");
          script += `                part(${partId}; ${target}) { caption = "${el.caption || el.label || partId}"; }\n`;
        } else if (el.type === "grid") {
          const gridId = (el.caption || el.label || "Matrix").replace(/[^A-Za-z0-9_]/g, "") || "Matrix";
          script += `                grid(${gridId}) { caption = "${el.caption || el.label || gridId}"; }\n`;
        } else if (el.type === "tile") {
          const tileId = (el.caption || el.label || "Cue").replace(/[^A-Za-z0-9_]/g, "") || "Cue";
          script += `                tile(${tileId}) { caption = "${el.caption || el.label || tileId}"; }\n`;
        } else if (el.type === "step") {
          const stepId = (el.caption || el.label || "Step").replace(/[^A-Za-z0-9_]/g, "") || "Step";
          script += `                step(${stepId}) { caption = "${el.caption || el.label || stepId}"; }\n`;
        } else {
          const fldId = (el.sourceField || el.label || "id").replace(/[^A-Za-z0-9_]/g, "") || "id";
          const props = [`caption = "${el.caption || el.label || fldId}";`];
          if (el.type === "decimal") props.push(`style = currency;`);
          else if (el.type === "date") props.push(`style = date;`);
          else if (el.type === "lookup") props.push(`style = lookup;`);
          else if (el.type === "toggle") props.push(`style = switch;`);
          if (el.editable === false) props.push(`editable = false;`);
          script += `                field(${fldId}) { ${props.join(" ")} }\n`;
        }
      }
      script += `            }\n`;
    }
    script += `        }\n    }\n`;

    if (actions.length > 0) {
      script += `\n    actions {\n`;
      for (const act of actions) {
        script += `        action(${act.id}) {\n            caption = "${act.caption}";\n`;
        if (act.script) {
          script += `            trigger onAction() {\n`;
          for (const s_line of act.script.split("\n")) {
            script += `                ${s_line}\n`;
          }
          script += `            }\n`;
        }
        script += `        }\n`;
      }
      script += `    }\n`;
    }

    script += `}\n`;
    return script;
  }

  function switchToCodeView() {
    const emitted = emitSource();
    if (onJumpToCode) {
      onJumpToCode(emitted, 1);
    } else {
      mode = "code";
    }
  }

  async function handleApplyPage() {
    const emitted = emitSource();
    isApplying = true;
    statusBanner = null;
    try {
      const res = await StudioApi.applyScript(emitted, packageName || "bos.app");
      if (res && (res.status === "success" || res.success !== false)) {
        statusBanner = { tone: "success", message: `Page '${pageName || "NewCard"}' (${layoutType}) compiled and saved to catalog!` };
      } else {
        statusBanner = { tone: "danger", message: res?.message || "Failed to compile page" };
      }
    } catch (err) {
      statusBanner = { tone: "danger", message: err.message || "Error saving page" };
    } finally {
      isApplying = false;
    }
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden">
  <Toolbar label="Page designer">
    <Tabs
      value={mode === "code" ? "design" : mode}
      variant="segmented"
      items={[
        { id: "design", label: "Design" },
        { id: "preview", label: "Preview" },
      ]}
      onValueChange={(next) => (mode = next)}
    />
    <Button size="xs" variant="ghost" icon={FileCode} onclick={switchToCodeView}>
      Code
    </Button>
    <div class="w-44">
      <Select
        bind:value={layoutType}
        placeholder="Layout type"
        options={PAGE_LAYOUT_TYPES}
      />
    </div>
    {#if availableTables.length > 0}
      <div class="w-40">
        <Select
          bind:value={resolvedTable}
          placeholder="Source table"
          options={availableTables.map((t) => ({ value: t.name || t.object_name, label: t.name || t.object_name }))}
        />
      </div>
    {/if}
    {#if mode === "design"}
      <Button size="xs" variant={isToolboxOpen ? "secondary" : "ghost"} icon={LayoutGrid} onclick={() => (isToolboxOpen = !isToolboxOpen)}>
        Toolbox
      </Button>
      <Button size="xs" variant={isPropertySheetOpen ? "secondary" : "ghost"} icon={Sliders} onclick={() => (isPropertySheetOpen = !isPropertySheetOpen)}>
        Properties
      </Button>
    {/if}
    <span class="flex-1"></span>
    <Button size="xs" variant="secondary" icon={Sparkles} onclick={switchToCodeView}>
      Source (.bos)
    </Button>
    <Button size="xs" icon={Play} loading={isApplying} onclick={handleApplyPage}>
      Apply F5
    </Button>
  </Toolbar>

  {#if statusBanner}
    <div class="px-3 pt-2">
      <Banner
        tone={statusBanner.tone}
        title={statusBanner.tone === "success" ? "Page Saved" : "Compilation Error"}
        dismissible
        onDismiss={() => (statusBanner = null)}
      >
        {statusBanner.message}
      </Banner>
    </div>
  {/if}

  <!-- Main Body: Left Panels (Toolbox + Property Sheet) + Center/Right Canvas -->
  <div class="flex-1 flex min-h-0 overflow-hidden">
    {#if mode === "design"}
      {#if isPropertySheetOpen}
        <aside class="w-72 border-r border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 overflow-hidden">
          <PropertySheet
            bind:selectedElement
            onClose={() => (isPropertySheetOpen = false)}
            onJumpToCode={switchToCodeView}
          />
        </aside>
      {/if}

      {#if isToolboxOpen}
        <PageToolbox
          onAddElement={handleAddElement}
          onClose={() => (isToolboxOpen = false)}
        />
      {/if}
    {/if}

    <!-- 3. Canvas / Live Preview -->
    <main class="flex-1 min-w-0 min-h-0 overflow-auto">
      {#if isLoading && elements.length === 0}
        <div class="h-full flex flex-col items-center justify-center text-xs text-slate-400 space-y-2">
          <Spinner />
          <p>Loading schema for '{resolvedTable || pageName}'...</p>
        </div>
      {:else}
        <PageCanvas
          bind:elements
          bind:selectedElement
          {layoutType}
          pageCaption={pageName || "Page Layout"}
          sourceTable={resolvedTable}
          previewMode={mode === "preview"}
          onSelect={(el) => (selectedElement = el)}
        />
      {/if}
    </main>
  </div>
</div>
