import { chromium } from '../ui/node_modules/playwright/index.mjs';

const RESULTS = {
  uxFixes: [],
  recordsCreated: [],
  failures: []
};

async function testAllFixes() {
  console.log('================================================================');
  console.log('VERIFYING ALL UX / DX IMPLEMENTATIONS & MULTI-TABLE OPERATIONS');
  console.log('================================================================\n');

  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  page.on('console', msg => {
    if (msg.type() === 'error' || msg.text().includes('Uncaught') || msg.text().includes('TypeError')) {
      console.log('  [BROWSER ERROR]', msg.text());
    }
  });

  // Step 1: Sign in
  console.log('[STEP 1] Authenticating as Admin...');
  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);
  console.log('✓ Successfully authenticated\n');

  async function openPageTab(pageName, mode = 'create') {
    await page.evaluate(({ pageName, mode }) => {
      const tabId = `${pageName}:${mode}:${Date.now()}`;
      window.__bos_shell.openTab({
        id: tabId,
        title: `Test ${pageName}`,
        type: 'page',
        page: pageName,
        params: { mode }
      });
    }, { pageName, mode });
    await page.waitForTimeout(1500);
  }

  // --- UX FIX 1: Pristine Save & Inline Field Error Highlighting & Auto-focus ---
  console.log('[TEST UX 1] Testing Pristine Save & Inline Field Error Surface on RoleCard...');
  await openPageTab('RoleCard', 'create');

  const saveBtn = page.locator('button:has-text("Save")').first();
  const isSaveDisabledInitial = await saveBtn.isDisabled();
  console.log(`  Save button disabled on initial new record? ${isSaveDisabledInitial} (Expected: false)`);
  if (!isSaveDisabledInitial) {
    RESULTS.uxFixes.push('Pristine new record Save button enabled');
  } else {
    RESULTS.failures.push('Save button was disabled on pristine new record');
  }

  // Click Save without filling required fields
  console.log('  Clicking Save with empty form to trigger validation...');
  await saveBtn.click();
  await page.waitForTimeout(800);

  // Check for inline error text and has-error border
  const inlineErrors = await page.locator('.bos-field-error').allTextContents();
  console.log('  Inline field errors rendered:', inlineErrors);
  const hasErrorContainers = await page.locator('.bos-field.has-error').count();
  console.log(`  Fields with .has-error highlight: ${hasErrorContainers}`);

  if (inlineErrors.length > 0 && hasErrorContainers > 0) {
    RESULTS.uxFixes.push('Inline field error display & .has-error border verified');
    console.log('  ✓ Inline error surface confirmed');
  } else {
    RESULTS.failures.push('Inline field errors were not rendered');
  }

  // Verify typing clears the error
  console.log('  Typing into role code to verify dynamic error clearing...');
  const codeInput = page.locator('[data-field="code"] input, input[name="code"]').first();
  await codeInput.fill('ROLE_TEST_AUTO');
  await page.waitForTimeout(400);

  const errorsAfterTyping = await page.locator('[data-field="code"] .bos-field-error').count();
  console.log(`  Errors remaining on role code after typing: ${errorsAfterTyping} (Expected: 0)`);
  if (errorsAfterTyping === 0) {
    RESULTS.uxFixes.push('Dynamic inline error clearing on input verified');
  } else {
    RESULTS.failures.push('Error was not cleared after user entered value');
  }

  // --- UX FIX 2: Password Masking and Eye Toggle on UserCard ---
  console.log('\n[TEST UX 2] Testing Password Input Masking and Toggle on UserCard...');
  await openPageTab('UserCard', 'create');

  const passwordInput = page.locator('[data-field="passwordHash"] input, [data-key="password_hash"] input').first();
  const passwordType = await passwordInput.getAttribute('type');
  console.log(`  UserCard password input type: "${passwordType}" (Expected: "password")`);
  if (passwordType === 'password') {
    RESULTS.uxFixes.push('UserCard password input type="password" security mask');
  } else {
    RESULTS.failures.push(`UserCard password type was "${passwordType}", expected "password"`);
  }

  // --- UX FIX 3: Unsaved Changes Dirty State Tracking in Tabs ---
  console.log('\n[TEST UX 3] Testing Tab Unsaved Changes Indicator (isDirty)...');
  const dirtyDotsBefore = await page.locator('.h-9 .bg-amber-500').count();
  console.log(`  Dirty indicators before input: ${dirtyDotsBefore}`);

  const userInput = page.locator('[data-field="fullName"] input, input[name="full_name"]').first();
  await userInput.fill('Test Officer');
  await page.waitForTimeout(600);

  const dirtyDotsAfter = await page.locator('.h-9 .bg-amber-500').count();
  console.log(`  Dirty indicators after modifying input: ${dirtyDotsAfter} (Expected: >= 1)`);
  if (dirtyDotsAfter >= 1) {
    RESULTS.uxFixes.push('Tab dirty indicator (isDirty amber dot) reactive sync verified');
    console.log('  ✓ Tab dirty indicator active');
  } else {
    RESULTS.failures.push('Tab dirty indicator did not activate on input');
  }

  // --- UX FIX 4: Combobox ERP Keyboard Autocomplete on PurchaseOrderCard ---
  console.log('\n[TEST UX 4] Testing Combobox ERP Keyboard Navigation on PurchaseOrderCard...');
  await openPageTab('PurchaseOrderCard', 'create');

  const vendorCombobox = page.locator('[data-field="vendorNo"] [role="combobox"]').first();
  await vendorCombobox.focus();
  await page.waitForTimeout(300);

  // Type alphanumeric character "V" directly on focused trigger
  console.log('  Typing "V" directly into focused Combobox trigger...');
  await page.keyboard.type('V');
  await page.waitForTimeout(800);

  const popoverVisible = await page.locator('input[placeholder*="Type to filter"]').isVisible();
  console.log(`  Popover opened automatically on alphanumeric keystroke? ${popoverVisible}`);
  if (popoverVisible) {
    RESULTS.uxFixes.push('Combobox alphanumeric auto-open verified');
  } else {
    RESULTS.failures.push('Combobox did not auto-open on typing');
  }

  // Now press Enter to select first matching vendor
  console.log('  Pressing Enter in searchbox to autocomplete top item...');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(500);

  const selectedVendorText = await vendorCombobox.innerText();
  console.log(`  Selected vendor text in Combobox: "${selectedVendorText.trim()}"`);
  if (selectedVendorText.includes('VEND-')) {
    RESULTS.uxFixes.push('Combobox Enter/Tab keyboard autoselect verified');
    console.log('  ✓ Combobox keyboard selection complete');
  } else {
    RESULTS.failures.push('Combobox did not select item on Enter');
  }

  // --- MULTI-TABLE 15-RECORD COMPREHENSIVE RUN WITH ALL FIXES ACTIVE ---
  console.log('\n[STEP 2] Running Multi-Table 15-Record Verification Run with Active Fixes...');
  const runId = Math.floor(Math.random() * 8999 + 1000);

  async function fillField(fieldName, value) {
    const container = page.locator(`[data-field="${fieldName}"]`);
    if (await container.count() > 0) {
      const input = container.locator('input, textarea, select').first();
      if (await input.count() > 0) {
        const type = await input.getAttribute('type');
        const tagName = await input.evaluate(el => el.tagName.toLowerCase());
        if (type === 'checkbox' || type === 'switch') {
          if (value) await input.check();
          else await input.uncheck();
        } else if (tagName === 'select') {
          await input.selectOption(String(value));
        } else {
          await input.fill(String(value));
        }
        return true;
      }
    }
    const direct = page.locator(`input[name="${fieldName}"], textarea[name="${fieldName}"], select[name="${fieldName}"]`).first();
    if (await direct.count() > 0) {
      await direct.fill(String(value));
      return true;
    }
    return false;
  }

  async function saveRecord(pageName, label) {
    const saveBtn = page.locator('button:has-text("Save")').first();
    await saveBtn.click();
    await page.waitForTimeout(2000);
    const toasts = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast')).map(t => t.innerText.trim());
    });
    const hasFail = toasts.some(t => t.includes('Action Failed') || t.includes('Error'));
    if (!hasFail) {
      RESULTS.recordsCreated.push(`${pageName}: ${label}`);
      console.log(`  ✓ Created ${pageName} (${label})`);
      return true;
    } else {
      RESULTS.failures.push(`${pageName}: ${label} - ${toasts.join('; ')}`);
      console.log(`  ✗ FAILED ${pageName} (${label}): ${toasts.join('; ')}`);
      return false;
    }
  }

  // 1. Roles
  for (const c of [`CHIEF_EXEC_${runId}`, `AUDIT_LEAD_${runId}`, `DEV_DIRECTOR_${runId}`]) {
    await openPageTab('RoleCard', 'create');
    await fillField('code', c);
    await fillField('name', `Title for ${c}`);
    await fillField('description', 'Executive role');
    await saveRecord('RoleCard', c);
  }

  // 2. Users
  for (const u of [`exec.${runId}`, `auditor.${runId}`, `dev.${runId}`]) {
    await openPageTab('UserCard', 'create');
    await fillField('username', u);
    await fillField('email', `${u}@enterprise.local`);
    await fillField('fullName', `Full Name ${u}`);
    await fillField('passwordHash', 'EnterpriseSec#2026!');
    await saveRecord('UserCard', u);
  }

  // 3. Vendors
  for (const v of [`VEND-A${runId}`, `VEND-B${runId}`, `VEND-C${runId}`]) {
    await openPageTab('VendorCard', 'create');
    await fillField('no', v);
    await fillField('name', `Vendor Enterprise ${v}`);
    await fillField('contactEmail', `contact@${v.toLowerCase()}.com`);
    await saveRecord('VendorCard', v);
  }

  // 4. Purchase Orders
  for (const po of [`PO-801-${runId}`, `PO-802-${runId}`, `PO-803-${runId}`]) {
    await openPageTab('PurchaseOrderCard', 'create');
    await fillField('orderNo', po);
    // Use combobox
    const cb = page.locator('[data-field="vendorNo"] [role="combobox"]').first();
    await cb.click();
    await page.waitForTimeout(400);
    const search = page.locator('input[placeholder*="Type to filter"]').first();
    await search.fill('VEND-');
    await page.waitForTimeout(400);
    await page.keyboard.press('Enter');
    await page.waitForTimeout(400);
    await fillField('currency', 'USD');
    await fillField('totalAmount', '12500');
    await saveRecord('PurchaseOrderCard', po);
  }

  // 5. Auth Configurations
  for (const cfg of [`CFG_HQ_${runId}`, `CFG_CLOUD_${runId}`, `CFG_EDGE_${runId}`]) {
    await openPageTab('AuthConfigurationCard', 'create');
    await fillField('id', cfg);
    await fillField('applicationTitle', `Platform Gateway ${cfg}`);
    await fillField('welcomeMessage', 'Sign in to access corporate cloud nodes');
    await fillField('primaryColor', '#4f46e5');
    await saveRecord('AuthConfigurationCard', cfg);
  }

  console.log('\n================================================================');
  console.log('FINAL AUDIT RESULTS:');
  console.log(`  UX Fixes Verified: ${RESULTS.uxFixes.length}`);
  RESULTS.uxFixes.forEach(f => console.log(`   + ${f}`));
  console.log(`  Records Successfully Created: ${RESULTS.recordsCreated.length} / 15`);
  console.log(`  Failures: ${RESULTS.failures.length}`);
  if (RESULTS.failures.length > 0) {
    RESULTS.failures.forEach(f => console.log(`   - ${f}`));
  }
  console.log('================================================================\n');

  await page.screenshot({ path: 'scratch/all_fixes_verified.png' });
  await browser.close();
}

testAllFixes().catch(console.error);
