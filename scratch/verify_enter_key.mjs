import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });

  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.waitForTimeout(1000);

  const passInput = page.locator('input[type="password"]');
  await passInput.fill("Boss@123");
  console.log("Pressing Enter in password field...");
  await passInput.press("Enter");

  await page.waitForTimeout(3000);
  const bodyText = await page.evaluate(() => document.body.innerText);
  const loggedIn = bodyText.includes("Welcome to BOS") || bodyText.includes("System Super Administrator");
  console.log("Logged in via Enter key:", loggedIn);

  await browser.close();
}

main().catch(console.error);
