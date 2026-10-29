import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const pass = page.locator('#bos-f-LoginPage-2');
  await pass.focus();

  // Dispatch keydown with getModifierState returning true for CapsLock
  await page.evaluate(() => {
    const input = document.querySelector('#bos-f-LoginPage-2');
    const evt = new KeyboardEvent('keydown', {
      key: 'a',
      bubbles: true,
      cancelable: true,
      modifierCapsLock: true
    });
    // Override getModifierState to return true
    evt.getModifierState = (mod) => mod === 'CapsLock';
    input.dispatchEvent(evt);
  });
  await page.waitForTimeout(300);

  const badge = page.locator('span[title="Caps Lock is on"]');
  const badgeCount = await badge.count();
  console.log('CapsLock badge count with mocked event:', badgeCount);
  if (badgeCount > 0) {
    console.log('CapsLock badge text:', await badge.innerText());
  }

  await browser.close();
}

main().catch(console.error);
