import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(1500);

  // Type credentials
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  console.log('URL after login:', page.url());
  const text = await page.evaluate(() => document.body.innerText);
  console.log('Snippet after login:\n', text.slice(0, 400));

  await browser.close();
}

main().catch(console.error);
