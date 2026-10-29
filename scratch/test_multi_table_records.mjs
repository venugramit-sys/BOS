import { chromium } from '../ui/node_modules/playwright/index.mjs';

const RESULTS = {
  passed: [],
  failed: [],
  gaps: [],
  observations: []
};

async function run() {
  console.log('================================================================');
  console.log('BOS MULTI-TABLE PRODUCTION RECORD CREATION & UX/DX TEST SUITE');
  console.log('================================================================\n');

  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const networkLogs = [];
  page.on('response', async (res) => {
    const url = res.url();
    if (url.includes('/api/entities/')) {
      let body = '';
      try {
        body = await res.text();
      } catch {}
      networkLogs.push({
        url,
        status: res.status(),
        method: res.request().method(),
        postData: res.request().postData(),
        response: body.slice(0, 300)
      });
    }
  });

  page.on('console', msg => {
    if (msg.type() === 'error' || msg.text().includes('Uncaught') || msg.text().includes('TypeError')) {
      console.log('  [BROWSER CONSOLE ERROR]', msg.text());
    }
  });

  // Step 1: Sign in
  console.log('[STEP 1] Navigating to BOS and Authenticating...');
  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1000);
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  await page.waitForTimeout(2000);
  console.log('✓ Successfully authenticated as admin\n');

  // Helper to open a page tab
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

  // Helper to fill form field
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

  // Helper to select lookup/combobox item
  async function selectLookup(fieldName, value) {
    const container = page.locator(`[data-field="${fieldName}"]`);
    const trigger = container.locator('button[role="combobox"]').first();
    if (await trigger.count() > 0) {
      await trigger.click();
      await page.waitForTimeout(600);
      const searchBox = page.locator('input[role="searchbox"]').first();
      if (await searchBox.count() > 0) {
        await searchBox.fill(value);
        await page.waitForTimeout(400);
      }
      const option = page.locator(`tr:has-text("${value}")`).first();
      if (await option.count() > 0) {
        await option.click();
        await page.waitForTimeout(400);
        return true;
      }
    }
    return false;
  }

  // Helper to click Save and verify
  async function saveAndVerify(tableName, recordId) {
    const saveBtn = page.locator('button:has-text("Save")').first();
    const isSaveDisabled = await saveBtn.isDisabled();
    if (isSaveDisabled) {
      throw new Error(`Save button is disabled for ${tableName} ${recordId}`);
    }

    const netCountBefore = networkLogs.length;
    await saveBtn.click();
    await page.waitForTimeout(2500);

    const relevant = networkLogs.slice(netCountBefore).filter(n => n.url.includes(`/api/entities/${tableName.toLowerCase()}`));
    const postReq = relevant.find(n => n.method === 'POST');
    const getReq = relevant.find(n => n.method === 'GET');

    const toasts = await page.evaluate(() => {
      return Array.from(document.querySelectorAll('[role="status"], [role="alert"], .toast')).map(t => t.innerText.trim());
    });

    return {
      postStatus: postReq?.status,
      getStatus: getReq?.status,
      postData: postReq?.postData,
      response: postReq?.response,
      toasts
    };
  }

  const runId = Math.floor(Math.random() * 8999 + 1000);

  // ==========================================================================
  // TABLE 1: Role (via RoleCard) - 3 Records
  // ==========================================================================
  console.log('--- [TABLE 1: Role] Testing 3 Records via RoleCard ---');
  const roles = [
    { code: `ROLE_COMPLIANCE_${runId}`, name: 'Compliance Officer', description: 'Oversees GDPR and statutory reporting' },
    { code: `ROLE_LOGISTICS_${runId}`, name: 'Logistics Supervisor', description: 'Manages multi-warehouse fleet and freight' },
    { code: `ROLE_TREASURY_${runId}`, name: 'Treasury Analyst', description: 'Monitors liquidity and banking cash flow' }
  ];

  for (const r of roles) {
    try {
      await openPageTab('RoleCard', 'create');
      await fillField('code', r.code);
      await fillField('name', r.name);
      await fillField('description', r.description);

      const res = await saveAndVerify('role', r.code);
      console.log(`  Role ${r.code}: POST=${res.postStatus}, GET=${res.getStatus}, toasts=${JSON.stringify(res.toasts)}`);
      if (res.postStatus === 201 && res.getStatus === 200) {
        RESULTS.passed.push(`Role: ${r.code}`);
      } else {
        RESULTS.failed.push(`Role: ${r.code} (POST=${res.postStatus}, GET=${res.getStatus})`);
      }
    } catch (err) {
      console.log(`  Role ${r.code} ERROR:`, err.message);
      RESULTS.failed.push(`Role: ${r.code} - ${err.message}`);
    }
  }

  // ==========================================================================
  // TABLE 2: User (via UserCard) - 3 Records
  // ==========================================================================
  console.log('\n--- [TABLE 2: User] Testing 3 Records via UserCard ---');
  const users = [
    { username: `grace.compliance${runId}`, email: `grace.compliance${runId}@enterprise.local`, fullName: 'Grace Compliance', password: 'SecretPassword@123' },
    { username: `henry.logistics${runId}`, email: `henry.logistics${runId}@enterprise.local`, fullName: 'Henry Logistics', password: 'SecretPassword@123' },
    { username: `isabel.treasury${runId}`, email: `isabel.treasury${runId}@enterprise.local`, fullName: 'Isabel Treasury', password: 'SecretPassword@123' }
  ];

  for (const u of users) {
    try {
      await openPageTab('UserCard', 'create');
      await fillField('username', u.username);
      await fillField('email', u.email);
      await fillField('fullName', u.fullName);
      await fillField('passwordHash', u.password);

      const res = await saveAndVerify('user', u.username);
      console.log(`  User ${u.username}: POST=${res.postStatus}, GET=${res.getStatus}, toasts=${JSON.stringify(res.toasts)}`);
      if (res.postStatus === 201 && res.getStatus === 200) {
        RESULTS.passed.push(`User: ${u.username}`);
      } else {
        RESULTS.failed.push(`User: ${u.username} (POST=${res.postStatus}, GET=${res.getStatus})`);
      }
    } catch (err) {
      console.log(`  User ${u.username} ERROR:`, err.message);
      RESULTS.failed.push(`User: ${u.username} - ${err.message}`);
    }
  }

  // ==========================================================================
  // TABLE 3: Vendor (via VendorCard) - 3 Records
  // ==========================================================================
  console.log('\n--- [TABLE 3: Vendor] Testing 3 Records via VendorCard ---');
  const vendors = [
    { no: `VEND-601`, name: 'Nordic Sensor Dynamics AS', contactEmail: 'sales@nordicsensors.no', tier: 'Volume' },
    { no: `VEND-602`, name: 'Alpine Precision Valves AG', contactEmail: 'info@alpinevalves.ch', tier: 'Standard' },
    { no: `VEND-603`, name: 'Pacific Composite Materials', contactEmail: 'export@pacificcomposites.au', tier: 'Enterprise' }
  ];

  for (const v of vendors) {
    try {
      await openPageTab('VendorCard', 'create');
      await fillField('no', v.no);
      await fillField('name', v.name);
      await fillField('contactEmail', v.contactEmail);
      if (v.tier) await fillField('tier', v.tier);

      const res = await saveAndVerify('vendor', v.no);
      console.log(`  Vendor ${v.no}: POST=${res.postStatus}, GET=${res.getStatus}, toasts=${JSON.stringify(res.toasts)}`);
      if (res.postStatus === 201 && res.getStatus === 200) {
        RESULTS.passed.push(`Vendor: ${v.no}`);
      } else {
        RESULTS.failed.push(`Vendor: ${v.no} (POST=${res.postStatus}, GET=${res.getStatus})`);
      }
    } catch (err) {
      console.log(`  Vendor ${v.no} ERROR:`, err.message);
      RESULTS.failed.push(`Vendor: ${v.no} - ${err.message}`);
    }
  }

  // ==========================================================================
  // TABLE 4: PurchaseOrderHeader (via PurchaseOrderCard) - 3 Records
  // ==========================================================================
  console.log('\n--- [TABLE 4: PurchaseOrderHeader] Testing 3 Records via PurchaseOrderCard ---');
  const pos = [
    { orderNo: `PO-7001-${runId}`, vendorNo: 'VEND-601', currency: 'USD', totalAmount: '24500', notes: 'Thermal imaging sensor kits' },
    { orderNo: `PO-7002-${runId}`, vendorNo: 'VEND-602', currency: 'USD', totalAmount: '51200', notes: 'High-pressure cryogenic valves' },
    { orderNo: `PO-7003-${runId}`, vendorNo: 'VEND-603', currency: 'USD', totalAmount: '18900', notes: 'Carbon fiber prepreg rolls' }
  ];

  for (const po of pos) {
    try {
      await openPageTab('PurchaseOrderCard', 'create');
      await fillField('orderNo', po.orderNo);
      await selectLookup('vendorNo', po.vendorNo);
      await fillField('currency', po.currency);
      await fillField('totalAmount', po.totalAmount);
      await fillField('notes', po.notes);

      const res = await saveAndVerify('purchase_order_header', po.orderNo);
      console.log(`  PO ${po.orderNo}: POST=${res.postStatus}, GET=${res.getStatus}, toasts=${JSON.stringify(res.toasts)}`);
      if (res.postStatus === 201 && res.getStatus === 200) {
        RESULTS.passed.push(`PO: ${po.orderNo}`);
      } else {
        RESULTS.failed.push(`PO: ${po.orderNo} (POST=${res.postStatus}, GET=${res.getStatus})`);
      }
    } catch (err) {
      console.log(`  PO ${po.orderNo} ERROR:`, err.message);
      RESULTS.failed.push(`PO: ${po.orderNo} - ${err.message}`);
    }
  }

  // ==========================================================================
  // TABLE 5: AuthConfiguration (via AuthConfigurationCard) - 3 Records
  // ==========================================================================
  console.log('\n--- [TABLE 5: AuthConfiguration] Testing 3 Records via AuthConfigurationCard ---');
  const configs = [
    { id: `CFG_INDIA_${runId}`, applicationTitle: 'BOS India Cloud Gateway', welcomeMessage: 'Secure SSO for Bangalore & Mumbai DC', primaryColor: '#f97316' },
    { id: `CFG_JAPAN_${runId}`, applicationTitle: 'BOS Tokyo Sovereign Node', welcomeMessage: 'Welcome to BOS Tokyo Enterprise Gateway', primaryColor: '#dc2626' },
    { id: `CFG_UK_${runId}`, applicationTitle: 'BOS London Financial Node', welcomeMessage: 'Welcome to BOS UK & Ireland Portal', primaryColor: '#1d4ed8' }
  ];

  for (const cfg of configs) {
    try {
      await openPageTab('AuthConfigurationCard', 'create');
      await fillField('id', cfg.id);
      await fillField('applicationTitle', cfg.applicationTitle);
      await fillField('welcomeMessage', cfg.welcomeMessage);
      await fillField('primaryColor', cfg.primaryColor);

      const res = await saveAndVerify('auth_configuration', cfg.id);
      console.log(`  AuthConfig ${cfg.id}: POST=${res.postStatus}, GET=${res.getStatus}, toasts=${JSON.stringify(res.toasts)}`);
      if (res.postStatus === 201 && res.getStatus === 200) {
        RESULTS.passed.push(`AuthConfig: ${cfg.id}`);
      } else {
        RESULTS.failed.push(`AuthConfig: ${cfg.id} (POST=${res.postStatus}, GET=${res.getStatus})`);
      }
    } catch (err) {
      console.log(`  AuthConfig ${cfg.id} ERROR:`, err.message);
      RESULTS.failed.push(`AuthConfig: ${cfg.id} - ${err.message}`);
    }
  }

  // Summary
  console.log('\n================================================================');
  console.log('FINAL TEST RUN SUMMARY:');
  console.log(`  Total Tested: ${RESULTS.passed.length + RESULTS.failed.length}`);
  console.log(`  Passed Records: ${RESULTS.passed.length}`);
  console.log(`  Failed Records: ${RESULTS.failed.length}`);
  if (RESULTS.failed.length > 0) {
    console.log('  Failures:', RESULTS.failed);
  }
  console.log('================================================================\n');

  await page.screenshot({ path: 'scratch/multi_table_final_verification.png' });
  await browser.close();
}

run().catch(console.error);
