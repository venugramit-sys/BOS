import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const { chromium } = require(path.resolve(process.cwd(), 'ui/node_modules/playwright'));

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '../../');

const UI_URL = process.env.BOS_UI_URL || 'http://localhost:3000';
const SCREENSHOT_DIR = path.join(rootDir, 'scratch/visual_tests/templates');
if (!fs.existsSync(SCREENSHOT_DIR)) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
}

console.log('========================================================================');
console.log('🎨 CAPTURING VISUAL SCREENSHOTS FOR READY-MADE TEMPLATES');
console.log('========================================================================\n');

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });

  try {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 }
    });
    const page = await context.newPage();

    // 1. Sign In screen (Standard LoginPage with template bar)
    console.log('▶ [1/6] Loading Sign-In Screen (Standard)...');
    await page.goto(`${UI_URL}/`);
    await page.waitForTimeout(2000);
    const shot1 = path.join(SCREENSHOT_DIR, '01_login_standard.png');
    await page.screenshot({ path: shot1, fullPage: true });
    console.log(`  ✓ Saved: ${shot1}`);

    // 2. Click "Split Hero" button
    console.log('▶ [2/6] Switching to Split Hero Login...');
    const splitBtn = page.getByRole('button', { name: 'Split Hero' });
    if (await splitBtn.count() > 0) {
      await splitBtn.click();
      await page.waitForTimeout(1500);
      const shot2 = path.join(SCREENSHOT_DIR, '02_login_split_hero.png');
      await page.screenshot({ path: shot2, fullPage: true });
      console.log(`  ✓ Saved: ${shot2}`);
    }

    // 3. Click "Glassmorphism" button
    console.log('▶ [3/6] Switching to Glassmorphism Login...');
    const glassBtn = page.getByRole('button', { name: 'Glassmorphism' });
    if (await glassBtn.count() > 0) {
      await glassBtn.click();
      await page.waitForTimeout(1500);
      const shot3 = path.join(SCREENSHOT_DIR, '03_login_glassmorphism.png');
      await page.screenshot({ path: shot3, fullPage: true });
      console.log(`  ✓ Saved: ${shot3}`);
    }

    // 4. Click "POS Kiosk PIN" button
    console.log('▶ [4/6] Switching to POS Kiosk PIN Pad...');
    const kioskBtn = page.getByRole('button', { name: 'POS Kiosk PIN' });
    if (await kioskBtn.count() > 0) {
      await kioskBtn.click();
      await page.waitForTimeout(1500);
      const shot4 = path.join(SCREENSHOT_DIR, '04_login_kiosk_pin.png');
      await page.screenshot({ path: shot4, fullPage: true });
      console.log(`  ✓ Saved: ${shot4}`);
    }

    // 5. Sign In to view Workspace & Templates Gallery
    console.log('▶ [5/6] Signing into Workspace...');
    // Switch back to standard login to submit credentials
    const standardBtn = page.getByRole('button', { name: 'Standard' });
    if (await standardBtn.count() > 0) {
      await standardBtn.click();
      await page.waitForTimeout(500);
    }

    const signInBtn = page.getByRole('button', { name: 'Sign In' });
    if (await signInBtn.count() > 0) {
      await signInBtn.first().click();
      await page.waitForTimeout(2500);
    }

    const shot5 = path.join(SCREENSHOT_DIR, '05_workspace_templates_gallery.png');
    await page.screenshot({ path: shot5, fullPage: true });
    console.log(`  ✓ Saved: ${shot5}`);

    // 6. Launch Executive Operations Cockpit Template
    console.log('▶ [6/6] Launching Executive Dashboard Template...');
    const launchExecutiveBtn = page.getByRole('button', { name: 'Launch Template' }).first();
    if (await launchExecutiveBtn.count() > 0) {
      await launchExecutiveBtn.click();
      await page.waitForTimeout(2500);
      const shot6 = path.join(SCREENSHOT_DIR, '06_executive_dashboard_launched.png');
      await page.screenshot({ path: shot6, fullPage: true });
      console.log(`  ✓ Saved: ${shot6}`);
    }

    console.log('\n✅ All screenshots captured successfully in scratch/visual_tests/templates!');
  } catch (err) {
    console.error('Visual capture error:', err);
  } finally {
    await browser.close();
  }
}

main();
