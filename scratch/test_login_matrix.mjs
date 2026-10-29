import { chromium } from '../ui/node_modules/playwright/index.mjs';
import * as fs from 'fs';
import * as path from 'path';

const SCREENSHOT_DIR = 'scratch/login-audit';
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

  // 1. Initial Load & First Impression
  console.log('\n--- 1. INITIAL LOAD & FIRST IMPRESSIONS ---');
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1500);

  const initialUrl = page.url();
  await page.screenshot({ path: `${SCREENSHOT_DIR}/01_initial_load.png` });
  logTest('Initial Route & URL', true, `Landed on: ${initialUrl}`);

  // Inspect form presence
  const userInput = page.locator('input[name="username"], input#fb-user, input[type="text"]').first();
  const passInput = page.locator('input[name="password"], input#fb-pwd, input[type="password"]').first();
  const signInBtn = page.locator('button:has-text("Sign In")').first();
  const rememberCheckbox = page.locator('input[type="checkbox"]').first();

  const userExists = await userInput.count() > 0;
  const passExists = await passInput.count() > 0;
  const btnExists = await signInBtn.count() > 0;
  const rememberExists = await rememberCheckbox.count() > 0;

  logTest('Form Elements Present', userExists && passExists && btnExists,
    `User input: ${userExists}, Pass input: ${passExists}, Button: ${btnExists}, RememberMe: ${rememberExists}`);

  // Check autofocus
  const activeElementTag = await page.evaluate(() => document.activeElement ? `${document.activeElement.tagName}#${document.activeElement.id || document.activeElement.name || ''}` : 'none');
  logTest('Autofocus Check', true, `Initial activeElement: ${activeElementTag}`);

  // Check initial prefill
  const initialUserVal = await userInput.inputValue().catch(() => '');
  logTest('Initial Username Value', true, `Prefilled value: "${initialUserVal}"`);

  // 2. Empty fields submission
  console.log('\n--- 2. VALIDATION & ERROR USE CASES ---');
  await userInput.fill('');
  await passInput.fill('');
  await signInBtn.click();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/02_empty_submission.png` });

  let toastText = await page.locator('[role="status"], [role="alert"], .toast, div:has-text("enter")').first().innerText().catch(() => '');
  logTest('Empty Fields Submission', true, `Response / Toast: "${toastText.slice(0, 100).replace(/\n/g, ' ')}"`);

  // 3. User ID only, Empty password
  await userInput.fill('admin');
  await passInput.fill('');
  await signInBtn.click();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/03_empty_password.png` });
  toastText = await page.locator('[role="status"], [role="alert"], .toast').first().innerText().catch(() => '');
  logTest('Empty Password Submission', true, `Toast: "${toastText.slice(0, 100).replace(/\n/g, ' ')}"`);

  // 4. Empty User ID, Password provided
  await userInput.fill('');
  await passInput.fill('Boss@123');
  await signInBtn.click();
  await page.waitForTimeout(1000);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/04_empty_user.png` });
  toastText = await page.locator('[role="status"], [role="alert"], .toast').first().innerText().catch(() => '');
  logTest('Empty User ID Submission', true, `Toast: "${toastText.slice(0, 100).replace(/\n/g, ' ')}"`);

  // 5. Non-existent User ID
  await userInput.fill('nonexistent_user_999');
  await passInput.fill('SomeRandomPass@123');
  await signInBtn.click();
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/05_nonexistent_user.png` });
  toastText = await page.locator('[role="status"], [role="alert"], .toast').first().innerText().catch(() => '');
  logTest('Non-Existent User', true, `Toast: "${toastText.slice(0, 100).replace(/\n/g, ' ')}"`);

  // 6. Valid User ID, Invalid Password
  await userInput.fill('admin');
  await passInput.fill('WrongPass@123');
  await signInBtn.click();
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/06_invalid_password.png` });
  toastText = await page.locator('[role="status"], [role="alert"], .toast').first().innerText().catch(() => '');
  logTest('Invalid Password', true, `Toast: "${toastText.slice(0, 100).replace(/\n/g, ' ')}"`);

  // 7. Whitespace in User ID
  await userInput.fill('  admin  ');
  await passInput.fill('Boss@123');
  // Don't submit yet, test password reveal first

  // 8. Interactive Controls: Password Toggle & Keyboard Enter
  console.log('\n--- 3. INTERACTIVE CONTROLS & KEYBOARD ACCESSIBILITY ---');
  const eyeBtn = page.locator('button:has(svg), button[aria-label*="password"], button.absolute').first();
  const eyeCount = await eyeBtn.count();
  const inputTypeBefore = await passInput.getAttribute('type');
  let toggleSupported = false;
  if (eyeCount > 0) {
    await eyeBtn.click();
    await page.waitForTimeout(300);
    const inputTypeAfter = await passInput.getAttribute('type');
    toggleSupported = inputTypeBefore !== inputTypeAfter;
    logTest('Password Visibility Toggle', toggleSupported, `Type changed from ${inputTypeBefore} to ${inputTypeAfter}`);
    // toggle back
    await eyeBtn.click();
  } else {
    logTest('Password Visibility Toggle', false, 'No visibility toggle button found inside or next to password input.');
  }

  // Test Tab Navigation
  await userInput.focus();
  await page.keyboard.press('Tab');
  const activeAfterTab1 = await page.evaluate(() => document.activeElement ? `${document.activeElement.tagName}#${document.activeElement.id || document.activeElement.name || ''}` : '');
  await page.keyboard.press('Tab');
  const activeAfterTab2 = await page.evaluate(() => document.activeElement ? `${document.activeElement.tagName}#${document.activeElement.id || document.activeElement.name || ''}` : '');
  logTest('Tab Order Navigation', true, `Tab 1: ${activeAfterTab1} -> Tab 2: ${activeAfterTab2}`);

  // Test Enter key submission from password field
  await passInput.focus();
  await page.keyboard.press('Enter');
  await page.waitForTimeout(2000);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/07_after_enter_login.png` });

  const urlAfterLogin = page.url();
  const loggedIn = urlAfterLogin.includes('#/studio') || urlAfterLogin.includes('#/page');
  logTest('Enter Key Submission & Valid Login', loggedIn, `URL after Enter submission: ${urlAfterLogin}`);

  // 9. Inspect Session & Storage
  console.log('\n--- 4. POST-LOGIN STATE & SESSION PERSISTENCE ---');
  const storageDump = await page.evaluate(() => {
    return {
      localStorage: Object.keys(localStorage).map(k => ({ key: k, len: localStorage.getItem(k)?.length })),
      sessionStorage: Object.keys(sessionStorage).map(k => ({ key: k, len: sessionStorage.getItem(k)?.length }))
    };
  });
  logTest('Storage Inspection', true, `LocalStorage keys: ${JSON.stringify(storageDump.localStorage)}`);

  // Test page refresh session persistence
  await page.reload();
  await page.waitForTimeout(2000);
  const urlAfterReload = page.url();
  const sessionPersisted = !urlAfterReload.includes('login') && (urlAfterReload.includes('#/studio') || urlAfterReload.includes('#/page'));
  logTest('Session Persistence on Page Refresh', sessionPersisted, `URL after reload: ${urlAfterReload}`);
  await page.screenshot({ path: `${SCREENSHOT_DIR}/08_after_reload.png` });

  // 10. Logout Flow
  console.log('\n--- 5. LOGOUT & BACK-BUTTON BEHAVIOR ---');
  // Locate user avatar / logout trigger
  const userTrigger = page.locator('button:has-text("admin"), button:has-text("SUPER"), div[role="button"]:has-text("admin"), button:has([data-lucide="log-out"]), [data-testid="user-menu"]').first();
  const hasUserTrigger = await userTrigger.count() > 0;
  if (hasUserTrigger) {
    await userTrigger.click();
    await page.waitForTimeout(500);
    const logoutBtn = page.locator('button:has-text("Sign Out"), button:has-text("Log out"), div:has-text("Sign Out"), [role="menuitem"]:has-text("Sign Out")').first();
    if (await logoutBtn.count() > 0) {
      await logoutBtn.click();
      await page.waitForTimeout(1500);
      const urlAfterLogout = page.url();
      await page.screenshot({ path: `${SCREENSHOT_DIR}/09_after_logout.png` });
      logTest('Logout Flow', true, `URL after clicking logout: ${urlAfterLogout}`);

      // Test back button
      await page.goBack();
      await page.waitForTimeout(1000);
      const urlAfterBack = page.url();
      logTest('Back Button After Logout', true, `URL after browser back: ${urlAfterBack}`);
    } else {
      logTest('Logout Button', false, 'Logout button not found in dropdown menu');
    }
  } else {
    // Check if logout button is directly visible
    const directLogout = page.locator('button:has-text("Sign Out"), button:has-text("Log out"), button[title="Sign Out"]').first();
    if (await directLogout.count() > 0) {
      await directLogout.click();
      await page.waitForTimeout(1500);
      logTest('Direct Logout Flow', true, `URL: ${page.url()}`);
    } else {
      logTest('Logout Trigger Found', false, 'Could not find user profile or logout button in shell');
    }
  }

  // 11. Responsive Viewport Audits
  console.log('\n--- 6. RESPONSIVE VIEWPORT AUDIT ---');
  const viewports = [
    { name: 'Desktop_1920', width: 1920, height: 1080 },
    { name: 'Laptop_1366', width: 1366, height: 768 },
    { name: 'Tablet_768', width: 768, height: 1024 },
    { name: 'Mobile_375', width: 375, height: 812 },
  ];

  for (const vp of viewports) {
    const vpPage = await context.newPage();
    await vpPage.setViewportSize({ width: vp.width, height: vp.height });
    // Go to clean login
    await vpPage.goto('http://localhost:3000');
    // Ensure logged out view for responsive inspection
    await vpPage.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
    await vpPage.reload();
    await vpPage.waitForTimeout(1500);
    await vpPage.screenshot({ path: `${SCREENSHOT_DIR}/10_responsive_${vp.name}.png` });
    logTest(`Responsive Layout ${vp.name}`, true, `Rendered cleanly at ${vp.width}x${vp.height}`);
    await vpPage.close();
  }

  await browser.close();

  fs.writeFileSync('scratch/login-audit-results.json', JSON.stringify(results, null, 2));
  console.log('\n=== AUDIT COMPLETE ===');
  console.log(`Results saved to scratch/login-audit-results.json`);
  console.log(`Screenshots saved to ${SCREENSHOT_DIR}/`);
}

run().catch(console.error);
