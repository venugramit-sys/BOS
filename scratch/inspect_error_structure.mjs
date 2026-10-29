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

  const dump = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('.bos-page [role="alert"], .bos-page .rounded-md, .bos-cell p')).map(e => ({
      tag: e.tagName,
      cls: e.className,
      text: e.innerText
    }));
  });
  console.log('Error dump:', JSON.stringify(dump, null, 2));

  // Also inspect what FieldInput renders for error
  const userField = await page.evaluate(() => {
    const el = document.querySelector('.bos-cell.bos-field');
    return el ? el.innerHTML : 'none';
  });
  console.log('User field inner HTML:', userField);

  await browser.close();
}

main().catch(console.error);
