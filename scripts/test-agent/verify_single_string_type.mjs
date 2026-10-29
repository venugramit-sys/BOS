import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifySingleStringType() {
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

  // Open LoginPage
  console.log('3. Opening LoginPage in Designer...');
  const pagesHeader = page.locator('button:has(span.font-bold:text-is("Pages"))').first();
  const loginPageItem = page.locator('text=LoginPage').first();
  if (await loginPageItem.count() === 0) {
    await pagesHeader.click();
    await page.waitForTimeout(500);
  }
  await page.locator('text=LoginPage').first().click();
  await page.waitForTimeout(2500);

  // Open Declare Page Variable modal
  console.log('4. Opening Declare Page Variable modal...');
  const newVarBtn = page.locator('button:has-text("New Var")').first();
  await newVarBtn.click();
  await page.waitForTimeout(600);

  // Click the BOS Data Type select trigger to open dropdown options
  console.log('5. Opening BOS Data Type select dropdown...');
  // The select button inside the dialog
  const selectTrigger = page.locator('[role="dialog"] button:has-text("String")').first();
  await selectTrigger.click();
  await page.waitForTimeout(500);

  const shotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/declare_var_single_string_type.png';
  await page.screenshot({ path: shotPath });
  console.log('Saved single string type screenshot to:', shotPath);

  await browser.close();
  console.log('VERIFICATION COMPLETE');
}

verifySingleStringType().catch(err => {
  console.error(err);
  process.exit(1);
});
