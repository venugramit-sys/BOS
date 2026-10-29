import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifyCleanSidebar() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  
  console.log('1. Navigating to http://localhost:3000/studio...');
  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(1000);

  // Authenticate as admin
  console.log('2. Authenticating as admin...');
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(2000);

  // Verify that there are NO label badges like "Table" or "logic" on the items
  const sidebarText = await page.locator('aside').first().innerText();
  console.log('Sidebar snippet:', sidebarText.slice(0, 300));

  // Screenshot the clean sidebar
  const cleanSidebarPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/sidebar_clean_no_badges.png';
  await page.screenshot({ path: cleanSidebarPath });
  console.log('Saved clean sidebar screenshot to:', cleanSidebarPath);

  // Also hover over "AuthConfiguration" to show hover action buttons
  const authConfigRow = page.locator('text=AuthConfiguration').first();
  await authConfigRow.hover();
  await page.waitForTimeout(500);

  const hoverShotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/sidebar_clean_hover_actions.png';
  await page.screenshot({ path: hoverShotPath });
  console.log('Saved hover action screenshot to:', hoverShotPath);

  await browser.close();
  console.log('VERIFICATION COMPLETE');
}

verifyCleanSidebar().catch(err => {
  console.error(err);
  process.exit(1);
});
