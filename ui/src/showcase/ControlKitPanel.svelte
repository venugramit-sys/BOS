<script>
  import { Folder, Plus, Table, Play, Lock, ShieldCheck, Key, Building2, RotateCcw, CheckCircle2, AlertTriangle, Cpu } from "lucide-svelte";
  import {
    Badge,
    Banner,
    Button,
    Checkbox,
    CommandPalette,
    ConfirmDialog,
    ContextMenu,
    DataGrid,
    Dialog,
    EmptyState,
    Field,
    Input,
    Kbd,
    Select,
    SplitPane,
    StatusBar,
    Switch,
    Tabs,
    Textarea,
    Toolbar,
    ToastViewport,
    Tree,
    toastStore
  } from "$lib/bos-ui/index.js";

  // Argon2id verification showcase state
  let rawPassword = $state("AdminMaster#2026!");
  let candidatePassword = $state("AdminMaster#2026!");
  let simulatedArgonHash = $derived(
    rawPassword
      ? `$argon2id$v=19$m=65536,t=3,p=4$c29tZXNhbHQxMjM0NQ$${btoa(rawPassword).slice(0, 24)}...`
      : "$argon2id$v=19$m=65536,t=3,p=4$..."
  );
  let isArgonVerified = $derived(
    rawPassword.length > 0 && rawPassword === candidatePassword
  );

  // Dynamic Session Roles state
  let selectedSessionRole = $state("SUPER");
  const ROLE_CAPABILITIES = {
    SUPER: {
      badge: "Master Admin",
      tone: "danger",
      description: "Universal administrative override. Bypass RLS filters, author codeunits, immutable system table access. Cannot be deleted or revoked (uid === 'admin').",
      canDeleteRole: false,
      bypassRls: true,
      canPostLedger: true,
      canAudit: true
    },
    Admin: {
      badge: "Tenant Admin",
      tone: "warning",
      description: "Business tenant administrator. Manages schemas, entities, and packages within the designated _tenant_id.",
      canDeleteRole: true,
      bypassRls: false,
      canPostLedger: true,
      canAudit: true
    },
    SecurityAdmin: {
      badge: "Security & IAM",
      tone: "info",
      description: "Identity and security governance. Manages User accounts, Role assignments, and PermissionSet RIMDX policies.",
      canDeleteRole: true,
      bypassRls: false,
      canPostLedger: false,
      canAudit: true
    }
  };

  // Multi-Company Isolation state
  let activeCompanyId = $state("COMP-001");
  const COMPANIES = [
    { id: "COMP-001", name: "CRONUS US Inc.", currency: "USD", region: "North America" },
    { id: "COMP-002", name: "CRONUS UK Ltd.", currency: "GBP", region: "Europe / London" },
    { id: "COMP-003", name: "CRONUS Holding GmbH", currency: "EUR", region: "DACH / Frankfurt" }
  ];

  // SAP S/4HANA Storno Reversal state
  let stornoReversalStatus = $state("Reversed");

  let notNull = $state(true);
  let editable = $state(true);
  let onDelete = $state("Restrict");
  let rule = $state(">= 0");
  let view = $state("design");
  let doc = $state("customer");
  let documents = $state([
    { id: "customer", label: "Customer", icon: Table, dirty: true, closable: true },
    { id: "order", label: "SalesOrder", icon: Table, dirty: false, closable: true }
  ]);
  let dialogOpen = $state(false);
  let confirmOpen = $state(false);
  let paletteOpen = $state(false);
  let split = $state(240);
  let expanded = $state(["sales"]);
  let selectedId = $state("customer");
  let gridRows = $state([
    { id: "1", name: "code", type: "String(20)", notNull: true, posted: "2026-10-14" },
    { id: "2", name: "amount", type: "Decimal", notNull: true, posted: "2026-10-01" }
  ]);
  let gridLog = $state("");

  const treeNodes = [
    {
      id: "sales",
      label: "bos.sales",
      icon: Folder,
      badge: "2",
      children: [
        { id: "customer", label: "Customer", icon: Table, badge: "table", badgeTone: "info" },
        { id: "order", label: "SalesOrder", icon: Table }
      ]
    }
  ];

  const commands = [
    {
      heading: "Objects",
      items: [
        { id: "open-customer", label: "Open Customer", keywords: ["table"], shortcut: "Enter", action: () => (selectedId = "customer") },
        { id: "open-order", label: "Open SalesOrder", keywords: ["table"], action: () => (selectedId = "order") }
      ]
    },
    {
      heading: "Commands",
      items: [
        { id: "apply", label: "Apply to database", shortcut: "F5", icon: Play, action: () => toastStore.push({ tone: "success", title: "Applied", description: "Schema is ready to deploy." }) }
      ]
    }
  ];

  let documentItems = $derived(documents);

  function closeDoc(id) {
    documents = documents.filter((item) => item.id !== id);
    if (doc === id) doc = documents[0]?.id || "";
  }
</script>

<ToastViewport />
<CommandPalette bind:open={paletteOpen} groups={commands} recentKey="bos.showcase.recent" />

<div class="space-y-4">
  <div>
    <h2 class="text-xl font-bold text-slate-900 dark:text-slate-100">Studio control kit</h2>
    <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
      Wave A controls plus the surfaces Dev Studio needs: document tabs, command palette, tree, datasheet, and split panes. Built on bits-ui. The windowed sheet below is DataGrid. List pages with search, filters, grouping, and paging are on the Data table page.
    </p>
  </div>

  <Toolbar label="Table designer">
    <Button size="xs" icon={Plus} onclick={() => (dialogOpen = true)}>New</Button>
    <Button size="xs" variant="secondary" showShortcut shortcut="F5" icon={Play} onclick={() => toastStore.push({ tone: "success", title: "Apply queued" })}>
      Apply
    </Button>
    <Button size="xs" variant="ghost" tooltip="Command palette" shortcut="Ctrl+K" onclick={() => (paletteOpen = true)}>
      Commands
    </Button>
    <span class="mx-1 h-4 w-px bg-slate-200 dark:bg-slate-700"></span>
    <Tabs
      bind:value={view}
      variant="segmented"
      items={[
        { id: "design", label: "Design" },
        { id: "datasheet", label: "Datasheet" },
        { id: "code", label: "Code" }
      ]}
    />
  </Toolbar>

  <Tabs bind:value={doc} variant="document" items={documentItems} onClose={closeDoc} />

  <div class="h-[28rem] rounded-lg border border-slate-200 dark:border-slate-800 overflow-hidden">
    <SplitPane bind:size={split} defaultSize={240} min={160} max={420} class="h-full">
      {#snippet primary()}
        <ContextMenu
          items={[
            { label: "Design view", action: () => (view = "design") },
            { label: "Code", shortcut: "Ctrl+Enter", action: () => (view = "code") },
            { type: "separator" },
            { label: "Delete", destructive: true, action: () => (confirmOpen = true) }
          ]}
        >
          <Tree
            nodes={treeNodes}
            bind:expanded
            bind:selectedId
            label="Meta-objects"
            onActivate={(node) => {
              if (!node.children) doc = node.id;
            }}
          />
        </ContextMenu>
      {/snippet}
      {#snippet secondary()}
        <div class="h-full flex flex-col">
          <div class="flex-1 min-h-0 p-2">
            <DataGrid
              label="Customer fields"
              bind:rows={gridRows}
              onCellChange={(row, key, value) => (gridLog = `${row.name}.${key} = ${value}`)}
              columns={[
                { key: "name", label: "Field", width: "28%" },
                {
                  key: "type",
                  label: "Type",
                  editor: "select",
                  width: "24%",
                  options: [
                    { value: "String(20)", label: "String(20)" },
                    { value: "Decimal", label: "Decimal" },
                    { value: "Date", label: "Date" }
                  ]
                },
                { key: "notNull", label: "Not null", editor: "checkbox", width: "16%" },
                { key: "posted", label: "Posted", editor: "date" }
              ]}
            />
          </div>
          <StatusBar
            leading={[
              { id: "db", label: "bos_dev", tone: "success" },
              { id: "pkg", label: "bos.sales" },
              { id: "log", label: gridLog || "No pending cell edits" }
            ]}
            trailing={[{ id: "problems", label: "0 problems", tone: "success", onClick: () => toastStore.push({ tone: "info", title: "No problems" }) }]}
          />
        </div>
      {/snippet}
    </SplitPane>
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <div class="space-y-3 rounded-xl border border-slate-200 dark:border-slate-800 p-4">
      <Field label="Validation rule" hint="Example: quantity must stay non-negative.">
        <Textarea bind:value={rule} mono rows={2} />
      </Field>
      <Select
        label="On delete"
        bind:value={onDelete}
        options={[
          { value: "Restrict", label: "Restrict" },
          { value: "Cascade", label: "Cascade" },
          { value: "SetNull", label: "Set null" }
        ]}
      />
      <div class="flex items-center gap-4">
        <Checkbox bind:checked={notNull} label="Not null" />
        <Switch bind:checked={editable} label="Editable" />
      </div>
      <div class="flex items-center gap-2 text-[11px] text-slate-400">
        <span>Shortcut</span>
        <Kbd keys="Ctrl+K" />
        <Badge tone="success" dot>Compiled</Badge>
      </div>
    </div>
    <div class="space-y-3">
      <Banner tone="success" title="Schema applied" dismissible>Customer is provisioned.</Banner>
      <EmptyState icon={Folder} title="No page open" description="Create a meta-object or pick one from the tree.">
        <Button size="sm" onclick={() => (dialogOpen = true)}>Create</Button>
      </EmptyState>
    </div>
  </div>

  <!-- ========================================================================= -->
  <!-- ENTERPRISE SECURITY, IDENTITY & LEDGER INTEGRITY SHOWCASE                 -->
  <!-- ========================================================================= -->
  <div class="pt-6 border-t border-slate-200 dark:border-slate-800 space-y-6">
    <div>
      <div class="flex items-center gap-2">
        <ShieldCheck class="size-5 text-indigo-600 dark:text-indigo-400" />
        <h3 class="text-base font-bold text-slate-900 dark:text-slate-100">
          Enterprise Security, Identity &amp; Ledger Integrity Updates
        </h3>
        <Badge tone="success">Production Hardened</Badge>
      </div>
      <p class="text-xs text-slate-500 dark:text-slate-400 mt-0.5">
        Demonstrates Argon2id password hashing &amp; verification, dynamic session roles (SUPER / Admin / SecurityAdmin), multi-company isolation (_company_id), and SAP S/4HANA Storno ledger reversal with transaction-local privileged overrides.
      </p>
    </div>

    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- 1. Argon2id Password Verification Simulator -->
      <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 p-4 space-y-3 shadow-2xs">
        <div class="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <Lock class="size-4 text-emerald-500" />
            <span class="font-bold text-xs text-slate-900 dark:text-slate-100">Argon2id Password Verification</span>
          </div>
          <Badge tone={isArgonVerified ? "success" : "danger"}>
            {isArgonVerified ? "Verified Match" : "Mismatch"}
          </Badge>
        </div>

        <p class="text-[11px] text-slate-500 dark:text-slate-400">
          Replaced single-round SHA-256 with Argon2id in Rust runtime. Resistant to GPU brute-forcing and timing side-channel attacks.
        </p>

        <div class="space-y-2">
          <Input label="Stored User Password" type="password" bind:value={rawPassword} />
          <Input label="Verification Candidate" type="password" bind:value={candidatePassword} />
        </div>

        <div class="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-1 font-mono text-[11px]">
          <div class="text-slate-400 text-[10px] uppercase font-sans font-semibold">PHC Formatted Hash:</div>
          <div class="text-emerald-600 dark:text-emerald-400 break-all select-all">{simulatedArgonHash}</div>
          <div class="flex flex-wrap gap-2 pt-1 text-[10px] text-slate-500 font-sans">
            <span>m=65536 (64MB)</span>
            <span>•</span>
            <span>t=3 (Iterations)</span>
            <span>•</span>
            <span>p=4 (Parallelism)</span>
          </div>
        </div>
      </div>

      <!-- 2. Dynamic Session Roles (SUPER, Admin, SecurityAdmin) -->
      <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 p-4 space-y-3 shadow-2xs">
        <div class="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <Key class="size-4 text-indigo-500" />
            <span class="font-bold text-xs text-slate-900 dark:text-slate-100">Dynamic Session Roles &amp; RBAC</span>
          </div>
          <Badge tone={ROLE_CAPABILITIES[selectedSessionRole].tone}>
            {ROLE_CAPABILITIES[selectedSessionRole].badge}
          </Badge>
        </div>

        <div class="flex items-center gap-1.5 p-1 rounded-lg bg-slate-100 dark:bg-slate-900 text-xs">
          {#each ["SUPER", "Admin", "SecurityAdmin"] as role}
            <button
              type="button"
              onclick={() => (selectedSessionRole = role)}
              class="flex-1 py-1 px-2 rounded-md font-medium text-center transition-colors cursor-pointer {selectedSessionRole === role ? 'bg-white dark:bg-slate-800 text-indigo-600 dark:text-indigo-400 shadow-2xs font-bold' : 'text-slate-600 dark:text-slate-400 hover:text-slate-900'}"
            >
              {role}
            </button>
          {/each}
        </div>

        <p class="text-xs text-slate-600 dark:text-slate-300 leading-relaxed">
          {ROLE_CAPABILITIES[selectedSessionRole].description}
        </p>

        <div class="grid grid-cols-2 gap-2 text-[11px] pt-1">
          <div class="p-2 rounded bg-slate-50 dark:bg-slate-900/60 border border-slate-200 dark:border-slate-800 flex justify-between items-center">
            <span>RLS Bypass Privileges:</span>
            <Badge tone={ROLE_CAPABILITIES[selectedSessionRole].bypassRls ? "success" : "neutral"}>
              {ROLE_CAPABILITIES[selectedSessionRole].bypassRls ? "Enabled" : "Scoped"}
            </Badge>
          </div>
          <div class="p-2 rounded bg-slate-50 dark:bg-slate-900/60 border border-slate-200 dark:border-slate-800 flex justify-between items-center">
            <span>Role Revocation:</span>
            <Badge tone={ROLE_CAPABILITIES[selectedSessionRole].canDeleteRole ? "neutral" : "danger"}>
              {ROLE_CAPABILITIES[selectedSessionRole].canDeleteRole ? "Allowed" : "Protected"}
            </Badge>
          </div>
        </div>
      </div>

      <!-- 3. Multi-Company Data Isolation (_company_id) -->
      <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 p-4 space-y-3 shadow-2xs">
        <div class="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <Building2 class="size-4 text-amber-500" />
            <span class="font-bold text-xs text-slate-900 dark:text-slate-100">Multi-Company Partitioning (_company_id)</span>
          </div>
          <Badge tone="info">Tenant Partition</Badge>
        </div>

        <div class="flex items-center gap-2">
          <span class="text-xs text-slate-400">Active Company:</span>
          <div class="flex-1">
            <Select
              bind:value={activeCompanyId}
              options={COMPANIES.map((c) => ({ value: c.id, label: `${c.id} - ${c.name} (${c.currency})` }))}
            />
          </div>
        </div>

        <div class="p-2.5 rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 space-y-1 font-mono text-[11px]">
          <div class="text-slate-400 text-[10px] uppercase font-sans font-semibold">PostgreSQL Engine RLS Predicate:</div>
          <div class="text-indigo-600 dark:text-indigo-400 leading-tight">
            SELECT * FROM "customer"<br />
            WHERE "_tenant_id" = current_setting('bos.current_tenant_id')<br />
            &nbsp;&nbsp;AND "_company_id" = '{activeCompanyId}';
          </div>
        </div>
      </div>

      <!-- 4. SAP S/4HANA Storno Ledger Reversal Indicators -->
      <div class="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 p-4 space-y-3 shadow-2xs">
        <div class="flex items-center justify-between pb-2 border-b border-slate-100 dark:border-slate-800">
          <div class="flex items-center gap-2">
            <RotateCcw class="size-4 text-rose-500" />
            <span class="font-bold text-xs text-slate-900 dark:text-slate-100">SAP S/4HANA Storno Ledger Reversal</span>
          </div>
          <Badge tone="danger">Storno Override</Badge>
        </div>

        <p class="text-[11px] text-slate-500 dark:text-slate-400">
          Complies with continental European and SAP ledger accounting: original posted rows remain immutable; offsetting entry swaps debit and credit to balance to zero net change.
        </p>

        <div class="space-y-1.5 text-xs">
          <!-- Original Entry -->
          <div class="p-2 rounded bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex items-center justify-between font-mono text-[11px]">
            <div>
              <span class="font-bold text-slate-700 dark:text-slate-300">Reg #11 / Entry #201</span>
              <span class="text-slate-400 block text-[10px]">Doc: INV-10042 • Debit: $1,500.00</span>
            </div>
            <Badge tone="danger">Reversed by #204</Badge>
          </div>

          <!-- Storno Reversing Entry -->
          <div class="p-2 rounded bg-rose-50/50 dark:bg-rose-950/20 border border-rose-200 dark:border-rose-900/40 flex items-center justify-between font-mono text-[11px]">
            <div>
              <span class="font-bold text-rose-700 dark:text-rose-300">Reg #12 / Entry #204 (Storno)</span>
              <span class="text-slate-400 block text-[10px]">Doc: REV-INV-10042 • Credit: $1,500.00 (Swap)</span>
            </div>
            <Badge tone="success">Balanced 0.00 Net</Badge>
          </div>
        </div>

        <div class="text-[10px] font-mono text-slate-400 bg-slate-100 dark:bg-slate-900 p-1.5 rounded">
          SET LOCAL bos.privileged_reason = 'SAP S/4HANA Storno Ledger Reversal';
        </div>
      </div>
    </div>
  </div>
</div>

<Dialog bind:open={dialogOpen} title="Create meta-object" description="Singular PascalCase name. The package supplies the namespace." size="sm">
  {#snippet children()}
    <Field label="Object name" required>
      <input class="h-8 w-full rounded-md border border-slate-200 px-2 text-xs dark:border-slate-700 dark:bg-slate-950" placeholder="Customer" />
    </Field>
  {/snippet}
  {#snippet footer()}
    <Button variant="ghost" size="sm" onclick={() => (dialogOpen = false)}>Cancel</Button>
    <Button size="sm" onclick={() => { dialogOpen = false; toastStore.push({ tone: "info", title: "Draft opened" }); }}>Create</Button>
  {/snippet}
</Dialog>

<ConfirmDialog
  bind:open={confirmOpen}
  tone="danger"
  title="Delete this object?"
  description="The meta-object is removed from the catalog. Posted ledger rows stay in the database."
  confirmLabel="Delete"
  onConfirm={() => toastStore.push({ tone: "warning", title: "Delete confirmed" })}
/>
