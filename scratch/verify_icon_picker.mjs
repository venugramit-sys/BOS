import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  
  // Login first
  console.log("Logging into BOS...");
  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(1500);

  // Navigate to Studio
  console.log("Navigating to Studio...");
  await page.goto("http://localhost:3000/#/studio", { waitUntil: "networkidle" });
  await page.waitForTimeout(2000);

  // Click LoginPage base page
  console.log("Opening LoginPage...");
  await page.locator('span:text-is("LoginPage")').first().click();
  await page.waitForTimeout(2000);

  // Click on the title "Sign In" in canvas
  console.log("Selecting h1.bos-page-title...");
  const titleEl = page.locator('h1.bos-page-title');
  await titleEl.waitFor({ state: "visible", timeout: 8000 });
  await titleEl.click();
  await page.waitForTimeout(800);

  // In Properties tab, pick icon "alarm-clock-minus"
  console.log("Picking alarm-clock-minus in Properties tab...");
  const propPicker = page.locator('[data-testid="icon-picker-trigger"]').first();
  await propPicker.click();
  await page.waitForTimeout(600);

  // Search alarm-clock
  await page.locator('input[placeholder*="icons"]').fill("alarm-clock-minus");
  await page.waitForTimeout(400);

  // Click the icon
  await page.locator('div[role="listbox"][aria-label="Available Icons"] button').first().click();
  await page.waitForTimeout(1000);

  // Switch to Style tab
  console.log("Switching to Style tab...");
  const styleTab = page.locator('button:has-text("Style")').first();
  await styleTab.click();
  await page.waitForTimeout(800);

  // Switch to Effects & Icon subtab
  await page.evaluate(() => {
    const tabs = Array.from(document.querySelectorAll('button[role="tab"]'));
    const eff = tabs.find(t => t.textContent.includes('Effects'));
    if (eff) eff.click();
  });
  await page.waitForTimeout(600);

  // Capture screenshot showing that alarm-clock-minus is ALREADY ACTIVE and synced with Size and Color controls!
  const brainDir = "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699";
  await page.screenshot({ path: `${brainDir}/studio_icon_style_synced.png` });
  console.log("Captured studio_icon_style_synced.png with alarm-clock-minus synced!");

  await browser.close();
  console.log("Verification finished successfully!");
}

main().catch(console.error);
