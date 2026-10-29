import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  // Clear username
  await page.locator('#bos-f-LoginPage-1').fill('');
  await page.locator('footer button:has-text("Sign In")').click();
  await page.waitForTimeout(1000);

  const banner = await page.locator('.bos-page .rounded-md.border').first().innerText();
  console.log('Banner text:\n', banner);

  const userErr = await page.locator('.bos-cell.bos-field:has-text("User ID") p, .bos-cell.bos-field:has-text("User ID") span').allInnerTexts();
  console.log('User field texts:', userErr);

  await browser.close();
}

main().catch(console.error);
