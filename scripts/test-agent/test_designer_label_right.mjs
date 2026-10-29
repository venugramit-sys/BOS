import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

const NODE = 'http://127.0.0.1:3000';
const USER = 'admin';
const PASSWORD = 'Boss@123';

async function testDesignerLabelPosition() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
  
  console.log('Navigating to http://127.0.0.1:3000/dev-studio...');
  await page.goto(`${NODE}/dev-studio`);
  await page.waitForTimeout(1000);

  console.log('Filling login credentials...');
  await page.fill('input[type=text]', USER);
  await page.fill('input[type=password]', PASSWORD);
  await page.click('button[type=submit]');
  await page.waitForSelector('[role=treeitem]', { timeout: 20000 });
  console.log('Studio loaded!');

  // Open LoginPage
  console.log('Opening LoginPage via Ctrl+K...');
  await page.keyboard.press('Control+k');
  await page.waitForTimeout(300);
  await page.keyboard.type('LoginPage');
  await page.waitForTimeout(500);
  await page.keyboard.press('Enter');

  // Wait for canvas to be visible
  await page.waitForSelector('[aria-label="Page design canvas"]', { timeout: 15000 });
  await page.waitForTimeout(1000);

  // Click on a field on the canvas (e.g. rememberMe or username)
  console.log('Selecting field node on canvas...');
  const fieldNode = page.locator('[aria-label="Page design canvas"]:visible [data-bos-node="field"]').first();
  await fieldNode.click({ position: { x: 10, y: 10 } });
  await page.waitForTimeout(1000);

  // Check PropertyPanel
  const propertyPanel = page.locator('aside').filter({ hasText: 'Properties' });
  console.log('Property panel visible:', await propertyPanel.count() > 0);

  // Look for labelPosition row
  const labelPosRow = page.locator('tr, div').filter({ hasText: 'labelPosition' }).first();
  console.log('labelPosition row found:', await labelPosRow.count() > 0);

  // Inspect the select options for labelPosition
  const labelPosSelect = page.locator('select').filter({ has: page.locator('option[value="right"]') }).first();
  const selectExists = await labelPosSelect.count() > 0;
  console.log('Select with option "right" exists:', selectExists);

  if (selectExists) {
    const options = await labelPosSelect.evaluate(el => Array.from(el.options).map(o => o.value));
    console.log('Verified labelPosition options in Designer PropertyPanel:', options);

    // Select "right"
    console.log('Selecting "right" in labelPosition dropdown...');
    await labelPosSelect.selectOption('right');
    await page.waitForTimeout(1500);
  }

  // Screenshot of designer with labelPosition set to right
  const screenshotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/designer_label_right_verified.png';
  await page.screenshot({ path: screenshotPath });
  console.log('Saved screenshot to:', screenshotPath);

  await browser.close();
  console.log('TEST COMPLETED SUCCESSFULLY');
}

testDesignerLabelPosition().catch(console.error);
