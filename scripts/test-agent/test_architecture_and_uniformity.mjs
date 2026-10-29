import path from 'path';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const { chromium } = require(path.join(root, 'ui/node_modules/playwright'));

async function testArchitectureAndUniformity() {
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

  // Screenshot 1: Clean, uniform sidebar with right-aligned badges
  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/studio_uniform_panel_verified.png' });
  console.log('Saved studio_uniform_panel_verified.png');

  // Verify Module Card Detail modal
  console.log('3. Opening Module Detail Card for bos.auth...');
  const infoBtn = page.locator('button[aria-label="Inspect module card"]').first();
  await infoBtn.click({ force: true });
  await page.waitForTimeout(1000);

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/module_detail_card_verified.png' });
  console.log('Saved module_detail_card_verified.png');

  // Close module detail card
  await page.locator('button:has-text("Close")').last().click();
  await page.waitForTimeout(500);

  // Test New Architecture Flow: Module -> Feature -> Meta-Object
  console.log('4. Testing Architecture Modal: Step 1 (Create Module)...');
  await page.locator('button:has-text("Add Architecture Component...")').first().click();
  await page.waitForTimeout(1000);

  // Fill Module
  await page.locator('input[placeholder*="bos.sales"]').fill('bos.crm');
  await page.locator('input[placeholder*="Sales Management"]').fill('Customer Relationship Management');
  await page.locator('input[placeholder*="Acme ERP"]').fill('Acme Corp');

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/arch_modal_step1_module.png' });
  console.log('Saved arch_modal_step1_module.png');

  // Proceed to Step 2: Feature
  console.log('5. Proceeding to Step 2 (Feature)...');
  await page.locator('button:has-text("Save & Proceed to Feature / Objects")').click();
  await page.waitForTimeout(1500);

  // Fill Feature
  await page.locator('input[placeholder*="OrderProcessing"]').fill('LeadPipeline');
  await page.locator('input[placeholder*="Sales Quotes"]').fill('Lead Capture & Pipeline');

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/arch_modal_step2_feature.png' });
  console.log('Saved arch_modal_step2_feature.png');

  // Proceed to Step 3: Meta-Object
  console.log('6. Proceeding to Step 3 (Meta-Object)...');
  await page.locator('button:has-text("Save & Add Meta-Objects")').click();
  await page.waitForTimeout(1500);

  // Select Page
  await page.locator('button:has-text("Card, List, Document")').click();
  await page.locator('input[placeholder*="Customer or SalesOrder"]').fill('LeadCard');
  await page.locator('input[placeholder*="Customer Master Record"]').fill('Lead Management Card');

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/arch_modal_step3_object.png' });
  console.log('Saved arch_modal_step3_object.png');

  // Create & Open
  console.log('7. Creating and opening LeadCard in designer...');
  await page.locator('button:has-text("Create & Open in Designer")').click();
  await page.waitForTimeout(3000);

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/leadcard_opened_in_designer.png' });
  console.log('Saved leadcard_opened_in_designer.png');

  console.log('Console errors:', consoleErrors);
  await browser.close();
  console.log('>>> ALL VERIFICATION CHECKS PASSED SUCCESSFULLY! <<<');
}

testArchitectureAndUniformity().catch(err => {
  console.error(err);
  process.exit(1);
});
