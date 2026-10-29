import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function testSwitchAndFocus() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const page = await context.newPage();

  console.log('Navigating to http://localhost:3000/#/login...');
  await page.goto('http://localhost:3000/#/login');
  await page.waitForTimeout(1500);

  // 1. Check focused element on load (auto-focus first editable)
  const activeField = await page.evaluate(() => {
    return document.activeElement ? document.activeElement.getAttribute('name') || document.activeElement.tagName : null;
  });
  console.log('Initial focused element:', activeField);

  // 2. Type User ID and press Enter
  console.log('Typing User ID and pressing Enter...');
  await page.keyboard.type('ADMIN');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(300);

  const focusedAfterEnter1 = await page.evaluate(() => {
    return document.activeElement ? document.activeElement.getAttribute('name') || document.activeElement.getAttribute('type') || document.activeElement.tagName : null;
  });
  console.log('Focused after Enter on User ID:', focusedAfterEnter1);

  // 3. Type Password and press Enter
  console.log('Typing Password and pressing Enter...');
  await page.keyboard.type('Admin@123');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(300);

  const focusedAfterEnter2 = await page.evaluate(() => {
    return document.activeElement ? document.activeElement.getAttribute('role') || document.activeElement.getAttribute('name') || document.activeElement.tagName : null;
  });
  console.log('Focused after Enter on Password:', focusedAfterEnter2);

  // 4. We are now focused on Switch!
  // Check initial state of switch
  const initialChecked = await page.evaluate(() => {
    const sw = document.querySelector('[role="switch"]');
    return sw ? sw.getAttribute('aria-checked') : null;
  });
  console.log('Switch initial aria-checked:', initialChecked);

  // Test Space key on switch -> MUST TOGGLE
  console.log('Pressing Space on Switch...');
  await page.keyboard.press('Space');
  await page.waitForTimeout(300);

  const checkedAfterSpace = await page.evaluate(() => {
    const sw = document.querySelector('[role="switch"]');
    return sw ? sw.getAttribute('aria-checked') : null;
  });
  console.log('Switch aria-checked after Space:', checkedAfterSpace);

  // Test Space key again -> MUST TOGGLE BACK
  console.log('Pressing Space on Switch again...');
  await page.keyboard.press('Space');
  await page.waitForTimeout(300);

  const checkedAfterSpace2 = await page.evaluate(() => {
    const sw = document.querySelector('[role="switch"]');
    return sw ? sw.getAttribute('aria-checked') : null;
  });
  console.log('Switch aria-checked after 2nd Space:', checkedAfterSpace2);

  // Test Enter key on switch -> MUST NOT TOGGLE! Instead it should trigger or advance to Sign In button
  console.log('Pressing Enter on Switch...');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(500);

  const checkedAfterEnter = await page.evaluate(() => {
    const sw = document.querySelector('[role="switch"]');
    return sw ? sw.getAttribute('aria-checked') : null;
  });
  console.log('Switch aria-checked after Enter (must be unchanged):', checkedAfterEnter);

  const focusedAfterEnterSwitch = await page.evaluate(() => {
    return document.activeElement ? document.activeElement.innerText || document.activeElement.tagName : null;
  });
  console.log('Focused/Triggered after Enter on Switch:', focusedAfterEnterSwitch);

  // 5. Test Mobile viewport
  console.log('Testing mobile viewport (375x812)...');
  await page.setViewportSize({ width: 375, height: 812 });
  await page.waitForTimeout(500);

  // Take screenshot of mobile view
  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/login_mobile_responsive.png' });
  console.log('Saved mobile screenshot to login_mobile_responsive.png');

  // Verify switch min-height on mobile
  const switchBox = await page.evaluate(() => {
    const sw = document.querySelector('[role="switch"]');
    if (!sw) return null;
    const rect = sw.getBoundingClientRect();
    return { width: rect.width, height: rect.height };
  });
  console.log('Switch touch box on mobile:', switchBox);

  await browser.close();
  console.log('TEST COMPLETE');
}

testSwitchAndFocus().catch(console.error);
