import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(1500);
  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(2000);

  const btn = page.locator('button[aria-expanded]').first();
  console.log('aria-expanded before click:', await btn.getAttribute('aria-expanded'));
  
  const beforeCount = await page.evaluate(() => document.querySelectorAll('aside button').length);
  console.log('Buttons count before click:', beforeCount);

  await btn.click();
  await page.waitForTimeout(500);
  console.log('aria-expanded after click:', await btn.getAttribute('aria-expanded'));

  const afterCount = await page.evaluate(() => document.querySelectorAll('aside button').length);
  console.log('Buttons count after click:', afterCount);

  await btn.click();
  await page.waitForTimeout(500);
  console.log('aria-expanded after 2nd click:', await btn.getAttribute('aria-expanded'));
  const html = await page.evaluate(() => {
    const el = document.querySelector('[role="tree"]');
    return el ? el.outerHTML : 'tree not found';
  });
  console.log('TREE HTML:\n', html.slice(0, 1500));

  await browser.close();
}

main().catch(console.error);
