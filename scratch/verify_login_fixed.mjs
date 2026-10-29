import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  
  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.type(), msg.text()));
  page.on('pageerror', err => console.log('PAGE ERROR:', err.message));
  page.on('request', req => {
    if (req.url().includes('/api/')) console.log('REQ:', req.method(), req.url());
  });
  page.on('response', res => {
    if (res.url().includes('/api/')) console.log('RES:', res.status(), res.url());
  });

  console.log("Navigating to http://localhost:3000 ...");
  await page.goto("http://localhost:3000", { waitUntil: "networkidle" });
  await page.waitForTimeout(1000);

  const buttons = await page.locator("button").allInnerTexts();
  console.log("Buttons found on page:", buttons);

  // Take screenshot before login
  await page.screenshot({ path: "scratch/login_before_submit.png" });

  const signInBtn = page.locator('button:has-text("Sign In")');
  console.log("Sign In button count:", await signInBtn.count());

  // Fill password
  const passInput = page.locator('input[type="password"]');
  await passInput.fill("Boss@123");

  console.log("Clicking Sign In button...");
  await signInBtn.click();

  // Wait for login transition
  await page.waitForTimeout(4000);

  console.log("Current page URL:", page.url());
  await page.screenshot({ path: "scratch/login_after_submit.png" });

  const bodyText = await page.evaluate(() => document.body.innerText);
  console.log("Body text after login:\n", bodyText.slice(0, 500));

  await browser.close();
}

main().catch(console.error);
