import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifySingleModule() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const page = await browser.newPage({ viewport: { width: 1600, height: 950 } });

  console.log('1. Navigating to http://localhost:3000/#/studio...');
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(1000);

  // Authenticate
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // Open studio
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(2000);

  // Check module count
  const moduleElements = await page.evaluate(() => {
    // Find module cards
    const text = document.body.innerText;
    const hasAuth = text.includes('bos.auth');
    const hasCustomizations = text.includes('bos.customizations');
    
    // Find all module titles
    const cards = Array.from(document.querySelectorAll('aside button')).filter(b => b.innerText && b.innerText.includes('CORE') || b.innerText.includes('v1.0.0'));
    return {
      hasAuth,
      hasCustomizations,
      cardTexts: cards.map(c => c.innerText.trim())
    };
  });
  console.log('Module evaluation in UI:', JSON.stringify(moduleElements, null, 2));

  // Check if LoginPage_Customization is shown under bos.auth Extensions
  const extFound = await page.locator('text=LoginPage_Customization').count();
  console.log('LoginPage_Customization found:', extFound > 0);

  const screenshotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/studio_single_module_verified.png';
  await page.screenshot({ path: screenshotPath });
  console.log('Saved screenshot to:', screenshotPath);

  await browser.close();
  console.log('TEST COMPLETE');
}

verifySingleModule().catch(console.error);
