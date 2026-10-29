import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });

  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  // Fill credentials and click Sign In
  await page.locator('#bos-f-LoginPage-1').fill('admin');
  await page.locator('#bos-f-LoginPage-2').fill('Boss@123');
  await page.locator('footer button:has-text("Sign In")').click();
  await page.waitForTimeout(2500);

  console.log('Logged in URL:', page.url());
  await page.screenshot({ path: 'scratch/login-audit/08_logged_in_dashboard.png' });

  // Find user avatar / menu in shell
  const userBtn = page.locator('button:has-text("SS"), button:has-text("admin"), header button:has(svg)').last();
  console.log('Clicking user menu...');
  await userBtn.click();
  await page.waitForTimeout(800);
  await page.screenshot({ path: 'scratch/login-audit/09_user_menu_open.png' });

  // Click Sign Out
  const signOutBtn = page.locator('button:has-text("Sign Out"), [role="menuitem"]:has-text("Sign Out")').first();
  if (await signOutBtn.count() > 0) {
    console.log('Clicking Sign Out...');
    await signOutBtn.click();
    await page.waitForTimeout(2000);
    console.log('URL after sign out:', page.url());
    await page.screenshot({ path: 'scratch/login-audit/10_after_sign_out.png' });
  }

  await browser.close();
  console.log('Done.');
}

main().catch(console.error);
