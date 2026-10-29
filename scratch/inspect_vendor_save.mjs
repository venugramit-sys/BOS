import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  page.on('request', req => {
    if (req.url().includes('/api/entities/')) {
      console.log('API REQUEST:', req.method(), req.url());
      if (req.method() === 'POST' || req.method() === 'PATCH') {
        console.log('PAYLOAD:', req.postData());
      }
    }
  });

  page.on('response', async res => {
    if (res.url().includes('/api/entities/')) {
      console.log('API RESPONSE:', res.status(), res.url());
      try {
        const json = await res.json();
        console.log('RESPONSE BODY:', JSON.stringify(json));
      } catch (e) {
        console.log('RESPONSE TEXT:', await res.text());
      }
    }
  });

  // Login
  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // Navigate to Vendor Details
  console.log('Opening Vendor Details...');
  await page.locator('button:has-text("Vendor Details")').first().click();
  await page.waitForTimeout(2000);

  // Check state of VendorCard
  const state = await page.evaluate(() => {
    const p = window.__bos_active_page; // if any
    return {
      inputs: Array.from(document.querySelectorAll('input, select, textarea')).map(i => ({
        name: i.name,
        value: i.value,
        id: i.id
      }))
    };
  });
  console.log('Vendor inputs:', state.inputs);

  // Click Save
  const saveBtn = page.locator('button:has-text("Save")').first();
  console.log('Save button exists:', await saveBtn.count() > 0);
  if (await saveBtn.count() > 0) {
    console.log('Save button disabled:', await saveBtn.isDisabled());
    await saveBtn.click({ force: true });
    await page.waitForTimeout(2000);
  }

  await browser.close();
}

main().catch(console.error);
