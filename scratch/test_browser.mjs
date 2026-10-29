import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext();
  const page = await context.newPage();

  page.on('console', msg => console.log('BROWSER LOG:', msg.type(), msg.text()));
  page.on('pageerror', err => console.log('BROWSER PAGE ERROR:', err));

  console.log('Navigating to http://localhost:3000 ...');
  await page.goto('http://localhost:3000', { waitUntil: 'networkidle' });
  await page.waitForTimeout(2000);

  await page.screenshot({ path: 'scratch/login_page_loaded.png' });
  console.log('Screenshot saved to scratch/login_page_loaded.png');

  const errorEl = await page.$('.bos-banner, [role="alert"]');
  if (errorEl) {
    console.log('ALERT TEXT:', await errorEl.innerText());
  }

  await browser.close();
}

main().catch(console.error);
