import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function run() {
  console.log('--- Starting Playwright UI Page Testing ---');
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const consoleLogs = [];
  page.on('console', msg => {
    const text = msg.text();
    consoleLogs.push(`[${msg.type()}] ${text}`);
    if (msg.type() === 'error' || text.includes('error') || text.includes('Error')) {
      console.log('PAGE ERROR:', text);
    }
  });

  page.on('pageerror', err => {
    console.log('PAGE UNHANDLED EXCEPTION:', err.message);
  });

  // 1. Visit App
  console.log('Navigating to http://localhost:3000/#/...');
  await page.goto('http://localhost:3000/');
  await page.waitForTimeout(1500);

  // 2. Login
  const usernameInput = page.locator('input[name="username"]');
  if (await usernameInput.count() > 0) {
    console.log('Logging in as admin...');
    await usernameInput.fill('admin');
    await page.locator('input[type="password"]').fill('Boss@123');
    await page.locator('button:has-text("Sign In")').click();
    await page.waitForTimeout(2000);
  }

  console.log('Current URL:', page.url());

  // Capture screenshot of Home / Workspace
  await page.screenshot({ path: 'scratch/workspace_home.png' });
  console.log('Saved workspace_home.png');

  // 3. Test Navigation to RoleList
  console.log('Looking for RoleList / Roles in navigation...');
  const roleListNav = page.locator('button:has-text("Roles & Permissions")').first();
  if (await roleListNav.count() > 0) {
    console.log('Clicking "Roles & Permissions"...');
    await roleListNav.click();
    await page.waitForTimeout(3000);

    await page.screenshot({ path: 'scratch/roles_list_page.png' });
    console.log('Saved roles_list_page.png');

    const rolePageText = await page.evaluate(() => document.body.innerText);
    console.log('RoleList text snippet:\n', rolePageText.slice(0, 500));
  } else {
    console.log('Roles & Permissions nav item not found.');
  }

  // 4. Test Navigation to UserList
  console.log('Looking for UserList / Users in navigation...');
  const userListNav = page.locator('button:has-text("Users")').first();
  if (await userListNav.count() > 0) {
    console.log('Clicking "Users"...');
    await userListNav.click();
    await page.waitForTimeout(3000);

    await page.screenshot({ path: 'scratch/users_list_page.png' });
    console.log('Saved users_list_page.png');

    const userPageText = await page.evaluate(() => document.body.innerText);
    console.log('UserList text snippet:\n', userPageText.slice(0, 500));
  }

  // 5. Check if there are "+ New" buttons or Create buttons
  const newButton = page.locator('button:has-text("New"), button:has-text("Create"), button:has-text("Add")').first();
  console.log('Found New/Create button:', await newButton.count() > 0);
  if (await newButton.count() > 0) {
    console.log('Button text:', await newButton.innerText());
  }

  await browser.close();
  console.log('--- UI Page Testing Completed ---');
}

run().catch(console.error);
