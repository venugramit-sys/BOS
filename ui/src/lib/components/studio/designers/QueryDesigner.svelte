<script>
  /**
   * QueryDesigner.svelte (Svelte 5 Runes)
   * Microsoft Access-Style Visual Query Designer (QBE)
   * Upper Join Canvas + Lower QBE Matrix + Live Results Datasheet + SQL View
   */
  import { onMount } from "svelte";
  import { Play, FileCode } from "lucide-svelte";
  import { Button, Tabs, Toolbar, Banner, Spinner, EmptyState } from "$lib/bos-ui/index.js";
  import QueryJoinCanvas from "./QueryJoinCanvas.svelte";
  import QueryGridMatrix from "./QueryGridMatrix.svelte";
  import { StudioApi } from "../stores/studioApi.js";
  import { studioStore } from "../stores/studioStore.svelte.js";

  let {
    queryName = "",
    sourceCode = "",
    packageName = "bos.app",
    isDirty = $bindable(true),
    onSourceChange = null,
    onJumpToCode = null,
    mode = $bindable("design"), // 'design' | 'results' | 'sql'
  } = $props();

  let selectedTables = $state([]);
  let joins = $state([]);
  let columns = $state([]);
  let isLoading = $state(false);
  let hydrated = $state(false);
  let lastAppliedScript = $state("");

  onMount(async () => {
    await loadQueryDefinition();
  });

  async function loadQueryDefinition() {
    isLoading = true;
    try {
      // 1. Try to load registered query meta-object
      if (queryName) {
        try {
          const metaObj = await StudioApi.getMetaObject(queryName);
          if (metaObj && metaObj.definition) {
            const def = metaObj.definition;
            if (Array.isArray(def.tables) && def.tables.length > 0) {
              selectedTables = def.tables;
              joins = Array.isArray(def.joins) ? def.joins : [];
              columns = Array.isArray(def.columns) ? def.columns : [];
              isDirty = false;
              hydrated = true;
              return;
            }
          }
        } catch {
          // not found or new draft
        }
      }

      // 2. Pick first available table from catalog if available
      const tableObjs = studioStore.catalog.filter((o) => o.object_type === "table");
      if (tableObjs.length > 0) {
        const firstTbl = tableObjs[0].object_name;
        let fields = ["id"];
        try {
          const schema = await StudioApi.getEntitySchema(firstTbl);
          if (schema && schema.columns && schema.columns.length > 0) {
            fields = schema.columns.map((c) => c.name).filter((n) => !n.startsWith("_"));
          }
        } catch {
          // fallback
        }

        selectedTables = [
          {
            name: firstTbl,
            x: 30,
            y: 30,
            fields,
          },
        ];
        joins = [];

        // Prepopulate first 2-3 fields into matrix
        columns = fields.slice(0, 3).map((f, i) => ({
          id: `c_${i + 1}`,
          field: f,
          table: firstTbl,
          alias: f,
          total: "Group By",
          sort: i === 0 ? "Ascending" : "None",
          show: true,
          criteria: "",
          orCriteria: "",
        }));
      } else {
        selectedTables = [];
        joins = [];
        columns = [];
      }
      hydrated = true;
    } finally {
      isLoading = false;
    }
  }

  function generateQueryBosScript() {
    const safeName = (queryName || "NewQuery").replace(/[^A-Za-z0-9_]/g, "");
    const firstTbl = selectedTables[0]?.name || columns[0]?.table || "Record";
    const colLines = columns
      .filter((c) => c.show !== false && c.field)
      .map((c) => {
        const alias = (c.alias || c.field).replace(/[^A-Za-z0-9_]/g, "") || c.field;
        const fnMap = { Sum: "sum", Count: "count", Avg: "average", Min: "min", Max: "max" };
        if (fnMap[c.total]) {
          return `        column(${alias}, ${fnMap[c.total]}(${c.field}));`;
        }
        return `        column(${alias}, ${c.field});`;
      });

    let bos = `query ${safeName} {\n    sourceTable: ${firstTbl};\n    columns {\n`;
    bos += colLines.length > 0 ? `${colLines.join("\n")}\n` : `        column(id, id);\n`;
    bos += `    }\n}\n`;
    return bos;
  }

  $effect(() => {
    if (!hydrated || (selectedTables.length === 0 && columns.length === 0)) return;
    const generated = generateQueryBosScript();
    if (lastAppliedScript && generated !== lastAppliedScript) {
      isDirty = true;
    }
    onSourceChange?.(generated);
  });

  function switchToCodeView() {
    const script = generateQueryBosScript();
    onSourceChange?.(script);
    if (onJumpToCode) {
      onJumpToCode(script, 1);
    } else {
      mode = "code";
    }
  }

  // Live Results State
  let queryResults = $state([]);
  let isRunning = $state(false);
  let runError = $state(null);
  let durationMs = $state(0);

  function generateSql() {
    const projected = columns
      .filter((c) => c.show)
      .map((c) => {
        let expr = `${c.table}.${c.field}`;
        if (c.total === "Sum") expr = `SUM(${expr})`;
        else if (c.total === "Count") expr = `COUNT(${expr})`;
        else if (c.total === "Avg") expr = `AVG(${expr})`;
        else if (c.total === "Min") expr = `MIN(${expr})`;
        else if (c.total === "Max") expr = `MAX(${expr})`;

        return c.alias ? `${expr} AS "${c.alias}"` : expr;
      });

    let sql = `SELECT\n    ${projected.join(",\n    ")}\n`;
    if (selectedTables.length > 0) {
      sql += `FROM ${selectedTables[0].name}\n`;
      for (const j of joins) {
        sql += `${j.type} ${j.toTable} ON ${j.fromTable}.${j.fromField} = ${j.toTable}.${j.toField}\n`;
      }
    }

    const whereClauses = columns
      .filter((c) => c.criteria.trim())
      .map((c) => `${c.table}.${c.field} ${c.criteria}`);

    if (whereClauses.length > 0) {
      sql += `WHERE ${whereClauses.join(" AND ")}\n`;
    }

    const groupBys = columns
      .filter((c) => c.total === "Group By")
      .map((c) => `${c.table}.${c.field}`);

    if (groupBys.length > 0) {
      sql += `GROUP BY ${groupBys.join(", ")}\n`;
    }

    const orderBys = columns
      .filter((c) => c.sort !== "None")
      .map((c) => `${c.alias || `${c.table}.${c.field}`} ${c.sort === "Ascending" ? "ASC" : "DESC"}`);

    if (orderBys.length > 0) {
      sql += `ORDER BY ${orderBys.join(", ")}\n`;
    }

    sql += `LIMIT 100;`;
    return sql;
  }

  async function handleRunQuery(limit = 100) {
    mode = "results";
    isRunning = true;
    runError = null;
    queryResults = [];
    const start = performance.now();

    try {
      const targetName = (queryName || "NewQuery").replace(/[^A-Za-z0-9_]/g, "");
      const script = generateQueryBosScript();
      const isRegistered = (studioStore.catalog || []).some(
        (o) => (o.name || o.object_name) === targetName && o.object_type === "query"
      );

      if (isDirty || !isRegistered || script !== lastAppliedScript) {
        await StudioApi.applyScript(script, packageName || "bos.app");
        lastAppliedScript = script;
        isDirty = false;
      }

      // Execute live query via Axum endpoint POST /api/queries/run
      const res = await StudioApi.runQuery(targetName, limit);

      durationMs = Math.round(performance.now() - start);
      queryResults = Array.isArray(res) ? res : res.rows || res.records || res.data || [];
    } catch (err) {
      durationMs = Math.round(performance.now() - start);
      runError = err.message || "Failed to execute query";
    } finally {
      isRunning = false;
    }
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden">
  <Toolbar label="Query designer">
    <Tabs
      value={mode === "code" ? "design" : mode}
      variant="segmented"
      items={[
        { id: "design", label: "Design" },
        { id: "results", label: "Results" },
        { id: "sql", label: "SQL" },
      ]}
      onValueChange={(next) => {
        if (next === "results") handleRunQuery();
        else mode = next;
      }}
    />
    <Button size="xs" variant="ghost" icon={FileCode} onclick={switchToCodeView}>
      Code
    </Button>
    <span class="flex-1"></span>
    <Button size="xs" icon={Play} loading={isRunning} onclick={handleRunQuery}>
      {isRunning ? "Executing..." : "Run query"}
    </Button>
  </Toolbar>

  <!-- Main Canvas -->
  <div class="flex-1 flex flex-col min-h-0">
    {#if mode === "design"}
      <!-- DESIGN VIEW: Upper Join Canvas + Lower QBE Matrix -->
      <QueryJoinCanvas
        bind:selectedTables
        bind:joins
        onFieldSelect={(tbl, fld) => {
          columns = [
            ...columns,
            {
              id: Math.random().toString(36).substring(2, 9),
              field: fld,
              table: tbl,
              alias: "",
              total: "Group By",
              sort: "None",
              show: true,
              criteria: "",
              orCriteria: "",
            },
          ];
        }}
      />
      <QueryGridMatrix
        bind:columns
        defaultTable={selectedTables[0]?.name || ""}
      />
    {:else if mode === "results"}
      <!-- RESULTS DATASHEET VIEW -->
      <div class="flex-1 flex flex-col min-h-0 bg-white dark:bg-slate-950 overflow-auto p-4">
        {#if isRunning}
          <div class="py-12 text-center text-xs text-slate-400 space-y-2">
            <Spinner class="mx-auto" />
            <p>Running query against PostgreSQL...</p>
          </div>
        {:else if runError}
          <div class="p-4">
            <Banner tone="danger" title="Query failed">{runError}</Banner>
          </div>
        {:else if queryResults.length === 0}
          <EmptyState title="No rows" description="The query returned nothing. Check the criteria or the tables." />
        {:else}
          <table class="w-full text-left border-collapse text-xs font-mono">
            <thead class="sticky top-0 bg-slate-100 dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-300">
              <tr>
                {#each Object.keys(queryResults[0]) as key}
                  <th class="py-2 px-3 border-r border-slate-200 dark:border-slate-800">{key}</th>
                {/each}
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100 dark:divide-slate-900">
              {#each queryResults as row}
                <tr class="hover:bg-slate-50 dark:hover:bg-slate-900/40">
                  {#each Object.values(row) as val}
                    <td class="py-1.5 px-3 border-r border-slate-100 dark:border-slate-900 text-slate-800 dark:text-slate-200">
                      {val !== null && val !== undefined ? String(val) : ""}
                    </td>
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      </div>
    {:else}
      <!-- SQL VIEW -->
      <div class="flex-1 p-4 bg-slate-950 text-slate-200 font-mono text-xs overflow-auto">
        <div class="pb-2 mb-3 border-b border-slate-800 text-slate-400">
          Generated PostgreSQL Query:
        </div>
        <pre class="text-sky-400 leading-relaxed">{generateSql()}</pre>
      </div>
    {/if}
  </div>
</div>
