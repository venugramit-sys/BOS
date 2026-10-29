import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const passInput = page.locator('#bos-f-LoginPage-2');
  await passInput.fill('Boss@123');
  console.log('Password filled, pressing Enter...');
  await passInput.press('Enter');
  await page.waitForTimeout(2500);

  console.log('URL after Enter:', page.url());
  const bodyText = await page.evaluate(() => document.body.innerText);
  console.log('Page snippet after Enter:', bodyText.slice(0, 300));

  await browser.close();
}

main().catch(console.error);
