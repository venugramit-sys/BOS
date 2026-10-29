import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function verifyMaturePageDesigner() {
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
  // Ensure Pages is expanded
  const loginPageItem = page.locator('text=LoginPage').first();
  if (await loginPageItem.count() === 0) {
    await pagesHeader.click();
    await page.waitForTimeout(500);
  }

  // Click LoginPage to open in designer
  await page.locator('text=LoginPage').first().click();
  await page.waitForTimeout(3000);

  // Capture full designer view showing mature Toolbox and canvas
  const fullShotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/page_designer_mature_overview.png';
  await page.screenshot({ path: fullShotPath });
  console.log('Saved mature overview screenshot to:', fullShotPath);

  // Select the "User ID" / username field on the canvas to inspect PropertyPanel
  console.log('4. Selecting username field to inspect mature bos-ui PropertyPanel...');
  const userField = page.locator('[data-bos-node*="username"]').first();
  if (await userField.count() > 0) {
    await userField.click();
  } else {
    // Click on input container
    await page.locator('input[placeholder="User ID"]').first().click({ force: true });
  }
  await page.waitForTimeout(1000);

  const selectedShotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/page_designer_mature_selected_property_panel.png';
  await page.screenshot({ path: selectedShotPath });
  console.log('Saved selected property panel screenshot to:', selectedShotPath);

  // Open "New Variable" dialog in Toolbox
  console.log('5. Testing New Variable dialog...');
  const newVarBtn = page.locator('button:has-text("New Var")').first();
  if (await newVarBtn.count() > 0) {
    await newVarBtn.click();
    await page.waitForTimeout(800);

    const dialogShotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/page_designer_new_variable_dialog.png';
    await page.screenshot({ path: dialogShotPath });
    console.log('Saved new variable dialog screenshot to:', dialogShotPath);
  }

  await browser.close();
  console.log('VERIFICATION COMPLETE');
}

verifyMaturePageDesigner().catch(err => {
  console.error(err);
  process.exit(1);
});
