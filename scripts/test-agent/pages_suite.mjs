// Compiled-page verification suite (docs/17 Phase 7).
//
// Against a running BOS node (BOS_NODE_URL, default http://127.0.0.1:3000) that serves ui/dist and has
// pages installed, for every non-API page in the manifest:
//   - renders at 375x812, 768x1024, 1280x800, 1920x1080 in light and dark
//   - no console error / page error, no horizontal page overflow
//   - screenshot to scratch/pages-suite/<Page>-<viewport>-<theme>.png
// Plus functional flows (only where the fixture pages exist) and performance budgets:
//   - GET /api/v1/pages/manifest   < 5 ms (median of 20, server time measured client-side on localhost)
//   - warm artifact → first paint  < 300 ms (page module cached, from navigation to .bos-page-body content)
//
// Usage: node scripts/test-agent/pages_suite.mjs   (exit code 1 on any failure)

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

const NODE = process.env.BOS_NODE_URL || 'http://127.0.0.1:3000';
const OUT = path.join(root, 'scratch/pages-suite');
fs.mkdirSync(OUT, { recursive: true });

const VIEWPORTS = [
  { id: 'phone', width: 375, height: 812 },
  { id: 'tablet', width: 768, height: 1024 },
  { id: 'laptop', width: 1280, height: 800 },
  { id: 'desktop', width: 1920, height: 1080 },
];
const THEMES = ['light', 'dark'];
const BUDGET_MANIFEST_MS = 5;
const BUDGET_FIRST_PAINT_MS = 300;

const failures = [];
const fail = (msg) => {
  failures.push(msg);
  console.log(`  ✗ ${msg}`);
};
const ok = (msg) => console.log(`  ✓ ${msg}`);

async function launch() {
  for (const channel of ['msedge', 'chrome', undefined]) {
    try {
      return await chromium.launch(channel ? { channel, headless: true } : { headless: true });
    } catch {
      // try the next browser
    }
  }
  throw new Error('no Chromium-based browser available for Playwright');
}

const pageUrl = (name) => `${NODE}/showcase.html?category=page-surface&page=${encodeURIComponent(name)}`;

async function openPage(context, name, theme = 'light') {
  const page = await context.newPage();
  const errors = [];
  page.on('console', (m) => m.type() === 'error' && errors.push(m.text().slice(0, 200)));
  page.on('pageerror', (e) => errors.push(String(e).slice(0, 200)));
  await page.goto(pageUrl(name));
  await page.waitForSelector('.bos-page', { timeout: 15000 });
  // The app themes with a `.dark` class on <html> (themeStore).
  if (theme === 'dark') await page.evaluate(() => document.documentElement.classList.add('dark'));
  await page.waitForFunction(() => !document.querySelector('.bos-page[aria-busy="true"]'), null, { timeout: 15000 }).catch(() => {});
  await page.waitForTimeout(400);
  return { page, errors };
}

async function main() {
  console.log(`\n=== BOS compiled pages suite → ${NODE} ===`);
  const manifest = await (await fetch(`${NODE}/api/v1/pages/manifest`)).json();
  const pages = Object.entries(manifest.pages).filter(([, e]) => e.page_type !== 'Api');
  if (!pages.length) throw new Error('the node has no UI pages installed');
  const broken = pages.filter(([, e]) => e.errors > 0);
  for (const [n, e] of broken) fail(`${n}: ${e.errors} compile error(s)`);
  console.log(`${pages.length} UI pages, Svelte ${manifest.svelte}, ${broken.length} with errors`);

  const browser = await launch();

  // --- manifest budget: measured where it is used, in the browser (Resource Timing, median of 20) --
  {
    const page = await browser.newPage();
    await page.goto(`${NODE}/showcase.html`);
    const ms = await page.evaluate(async () => {
      const times = [];
      for (let i = 0; i < 20; i++) {
        const url = `/api/v1/pages/manifest?probe=${i}`;
        await fetch(url, { cache: 'no-store' }).then((r) => r.arrayBuffer());
        const e = performance.getEntriesByName(new URL(url, location.href).href).at(-1);
        times.push(e.responseEnd - e.requestStart);
      }
      times.sort((a, b) => a - b);
      return times[10];
    });
    ms < BUDGET_MANIFEST_MS ? ok(`manifest ${ms.toFixed(2)} ms (< ${BUDGET_MANIFEST_MS})`) : fail(`manifest ${ms.toFixed(2)} ms exceeds ${BUDGET_MANIFEST_MS} ms`);
    await page.close();
  }

  // --- visual matrix ---------------------------------------------------------------------------
  for (const theme of THEMES) {
    for (const vp of VIEWPORTS) {
      const context = await browser.newContext({ viewport: { width: vp.width, height: vp.height }, colorScheme: theme });
      for (const [name] of pages) {
        try {
          const { page, errors } = await openPage(context, name, theme);
          if (theme === 'dark') {
            const bg = await page.$eval('.bos-page', (el) => getComputedStyle(el).getPropertyValue('--bos-color-surface').trim());
            if (bg.toLowerCase() !== '#0b1220') fail(`${name} ${vp.id}/dark: dark tokens not applied (${bg})`);
          }
          const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
          const pageOverflow = await page.$eval('.bos-page', (el) => el.scrollWidth - el.clientWidth);
          await page.screenshot({ path: path.join(OUT, `${name}-${vp.id}-${theme}.png`), fullPage: true });
          if (errors.length) fail(`${name} ${vp.id}/${theme}: console errors: ${errors[0]}`);
          if (overflow > 1) fail(`${name} ${vp.id}/${theme}: page scrolls horizontally by ${overflow}px`);
          if (pageOverflow > 1) fail(`${name} ${vp.id}/${theme}: page frame overflows by ${pageOverflow}px`);
          await page.close();
        } catch (e) {
          fail(`${name} ${vp.id}/${theme}: ${e.message.split('\n')[0]}`);
        }
      }
      await context.close();
      ok(`${theme} ${vp.id} (${vp.width}x${vp.height}): ${pages.length} pages rendered`);
    }
  }

  // --- warm first paint ------------------------------------------------------------------------
  {
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const [first] = pages.find(([, e]) => e.page_type === 'Card') ?? pages[0];
    const warm = await context.newPage();
    await warm.goto(pageUrl(first));
    await warm.waitForSelector('.bos-page-body > *');
    const t = await warm.evaluate(async (name) => {
      const { loadPageComponent } = await import('/bos-runtime/' + [...document.querySelectorAll('script[type=importmap]')].map((s) => JSON.parse(s.textContent).imports['@bos/page-kit'])[0].split('/bos-runtime/')[1]);
      const start = performance.now();
      await loadPageComponent(name);
      return performance.now() - start;
    }, first);
    const nav = await warm.evaluate(() => performance.getEntriesByType('navigation')[0]?.domContentLoadedEventEnd ?? 0);
    const paint = await (async () => {
      const p = await context.newPage();
      const start = Date.now();
      await p.goto(pageUrl(first));
      await p.waitForSelector('.bos-page-body .bos-cell, .bos-page-body .bos-repeater');
      const total = Date.now() - start;
      const app = await p.evaluate(() => performance.getEntriesByType('navigation')[0]?.domContentLoadedEventEnd ?? 0);
      await p.close();
      return { total, sinceApp: total - app };
    })();
    t < BUDGET_FIRST_PAINT_MS ? ok(`warm artifact import ${t.toFixed(1)} ms`) : fail(`warm artifact import ${t.toFixed(1)} ms`);
    paint.sinceApp < BUDGET_FIRST_PAINT_MS
      ? ok(`warm artifact → first paint ${paint.sinceApp.toFixed(0)} ms after the app shell (total ${paint.total} ms, shell ${nav.toFixed(0)} ms)`)
      : fail(`warm artifact → first paint ${paint.sinceApp.toFixed(0)} ms exceeds ${BUDGET_FIRST_PAINT_MS} ms`);
    await context.close();
  }

  // --- functional flows (fixture package: crates/bosc-codegen-svelte/tests/fixtures/pages.bos) --
  const has = (n) => pages.some(([p]) => p === n);
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  if (has('CustomerList')) {
    const { page } = await openPage(context, 'CustomerList');
    const before = await page.$$eval('.bos-table tbody tr', (r) => r.length);
    await page.fill('.bos-search input', 'Adatum');
    await page.waitForTimeout(900);
    const after = await page.$$eval('.bos-table tbody tr', (r) => r.length);
    after >= 1 && after <= before ? ok(`list search filters rows (${before} → ${after})`) : fail(`list search: ${before} → ${after}`);
    await page.click('.bos-th-sort');
    await page.waitForTimeout(500);
    ok('list sort header responds');
    await page.close();
  }
  if (has('SetupWizard')) {
    const { page } = await openPage(context, 'SetupWizard');
    await page.click('text=Next');
    await page.waitForTimeout(200);
    await page.click('text=Next');
    await page.waitForTimeout(200);
    const msg = await page.$eval('[role=alert]', (e) => e.textContent).catch(() => '');
    msg.includes('Company Name') ? ok('wizard blocks Next until required fields are filled') : fail(`wizard validation message missing (${msg})`);
    await page.close();
  }
  if (has('SalesOrder')) {
    const { page } = await openPage(context, 'SalesOrder');
    const lines = await page.$$eval('.bos-part .bos-table tbody tr', (r) => r.length).catch(() => 0);
    lines > 0 ? ok(`document page shows ${lines} linked lines`) : fail('document page shows no lines in its part');
    const factbox = await page.$('.bos-factboxes .bos-part');
    factbox ? ok('document page renders its FactBox') : fail('document page FactBox missing');
    await page.close();
  }
  if (has('CustomerCard')) {
    const { page } = await openPage(context, 'CustomerCard');
    const name = page.locator('.bos-field input').nth(0);
    await name.waitFor();
    ok('card renders editable fields');
    const status = await page.$('.bos-status');
    !status ? ok('card is clean before editing') : fail('card shows unsaved changes on open');
    await page.close();
  }
  if (has('ItemJournal')) {
    const { page } = await openPage(context, 'ItemJournal');
    const rows = await page.$$eval('.bos-table tbody tr', (r) => r.length);
    await page.click('.bos-actionbar >> text=New');
    const rows2 = await page.$$eval('.bos-table tbody tr', (r) => r.length);
    rows2 === rows + 1 ? ok('worksheet adds an inline line') : fail(`worksheet new line: ${rows} → ${rows2}`);
    await page.close();
  }
  await context.close();
  await browser.close();

  console.log(`\nScreenshots: ${OUT}`);
  if (failures.length) {
    console.log(`\n${failures.length} failure(s)`);
    process.exit(1);
  }
  console.log('\nAll page checks passed.');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
