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
  await page.waitForTimeout(2500);

  // Click on LoginPage treeitem in sidebar
  const lp = page.locator('[role="treeitem"]:has-text("LoginPage")').first();
  await lp.click();
  await page.waitForTimeout(3000);

  // Click on the title "Sign In" in canvas
  const titleEl = page.locator('h1.bos-page-title');
  await titleEl.waitFor({ state: "visible", timeout: 5000 });
  await titleEl.click();
  await page.waitForTimeout(1000);

  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/studio_loginpage_h1.png" });
  console.log("Screenshot saved to studio_loginpage_h1.png");

  // Click Style tab in PropertyPanel
  const styleTab = page.locator('button:has-text("Style")').first();
  await styleTab.click();
  await page.waitForTimeout(1000);

  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/studio_loginpage_h1_style.png" });
  console.log("Style tab screenshot saved to studio_loginpage_h1_style.png");

  await browser.close();
}

main().catch(console.error);
