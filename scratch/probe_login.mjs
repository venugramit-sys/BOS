import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const passInput = page.locator('#bos-f-LoginPage-2');
  console.log('Pass input type before:', await passInput.getAttribute('type'));

  const eye = page.locator('button[aria-label="Show password"]');
  console.log('Eye button count:', await eye.count());
  await eye.click();
  await page.waitForTimeout(500);
  console.log('Pass input type after click 1:', await passInput.getAttribute('type'));

  const eyeOff = page.locator('button[aria-label="Hide password"]');
  console.log('EyeOff button count:', await eyeOff.count());
  await eyeOff.click();
  await page.waitForTimeout(500);
  console.log('Pass input type after click 2:', await passInput.getAttribute('type'));

  // Test Enter key on password field
  await passInput.focus();
  await passInput.fill('Boss@123');
  console.log('Pressing Enter on password field...');
  await page.keyboard.press('Enter');
  await page.waitForTimeout(2000);
  console.log('URL after pressing Enter:', page.url());

  // Test clicking Sign In button directly
  if (page.url() === 'http://localhost:3000/' || page.url().endsWith(':3000/')) {
    console.log('Clicking Sign In button...');
    const btn = page.locator('footer button:has-text("Sign In")');
    await btn.click();
    await page.waitForTimeout(2000);
    console.log('URL after clicking Sign In button:', page.url());
  }

  await browser.close();
}

main().catch(console.error);
