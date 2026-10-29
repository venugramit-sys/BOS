import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  
  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.waitForTimeout(2000);
  
  await page.screenshot({ path: "C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/login_page_fixed.png" });
  console.log("Screenshot saved successfully!");
  
  await browser.close();
}

main().catch(console.error);
