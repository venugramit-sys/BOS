import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  console.log('1. Navigating to http://localhost:3000/#/studio...');
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(1500);

  // Authenticate
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  console.log('2. Authenticated. Opening LoginPage in Page Designer...');
  // Open LoginPage in designer
  await page.evaluate(() => {
    // Look for studioStore on window or dispatch
    const navItems = Array.from(document.querySelectorAll('button, a, div'));
    const loginItem = navItems.find(el => el.innerText && el.innerText.trim() === 'LoginPage');
    if (loginItem) {
      loginItem.click();
    }
  });

  // If not found in DOM directly, use studioStore via evaluate
  await page.evaluate(async () => {
    try {
      // Find module or global if any
      const s = window.__studioStore;
      if (s) {
        s.openObject("LoginPage", "design", "page", "bos.auth");
      }
    } catch (e) {
      console.log('direct call error:', e);
    }
  });

  // Also check navigation tree
  const pagesHeader = page.locator('text=Pages').first();
  if (await pagesHeader.count() > 0) {
    await pagesHeader.click();
    await page.waitForTimeout(500);
  }

  // Click on LoginPage in navigation if visible
  const loginPageLink = page.locator('text=LoginPage').first();
  if (await loginPageLink.count() > 0) {
    await loginPageLink.click();
    console.log('Clicked LoginPage in navigation');
    await page.waitForTimeout(2000);
  }

  // Check if Page Designer is open
  await page.waitForTimeout(1500);

  // Take screenshot of current state
  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/studio_workspace_view.png' });
  console.log('Saved studio_workspace_view.png');

  // Find all select dropdowns
  const selects = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('select')).map(s => ({
      name: s.name || s.id || s.getAttribute('aria-label') || 'unnamed',
      options: Array.from(s.options).map(o => o.value)
    }));
  });
  console.log('Select dropdowns found:', JSON.stringify(selects, null, 2));

  await browser.close();
}

main().catch(console.error);
