import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifyBoldHeadingsFadedIcons() {
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

  // 1. Capture with Tables expanded (showing faded blue Table and faded emerald Logic icons)
  const expandedShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/sidebar_bold_headings_tables_expanded.png';
  await page.screenshot({ path: expandedShot });
  console.log('Saved tables expanded screenshot to:', expandedShot);

  // 2. Collapse Tables to match user's screenshot layout
  console.log('3. Collapsing Tables to show matching layout with Extensions...');
  const tablesSubgroupHeader = page.locator('button:has(span.font-bold:text-is("Tables"))').first();
  await tablesSubgroupHeader.click();
  await page.waitForTimeout(400);

  // Expand Extensions & Customizations if collapsed
  const extSubgroupHeader = page.locator('button:has(span.font-bold:text-is("Extensions & Customizations"))').first();
  const extContent = page.locator('text=LoginPage_Customization');
  if (await extContent.count() === 0) {
    await extSubgroupHeader.click();
    await page.waitForTimeout(400);
  }

  const collapsedShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/sidebar_bold_headings_extensions_view.png';
  await page.screenshot({ path: collapsedShot });
  console.log('Saved extensions view screenshot to:', collapsedShot);

  await browser.close();
  console.log('VERIFICATION COMPLETE');
}

verifyBoldHeadingsFadedIcons().catch(err => {
  console.error(err);
  process.exit(1);
});
