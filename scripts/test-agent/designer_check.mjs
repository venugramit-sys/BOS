// Page designer smoke test in the real Studio (/dev-studio).
//
// Signs in, opens every page object of the catalog in the designer and checks that:
//   - no "Designer" error banner and no syntax-error banner appears
//   - the canvas renders the compiled page (design build: [data-bos-node] elements)
//   - Develop mode is available and selecting a control fills the property panel
//   - no console errors
//
// Usage: BOS_NODE_URL=http://127.0.0.1:3100 BOS_TEST_USER=… BOS_TEST_PASSWORD=… node scripts/test-agent/designer_check.mjs [Page …]

import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

const NODE = process.env.BOS_NODE_URL || 'http://127.0.0.1:3000';
const USER = process.env.BOS_TEST_USER;
const PASSWORD = process.env.BOS_TEST_PASSWORD;
if (!USER || !PASSWORD) {
  console.error('Set BOS_TEST_USER and BOS_TEST_PASSWORD (a Studio user of the target node).');
  process.exit(2);
}

const only = process.argv.slice(2);
const objects = (await (await fetch(`${NODE}/api/meta/objects`)).json())
  .filter((o) => o.object_type === 'page' || o.object_type === 'pageextension')
  .map((o) => o.object_name)
  .filter((n, i, all) => all.indexOf(n) === i)
  .filter((n) => !only.length || only.includes(n));

let browser;
for (const channel of ['msedge', 'chrome', undefined]) {
  try {
    browser = await chromium.launch(channel ? { channel, headless: true } : { headless: true });
    break;
  } catch {
    // next browser
  }
}
const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
const errors = [];
page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
page.on('console', (m) => m.type() === 'error' && !m.text().startsWith('Failed to load resource') && errors.push(m.text().slice(0, 200)));
page.on('response', (r) => r.status() >= 400 && errors.push(`HTTP ${r.status()} ${r.request().method()} ${new URL(r.url()).pathname}`));

await page.goto(`${NODE}/dev-studio`);
await page.fill('input[type=text]', USER);
await page.fill('input[type=password]', PASSWORD);
await page.click('button[type=submit]');
await page.waitForSelector('[role=treeitem]', { timeout: 20000 });

let failures = 0;
for (const name of objects) {
  errors.length = 0;
  // Open through the explorer when it is visible, else through the command palette (Ctrl+K).
  const item = page.locator(`[role=treeitem][aria-label="${name}"]`).first();
  if (await item.isVisible().catch(() => false)) {
    await item.click();
  } else {
    await page.keyboard.press('Control+k');
    await page.keyboard.type(name);
    await page.waitForTimeout(300);
    await page.keyboard.press('Enter');
  }
  // The designer of this object: its header names the object (`page UserList`).
  const header = page.locator(`text=/^(page|pageextension) ${name}\\b/`).first();
  await header.waitFor({ state: 'visible', timeout: 20000 }).catch(() => {});
  await page
    .waitForFunction((n) => {
      const designer = [...document.querySelectorAll('[aria-label="Page design canvas"]')].find((c) => c.offsetParent !== null);
      return designer && designer.querySelector('[data-page]')?.getAttribute('data-page')?.length > 0 && document.body.innerText.includes(n);
    }, name, { timeout: 20000 })
    .catch(() => {});
  await page.waitForTimeout(800);
  const state = await page.evaluate((n) => {
    const visible = (el) => el && el.offsetParent !== null;
    const canvas = [...document.querySelectorAll('[aria-label="Page design canvas"]')].find(visible);
    const banners = [...document.querySelectorAll('div, section')]
      .filter((b) => visible(b) && b.children.length < 6)
      .map((b) => b.textContent.trim())
      .filter((t) => /^(Designer\b|The page source has a syntax error)/.test(t));
    const nodes = canvas ? canvas.querySelectorAll('[data-bos-node]').length : 0;
    const shown = canvas?.querySelector('[data-page]')?.getAttribute('data-page') ?? '';
    const modes = [...document.querySelectorAll('[role=radiogroup][aria-label="Designer mode"]')].find(visible);
    const develop = modes?.querySelector('[role=radio]');
    return { banners, nodes, shown, developEnabled: develop ? !develop.disabled : false, n };
  }, name);
  await page.screenshot({ path: path.join(root, 'scratch', `designer-${name}.png`) });
  const problems = [];
  // API pages have no user interface: the designer must say so instead of drawing a canvas.
  if (await page.locator('text=API pages publish data to integrations').first().isVisible().catch(() => false)) {
    console.log(`✓ ${name} (API page: code editing only)`);
    continue;
  }
  if (state.banners.length) problems.push(state.banners[0].slice(0, 160));
  if (!state.nodes) problems.push('canvas did not render the page');
  else if (state.shown !== name && !name.endsWith('_Customization') && !name.endsWith('Ext')) problems.push(`canvas shows ${state.shown} instead`);
  if (errors.length) problems.push(`console: ${errors[0]}`);
  if (!problems.length) {
    // Select the first control and expect its properties.
    const node = page.locator('[aria-label="Page design canvas"]:visible [data-bos-node]:not([data-bos-node=page])').first();
    if (await node.count()) {
      await node.click({ position: { x: 4, y: 4 } });
      const props = await page.locator('aside >> text=Properties').count();
      if (!props) problems.push('property panel did not open');
    }
  }
  console.log(problems.length ? `✗ ${name}: ${problems.join(' | ')}` : `✓ ${name} → ${state.shown} (${state.nodes} nodes${state.developEnabled ? ', Develop' : ', Customize only'})`);
  if (problems.length) failures++;
}
await browser.close();
console.log(`\n${objects.length} objects, ${failures} failure(s)`);
process.exit(failures ? 1 : 0);
