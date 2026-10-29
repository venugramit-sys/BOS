<script>
  /**
   * PermissionSetDesigner.svelte (Svelte 5 Runes)
   * Interactive Visual RIMDX Permission Matrix & Row-Level Security (RLS) Filter Designer
   * Bi-directionally synced with BOS-Script `permissionset` AST.
   */
  import { Shield, Plus, Trash2, Code2, Play } from "lucide-svelte";
  import { Button, Input, Badge, Banner } from "$lib/bos-ui/index.js";
  import { StudioApi } from "../stores/studioApi.js";

  let {
    permissionSetName = "",
    sourceCode = "",
    packageName = "bos.app",
    mode = $bindable("design"),
    onSourceChange = null,
    onJumpToCode = null,
  } = $props();

  let caption = $state("Sales Clerk Permissions");
  let rules = $state([
    { entity: "Customer", read: true, insert: true, modify: true, delete: false, execute: false, rlsFilter: "blocked == false", exclude: "" },
    { entity: "SalesHeader", read: true, insert: true, modify: true, delete: false, execute: true, rlsFilter: 'status == "Open"', exclude: "creditLimit" },
  ]);

  let isApplying = $state(false);
  let statusBanner = $state(null);

  function parseFromSource(src) {
    if (!src || !src.trim()) return;
    const capMatch = src.match(/caption\s*[:=]\s*"([^"]*)"/);
    if (capMatch) caption = capMatch[1];

    const parsedRules = [];
    const ruleRegex =
      /(?:(tabledata|codeunit|page|query|table)\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([RIMDXrimdx]+)(?:\s*(?:\{([^}]*)\}|where\s*(?:\(([^)]*)\))))?\s*;/g;
    let m;
    while ((m = ruleRegex.exec(src)) !== null) {
      const ent = m[2];
      if (ent === "caption" || ent === "permissions" || ent === "execute") continue;
      const rights = m[3].toUpperCase();
      const blockBody = m[4] || "";
      const directWhere = (m[5] || "").trim();

      let rlsFilter = directWhere;
      let exclude = "";

      if (blockBody) {
        const exMatch = blockBody.match(/exclude(?:Fields)?\s*[:=]\s*\[([^\]]*)\]/i);
        if (exMatch) {
          exclude = exMatch[1].trim();
        }
        const whereMatch = blockBody.match(/where\s*(?:\(([^)]*)\)|\{([^}]*)\})/i);
        if (whereMatch) {
          rlsFilter = (whereMatch[1] || whereMatch[2] || "").trim();
        }
      }

      parsedRules.push({
        entity: ent,
        read: rights.includes("R"),
        insert: rights.includes("I"),
        modify: rights.includes("M"),
        delete: rights.includes("D"),
        execute: rights.includes("X"),
        rlsFilter,
        exclude,
      });
    }
    if (parsedRules.length > 0) rules = parsedRules;
  }

  $effect(() => {
    parseFromSource(sourceCode);
  });

  function syncToSource() {
    const psName = permissionSetName || "SalesClerk";
    const lines = [
      `permissionset ${psName} {`,
      `    caption = "${caption.replace(/"/g, '\\"')}";`,
    ];
    for (const r of rules) {
      let mask = "";
      if (r.read) mask += "R";
      if (r.insert) mask += "I";
      if (r.modify) mask += "M";
      if (r.delete) mask += "D";
      if (r.execute) mask += "X";
      if (!mask) mask = "R";

      const hasExclude = Boolean(r.exclude && r.exclude.trim());
      const hasFilter = Boolean(r.rlsFilter && r.rlsFilter.trim());

      if (hasExclude) {
        lines.push(`    tabledata ${r.entity} = ${mask} {`);
        const exList = r.exclude.split(',').map(s => s.trim()).filter(Boolean).join(', ');
        lines.push(`        exclude: [${exList}];`);
        if (hasFilter) {
          const cleanWhere = r.rlsFilter.trim().replace(/^\(|\)$/g, "");
          lines.push(`        where (${cleanWhere});`);
        }
        lines.push(`    };`);
      } else if (hasFilter) {
        const cleanWhere = r.rlsFilter.trim().replace(/^\(|\)$/g, "");
        lines.push(`    tabledata ${r.entity} = ${mask} where (${cleanWhere});`);
      } else {
        lines.push(`    tabledata ${r.entity} = ${mask};`);
      }
    }
    lines.push("}");
    const generated = lines.join("\n");
    if (onSourceChange) onSourceChange(generated);
    return generated;
  }

  function addRule() {
    rules = [
      ...rules,
      { entity: "NewEntity", read: true, insert: false, modify: false, delete: false, execute: false, rlsFilter: "", exclude: "" },
    ];
    syncToSource();
  }

  function removeRule(idx) {
    rules = rules.filter((_, i) => i !== idx);
    syncToSource();
  }

  async function handleApply() {
    const script = syncToSource();
    isApplying = true;
    statusBanner = null;
    try {
      const res = await StudioApi.applyScript(script, packageName);
      if (res && (res.status === "success" || res.success !== false)) {
        statusBanner = { tone: "success", message: "PermissionSet compiled and applied to database!" };
      } else {
        statusBanner = { tone: "danger", message: res?.message || "Compilation error" };
      }
    } catch (e) {
      statusBanner = { tone: "danger", message: e.message };
    } finally {
      isApplying = false;
    }
  }
</script>

<div class="h-full flex flex-col bg-white dark:bg-slate-950 rounded-lg border border-slate-200 dark:border-slate-800 overflow-hidden">
  <div class="px-4 py-2.5 border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <Shield class="size-4 text-emerald-500" />
      <span class="font-semibold text-sm">{permissionSetName || "PermissionSet"}</span>
      <Badge tone="success">{rules.length} Entity Grants</Badge>
    </div>
    <div class="flex items-center gap-2">
      <Button
        size="xs"
        variant="secondary"
        icon={Code2}
        onclick={() => {
          const script = syncToSource();
          if (onJumpToCode) onJumpToCode(script, 1);
          else mode = "code";
        }}
      >
        Source (.bos)
      </Button>
      <Button size="xs" icon={Play} loading={isApplying} onclick={handleApply}>
        Apply F5
      </Button>
    </div>
  </div>

  {#if statusBanner}
    <div class="p-3 pb-0">
      <Banner tone={statusBanner.tone} title={statusBanner.tone === "success" ? "Applied" : "Error"}>
        {statusBanner.message}
      </Banner>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto p-4 space-y-4">
    <Input label="Role Caption" bind:value={caption} onchange={syncToSource} />

    <div class="flex items-center justify-between">
      <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-500">RIMDX Matrix & Row-Level Security Filters</h3>
      <Button size="xs" variant="secondary" icon={Plus} onclick={addRule}>Add Entity Grant</Button>
    </div>

    <div class="rounded-lg border border-slate-200 dark:border-slate-800 overflow-hidden">
      <table class="w-full text-left text-xs">
        <thead class="bg-slate-100 dark:bg-slate-900 text-slate-600 dark:text-slate-400">
          <tr>
            <th class="px-3 py-2">Target Table / Codeunit</th>
            <th class="px-2 py-2 text-center">Read (R)</th>
            <th class="px-2 py-2 text-center">Insert (I)</th>
            <th class="px-2 py-2 text-center">Modify (M)</th>
            <th class="px-2 py-2 text-center">Delete (D)</th>
            <th class="px-2 py-2 text-center">Execute (X)</th>
            <th class="px-3 py-2">Row-Level Security (RLS) Filter</th>
            <th class="px-3 py-2">Field Redaction (exclude)</th>
            <th class="px-3 py-2 w-10"></th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-200 dark:divide-slate-800">
          {#each rules as rule, idx}
            <tr>
              <td class="px-3 py-2">
                <input
                  type="text"
                  bind:value={rule.entity}
                  onchange={syncToSource}
                  class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                />
              </td>
              <td class="px-2 py-2 text-center">
                <input type="checkbox" bind:checked={rule.read} onchange={syncToSource} />
              </td>
              <td class="px-2 py-2 text-center">
                <input type="checkbox" bind:checked={rule.insert} onchange={syncToSource} />
              </td>
              <td class="px-2 py-2 text-center">
                <input type="checkbox" bind:checked={rule.modify} onchange={syncToSource} />
              </td>
              <td class="px-2 py-2 text-center">
                <input type="checkbox" bind:checked={rule.delete} onchange={syncToSource} />
              </td>
              <td class="px-2 py-2 text-center">
                <input type="checkbox" bind:checked={rule.execute} onchange={syncToSource} />
              </td>
              <td class="px-3 py-2">
                <input
                  type="text"
                  bind:value={rule.rlsFilter}
                  onchange={syncToSource}
                  placeholder='e.g. status == "Open"'
                  class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                />
              </td>
              <td class="px-3 py-2">
                <input
                  type="text"
                  bind:value={rule.exclude}
                  onchange={syncToSource}
                  placeholder="e.g. creditLimit, bankAccount"
                  class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                />
              </td>
              <td class="px-3 py-2 text-right">
                <button type="button" onclick={() => removeRule(idx)} class="text-slate-400 hover:text-rose-500">
                  <Trash2 class="size-3.5" />
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </div>
</div>
