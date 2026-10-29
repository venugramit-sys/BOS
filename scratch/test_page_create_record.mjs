import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function run() {
  console.log('--- Testing Interactive Record Creation through BOS Pages ---');
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const logs = [];
  page.on('console', msg => {
    logs.push(`[${msg.type()}] ${msg.text()}`);
    if (msg.type() === 'error') {
      console.log('BROWSER ERROR:', msg.text());
    }
  });

  page.on('pageerror', err => {
    console.log('BROWSER UNCAUGHT:', err.message);
  });

  // Login
  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // 1. Open Roles & Permissions (RoleList)
  console.log('Navigating to Roles & Permissions...');
  await page.locator('button:has-text("Roles & Permissions")').first().click();
  await page.waitForTimeout(2000);

  // Read table rows
  const roleRows = await page.evaluate(() => {
    const rows = Array.from(document.querySelectorAll('tr, [role="row"]'));
    return rows.map(r => r.innerText.trim().replace(/\n+/g, ' | '));
  });
  console.log('RoleList Table Rows Count:', roleRows.length);
  console.log('RoleList Table Rows Sample:', roleRows.slice(0, 8));

  // 2. Click "+ New" to open RoleCard or create a new Role
  console.log('Clicking "+ New" button...');
  const newBtn = page.locator('button:has-text("New")').first();
  await newBtn.click();
  await page.waitForTimeout(2000);

  await page.screenshot({ path: 'scratch/role_new_clicked.png' });
  console.log('Saved role_new_clicked.png');

  // Check inputs on screen
  const inputs = await page.evaluate(() => {
    const list = Array.from(document.querySelectorAll('input, select, textarea, button'));
    return list.map(el => ({
      tag: el.tagName,
      type: el.getAttribute('type'),
      name: el.getAttribute('name'),
      id: el.id,
      placeholder: el.getAttribute('placeholder'),
      value: el.value,
      text: el.innerText ? el.innerText.trim().slice(0, 30) : ''
    })).filter(x => x.name || x.placeholder || (x.tag === 'BUTTON' && x.text));
  });
  console.log('Active Form Controls on Screen:\n', JSON.stringify(inputs, null, 2));

  // 3. Test opening Vendor Details
  console.log('Opening Vendor Details page from sidebar Pages list...');
  const vendorNav = page.locator('button:has-text("Vendor Details")').first();
  if (await vendorNav.count() > 0) {
    await vendorNav.click();
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'scratch/vendor_page.png' });
    console.log('Saved vendor_page.png');

    const vendorText = await page.evaluate(() => document.body.innerText);
    console.log('Vendor page text snippet:\n', vendorText.slice(0, 400));
  }

  // 4. Test opening Purchase Order page
  console.log('Opening Purchase Order page from sidebar Pages list...');
  const poNav = page.locator('button:has-text("Purchase Order")').first();
  if (await poNav.count() > 0) {
    await poNav.click();
    await page.waitForTimeout(2000);
    await page.screenshot({ path: 'scratch/purchase_order_page.png' });
    console.log('Saved purchase_order_page.png');

    const poText = await page.evaluate(() => document.body.innerText);
    console.log('Purchase Order page text snippet:\n', poText.slice(0, 400));
  }

  await browser.close();
  console.log('--- Completed Interactive Record Test ---');
}

run().catch(console.error);
