import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  
  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.waitForTimeout(1000);
  
  // Fill password
  await page.locator('input[type="password"]').fill('Boss@123');
  // Click Sign In
  await page.locator('button:has-text("Sign In")').click();
  
  // Wait for workspace to load
  await page.waitForTimeout(3000);
  
  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/workspace_logged_in.png" });
  console.log("Logged in! URL:", page.url());
  
  await browser.close();
}

main().catch(console.error);
