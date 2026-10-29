import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  console.log('=== RUNNING FINAL ENTERPRISE PAGE CREATION SUITE ===');
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  page.on('request', req => {
    if (req.url().includes('/api/entities/')) {
      console.log('>>> REQ:', req.method(), req.url());
      if (req.method() === 'POST' || req.method() === 'PATCH') {
        console.log('    BODY:', req.postData());
      }
    }
  });

  page.on('response', async res => {
    if (res.url().includes('/api/entities/')) {
      console.log('<<< RES:', res.status(), res.url());
      try {
        const body = await res.json();
        console.log('    BODY:', JSON.stringify(body));
      } catch {
        console.log('    TEXT:', await res.text());
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

  // ----------------------------------------------------
  // TEST 1: RoleCard - Create New Role
  // ----------------------------------------------------
  const roleCodeSuffix = Date.now().toString().slice(-4);
  const testRoleCode = `ROLE_${roleCodeSuffix}`;
  console.log(`\n--- 1. Testing Role creation: ${testRoleCode} ---`);
  await page.locator('button:has-text("Roles & Permissions")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  const roleCode = page.locator('input[name="code"], input[id="bos-f-RoleCard-2"]').first();
  await roleCode.fill(testRoleCode);

  const roleName = page.locator('input[name="name"]').first();
  await roleName.fill(`Security Specialist ${roleCodeSuffix}`);

  const roleDesc = page.locator('input[name="description"]').first();
  await roleDesc.fill('SOC operations and zero-trust verification');

  const roleSave = page.locator('button:has-text("Save")').first();
  console.log('Role Save disabled:', await roleSave.isDisabled());
  await roleSave.click();
  await page.waitForTimeout(2500);

  // ----------------------------------------------------
  // TEST 2: VendorCard - Create New Vendor
  // ----------------------------------------------------
  const vendorNo = `VEND-${Date.now().toString().slice(-4)}`;
  console.log(`\n--- 2. Testing Vendor creation: ${vendorNo} ---`);
  await page.locator('button:has-text("Vendor Details")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  const vNo = page.locator('input[name="no"], input[id="bos-f-VendorCard-2"]').first();
  await vNo.fill(vendorNo);

  const vName = page.locator('input[name="name"]').first();
  await vName.fill('Global Photonics Instruments');

  const vEmail = page.locator('input[name="contact_email"], input[name="contactEmail"]').first();
  await vEmail.fill('orders@globalphotonics.com');

  const vSave = page.locator('button:has-text("Save")').first();
  console.log('Vendor Save disabled:', await vSave.isDisabled());
  await vSave.click();
  await page.waitForTimeout(2500);

  // ----------------------------------------------------
  // TEST 3: UserCard - Create New User
  // ----------------------------------------------------
  const userSuffix = Date.now().toString().slice(-4);
  const testUsername = `user.${userSuffix}`;
  console.log(`\n--- 3. Testing User creation: ${testUsername} ---`);
  await page.locator('button:has-text("Users")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  const uName = page.locator('input[name="username"], input[id="bos-f-UserCard-2"]').first();
  await uName.fill(testUsername);

  const uEmail = page.locator('input[name="email"]').first();
  await uEmail.fill(`${testUsername}@enterprise.local`);

  const uFull = page.locator('input[name="full_name"], input[name="fullName"]').first();
  await uFull.fill(`Enterprise User ${userSuffix}`);

  const uPass = page.locator('input[name="passwordHash"], input[type="password"]').first();
  if (await uPass.count() > 0) {
    console.log('Filling password field...');
    await uPass.fill('SecureSecret@2026!');
  }

  const uSave = page.locator('button:has-text("Save")').first();
  console.log('User Save disabled:', await uSave.isDisabled());
  await uSave.click();
  await page.waitForTimeout(2500);

  // ----------------------------------------------------
  // TEST 4: PurchaseOrderCard - Create PO
  // ----------------------------------------------------
  const poNo = `PO-${Date.now().toString().slice(-4)}`;
  console.log(`\n--- 4. Testing Purchase Order creation: ${poNo} ---`);
  await page.locator('button:has-text("Purchase Order")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  const poOrderNo = page.locator('input[name="order_no"], input[name="orderNo"], input[id="bos-f-PurchaseOrderCard-2"]').first();
  if (await poOrderNo.count() > 0) {
    await poOrderNo.fill(poNo);
  }

  const poVendorNo = page.locator('input[name="vendor_no"], input[name="vendorNo"]').first();
  if (await poVendorNo.count() > 0) {
    await poVendorNo.fill('VEND-001');
  }

  const poCurrency = page.locator('input[name="currency"]').first();
  if (await poCurrency.count() > 0) {
    await poCurrency.fill('USD');
  }

  const poSave = page.locator('button:has-text("Save")').first();
  console.log('PO Save disabled:', await poSave.isDisabled());
  if (!await poSave.isDisabled()) {
    await poSave.click();
    await page.waitForTimeout(2500);
  }

  await page.screenshot({ path: 'scratch/final_all_pages_success.png' });
  await browser.close();
  console.log('\n=== ALL TESTS EXECUTED ===');
}

main().catch(console.error);
