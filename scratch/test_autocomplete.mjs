import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const user = page.locator('#bos-f-LoginPage-1');
  const pass = page.locator('#bos-f-LoginPage-2');

  console.log('Username autocomplete:', await user.getAttribute('autocomplete'));
  console.log('Password autocomplete:', await pass.getAttribute('autocomplete'));

  await browser.close();
}

main().catch(console.error);
