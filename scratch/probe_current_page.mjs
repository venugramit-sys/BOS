import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage();
  await page.goto("http://localhost:3000");
  await page.waitForTimeout(1000);
  const buttons = await page.locator("button").allInnerTexts();
  console.log("Buttons found:", buttons);
  const inputs = await page.$$eval("input", els => els.map(e => ({ id: e.id, type: e.type, value: e.value, placeholder: e.placeholder })));
  console.log("Inputs found:", inputs);
  
  // Let's click OK and see what network requests happen or console logs
  page.on('console', msg => console.log('CONSOLE:', msg.type(), msg.text()));
  page.on('request', req => {
    if (req.url().includes('/api/')) console.log('REQ:', req.method(), req.url());
  });
  page.on('response', res => {
    if (res.url().includes('/api/')) console.log('RES:', res.status(), res.url());
  });
  
  const okBtn = page.locator('button:has-text("OK")');
  if (await okBtn.count() > 0) {
    console.log("Clicking OK button...");
    await okBtn.click();
    await page.waitForTimeout(2000);
  }
  
  await browser.close();
}

main().catch(console.error);
