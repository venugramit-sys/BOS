import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
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
        console.log('    BODY:', JSON.stringify(await res.json()));
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

  // Navigate to Users
  await page.locator('button:has-text("Users")').first().click();
  await page.waitForTimeout(1500);

  await page.locator('button:has-text("New")').first().click();
  await page.waitForTimeout(1500);

  const uInputs = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('input')).map(i => ({
      name: i.name,
      id: i.id,
      type: i.type
    }));
  });
  console.log('UserCard actual inputs:', uInputs);

  const suffix = Date.now().toString().slice(-4);
  const testUser = `sec.user${suffix}`;

  const uName = page.locator('input[name="username"]').first();
  await uName.fill(testUser);

  const uEmail = page.locator('input[name="email"]').first();
  await uEmail.fill(`${testUser}@enterprise.local`);

  const uFull = page.locator('input[name="full_name"]').first();
  await uFull.fill(`Security Officer ${suffix}`);

  const uPass = page.locator('input[name="password_hash"], input[name="passwordHash"], input[type="password"]').first();
  if (await uPass.count() > 0) {
    console.log('Filling password into:', await uPass.getAttribute('name'));
    await uPass.fill('SecureSecret@2026!');
  }

  const uSave = page.locator('button:has-text("Save")').first();
  console.log('Save disabled:', await uSave.isDisabled());
  await uSave.click();
  await page.waitForTimeout(2500);

  await browser.close();
  console.log('User test completed.');
}

main().catch(console.error);
