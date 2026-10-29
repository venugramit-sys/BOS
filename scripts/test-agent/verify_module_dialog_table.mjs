import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifyModuleDialogTable() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  // 1. Desktop Test
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

  // Inspect module card
  console.log('3. Opening Module Detail Card for bos.auth...');
  const infoBtn = page.locator('button[aria-label="Inspect module card"]').first();
  await infoBtn.click({ force: true });
  await page.waitForTimeout(1000);

  // Desktop Screenshot of the new neat & clean table
  const desktopShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/module_dialog_table_desktop.png';
  await page.screenshot({ path: desktopShot });
  console.log('Saved desktop screenshot to:', desktopShot);

  // Test search query
  console.log('4. Testing search query in modal...');
  const searchInput = page.locator('input[placeholder*="Search objects"]');
  await searchInput.fill('User');
  await page.waitForTimeout(600);

  const searchShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/module_dialog_table_search.png';
  await page.screenshot({ path: searchShot });
  console.log('Saved search screenshot to:', searchShot);

  // Reset search
  await searchInput.fill('');
  await page.waitForTimeout(500);

  // 2. Mobile Viewport Test (375 x 812)
  console.log('5. Testing Mobile Viewport (375x812)...');
  await page.setViewportSize({ width: 375, height: 812 });
  await page.waitForTimeout(600);

  const mobileShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/module_dialog_table_mobile.png';
  await page.screenshot({ path: mobileShot });
  console.log('Saved mobile screenshot to:', mobileShot);

  // Scroll down in modal to show stacked cards
  await page.evaluate(() => {
    const dialogBody = document.querySelector('[role="dialog"] .overflow-y-auto');
    if (dialogBody) {
      dialogBody.scrollTop = 380;
    }
  });
  await page.waitForTimeout(500);

  const mobileCardsShot = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/module_dialog_table_mobile_cards.png';
  await page.screenshot({ path: mobileCardsShot });
  console.log('Saved mobile cards screenshot to:', mobileCardsShot);

  await browser.close();
  console.log('VERIFICATION FINISHED SUCCESSFULLY!');
}

verifyModuleDialogTable().catch(err => {
  console.error(err);
  process.exit(1);
});
