import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function testUiCreateRecords() {
  console.log('=== STEP 1: Browser Launch and Authentication ===');
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  page.on('console', msg => {
    if (msg.type() === 'error' || msg.text().includes('error') || msg.text().includes('fail')) {
      console.log('BROWSER LOG:', msg.type(), msg.text());
    }
  });

  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);

  // === STEP 2: Create a Role via RoleCard ===
  console.log('\n=== STEP 2: Creating Role via RoleCard ===');
  await page.locator('button:has-text("Roles & Permissions")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  // Inspect form fields on RoleCard
  const roleInputs = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('input')).map(i => ({
      name: i.name,
      id: i.id,
      placeholder: i.placeholder,
      value: i.value,
      type: i.type
    }));
  });
  console.log('RoleCard input fields:', roleInputs);

  // Find the inputs for name and description
  const nameInput = page.locator('input[name="name"]');
  const descInput = page.locator('input[name="description"]');
  const codeInput = page.locator('input[name="code"], input[name="role"], input[name="id"]');

  if (await codeInput.count() > 0) {
    console.log('Filling role code...');
    await codeInput.fill('WAREHOUSE_MGR');
  }
  if (await nameInput.count() > 0) {
    console.log('Filling role name...');
    await nameInput.fill('Warehouse Manager');
  }
  if (await descInput.count() > 0) {
    console.log('Filling role description...');
    await descInput.fill('Controls stock movements and inventory audits');
  }

  // Click Save
  console.log('Clicking Save on RoleCard...');
  const saveBtn = page.locator('button:has-text("Save")').first();
  await saveBtn.click();
  await page.waitForTimeout(2500);

  // Capture screenshot of result
  await page.screenshot({ path: 'scratch/role_saved_result.png' });

  // Check toasts or alerts
  const toasts = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast, .banner')).map(t => t.innerText.trim());
  });
  console.log('Toasts/Alerts after Role Save:', toasts);

  // === STEP 3: Create a User via UserList / UserCard ===
  console.log('\n=== STEP 3: Creating User via UserList -> New ===');
  await page.locator('button:has-text("Users")').first().click();
  await page.waitForTimeout(1500);

  const newUserBtn = page.locator('button:has-text("New")').first();
  if (await newUserBtn.count() > 0) {
    await newUserBtn.click();
    await page.waitForTimeout(1500);

    await page.screenshot({ path: 'scratch/user_card_opened.png' });

    const userInputs = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('input, select, textarea')).map(i => ({
        name: i.name,
        id: i.id,
        placeholder: i.placeholder,
        value: i.value,
        type: i.type
      }));
    });
    console.log('UserCard input fields:', userInputs);

    // Fill in user details
    for (const inp of userInputs) {
      if (inp.name === 'username' || inp.name === 'user_id' || inp.name === 'userId' || inp.name === 'code') {
        await page.locator(`input[name="${inp.name}"]`).fill('alice.warehouse');
      } else if (inp.name === 'name' || inp.name === 'fullName') {
        await page.locator(`input[name="${inp.name}"]`).fill('Alice Warehouse');
      } else if (inp.name === 'email') {
        await page.locator(`input[name="${inp.name}"]`).fill('alice.warehouse@system.local');
      } else if (inp.name === 'password' || inp.name === 'passwordHash') {
        await page.locator(`input[name="${inp.name}"]`).fill('SecurePass@2026!');
      }
    }

    console.log('Clicking Save on UserCard...');
    const userSaveBtn = page.locator('button:has-text("Save")').first();
    await userSaveBtn.click();
    await page.waitForTimeout(2500);

    await page.screenshot({ path: 'scratch/user_saved_result.png' });

    const userToasts = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast, .banner')).map(t => t.innerText.trim());
    });
    console.log('Toasts/Alerts after User Save:', userToasts);
  }

  // === STEP 4: Test VendorCard ===
  console.log('\n=== STEP 4: Testing VendorCard ===');
  const vendorNav = page.locator('button:has-text("Vendor Details")').first();
  if (await vendorNav.count() > 0) {
    await vendorNav.click();
    await page.waitForTimeout(1500);

    const vendorFields = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('input, select, textarea')).map(i => ({
        name: i.name,
        id: i.id,
        placeholder: i.placeholder,
        value: i.value,
        type: i.type
      }));
    });
    console.log('VendorCard input fields:', vendorFields);

    for (const inp of vendorFields) {
      if (inp.name === 'code' || inp.name === 'vendorNo' || inp.name === 'no') {
        await page.locator(`input[name="${inp.name}"]`).fill('VEND-004');
      } else if (inp.name === 'name') {
        await page.locator(`input[name="${inp.name}"]`).fill('Nordic Industrial Pumps Oy');
      } else if (inp.name === 'paymentTermsDays') {
        await page.locator(`input[name="${inp.name}"]`).fill('30');
      }
    }

    const vendorSaveBtn = page.locator('button:has-text("Save")').first();
    if (await vendorSaveBtn.count() > 0) {
      console.log('Clicking Save on VendorCard...');
      await vendorSaveBtn.click();
      await page.waitForTimeout(2500);
      await page.screenshot({ path: 'scratch/vendor_saved_result.png' });

      const vendorToasts = await page.evaluate(() => {
        return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast, .banner')).map(t => t.innerText.trim());
      });
      console.log('Toasts/Alerts after Vendor Save:', vendorToasts);
    }
  }

  // === STEP 5: Test PurchaseOrderCard ===
  console.log('\n=== STEP 5: Testing PurchaseOrderCard ===');
  const poNav = page.locator('button:has-text("Purchase Order")').first();
  if (await poNav.count() > 0) {
    await poNav.click();
    await page.waitForTimeout(1500);

    const poFields = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('input, select, textarea')).map(i => ({
        name: i.name,
        id: i.id,
        placeholder: i.placeholder,
        value: i.value,
        type: i.type
      }));
    });
    console.log('PurchaseOrderCard input fields:', poFields);

    for (const inp of poFields) {
      if (inp.name === 'no' || inp.name === 'orderNo') {
        await page.locator(`input[name="${inp.name}"]`).fill('PO-2001');
      } else if (inp.name === 'buyFromVendorNo' || inp.name === 'vendorNo') {
        await page.locator(`input[name="${inp.name}"]`).fill('VEND-001');
      }
    }

    const poSaveBtn = page.locator('button:has-text("Save")').first();
    if (await poSaveBtn.count() > 0) {
      console.log('Clicking Save on PurchaseOrderCard...');
      await poSaveBtn.click();
      await page.waitForTimeout(2500);
      await page.screenshot({ path: 'scratch/po_saved_result.png' });

      const poToasts = await page.evaluate(() => {
        return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast, .banner')).map(t => t.innerText.trim());
      });
      console.log('Toasts/Alerts after PO Save:', poToasts);
    }
  }

  await browser.close();
  console.log('=== All UI Page Tests Completed ===');
}

testUiCreateRecords().catch(console.error);
