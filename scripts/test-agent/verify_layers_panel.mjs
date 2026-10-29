import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  
  console.log('Navigating to Studio...');
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(2000);

  // Check if login needed
  const isLoginPage = await page.locator('input[name="username"]').count();
  if (isLoginPage > 0) {
    console.log('Logging in...');
    await page.locator('input[name="username"]').fill('admin');
    await page.locator('input[type="password"]').fill('Boss@123');
    await page.locator('button:has-text("Sign In")').click();
    await page.waitForTimeout(3000);
  }

  // Open LoginPage in Studio
  console.log('Opening LoginPage in Studio via navigation tree...');
  const pagesHeader = page.locator('button:has(span.font-bold:text-is("Pages"))').first();
  if (await pagesHeader.count() > 0) {
    await pagesHeader.click();
    await page.waitForTimeout(500);
  }
  const loginPageItem = page.locator('text=LoginPage').first();
  if (await loginPageItem.count() > 0) {
    await loginPageItem.click();
    await page.waitForTimeout(3000);
  } else {
    // Try Ctrl+K
    await page.keyboard.press('Control+k');
    await page.waitForTimeout(300);
    await page.keyboard.type('LoginPage');
    await page.waitForTimeout(500);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(3000);
  }

  // Switch to Design mode if in code mode
  const designBtn = page.locator('button[role="radio"]:has-text("Design")');
  if (await designBtn.count() > 0) {
    await designBtn.click();
    await page.waitForTimeout(2000);
  }

  // Check left tabs
  const layersTab = page.locator('button[role="tab"]:has-text("Layers")');
  console.log('Layers tab visible:', await layersTab.count());
  if (await layersTab.count() > 0) {
    await layersTab.click();
    await page.waitForTimeout(1000);
  }

  // Find "Sign In" title element in canvas
  const titleEl = page.locator('.bos-page-title');
  console.log('Found title element:', await titleEl.count());
  if (await titleEl.count() > 0) {
    console.log('Clicking "Sign In" title on canvas...');
    await titleEl.click({ force: true });
    await page.waitForTimeout(1500);
  }

  // Check the selection badge on canvas overlay
  const badgeText = await page.evaluate(() => {
    const el = document.querySelector('.pointer-events-none .rounded-t.bg-indigo-600');
    return el ? el.textContent.trim() : null;
  });
  console.log('Selection tag badge on canvas:', badgeText);

  // Check breadcrumb hierarchy
  const breadcrumbText = await page.evaluate(() => {
    const bar = document.querySelector('[aria-label="Hierarchy"]');
    return bar ? bar.textContent.trim() : null;
  });
  console.log('Breadcrumb text:', breadcrumbText);

  // Take screenshot
  const screenshotPath = 'C:/Users/mades/.gemini/antigravity/brain/b38b3a68-d1f8-4b5b-a147-14a4892c3699/layers_panel_verify.png';
  await page.screenshot({ path: screenshotPath, fullPage: true });
  console.log('Screenshot saved to:', screenshotPath);

  await browser.close();
}

main().catch(console.error);
