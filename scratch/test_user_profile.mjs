import fs from 'fs';
import path from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve(process.cwd(), 'ui/node_modules/playwright'));

const targetDir = 'C:/Users/mades/.gemini/antigravity/brain/54fcda67-daa3-441f-a5a2-bb111aa7a87f/screenshots';

async function run() {
  console.log('Launching browser (msedge)...');
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 }
  });
  const page = await context.newPage();

  console.log('Navigating to http://localhost:5173/...');
  await page.goto('http://localhost:5173/', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1000);

  // Click on the User Avatar / Widget in the TopNavBar
  console.log('Clicking on user avatar...');
  const avatar = page.locator('header .rounded-full.bg-gradient-to-tr').first();
  await avatar.click();
  await page.waitForTimeout(500);

  // Click on the user profile item in dropdown
  console.log('Clicking on user profile dropdown item...');
  const profileItem = page.locator('div[role="menuitem"]').first();
  await profileItem.click();
  await page.waitForTimeout(1000);

  const shot = path.join(targetDir, '28_user_profile_personalization_card.png');
  await page.screenshot({ path: shot, fullPage: false });
  console.log(`Saved screenshot: ${shot}`);

  await browser.close();
  console.log('UserProfile personalization test complete!');
}

run().catch((err) => {
  console.error('Error:', err);
  process.exit(1);
});
