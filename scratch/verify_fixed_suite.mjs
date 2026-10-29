import { chromium } from '../ui/node_modules/playwright/index.mjs';
import * as fs from 'fs';

const SCREENSHOT_DIR = 'scratch/login-audit-fixed';
if (!fs.existsSync(SCREENSHOT_DIR)) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
}

async function run() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const results = [];
  const logTest = (name, passed, details) => {
    results.push({ name, passed, details });
    console.log(`[${passed ? 'PASS' : 'FAIL'}] ${name}: ${details}`);
  };

  console.log('\n=============================================');
  console.log('   BOS ENTERPRISE LOGIN UX VERIFICATION (POST-FIX)');
  console.log('=============================================\n');

  // 1. Initial Load & Autocomplete
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1500);

  await page.screenshot({ path: `${SCREENSHOT_DIR}/01_initial_load.png` });
  const userInput = page.locator('#bos-f-LoginPage-1');
  const passInput = page.locator('#bos-f-LoginPage-2');
  const signInBtn = page.locator('footer button:has-text("Sign In")');

  const uAuto = await userInput.getAttribute('autocomplete');
  const pAuto = await passInput.getAttribute('autocomplete');
  logTest('Autocomplete Attributes', uAuto === 'username' && pAuto === 'current-password',
    `User: ${uAuto}, Pass: ${pAuto}`);

  // 2. Inline & Banner Error (Empty User ID)
  await userInput.fill('');
  await passInput.fill('');
  await signInBtn.click();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/02_empty_error_banner.png` });

  const bannerText = await page.locator('.bos-page .border-rose-200, .bos-page .dark\\:border-rose-900').first().innerText().catch(() => '');
  logTest('Contextual Error Banner Title', bannerText.includes('Sign In Failed'), `Banner text: "${bannerText.replace(/\n/g, ' ')}"`);

  const userInlineErr = await page.locator('.bos-cell:has-text("User ID") span.text-rose-600').first().innerText().catch(() => '');
  logTest('Inline Field Error (User ID)', userInlineErr.includes('User ID'), `Inline: "${userInlineErr}"`);

  // 3. Password field validation
  await userInput.fill('admin');
  await passInput.fill('');
  await signInBtn.click();
  await page.waitForTimeout(1000);
  const passInlineErr = await page.locator('.bos-cell:has-text("Password") span.text-rose-600').first().innerText().catch(() => '');
  logTest('Inline Field Error (Password)', passInlineErr.includes('password'), `Inline: "${passInlineErr}"`);

  // 4. CapsLock Indicator Verification
  await passInput.focus();
  await page.evaluate(() => {
    const input = document.querySelector('#bos-f-LoginPage-2');
    const evt = new KeyboardEvent('keydown', { key: 'a', bubbles: true, cancelable: true });
    evt.getModifierState = (m) => m === 'CapsLock';
    input.dispatchEvent(evt);
  });
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/03_capslock_indicator.png` });
  const capsBadge = page.locator('span[title="Caps Lock is on"]');
  const capsVisible = await capsBadge.count() > 0;
  logTest('CapsLock Warning Indicator', capsVisible, `Badge present: ${capsVisible}`);

  // 5. Password Eye Reveal Toggle
  const eyeBtn = page.locator('button[aria-label="Show password"]');
  const hasEye = await eyeBtn.count() > 0;
  if (hasEye) {
    await eyeBtn.click();
    await page.waitForTimeout(300);
    const typeAfter1 = await passInput.getAttribute('type');
    const eyeOffBtn = page.locator('button[aria-label="Hide password"]');
    await eyeOffBtn.click();
    await page.waitForTimeout(300);
    const typeAfter2 = await passInput.getAttribute('type');
    logTest('Password Visibility Toggle', typeAfter1 === 'text' && typeAfter2 === 'password',
      `Toggled text: ${typeAfter1}, reverted: ${typeAfter2}`);
  }

  // 6. Enter Key Submission & Authentication
  await userInput.fill('admin');
  await passInput.fill('Boss@123');
  console.log('Pressing Enter on password input...');
  await passInput.press('Enter');
  await page.waitForTimeout(2500);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/04_after_enter_login.png` });

  const urlAfterEnter = page.url();
  const pageBodyText = await page.evaluate(() => document.body.innerText);
  const loggedIn = pageBodyText.includes('Role Center') || pageBodyText.includes('File');
  logTest('Enter Key Submission (Native Execution)', loggedIn, `URL: ${urlAfterEnter}, Logged in: ${loggedIn}`);

  // 7. Verify Public Route Whitelist
  const publicPageRes = await page.evaluate(async () => {
    const res = await fetch('/api/pages/LoginPage');
    return { status: res.status, ok: res.ok };
  });
  logTest('Public LoginPage Endpoint', publicPageRes.ok, `Status: ${publicPageRes.status}`);

  const publicSseRes = await page.evaluate(async () => {
    const res = await fetch('/api/notifications/stream');
    return { status: res.status, ok: res.ok };
  });
  logTest('Public Notification Stream Endpoint', publicSseRes.ok, `Status: ${publicSseRes.status}`);

  // 8. Session Persistence on Reload
  await page.reload();
  await page.waitForTimeout(2000);
  const afterReloadBody = await page.evaluate(() => document.body.innerText);
  const reloadPersisted = afterReloadBody.includes('Role Center') || afterReloadBody.includes('File');
  logTest('Session Persistence on Page Refresh', reloadPersisted, 'Session persisted cleanly across reload');

  // 9. Logout Flow & Invalidation
  const userMenuBtn = page.locator('header button:has-text("SS"), header button:has-text("admin")').first();
  if (await userMenuBtn.count() > 0) {
    await userMenuBtn.click();
    await page.waitForTimeout(500);
    const signOut = page.locator('[role="menuitem"]:has-text("Sign Out"), button:has-text("Sign Out")').first();
    if (await signOut.count() > 0) {
      await signOut.click();
      await page.waitForTimeout(1500);
      await page.screenshot({ path: `${SCREENSHOT_DIR}/05_after_sign_out.png` });
      const signedOutBody = await page.evaluate(() => document.body.innerText);
      const isLoggedOut = signedOutBody.includes('Sign In');
      logTest('Logout Flow & Storage Clearing', isLoggedOut, 'Cleanly cleared session and returned to login');
    }
  }

  await browser.close();

  fs.writeFileSync('scratch/login-audit-fixed-results.json', JSON.stringify(results, null, 2));
  console.log('\n=== ALL POST-FIX VERIFICATIONS FINISHED ===');
}

run().catch(console.error);
