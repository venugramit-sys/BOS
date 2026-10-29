<script>
  /**
   * StateMachineDesigner.svelte (Svelte 5 Runes)
   * Interactive Visual State Machine Graph & Transition Matrix Designer
   * Bi-directionally synced with BOS-Script `statemachine` / `statemachineextension` AST.
   */
  import { Workflow, Plus, Trash2, Code2, Play, ArrowRight, ShieldCheck } from "lucide-svelte";
  import { Button, Input, Badge, Banner } from "$lib/bos-ui/index.js";
  import { StudioApi } from "../stores/studioApi.js";

  let {
    stateMachineName = "",
    sourceCode = "",
    packageName = "bos.app",
    mode = $bindable("design"),
    onSourceChange = null,
    onJumpToCode = null,
  } = $props();

  let targetTable = $state("SalesHeader");
  let statusField = $state("status");
  let states = $state([
    { name: "Draft", initial: true, final: false, allowModify: true, allowDelete: true },
    { name: "Released", initial: false, final: false, allowModify: false, allowDelete: false },
    { name: "Posted", initial: false, final: true, allowModify: false, allowDelete: false },
  ]);
  let transitions = $state([
    { from: "Draft", to: "Released", action: "release", roles: "SalesManager", guard: "rec.amount > 0" },
    { from: "Released", to: "Posted", action: "post", roles: "", guard: "" },
  ]);

  let isApplying = $state(false);
  let statusBanner = $state(null);
  let selectedState = $state(null);

  // Compute layout coordinates for SVG workflow diagram
  let statePositions = $derived.by(() => {
    const coords = {};
    const count = states.length;
    if (count === 0) return coords;
    const startX = 70;
    const stepX = 160;
    const baseY = 80;
    states.forEach((s, idx) => {
      coords[s.name] = {
        x: startX + idx * stepX,
        y: baseY + (idx % 2 === 1 ? 40 : -20),
      };
    });
    return coords;
  });

  let svgWidth = $derived.by(() => {
    return Math.max(700, states.length * 170 + 80);
  });

  function parseFromSource(src) {
    if (!src || !src.trim()) return;
    const headerMatch = src.match(
      /statemachine(?:extension)?\s+[A-Za-z0-9_]+(?:\s+(?:for|on|extends)\s+([A-Za-z0-9_]+)(?:\.([A-Za-z0-9_]+))?)?/
    );
    if (headerMatch) {
      if (headerMatch[1]) targetTable = headerMatch[1];
      if (headerMatch[2]) statusField = headerMatch[2];
    }
    const sfMatch = src.match(/field\s*:\s*([A-Za-z0-9_]+)/);
    if (sfMatch && sfMatch[1]) {
      statusField = sfMatch[1];
    }

    const parsedStates = [];

    // Support states: [Draft, PendingApproval, Approved];
    const statesArrayMatch = src.match(/states\s*:\s*\[([^\]]+)\]/);
    if (statesArrayMatch) {
      const names = statesArrayMatch[1].split(',').map(s => s.trim()).filter(Boolean);
      for (let i = 0; i < names.length; i++) {
        parsedStates.push({
          name: names[i],
          initial: i === 0,
          final: i === names.length - 1,
          allowModify: i === 0,
          allowDelete: i === 0,
        });
      }
    }

    const stateRegex = /(?:(initial|final)\s+)?state\s+([A-Za-z0-9_]+)\s*(?:\{([^}]*)\}|;)/g;
    let sm;
    while ((sm = stateRegex.exec(src)) !== null) {
      const modifier = (sm[1] || "").toLowerCase();
      const sName = sm[2];
      const sBody = sm[3] || "";
      const isInit = modifier === "initial" || /initial\s*[:=]\s*true/i.test(sBody);
      const isFinal = modifier === "final" || /final\s*[:=]\s*true/i.test(sBody);
      const modMatch = sBody.match(/allowModify\s*[:=]\s*(true|false)/i);
      const delMatch = sBody.match(/allowDelete\s*[:=]\s*(true|false)/i);
      if (!parsedStates.some(s => s.name === sName)) {
        parsedStates.push({
          name: sName,
          initial: isInit,
          final: isFinal,
          allowModify: modMatch ? modMatch[1].toLowerCase() === "true" : isInit,
          allowDelete: delMatch ? delMatch[1].toLowerCase() === "true" : isInit,
        });
      }
    }

    const parsedTransitions = [];
    const trHeaderRegex =
      /transition\s+([A-Za-z0-9_]+)\s*(?:from\s+([A-Za-z0-9_*]+)\s+to\s+([A-Za-z0-9_]+)|:\s*([A-Za-z0-9_*]+)\s*->\s*([A-Za-z0-9_]+))\s*(?:\{([^}]*)\}|;)/g;
    let tm;
    while ((tm = trHeaderRegex.exec(src)) !== null) {
      const action = tm[1];
      const from = tm[2] || tm[4];
      const to = tm[3] || tm[5];
      const body = tm[6] || "";

      const rolesMatch = body.match(/roles\s*[:=]\s*\[([^\]]*)\]/);
      const fnGuardMatch = body.match(/guard\s*\([^)]*\)\s*\{\s*(?:return\s+)?([^;}\n]+)/);
      const propGuardMatch = body.match(/guard\s*[:=]\s*([^;\n]+)/);
      const rawGuard = fnGuardMatch
        ? fnGuardMatch[1].trim()
        : propGuardMatch
          ? propGuardMatch[1].trim().replace(/^["']|["']$/g, "")
          : "";

      if (!parsedTransitions.some(t => t.action === action && t.from === from && t.to === to)) {
        parsedTransitions.push({
          action,
          from,
          to,
          roles: rolesMatch ? rolesMatch[1].trim() : "",
          guard: rawGuard,
        });
      }
    }

    if (parsedStates.length > 0) states = parsedStates;
    if (parsedTransitions.length > 0) transitions = parsedTransitions;
  }

  // Parse incoming BOS-Script `statemachine` source into states & transitions
  $effect(() => {
    parseFromSource(sourceCode);
  });

  function syncToSource() {
    const smName = stateMachineName || "OrderWorkflow";
    const lines = [`statemachine ${smName} for ${targetTable}.${statusField} {`];
    for (const s of states) {
      const prefix = s.initial ? "initial " : s.final ? "final " : "";
      const allowMod = s.allowModify !== undefined ? Boolean(s.allowModify) : Boolean(s.initial);
      const allowDel = s.allowDelete !== undefined ? Boolean(s.allowDelete) : Boolean(s.initial);
      lines.push(`    ${prefix}state ${s.name} {`);
      lines.push(`        allowModify: ${allowMod};`);
      lines.push(`        allowDelete: ${allowDel};`);
      lines.push(`    }`);
    }
    for (const tr of transitions) {
      const hasRoles = Boolean(tr.roles && tr.roles.trim());
      const hasGuard = Boolean(tr.guard && tr.guard.trim());
      if (!hasRoles && !hasGuard) {
        lines.push(`    transition ${tr.action} from ${tr.from} to ${tr.to} {}`);
      } else {
        lines.push(`    transition ${tr.action} from ${tr.from} to ${tr.to} {`);
        if (hasRoles) {
          lines.push(`        roles: [${tr.roles.trim()}];`);
        }
        if (hasGuard) {
          const cleanExpr = tr.guard.trim().replace(/^return\s+/i, "").replace(/;$/, "");
          lines.push(`        guard(rec) {`);
          lines.push(`            return ${cleanExpr};`);
          lines.push(`        }`);
        }
        lines.push(`    }`);
      }
    }
    lines.push("}");
    const generated = lines.join("\n");
    if (onSourceChange) onSourceChange(generated);
    return generated;
  }

  function addState() {
    const isFirst = states.length === 0;
    states = [
      ...states,
      {
        name: `State${states.length + 1}`,
        initial: isFirst,
        final: false,
        allowModify: isFirst,
        allowDelete: isFirst,
      },
    ];
    syncToSource();
  }

  function removeState(idx) {
    states = states.filter((_, i) => i !== idx);
    syncToSource();
  }

  function addTransition() {
    const fromState = states[0]?.name || "Draft";
    const toState = states[1]?.name || fromState;
    transitions = [
      ...transitions,
      { action: `step_${transitions.length + 1}`, from: fromState, to: toState, roles: "", guard: "" },
    ];
    syncToSource();
  }

  function removeTransition(idx) {
    transitions = transitions.filter((_, i) => i !== idx);
    syncToSource();
  }

  async function handleApply() {
    const script = syncToSource();
    isApplying = true;
    statusBanner = null;
    try {
      const res = await StudioApi.applyScript(script, packageName);
      if (res && (res.status === "success" || res.success !== false)) {
        statusBanner = { tone: "success", message: "State Machine compiled and deployed to PostgreSQL!" };
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
  <!-- Top Toolbar -->
  <div class="px-4 py-2.5 border-b border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-900 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <Workflow class="size-4 text-indigo-500" />
      <span class="font-semibold text-sm">{stateMachineName || "StateMachine"}</span>
      <Badge tone="info">Target: {targetTable}.{statusField}</Badge>
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

  <div class="flex-1 overflow-y-auto p-4 space-y-6">
    <!-- Target Binding Config -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4 p-3 rounded-lg bg-slate-50 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-800">
      <Input
        label="Target Table"
        bind:value={targetTable}
        onchange={syncToSource}
        placeholder="SalesHeader"
      />
      <Input
        label="Status Column"
        bind:value={statusField}
        onchange={syncToSource}
        placeholder="status"
      />
    </div>

    <!-- Interactive Visual Workflow Diagram -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-500">Interactive Document Workflow Diagram</h3>
        <div class="flex items-center gap-2">
          {#if selectedState}
            <Badge tone="info">Focused: {selectedState}</Badge>
            <Button size="xs" variant="ghost" onclick={() => (selectedState = null)}>Clear Focus</Button>
          {/if}
          <Button size="xs" variant="secondary" icon={Plus} onclick={addState}>Add State</Button>
        </div>
      </div>
      <div class="rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-900/5 dark:bg-slate-900/40 p-3 overflow-x-auto">
        <svg width={svgWidth} height="170" class="select-none">
          <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
              <path d="M 0 1 L 9 5 L 0 9 z" fill="#6366F1" />
            </marker>
            <marker id="arrow-active" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
              <path d="M 0 1 L 9 5 L 0 9 z" fill="#10B981" />
            </marker>
          </defs>

          <!-- Transition Curves -->
          {#each transitions as tr}
            {@const fromP = statePositions[tr.from]}
            {@const toP = statePositions[tr.to]}
            {#if fromP && toP}
              {@const isActive = selectedState === tr.from || selectedState === tr.to}
              {@const dx = toP.x - fromP.x}
              {@const dy = toP.y - fromP.y}
              {@const mx = (fromP.x + toP.x) / 2}
              {@const my = (fromP.y + toP.y) / 2 + (dy > 0 ? -15 : 15)}
              <path
                d="M {fromP.x} {fromP.y} Q {mx} {my - 20} {toP.x} {toP.y}"
                fill="none"
                stroke={isActive ? "#10B981" : "#818CF8"}
                stroke-width={isActive ? "2.5" : "1.5"}
                stroke-dasharray={tr.guard ? "4 2" : "none"}
                marker-end={isActive ? "url(#arrow-active)" : "url(#arrow)"}
                class="transition-colors duration-200"
              />
              <g transform="translate({mx}, {my - 10})">
                <rect
                  x="-32"
                  y="-9"
                  width="64"
                  height="18"
                  rx="9"
                  class="fill-white dark:fill-slate-800 stroke {isActive ? 'stroke-emerald-500' : 'stroke-indigo-400 dark:stroke-indigo-600'}"
                  stroke-width="1"
                />
                <text
                  x="0"
                  y="3"
                  text-anchor="middle"
                  class="text-[9px] font-mono font-medium fill-slate-700 dark:fill-slate-200 pointer-events-none"
                >
                  {tr.action}
                </text>
              </g>
            {/if}
          {/each}

          <!-- State Nodes -->
          {#each states as st}
            {@const pos = statePositions[st.name]}
            {#if pos}
              {@const isSelected = selectedState === st.name}
              <g
                transform="translate({pos.x}, {pos.y})"
                class="cursor-pointer"
                role="button"
                tabindex="0"
                onclick={() => (selectedState = isSelected ? null : st.name)}
                onkeydown={(e) => {
                  if (e.key === "Enter" || e.key === " ") selectedState = isSelected ? null : st.name;
                }}
              >
                <rect
                  x="-50"
                  y="-18"
                  width="100"
                  height="36"
                  rx="18"
                  class="fill-white dark:fill-slate-900 shadow-md transition-all duration-200 {isSelected ? 'stroke-indigo-500 stroke-2' : st.initial ? 'stroke-emerald-500 stroke-2' : st.final ? 'stroke-purple-500 stroke-2' : 'stroke-slate-300 dark:stroke-slate-700 stroke-1'}"
                />
                <text
                  x="0"
                  y="1"
                  text-anchor="middle"
                  class="font-mono text-[11px] font-semibold fill-slate-800 dark:fill-slate-100 pointer-events-none"
                >
                  {st.name}
                </text>
                {#if st.initial}
                  <text x="0" y="11" text-anchor="middle" class="text-[7px] font-bold fill-emerald-600 uppercase tracking-widest pointer-events-none">Init</text>
                {:else if st.final}
                  <text x="0" y="11" text-anchor="middle" class="text-[7px] font-bold fill-purple-600 uppercase tracking-widest pointer-events-none">Final</text>
                {/if}
              </g>
            {/if}
          {/each}
        </svg>
      </div>

      <!-- State Pills & Quick Config -->
      <div class="p-3 rounded-lg border border-slate-200 dark:border-slate-800 bg-slate-50/40 dark:bg-slate-900/30 flex items-center gap-2.5 flex-wrap">
        {#each states as st, idx}
          <div class="px-3 py-1.5 rounded-lg border bg-white dark:bg-slate-900 shadow-xs flex items-center gap-2 {st.initial ? 'border-emerald-500' : st.final ? 'border-purple-500' : 'border-slate-200 dark:border-slate-700'}">
            <input
              type="text"
              bind:value={st.name}
              onchange={syncToSource}
              class="font-mono text-xs font-semibold bg-transparent border-b border-transparent focus:border-indigo-500 focus:outline-none w-20"
            />
            <label class="flex items-center gap-1 text-[10px] text-slate-500 cursor-pointer">
              <input type="checkbox" bind:checked={st.initial} onchange={syncToSource} />
              Init
            </label>
            <label class="flex items-center gap-1 text-[10px] text-slate-500 cursor-pointer">
              <input type="checkbox" bind:checked={st.final} onchange={syncToSource} />
              Final
            </label>
            <button type="button" onclick={() => removeState(idx)} class="text-slate-400 hover:text-rose-500">
              <Trash2 class="size-3.5" />
            </button>
          </div>
        {/each}
      </div>
    </div>

    <!-- Transitions & Guard Policies Matrix -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-slate-500">Transitions & Guard Policies</h3>
        <Button size="xs" variant="secondary" icon={Plus} onclick={addTransition}>Add Transition</Button>
      </div>
      <div class="rounded-lg border border-slate-200 dark:border-slate-800 overflow-hidden">
        <table class="w-full text-left text-xs">
          <thead class="bg-slate-100 dark:bg-slate-900 text-slate-600 dark:text-slate-400">
            <tr>
              <th class="px-3 py-2">Action Trigger</th>
              <th class="px-3 py-2">From State</th>
              <th class="px-3 py-2">To State</th>
              <th class="px-3 py-2">Guard Condition (BOS-Script Expression)</th>
              <th class="px-3 py-2 w-10"></th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 dark:divide-slate-800">
            {#each transitions as tr, idx}
              <tr>
                <td class="px-3 py-2">
                  <input
                    type="text"
                    bind:value={tr.action}
                    onchange={syncToSource}
                    class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                  />
                </td>
                <td class="px-3 py-2">
                  <input
                    type="text"
                    bind:value={tr.from}
                    onchange={syncToSource}
                    class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                  />
                </td>
                <td class="px-3 py-2">
                  <input
                    type="text"
                    bind:value={tr.to}
                    onchange={syncToSource}
                    class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                  />
                </td>
                <td class="px-3 py-2">
                  <div class="flex items-center gap-1.5">
                    <ShieldCheck class="size-3.5 text-emerald-500 shrink-0" />
                    <input
                      type="text"
                      bind:value={tr.guard}
                      onchange={syncToSource}
                      placeholder="e.g. rec.amount > 0"
                      class="font-mono text-xs bg-transparent border rounded px-2 py-1 border-slate-200 dark:border-slate-700 w-full"
                    />
                  </div>
                </td>
                <td class="px-3 py-2 text-right">
                  <button type="button" onclick={() => removeTransition(idx)} class="text-slate-400 hover:text-rose-500">
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
</div>
