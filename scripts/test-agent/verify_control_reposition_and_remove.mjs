import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifyControlRepositionAndRemove() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
  
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

  // Expand Pages section if needed and open LoginPage in Designer
  console.log('3. Opening LoginPage in Page Designer...');
  const pagesHeader = page.locator('button:has(span.font-bold:text-is("Pages"))').first();
  const loginPageItem = page.locator('text=LoginPage').first();
  if (await loginPageItem.count() === 0) {
    await pagesHeader.click();
    await page.waitForTimeout(500);
  }

  await page.locator('text=LoginPage').first().click();
  await page.waitForTimeout(3000);

  // Click on a control / field on the canvas (e.g. username or id or password)
  console.log('4. Selecting field on canvas...');
  const userField = page.locator('[data-bos-node*="username"]').first();
  if (await userField.count() > 0) {
    await userField.click();
  } else {
    await page.locator('input[placeholder="User ID"]').first().click({ force: true });
  }
  await page.waitForTimeout(1000);

  // Capture screenshot showing floating action toolbar on canvas and Property Panel position & action buttons
  const shotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/designer_control_reposition_and_remove.png';
  await page.screenshot({ path: shotPath });
  console.log('Saved screenshot to:', shotPath);

  await browser.close();
  console.log('TEST COMPLETE');
}

verifyControlRepositionAndRemove().catch(err => {
  console.error(err);
  process.exit(1);
});
