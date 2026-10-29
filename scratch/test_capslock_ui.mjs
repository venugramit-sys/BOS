import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const pass = page.locator('#bos-f-LoginPage-2');
  await pass.focus();

  // Simulate CapsLock on by typing with modifier state
  await page.keyboard.press('CapsLock');
  await page.keyboard.type('A');
  await page.waitForTimeout(300);

  const badge = page.locator('span[title="Caps Lock is on"]');
  const badgeCount = await badge.count();
  console.log('CapsLock badge count:', badgeCount);
  if (badgeCount > 0) {
    console.log('CapsLock badge text:', await badge.innerText());
  }

  await browser.close();
}

main().catch(console.error);
