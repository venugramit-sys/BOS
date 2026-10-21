import fs from 'fs';
import path from 'path';
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
  await page.goto('http://localhost:5173/showcase.html', { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(1000);

  // Active category defaults to "input", so Section 4 is directly present on page!
  console.log('Waiting for Section 4...');
  const sec4 = page.locator('text=Section 4: Enterprise Typing Ergonomics');
  await sec4.waitFor({ state: 'visible', timeout: 15000 });
  await sec4.scrollIntoViewIfNeeded();
  await page.waitForTimeout(500);

  const shot1 = path.join(targetDir, '26_input_pattern_masking_and_f8.png');
  await page.screenshot({ path: shot1, fullPage: false });
  console.log(`Saved screenshot 1: ${shot1}`);

  // Test F8 keypress on Journal Line 2
  console.log('Testing F8 keypress in Journal Line 2...');
  const line2Input = page.locator('input[placeholder="Focus here & press F8"]');
  await line2Input.click();
  await page.waitForTimeout(300);
  await page.keyboard.press('F8');
  await page.waitForTimeout(600);

  const shot2 = path.join(targetDir, '27_f8_copy_above_executed.png');
  await page.screenshot({ path: shot2, fullPage: false });
  console.log(`Saved screenshot 2: ${shot2}`);

  await browser.close();
  console.log('Input Section 4 visual test completed successfully!');
}

run().catch((err) => {
  console.error('Visual test error:', err);
  process.exit(1);
});
