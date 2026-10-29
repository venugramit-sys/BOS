import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function inspectPageProps() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
  
  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(1000);

  // Authenticate as admin
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(2000);

  // Open LoginPage
  const loginPageItem = page.locator('text=LoginPage').first();
  if (await loginPageItem.count() === 0) {
    await page.locator('button:has(span.font-bold:text-is("Pages"))').first().click();
    await page.waitForTimeout(500);
  }
  await page.locator('text=LoginPage').first().click();
  await page.waitForTimeout(3000);

  // Click Outline tab
  await page.locator('button:has-text("Outline")').first().click();
  await page.waitForTimeout(500);

  // Click the root page item in outline
  await page.locator('div[role="tree"] button').first().click();
  await page.waitForTimeout(800);

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/page_root_properties.png' });
  console.log('Saved page_root_properties.png');

  await browser.close();
}

inspectPageProps().catch(err => {
  console.error(err);
  process.exit(1);
});
