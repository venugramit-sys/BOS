#!/usr/bin/env node
import { spawn, execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

// 1. Locate configuration file
const args = process.argv.slice(2);
let configFile = 'bos-node.toml';
const configIdx = args.findIndex((a) => a === '--config' || a === '-c');
if (configIdx !== -1 && args[configIdx + 1]) {
  configFile = args[configIdx + 1];
}

let resolvedConfig = configFile;
if (!fs.existsSync(resolvedConfig)) {
  let cur = process.cwd();
  while (true) {
    const candidate = path.join(cur, configFile);
    if (fs.existsSync(candidate)) {
      resolvedConfig = candidate;
      break;
    }
    const distCandidate = path.join(cur, 'dist', 'bos-server', 'config', configFile);
    if (fs.existsSync(distCandidate)) {
      resolvedConfig = distCandidate;
      break;
    }
    const parent = path.dirname(cur);
    if (parent === cur) break;
    cur = parent;
  }
}

if (fs.existsSync(resolvedConfig)) {
  const rootDir = path.dirname(path.resolve(resolvedConfig));
  process.chdir(rootDir);
  resolvedConfig = path.basename(resolvedConfig);
}

// 2. Extract port from config
let port = 3000;
if (fs.existsSync(resolvedConfig)) {
  const content = fs.readFileSync(resolvedConfig, 'utf8');
  const match = content.match(/port\s*=\s*(\d+)/);
  if (match) port = parseInt(match[1], 10);
}

// 3. Terminate running bosc instances & port occupiers to prevent "Access is denied (os error 5)" and "os error 10048"
try {
  if (process.platform === 'win32') {
    execSync(
      `powershell -NoProfile -Command "Get-Process -Name 'bosc' -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue; Get-NetTCPConnection -LocalPort ${port} -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"`,
      { stdio: 'ignore' }
    );
  } else {
    execSync(`pkill -9 bosc 2>/dev/null || true; lsof -ti :${port} | xargs kill -9 2>/dev/null || true`, { stdio: 'ignore' });
  }
} catch {
  // Ignore if no process was running
}

// 4. Launch bosc
const isRelease = args.includes('--release');
const cargoArgs = ['run'];
if (isRelease) cargoArgs.push('--release');
cargoArgs.push('-p', 'bosc-cli', '--', 'node', 'run', '--config', resolvedConfig);

console.log(`\x1b[36m[BOS Node Guardian]\x1b[0m Verified port ${port} is clear.`);
console.log(`\x1b[32m[BOS Node Launcher]\x1b[0m Starting node with config '${resolvedConfig}'...\n`);

const child = spawn('cargo', cargoArgs, { stdio: 'inherit', shell: true });

// 5. Optional UI hot reload: --ui starts Vite (http://localhost:5173), which proxies /api to this node.
let ui = null;
if (args.includes('--ui')) {
  console.log(`\x1b[32m[BOS Node Launcher]\x1b[0m Starting UI dev server (Vite, hot reload) on http://localhost:5173 ...`);
  const uiCmd = ['npm', 'run', 'dev', '--prefix', `"${path.join(repoRoot, 'ui')}"`];
  // --separate-windows (Windows): open the UI dev server in its own console window.
  if (process.platform === 'win32' && args.includes('--separate-windows')) uiCmd.unshift('start', '"BOS UI"');
  ui = spawn(uiCmd[0], uiCmd.slice(1), {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, BOS_NODE_URL: `http://127.0.0.1:${port}` },
  });
}

process.on('SIGINT', () => {
  child.kill('SIGINT');
  ui?.kill('SIGINT');
});
process.on('SIGTERM', () => {
  child.kill('SIGTERM');
  ui?.kill('SIGTERM');
});
child.on('exit', (code) => {
  ui?.kill();
  process.exit(code ?? 0);
});
