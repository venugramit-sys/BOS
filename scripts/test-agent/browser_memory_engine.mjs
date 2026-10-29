#!/usr/bin/env node
/**
 * =============================================================================
 * BOS Browser Test Memory & Playback Engine
 * =============================================================================
 * Provides a persistent memory store for interactive browser tests.
 * Allows recording human-like user workflows, assertions, and market improvement
 * suggestions, and replaying them on fresh code to catch regressions.
 * =============================================================================
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve(process.cwd(), 'ui/node_modules/playwright'));

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

const MEMORY_DIR = path.join(rootDir, 'tests/browser-memory');
const SCENARIOS_DIR = path.join(MEMORY_DIR, 'scenarios');
const SCREENSHOTS_DIR = path.join(MEMORY_DIR, 'screenshots');
const INDEX_FILE = path.join(MEMORY_DIR, 'index.json');

// Ensure directories exist
for (const dir of [MEMORY_DIR, SCENARIOS_DIR, SCREENSHOTS_DIR]) {
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

function loadIndex() {
  if (fs.existsSync(INDEX_FILE)) {
    return JSON.parse(fs.readFileSync(INDEX_FILE, 'utf-8'));
  }
  return {
    version: '1.0.0',
    description: 'BOS Persistent Browser Test Memory & Regression Store',
    updatedAt: new Date().toISOString(),
    scenarios: []
  };
}

function saveIndex(index) {
  index.updatedAt = new Date().toISOString();
  fs.writeFileSync(INDEX_FILE, JSON.stringify(index, null, 2), 'utf-8');
}

function loadScenario(scenarioId) {
  const filePath = path.join(SCENARIOS_DIR, `${scenarioId}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Scenario '${scenarioId}' not found at ${filePath}`);
  }
  return JSON.parse(fs.readFileSync(filePath, 'utf-8'));
}

function saveScenario(scenario) {
  const filePath = path.join(SCENARIOS_DIR, `${scenario.id}.json`);
  scenario.lastUpdated = new Date().toISOString();
  fs.writeFileSync(filePath, JSON.stringify(scenario, null, 2), 'utf-8');

  // Sync with master index
  const index = loadIndex();
  const existing = index.scenarios.find(s => s.id === scenario.id);
  const summary = {
    id: scenario.id,
    name: scenario.name,
    persona: scenario.persona,
    category: scenario.category,
    lastRunAt: scenario.lastResult?.executedAt || null,
    status: scenario.lastResult?.status || 'NOT_RUN',
    durationMs: scenario.lastResult?.durationMs || 0,
    stepsCount: scenario.steps.length,
    improvementsCount: scenario.improvementsLog?.length || 0
  };

  if (existing) {
    Object.assign(existing, summary);
  } else {
    index.scenarios.push(summary);
  }
  saveIndex(index);
}

function listScenarios() {
  const index = loadIndex();
  console.log('\n================================================================================');
  console.log('  🧠 BOS BROWSER TEST MEMORY REPOSITORY');
  console.log('================================================================================');
  console.log(`Repository Path: ${MEMORY_DIR}`);
  console.log(`Total Recorded Scenarios: ${index.scenarios.length}\n`);

  if (index.scenarios.length === 0) {
    console.log('No scenarios currently recorded in memory.');
    return;
  }

  console.log(
    'ID'.padEnd(38) +
    'STATUS'.padEnd(12) +
    'STEPS'.padEnd(8) +
    'IDEAS'.padEnd(8) +
    'TIME (ms)'.padEnd(12) +
    'NAME'
  );
  console.log('-'.repeat(100));

  for (const s of index.scenarios) {
    const statusColor = s.status === 'PASSED' ? '\x1b[32m' : s.status === 'FAILED' ? '\x1b[31m' : '\x1b[33m';
    console.log(
      s.id.padEnd(38) +
      `${statusColor}${s.status}\x1b[0m`.padEnd(20) +
      String(s.stepsCount).padEnd(8) +
      String(s.improvementsCount).padEnd(8) +
      String(s.durationMs).padEnd(12) +
      s.name
    );
  }
  console.log('================================================================================\n');
}

function showInfo(scenarioId) {
  const scenario = loadScenario(scenarioId);
  console.log('\n================================================================================');
  console.log(`  🧠 SCENARIO MEMORY: ${scenario.name} [${scenario.id}]`);
  console.log('================================================================================');
  console.log(`Description:  ${scenario.description}`);
  console.log(`Persona:      ${scenario.persona}`);
  console.log(`Viewport:     ${scenario.viewport.width}x${scenario.viewport.height} (${scenario.viewport.name})`);
  console.log(`Theme:        ${scenario.theme}`);
  console.log(`Target URL:   ${scenario.targetUrl}`);
  console.log(`Last Status:  ${scenario.lastResult?.status || 'NOT_RUN'}`);
  console.log(`Last Run:     ${scenario.lastResult?.executedAt || 'Never'}`);
  console.log(`Duration:     ${scenario.lastResult?.durationMs || 0}ms`);
  
  console.log('\n--- Human Steps ---');
  scenario.steps.forEach((st, i) => {
    console.log(`  ${i + 1}. [${st.action.toUpperCase()}] ${st.description}`);
    if (st.selector) console.log(`     Target: ${st.selector}`);
    if (st.value) console.log(`     Value:  ${st.value}`);
  });

  if (scenario.improvementsLog && scenario.improvementsLog.length > 0) {
    console.log('\n--- 💡 Logged Improvements & Market-Dominating Suggestions ---');
    scenario.improvementsLog.forEach((imp, i) => {
      console.log(`  [${imp.type.toUpperCase()} - Rating: ${imp.rating}/10] ${imp.title}`);
      console.log(`    Detail:        ${imp.detail}`);
      console.log(`    Market Bench:  ${imp.marketBenchmark}`);
    });
  }
  console.log('================================================================================\n');
}

async function replayScenario(scenarioId, options = {}) {
  const scenario = loadScenario(scenarioId);
  const baseUrl = options.url || scenario.targetUrl || 'http://127.0.0.1:3000';
  console.log(`\n▶ [REPLAY MEMORY] Launching scenario: ${scenario.name} (${scenario.id})`);
  console.log(`  Target URL: ${baseUrl}`);
  console.log(`  Persona:    ${scenario.persona}`);
  console.log(`  Viewport:   ${scenario.viewport.width}x${scenario.viewport.height}`);

  const startTime = Date.now();
  let browser;
  try {
    browser = await chromium.launch({
      headless: !options.headed,
      channel: 'chrome'
    });
  } catch (e) {
    browser = await chromium.launch({ headless: !options.headed });
  }

  const context = await browser.newContext({
    viewport: { width: scenario.viewport.width, height: scenario.viewport.height },
    deviceScaleFactor: 1
  });
  const page = await context.newPage();

  const stepResults = [];
  let scenarioFailed = false;
  let failureReason = null;

  try {
    for (let i = 0; i < scenario.steps.length; i++) {
      const step = scenario.steps[i];
      const stepStart = Date.now();
      process.stdout.write(`  Step ${i + 1}/${scenario.steps.length}: ${step.description}... `);

      try {
        switch (step.action) {
          case 'navigate': {
            const url = step.url.startsWith('http') ? step.url : `${baseUrl}${step.url}`;
            await page.goto(url, { waitUntil: 'networkidle', timeout: 20000 });
            break;
          }
          case 'fill': {
            const timeout = step.timeout || 15000;
            await page.waitForSelector(step.selector, { timeout });
            try { await page.locator(step.selector).first().scrollIntoViewIfNeeded({ timeout: 2000 }); } catch {}
            await page.fill(step.selector, step.value);
            // Simulate human debounce delay
            await page.waitForTimeout(100);
            break;
          }
          case 'click': {
            const timeout = step.timeout || 15000;
            await page.waitForSelector(step.selector, { timeout });
            try { await page.locator(step.selector).first().scrollIntoViewIfNeeded({ timeout: 2000 }); } catch {}
            await page.click(step.selector);
            await page.waitForTimeout(step.delayAfterMs || 300);
            break;
          }
          case 'keyboard_press': {
            await page.keyboard.press(step.key);
            await page.waitForTimeout(step.delayAfterMs || 150);
            break;
          }
          case 'wait': {
            await page.waitForTimeout(step.durationMs || 1000);
            break;
          }
          case 'assert_visible': {
            const timeout = step.timeout || 15000;
            await page.waitForSelector(step.selector, { state: 'visible', timeout });
            break;
          }
          case 'assert_text': {
            const timeout = step.timeout || 15000;
            await page.waitForSelector(step.selector, { timeout });
            const content = await page.textContent(step.selector);
            if (!content || !content.includes(step.expectedText)) {
              throw new Error(`Expected text '${step.expectedText}' not found in selector '${step.selector}' (got: '${content}')`);
            }
            break;
          }
          case 'screenshot': {
            const shotName = `${scenario.id}_step_${i + 1}_${step.name || 'shot'}.png`;
            const shotPath = path.join(SCREENSHOTS_DIR, shotName);
            await page.screenshot({ path: shotPath, fullPage: false });
            step.screenshotPath = path.relative(rootDir, shotPath);
            break;
          }
          default:
            throw new Error(`Unknown step action: ${step.action}`);
        }

        const stepDuration = Date.now() - stepStart;
        console.log(`\x1b[32mOK\x1b[0m (${stepDuration}ms)`);
        stepResults.push({ step: i + 1, status: 'PASSED', durationMs: stepDuration });
      } catch (stepErr) {
        const stepDuration = Date.now() - stepStart;
        console.log(`\x1b[31mFAILED\x1b[0m (${stepDuration}ms): ${stepErr.message}`);
        stepResults.push({ step: i + 1, status: 'FAILED', durationMs: stepDuration, error: stepErr.message });
        scenarioFailed = true;
        failureReason = stepErr.message;
        
        // Capture failure screenshot
        const failShot = path.join(SCREENSHOTS_DIR, `${scenario.id}_FAILED.png`);
        await page.screenshot({ path: failShot });
        break;
      }
    }
  } finally {
    await browser.close();
  }

  const totalDuration = Date.now() - startTime;
  const status = scenarioFailed ? 'FAILED' : 'PASSED';
  const color = status === 'PASSED' ? '\x1b[32m' : '\x1b[31m';

  console.log(`\nResult for ${scenario.id}: ${color}${status}\x1b[0m (${totalDuration}ms)`);
  if (failureReason) {
    console.log(`Failure Reason: ${failureReason}`);
  }

  // Update scenario record
  scenario.lastResult = {
    executedAt: new Date().toISOString(),
    status,
    durationMs: totalDuration,
    stepResults,
    error: failureReason
  };
  saveScenario(scenario);

  return { id: scenario.id, status, durationMs: totalDuration };
}

async function replayAll(options = {}) {
  const index = loadIndex();
  console.log(`\n🚀 [REPLAY ALL MEMORIES] Replaying ${index.scenarios.length} test scenario(s) on fresh code...`);
  
  const results = [];
  for (const s of index.scenarios) {
    try {
      const res = await replayScenario(s.id, options);
      results.push(res);
    } catch (e) {
      results.push({ id: s.id, status: 'FAILED', error: e.message });
    }
  }

  console.log('\n================================================================================');
  console.log('  🎯 REPLAY VERIFICATION SUMMARY');
  console.log('================================================================================');
  results.forEach(r => {
    const col = r.status === 'PASSED' ? '\x1b[32m' : '\x1b[31m';
    console.log(`  ${r.id.padEnd(38)}: ${col}${r.status}\x1b[0m (${r.durationMs || 0}ms)`);
  });
  console.log('================================================================================\n');
}

// CLI Argument Routing
const args = process.argv.slice(2);
const command = args[0] || '--list';

(async () => {
  try {
    if (command === '--list') {
      listScenarios();
    } else if (command === '--info') {
      const id = args[1];
      if (!id) {
        console.error('Error: Please specify a scenario ID: node browser_memory_engine.mjs --info <id>');
        process.exit(1);
      }
      showInfo(id);
    } else if (command === '--replay') {
      const id = args[1];
      if (!id) {
        console.error('Error: Please specify a scenario ID: node browser_memory_engine.mjs --replay <id>');
        process.exit(1);
      }
      const headed = args.includes('--headed');
      await replayScenario(id, { headed });
    } else if (command === '--replay-all') {
      const headed = args.includes('--headed');
      await replayAll({ headed });
    } else {
      console.log('Usage:');
      console.log('  node scripts/test-agent/browser_memory_engine.mjs --list');
      console.log('  node scripts/test-agent/browser_memory_engine.mjs --info <scenario-id>');
      console.log('  node scripts/test-agent/browser_memory_engine.mjs --replay <scenario-id> [--headed]');
      console.log('  node scripts/test-agent/browser_memory_engine.mjs --replay-all [--headed]');
    }
  } catch (err) {
    console.error('Execution Error:', err.message);
    process.exit(1);
  }
})();
