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
    const banners = Array.from(document.querySelectorAll('.bos-page > *')).map(e => ({
      tag: e.tagName,
      cls: e.className,
      text: e.innerText
    }));
    return banners;
  });
  console.log('DOM dump:', JSON.stringify(dump, null, 2));
  await browser.close();
}

main().catch(console.error);
