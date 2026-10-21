import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve(process.cwd(), 'ui/node_modules/playwright'));

const targetDir = 'C:/Users/mades/.gemini/antigravity/brain/54fcda67-daa3-441f-a5a2-bb111aa7a87f/screenshots';
if (!fs.existsSync(targetDir)) {
  fs.mkdirSync(targetDir, { recursive: true });
}

async function run() {
  console.log('Launching browser (msedge)...');
  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 }
  });
  const page = await context.newPage();

  console.log('Navigating to http://localhost:5173/showcase.html...');
  await page.goto('http://localhost:5173/showcase.html', { waitUntil: 'networkidle' });

  // 1. Click "Date / Time Picker" nav item
  console.log('Selecting Date / Time Picker in sidebar...');
  const dateNavBtn = page.locator('button:has-text("Date / Time Picker")');
  await dateNavBtn.click();
  await page.waitForTimeout(500);

  const shot1 = path.join(targetDir, '22_datepicker_showcase_initial.png');
  await page.screenshot({ path: shot1, fullPage: false });
  console.log(`Saved screenshot 1: ${shot1}`);

  // 2. Type formula "cm+10d" into the first DatePicker input
  console.log('Typing formula "cm+10d" into Posting Date input...');
  const dateInput = page.locator('input[placeholder*="cm+10d"]').first();
  await dateInput.click();
  await dateInput.fill('cm+10d');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(400);

  const shot2 = path.join(targetDir, '23_datepicker_formula_evaluated.png');
  await page.screenshot({ path: shot2, fullPage: false });
  console.log(`Saved screenshot 2: ${shot2}`);

  // 3. Click the calendar trigger button to open the popover
  console.log('Clicking calendar popover trigger...');
  const calTrigger = page.locator('button[title*="Open visual calendar"]').first();
  await calTrigger.click();
  await page.waitForTimeout(600);

  const shot3 = path.join(targetDir, '24_datepicker_calendar_dropdown.png');
  await page.screenshot({ path: shot3, fullPage: false });
  console.log(`Saved screenshot 3: ${shot3}`);

  // 4. Click outside to close calendar, then click Theme Store
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);

  console.log('Selecting Theme & Personalization in sidebar...');
  const themeNavBtn = page.locator('button:has-text("Theme & Personalization")');
  await themeNavBtn.click();
  await page.waitForTimeout(500);

  const shot4 = path.join(targetDir, '25_theme_store_showcase.png');
  await page.screenshot({ path: shot4, fullPage: false });
  console.log(`Saved screenshot 4: ${shot4}`);

  await browser.close();
  console.log('Visual test completed successfully!');
}

run().catch((err) => {
  console.error('Visual test error:', err);
  process.exit(1);
});
