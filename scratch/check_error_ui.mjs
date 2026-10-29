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

  const bannerText = await page.locator('.bos-page, .bos-banner, [role="alert"]').allInnerTexts();
  console.log('Visible alert elements:', bannerText);

  const errorBanners = await page.locator('.bos-banner, [data-slot="banner"]').allInnerTexts();
  console.log('Banners:', errorBanners);

  await browser.close();
}

main().catch(console.error);
