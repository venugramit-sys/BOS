import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function testCollapseAndUx() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });
  
  const consoleErrors = [];
  page.on('console', msg => {
    if (msg.type() === 'error') consoleErrors.push(msg.text());
  });

  console.log('1. Navigating to http://localhost:3000/studio directly...');
  const response = await page.goto('http://localhost:3000/studio');
  console.log('Initial HTTP response status:', response.status());

  await page.waitForTimeout(1000);

  // Authenticate
  console.log('2. Authenticating as admin...');
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // Go to /studio
  await page.goto('http://localhost:3000/studio');
  await page.waitForTimeout(2000);

  // Target the specific module accordion header
  const moduleHeaderBtn = page.locator('button[data-module-id="bos.auth"]');
  console.log('Found module header button:', (await moduleHeaderBtn.count()) > 0);

  const moduleContent = page.locator('div[data-module-content="bos.auth"]');
  const contentVisibleInitial = await moduleContent.isVisible();
  console.log('Module content visible initially (should be true):', contentVisibleInitial);
  console.log('aria-expanded initially:', await moduleHeaderBtn.getAttribute('aria-expanded'));

  // CLICK MODULE HEADER TO COLLAPSE
  console.log('3. Clicking module header to COLLAPSE...');
  await moduleHeaderBtn.click();
  await page.waitForTimeout(600);

  const contentVisibleAfterCollapse = await moduleContent.isVisible();
  console.log('Module content visible after 1st click (must be false):', contentVisibleAfterCollapse);
  console.log('aria-expanded after collapse:', await moduleHeaderBtn.getAttribute('aria-expanded'));

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/studio_module_collapsed.png' });
  console.log('Saved studio_module_collapsed.png');

  // CLICK MODULE HEADER TO RE-EXPAND
  console.log('4. Clicking module header to RE-EXPAND...');
  await moduleHeaderBtn.click();
  await page.waitForTimeout(600);

  const contentVisibleAfterExpand = await moduleContent.isVisible();
  console.log('Module content visible after 2nd click (must be true):', contentVisibleAfterExpand);
  console.log('aria-expanded after re-expand:', await moduleHeaderBtn.getAttribute('aria-expanded'));

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/studio_module_reexpanded.png' });
  console.log('Saved studio_module_reexpanded.png');

  console.log('Console errors captured:', consoleErrors);

  await browser.close();

  if (contentVisibleInitial && !contentVisibleAfterCollapse && contentVisibleAfterExpand && response.status() === 200) {
    console.log('>>> VERIFICATION PASSED: 404 resolved (HTTP 200) & Accordion collapses and expands perfectly! <<<');
  } else {
    throw new Error('Verification failed!');
  }
}

testCollapseAndUx().catch(err => {
  console.error(err);
  process.exit(1);
});
