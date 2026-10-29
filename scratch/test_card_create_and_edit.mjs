import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  page.on('request', req => {
    if (req.url().includes('/api/entities/')) {
      console.log('>>> REQ:', req.method(), req.url());
      if (req.method() === 'POST' || req.method() === 'PATCH') {
        console.log('>>> BODY:', req.postData());
      }
    }
  });

  page.on('response', async res => {
    if (res.url().includes('/api/entities/')) {
      console.log('<<< RES:', res.status(), res.url());
      try {
        console.log('<<< RES BODY:', JSON.stringify(await res.json()));
      } catch {
        console.log('<<< RES TEXT:', await res.text());
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

  // --- TEST A: VendorCard Edit existing record ---
  console.log('\n--- TEST A: Edit VendorCard existing record ---');
  await page.locator('button:has-text("Vendor Details")').first().click();
  await page.waitForTimeout(2000);

  const nameInput = page.locator('input[name="name"]');
  console.log('Original name value:', await nameInput.inputValue());
  await nameInput.fill('Acme Industrial Supplies Ltd');

  // Verify save button is now enabled
  const saveBtn = page.locator('button:has-text("Save")').first();
  console.log('Save button disabled after typing:', await saveBtn.isDisabled());

  console.log('Clicking Save...');
  await saveBtn.click();
  await page.waitForTimeout(2000);

  // --- TEST B: VendorCard Create New record ---
  console.log('\n--- TEST B: Create brand new record via VendorCard New action ---');
  const newBtn = page.locator('button:has-text("New")').first();
  await newBtn.click();
  await page.waitForTimeout(1500);

  const vendorInputs = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('input')).map(i => ({
      name: i.name,
      id: i.id,
      disabled: i.disabled,
      readOnly: i.readOnly
    }));
  });
  console.log('Inputs on VendorCard after clicking New:', vendorInputs);

  // Find vendor code / no input
  // In VendorCard, no is the primary key: field(no)
  const noInput = page.locator('input[name="no"], input[id="bos-f-VendorCard-2"]');
  if (await noInput.count() > 0) {
    console.log('Filling no/code...');
    await noInput.fill('VEND-888');
  }
  const newNameInput = page.locator('input[name="name"]');
  if (await newNameInput.count() > 0) {
    console.log('Filling name...');
    await newNameInput.fill('Apex Global Dynamics');
  }
  const emailInput = page.locator('input[name="contact_email"], input[name="contactEmail"]');
  if (await emailInput.count() > 0) {
    console.log('Filling contact email...');
    await emailInput.fill('orders@apex-global.com');
  }

  console.log('Clicking Save on new Vendor...');
  await saveBtn.click();
  await page.waitForTimeout(2500);

  // --- TEST C: RoleCard Create New record ---
  console.log('\n--- TEST C: RoleList -> New -> RoleCard ---');
  await page.locator('button:has-text("Roles & Permissions")').first().click();
  await page.waitForTimeout(1500);

  const roleNewBtn = page.locator('button:has-text("New")').first();
  await roleNewBtn.click();
  await page.waitForTimeout(1500);

  const roleInputs = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('input')).map(i => ({
      name: i.name,
      id: i.id,
      disabled: i.disabled,
      readOnly: i.readOnly
    }));
  });
  console.log('RoleCard inputs after clicking New:', roleInputs);

  // Fill in role fields
  const roleCodeInput = page.locator('input[name="code"], input[name="role"], input[name="id"], input[id="bos-f-RoleCard-2"]');
  if (await roleCodeInput.count() > 0) {
    console.log('Filling role code...');
    await roleCodeInput.fill('COMPLIANCE_OFFICER');
  }
  const roleNameInput = page.locator('input[name="name"]');
  if (await roleNameInput.count() > 0) {
    console.log('Filling role name...');
    await roleNameInput.fill('Compliance & Risk Officer');
  }
  const roleDescInput = page.locator('input[name="description"]');
  if (await roleDescInput.count() > 0) {
    console.log('Filling role description...');
    await roleDescInput.fill('Audits regulatory compliance and trade risks');
  }

  const roleSaveBtn = page.locator('button:has-text("Save")').first();
  console.log('Clicking Save on RoleCard...');
  await roleSaveBtn.click();
  await page.waitForTimeout(2500);

  // --- TEST D: UserList -> New -> UserCard ---
  console.log('\n--- TEST D: UserList -> New -> UserCard ---');
  await page.locator('button:has-text("Users")').first().click();
  await page.waitForTimeout(1500);

  const userNewBtn = page.locator('button:has-text("New")').first();
  await userNewBtn.click();
  await page.waitForTimeout(1500);

  const uInputs = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('input')).map(i => ({
      name: i.name,
      id: i.id,
      disabled: i.disabled,
      readOnly: i.readOnly
    }));
  });
  console.log('UserCard inputs after clicking New:', uInputs);

  for (const inp of uInputs) {
    if (inp.name === 'username' || inp.name === 'user_id' || inp.name === 'code') {
      await page.locator(`input[name="${inp.name}"]`).fill('david.compliance');
    } else if (inp.name === 'name' || inp.name === 'fullName') {
      await page.locator(`input[name="${inp.name}"]`).fill('David Compliance');
    } else if (inp.name === 'email') {
      await page.locator(`input[name="${inp.name}"]`).fill('david.compliance@system.local');
    } else if (inp.name === 'password' || inp.name === 'passwordHash') {
      await page.locator(`input[name="${inp.name}"]`).fill('Compliance#2026');
    }
  }

  const userSaveBtn = page.locator('button:has-text("Save")').first();
  console.log('Clicking Save on UserCard...');
  await userSaveBtn.click();
  await page.waitForTimeout(2500);

  await page.screenshot({ path: 'scratch/final_interactive_test.png' });
  await browser.close();
  console.log('\n=== Interactive Record Tests Finished ===');
}

main().catch(console.error);
