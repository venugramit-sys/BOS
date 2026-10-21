import fs from 'fs';
import path from 'path';
import { execSync } from 'child_process';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

console.log('--- BOS Code Change & Blast-Radius Tracker ---');

let changedFiles = [];
const hasGit = fs.existsSync(path.join(rootDir, '.git'));

if (hasGit) {
  try {
    const statusOutput = execSync('git status --porcelain', { cwd: rootDir, encoding: 'utf-8' });
    const lines = statusOutput.split('\n').filter(Boolean);
    for (const line of lines) {
      const file = line.substring(3).trim();
      if (file) changedFiles.push(file);
    }

    if (changedFiles.length === 0) {
      const diffOutput = execSync('git diff --name-only HEAD~1 HEAD', { cwd: rootDir, encoding: 'utf-8' });
      changedFiles = diffOutput.split('\n').map(s => s.trim()).filter(Boolean);
    }
  } catch (err) {
    console.warn('Git query fallback:', err.message);
  }
}

// Fallback: If no git or 0 files found via git, scan filesystem for recently modified files
if (changedFiles.length === 0) {
  console.log('Scanning workspace for recently modified files (last 72 hours)...');
  const now = Date.now();
  const maxAgeMs = 72 * 60 * 60 * 1000; // 72 hours

  function scanDir(dir) {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      const relPath = path.relative(rootDir, fullPath).replace(/\\/g, '/');

      if (entry.isDirectory()) {
        if (['node_modules', 'target', '.git', 'dist', '.vite', '.vite-temp'].includes(entry.name)) {
          continue;
        }
        scanDir(fullPath);
      } else if (entry.isFile()) {
        const stats = fs.statSync(fullPath);
        if (now - stats.mtimeMs < maxAgeMs) {
          changedFiles.push(relPath);
        }
      }
    }
  }

  scanDir(rootDir);
}

// Remove duplicates
changedFiles = Array.from(new Set(changedFiles));
console.log(`Detected ${changedFiles.length} recently changed/modified file(s).`);

// Load matrix if present
let matrix = null;
const matrixPath = path.join(rootDir, 'scratch/qa_task_matrix.json');
if (fs.existsSync(matrixPath)) {
  matrix = JSON.parse(fs.readFileSync(matrixPath, 'utf-8'));
}

const subsystemMapping = [
  { match: /^crates\/bosc-syntax/, subsystem: 'Compiler AST & Grammar', modules: ['bosc-syntax'], tasks: ['SYN-01', 'SYN-02', 'SYN-03', 'SYN-04'] },
  { match: /^crates\/bosc-analyzer/, subsystem: 'Semantic Analyzer', modules: ['bosc-analyzer'], tasks: ['ANA-01', 'ANA-02', 'ANA-03', 'ANA-04'] },
  { match: /^crates\/bosc-ir/, subsystem: 'IR & Type Stripper', modules: ['bosc-ir'], tasks: ['IR-01', 'IR-02'] },
  { match: /^crates\/bosc-codegen-db/, subsystem: 'Database Engine Codegen', modules: ['bosc-codegen-db'], tasks: ['DB-01', 'DB-02', 'DB-03', 'DB-04'] },
  { match: /^crates\/bosc-runtime\/src\/server\.rs/, subsystem: 'Axum HTTP REST API', modules: ['bosc-runtime'], tasks: ['API-01', 'API-02', 'API-03', 'API-04', 'API-05', 'API-06', 'API-07', 'API-08', 'API-09'] },
  { match: /^crates\/bosc-runtime/, subsystem: 'Core Runtime Engine', modules: ['bosc-runtime'], tasks: ['API-10', 'API-11', 'API-12'] },
  { match: /^crates\/bosc-cli/, subsystem: 'CLI Tooling', modules: ['bosc-cli'], tasks: ['CLI-01', 'CLI-02', 'CLI-03'] },
  { match: /^ui\/src\/App\.svelte/, subsystem: 'Svelte 5 API Studio Frontend', modules: ['ui'], tasks: ['UI-01', 'UI-02', 'UI-03', 'UI-04', 'UI-05', 'UI-06', 'UI-07'] },
  { match: /^ui\//, subsystem: 'Frontend Assets & Config', modules: ['ui'], tasks: ['UI-01', 'UI-02'] },
  { match: /^packages\//, subsystem: 'Business Packages & Meta-Objects', modules: ['packages'], tasks: ['API-03', 'API-04', 'API-10'] },
  { match: /^docs\//, subsystem: 'Architectural Documentation', modules: ['docs'], tasks: [] }
];

const impactedSubsystems = new Set();
const impactedTasks = new Set();
const fileDetails = [];

for (const file of changedFiles) {
  let matched = false;
  for (const mapping of subsystemMapping) {
    if (mapping.match.test(file)) {
      impactedSubsystems.add(mapping.subsystem);
      mapping.tasks.forEach(t => impactedTasks.add(t));
      fileDetails.push({ file, subsystem: mapping.subsystem });
      matched = true;
      break;
    }
  }
  if (!matched) {
    fileDetails.push({ file, subsystem: 'Workspace & Config' });
  }
}

// Map task details
const taskList = [];
if (matrix && matrix.modules) {
  for (const mod of matrix.modules) {
    for (const feat of mod.features) {
      for (const t of feat.tasks) {
        if (impactedTasks.has(t.id)) {
          taskList.push(t);
        }
      }
    }
  }
}

const impactReport = {
  timestamp: new Date().toISOString(),
  gitActive: hasGit,
  changedFilesCount: changedFiles.length,
  recentFiles: changedFiles.slice(0, 25),
  impactedSubsystems: Array.from(impactedSubsystems),
  impactedTasksCount: taskList.length,
  recommendedTasks: taskList
};

const scratchDir = path.join(rootDir, 'scratch');
if (!fs.existsSync(scratchDir)) fs.mkdirSync(scratchDir, { recursive: true });
fs.writeFileSync(path.join(scratchDir, 'impact_test_plan.json'), JSON.stringify(impactReport, null, 2));

console.log(`\n=== IMPACT & BLAST RADIUS SUMMARY ===`);
console.log(`Impacted Subsystems (${impactedSubsystems.size}):`);
impactedSubsystems.forEach(s => console.log(`  - [${s}]`));

console.log(`\nRecommended Targeted Test Tasks (${taskList.length}):`);
for (const t of taskList) {
  console.log(`  * [${t.priority}] (${t.id}) ${t.title} -> ${t.verification}`);
}

console.log(`\nTargeted Test Plan saved to: scratch/impact_test_plan.json\n`);
