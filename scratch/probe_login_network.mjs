import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();

  page.on('console', msg => console.log('BROWSER CONSOLE:', msg.type(), msg.text()));
  page.on('pageerror', err => console.log('PAGE ERROR:', err.message));
  page.on('request', req => {
    if (req.url().includes('/api/')) console.log('REQ:', req.method(), req.url());
  });
  page.on('response', async res => {
    if (res.url().includes('/api/')) {
      console.log('RES:', res.status(), res.url());
      try {
        const body = await res.text();
        console.log('BODY:', body.slice(0, 300));
      } catch {}
    }
  });

  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const userInput = page.locator('#bos-f-LoginPage-1');
  const passInput = page.locator('#bos-f-LoginPage-2');
  const btn = page.locator('footer button:has-text("Sign In")');

  console.log('Initial username:', await userInput.inputValue());
  console.log('Filling username and password...');
  await userInput.fill('admin');
  await passInput.fill('Boss@123');

  console.log('Clicking Sign In...');
  await btn.click();
  await page.waitForTimeout(3000);

  console.log('Final URL:', page.url());
  const bodyText = await page.evaluate(() => document.body.innerText);
  console.log('Visible text preview:\n', bodyText.slice(0, 400));

  await browser.close();
}

main().catch(console.error);
