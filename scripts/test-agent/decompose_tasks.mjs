import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

console.log('Generating BOS Feature & Module Test Task Matrix...');

const modules = [
  {
    module: 'bosc-syntax (AST & Lexer)',
    crate: 'crates/bosc-syntax',
    category: 'Compiler DSL',
    features: [
      {
        name: 'DSL Meta-Object Grammar Parsing',
        tasks: [
          { id: 'SYN-01', title: 'Parse 12 Meta-Object Declarations', type: 'UNIT', priority: 'CRITICAL', verification: 'cargo test -p bosc-syntax test_meta_object_parsing' },
          { id: 'SYN-02', title: 'Parse Non-Invasive Table & Page Extensions', type: 'UNIT', priority: 'HIGH', verification: 'cargo test -p bosc-syntax test_extension_parsing' },
          { id: 'SYN-03', title: 'Parse Field Validation Blocks validate(val)', type: 'UNIT', priority: 'HIGH', verification: 'cargo test -p bosc-syntax test_field_validation_syntax' },
          { id: 'SYN-04', title: 'Parse Declarative State Machines & Transitions', type: 'UNIT', priority: 'HIGH', verification: 'cargo test -p bosc-syntax test_statemachine_syntax' }
        ]
      }
    ]
  },
  {
    module: 'bosc-analyzer (Semantic Analyzer)',
    crate: 'crates/bosc-analyzer',
    category: 'Compiler DSL',
    features: [
      {
        name: 'Semantic Type & Symbol Checking',
        tasks: [
          { id: 'ANA-01', title: 'Validate Table Primary Key Constraints', type: 'UNIT', priority: 'CRITICAL', verification: 'cargo test -p bosc-analyzer test_pk_constraints' },
          { id: 'ANA-02', title: 'Validate FlowField Formula References & Cycle Detection', type: 'UNIT', priority: 'CRITICAL', verification: 'cargo test -p bosc-analyzer test_flowfield_cycles' },
          { id: 'ANA-03', title: 'Validate Extensible Enum Strategy Mapping', type: 'UNIT', priority: 'HIGH', verification: 'cargo test -p bosc-analyzer test_enum_extension_bindings' },
          { id: 'ANA-04', title: 'Validate Non-Invasive Field Conflict Protection', type: 'UNIT', priority: 'HIGH', verification: 'cargo test -p bosc-analyzer test_field_name_collisions' }
        ]
      }
    ]
  },
  {
    module: 'bosc-ir (IR & Type Stripper)',
    crate: 'crates/bosc-ir',
    category: 'Compiler DSL',
    features: [
      {
        name: 'TypeScript Zero-Overhead Type Stripping',
        tasks: [
          { id: 'IR-01', title: 'Strip TypeScript Types from BOS-Script to JS', type: 'UNIT', priority: 'CRITICAL', verification: 'cargo test -p bosc-ir test_typescript_stripping' },
          { id: 'IR-02', title: 'Compile Declarative AST to Relational IR Schema', type: 'UNIT', priority: 'CRITICAL', verification: 'cargo test -p bosc-ir test_ir_lowering' }
        ]
      }
    ]
  },
  {
    module: 'bosc-codegen-db (Database Generator)',
    crate: 'crates/bosc-codegen-db',
    category: 'Database Engine',
    features: [
      {
        name: 'PostgreSQL DDL & FlowField Views',
        tasks: [
          { id: 'DB-01', title: 'Generate PostgreSQL Tables with RLS Policies', type: 'INTEGRATION', priority: 'CRITICAL', verification: 'cargo test -p bosc-codegen-db test_ddl_generation' },
          { id: 'DB-02', title: 'Generate Stored Generated Formula Columns', type: 'INTEGRATION', priority: 'HIGH', verification: 'cargo test -p bosc-codegen-db test_generated_columns' },
          { id: 'DB-03', title: 'Generate SIFT Pushdown Aggregation Views', type: 'INTEGRATION', priority: 'HIGH', verification: 'cargo test -p bosc-codegen-db test_sift_views' },
          { id: 'DB-04', title: 'Generate Immutable Ledger Reversal Triggers', type: 'INTEGRATION', priority: 'CRITICAL', verification: 'cargo test -p bosc-codegen-db test_immutable_ledger_triggers' }
        ]
      }
    ]
  },
  {
    module: 'bosc-runtime (Execution Engine & API Server)',
    crate: 'crates/bosc-runtime',
    category: 'Core Runtime',
    features: [
      {
        name: 'Axum HTTP REST API & Security Context',
        tasks: [
          { id: 'API-01', title: 'Health & Database Detector Endpoints (/api/health, /api/detect-db)', type: 'API_CONTRACT', priority: 'CRITICAL', verification: 'curl http://localhost:3000/api/health' },
          { id: 'API-02', title: 'Dynamic Database Lifecycle (list, create, switch, manage, test-connection)', type: 'API_CONTRACT', priority: 'CRITICAL', verification: 'powershell scripts/test_new_db_install.ps1' },
          { id: 'API-03', title: 'System Provisioning & Meta Catalog (/api/meta/provision, /api/meta/objects)', type: 'API_CONTRACT', priority: 'CRITICAL', verification: 'curl -X POST http://localhost:3000/api/meta/provision' },
          { id: 'API-04', title: 'Entities Generic CRUD with Multi-Tenant RLS (/api/entities/{table})', type: 'SECURITY_RLS', priority: 'CRITICAL', verification: 'powershell test_security_api.ps1' },
          { id: 'API-05', title: 'Optimistic Concurrency Control (_system_version mismatch 409 Conflict)', type: 'CONCURRENCY_OCC', priority: 'CRITICAL', verification: 'powershell test_advanced_features.ps1' },
          { id: 'API-06', title: 'Table & Record Locking (/api/entities/{table}/lock)', type: 'CONCURRENCY_OCC', priority: 'HIGH', verification: 'curl -X POST http://localhost:3000/api/entities/customer/lock' },
          { id: 'API-07', title: 'Declarative State Transitions & Audit History (/api/entities/{table}/{id}/transition)', type: 'INTEGRATION', priority: 'HIGH', verification: 'curl -X POST http://localhost:3000/api/entities/order/UUID/transition' },
          { id: 'API-08', title: 'GraphQL-Like Query Protocol with Relation Inclusions (/api/query)', type: 'API_CONTRACT', priority: 'HIGH', verification: 'curl -X POST http://localhost:3000/api/query' },
          { id: 'API-09', title: 'Transactional Outbox & Worker Dispatcher (/api/events/outbox)', type: 'INTEGRATION', priority: 'HIGH', verification: 'curl http://localhost:3000/api/events/outbox' },
          { id: 'API-10', title: 'QuickJS Sandboxed Codeunit Runner (/api/codeunits/{name}/run)', type: 'INTEGRATION', priority: 'HIGH', verification: 'cargo test -p bosc-runtime test_quickjs_sandbox' },
          { id: 'API-11', title: 'WebAssembly wasmi 2.0 Plugin Runner (/api/wasm/{module}/{func}/invoke)', type: 'INTEGRATION', priority: 'HIGH', verification: 'cargo test -p bosc-runtime test_wasm_execution' },
          { id: 'API-12', title: 'Loopback IPC Sidecar Gateway (/api/sidecars/{name}/call)', type: 'INTEGRATION', priority: 'MEDIUM', verification: 'cargo test -p bosc-runtime test_sidecar_ipc' }
        ]
      }
    ]
  },
  {
    module: 'bosc-cli (Unified CLI)',
    crate: 'crates/bosc-cli',
    category: 'Developer Tooling',
    features: [
      {
        name: 'Command-Line Operations',
        tasks: [
          { id: 'CLI-01', title: 'Database Auto-Detection (bosc-cli detect-db)', type: 'UNIT', priority: 'HIGH', verification: 'cargo run -p bosc-cli -- detect-db' },
          { id: 'CLI-02', title: 'HTTP API Server Launch (bosc-cli serve)', type: 'INTEGRATION', priority: 'CRITICAL', verification: 'cargo run -p bosc-cli -- serve --help' },
          { id: 'CLI-03', title: 'Package Build & Validation (bosc-cli package build)', type: 'UNIT', priority: 'HIGH', verification: 'cargo run -p bosc-cli -- package --help' }
        ]
      }
    ]
  },
  {
    module: 'ui (Compact Svelte 5 API Studio)',
    crate: 'ui',
    category: 'Frontend Studio',
    features: [
      {
        name: 'Interactive Testing Studio & Developer UX',
        tasks: [
          { id: 'UI-01', title: 'Responsive 3-Column Studio Layout (Desktop, Tablet, Mobile)', type: 'UX_VISUAL', priority: 'CRITICAL', verification: 'node scripts/test-agent/visual_browser_test.mjs --viewport=all' },
          { id: 'UI-02', title: 'Dark and Light Theme Toggle Contrast & Aesthetics', type: 'UX_VISUAL', priority: 'HIGH', verification: 'node scripts/test-agent/visual_browser_test.mjs --theme=all' },
          { id: 'UI-03', title: 'Interactive Multi-Tenant Security Context Drawer', type: 'UI_INTERACTION', priority: 'HIGH', verification: 'node scripts/test-agent/visual_browser_test.mjs --test=drawer' },
          { id: 'UI-04', title: '1-Click Auto-Fill Preset Endpoint Execution', type: 'UI_INTERACTION', priority: 'CRITICAL', verification: 'node scripts/test-agent/visual_browser_test.mjs --test=presets' },
          { id: 'UI-05', title: 'Real-Time Database Cluster Switcher Dropdown', type: 'UI_INTERACTION', priority: 'HIGH', verification: 'node scripts/test-agent/visual_browser_test.mjs --test=db-switcher' },
          { id: 'UI-06', title: 'Dynamic JSON Request Builder & Response Inspector Tabs', type: 'UI_INTERACTION', priority: 'HIGH', verification: 'node scripts/test-agent/visual_browser_test.mjs --test=response-tabs' },
          { id: 'UI-07', title: 'Error & Validation Toast Notifications Dismissal', type: 'UX_VISUAL', priority: 'MEDIUM', verification: 'node scripts/test-agent/visual_browser_test.mjs --test=toasts' }
        ]
      }
    ]
  }
];

// Calculate summary stats
let totalTasks = 0;
const typeCounts = {};
const priorityCounts = {};

for (const mod of modules) {
  for (const feat of mod.features) {
    for (const task of feat.tasks) {
      totalTasks++;
      typeCounts[task.type] = (typeCounts[task.type] || 0) + 1;
      priorityCounts[task.priority] = (priorityCounts[task.priority] || 0) + 1;
    }
  }
}

const matrixData = {
  generatedAt: new Date().toISOString(),
  totalTasks,
  typeCounts,
  priorityCounts,
  modules
};

// Write machine-readable JSON
const scratchDir = path.join(rootDir, 'scratch');
if (!fs.existsSync(scratchDir)) fs.mkdirSync(scratchDir, { recursive: true });
fs.writeFileSync(path.join(scratchDir, 'qa_task_matrix.json'), JSON.stringify(matrixData, null, 2));

// Generate Markdown documentation docs/18_qa_test_task_matrix.md
let md = `# 18. BOS QA Test Task Matrix & Feature Decomposition Catalog\n\n`;
md += `> Automatically generated and maintained by the **QA Tester Agent (\`tester\`)**.\n\n`;
md += `## 📊 Executive Summary\n\n`;
md += `- **Total Granular Test Tasks:** \`${totalTasks}\`\n`;
md += `- **By Category:** ` + Object.entries(typeCounts).map(([k, v]) => `**${k}:** ${v}`).join(' | ') + `\n`;
md += `- **By Priority:** ` + Object.entries(priorityCounts).map(([k, v]) => `**${k}:** ${v}`).join(' | ') + `\n\n`;
md += `---\n\n`;
md += `## 🧩 Modules & Granular Test Tasks Breakdown\n\n`;

for (const mod of modules) {
  md += `### ${mod.module} (${mod.category})\n`;
  md += `*Directory: \`${mod.crate}\`*\n\n`;
  for (const feat of mod.features) {
    md += `#### ${feat.name}\n\n`;
    md += `| ID | Priority | Type | Task Title | Verification Command |\n`;
    md += `| :--- | :--- | :--- | :--- | :--- |\n`;
    for (const t of feat.tasks) {
      md += `| **\`${t.id}\`** | \`${t.priority}\` | \`${t.type}\` | ${t.title} | \`${t.verification}\` |\n`;
    }
    md += `\n`;
  }
}

const docsDir = path.join(rootDir, 'docs');
if (!fs.existsSync(docsDir)) fs.mkdirSync(docsDir, { recursive: true });
fs.writeFileSync(path.join(docsDir, '18_qa_test_task_matrix.md'), md);

console.log(`Success! Decomposed ${totalTasks} test tasks across ${modules.length} modules.`);
console.log(`- Machine JSON: scratch/qa_task_matrix.json`);
console.log(`- Documentation: docs/18_qa_test_task_matrix.md`);
