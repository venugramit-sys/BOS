import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  
  // Login first
  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.waitForTimeout(1000);
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // Navigate to Studio
  await page.goto("http://localhost:3000/#/studio", { waitUntil: "networkidle" });
  await page.waitForTimeout(2000);

  // Click LoginPage in the sidebar
  await page.locator('div:has-text("LoginPage")').last().click();
  await page.waitForTimeout(3000);

  // Click on the title "Sign In" in canvas
  const titleEl = page.locator('h1.bos-page-title');
  await titleEl.waitFor({ state: "visible", timeout: 5000 });
  await titleEl.click();
  await page.waitForTimeout(1000);

  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/studio_h1_fixed.png" });
  console.log("Screenshot saved to studio_h1_fixed.png");

  // Click Style tab in property sheet
  const styleTab = page.locator('button:has-text("Style")').first();
  await styleTab.click();
  await page.waitForTimeout(1000);

  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/studio_h1_style_tab.png" });
  console.log("Style tab screenshot saved to studio_h1_style_tab.png");

  await browser.close();
}

main().catch(console.error);
