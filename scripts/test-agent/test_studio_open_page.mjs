import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(1000);

  // Authenticate
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(1500);

  // Navigate to studio
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(2000);

  // Click on "Sign In" under PAGES
  console.log('Clicking "Sign In" in navigation...');
  const signInNav = page.locator('button:has-text("Sign In"), a:has-text("Sign In"), span:has-text("Sign In")').first();
  await signInNav.click();
  await page.waitForTimeout(2000);

  // Switch tab or view to Design if it opened in Run/View
  console.log('Looking for Design mode toggle if needed...');
  const designBtn = page.locator('button:has-text("Design"), button[title="Design"]').first();
  if (await designBtn.count() > 0) {
    await designBtn.click();
    await page.waitForTimeout(1500);
  }

  // Look for rememberMe switch on canvas or tree
  console.log('Looking for rememberMe switch...');
  const rememberMeEl = page.locator('text=Remember User ID, text=rememberMe').first();
  if (await rememberMeEl.count() > 0) {
    await rememberMeEl.click();
    console.log('Clicked rememberMe control');
    await page.waitForTimeout(1000);
  }

  // Look for labelPosition in PropertyPanel
  const labelPosRow = page.locator('text=labelPosition').first();
  console.log('labelPosition row found in PropertyPanel:', await labelPosRow.count() > 0);

  const selects = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('select')).map(s => {
      const opts = Array.from(s.options).map(o => o.value);
      return { id: s.id, name: s.name, options: opts };
    });
  });
  console.log('Select dropdowns in designer:', JSON.stringify(selects, null, 2));

  // Find the select for labelPosition
  const labelPosSelect = page.locator('select').filter({ has: page.locator('option[value="right"]') }).first();
  if (await labelPosSelect.count() > 0) {
    const opts = await labelPosSelect.evaluate(s => Array.from(s.options).map(o => o.value));
    console.log('SUCCESS: Found labelPosition select with options:', opts);
    
    // Select "right"
    await labelPosSelect.selectOption('right');
    await page.waitForTimeout(1000);
    console.log('Selected "right" in labelPosition dropdown.');
  }

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/designer_label_right_verified.png' });
  console.log('Saved screenshot to designer_label_right_verified.png');

  await browser.close();
}

main().catch(console.error);
