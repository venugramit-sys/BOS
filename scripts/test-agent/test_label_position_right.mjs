import { chromium } from '../../ui/node_modules/playwright/index.mjs';

async function testLabelPositionRight() {
  let browser;
  try {
    browser = await chromium.launch({ channel: 'chrome', headless: true });
  } catch {
    browser = await chromium.launch({ channel: 'msedge', headless: true });
  }

  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  console.log('Navigating to http://localhost:3000/#/pages/LoginPage/design...');
  await page.goto('http://localhost:3000/#/pages/LoginPage/design');
  await page.waitForTimeout(2000);

  // 1. Select the rememberMe control or username control in the tree or canvas
  console.log('Looking for fields in tree/canvas...');
  const fieldNode = page.locator('text=rememberMe').first();
  if (await fieldNode.count() > 0) {
    await fieldNode.click();
    console.log('Clicked rememberMe in tree/canvas');
  } else {
    // Click on canvas
    const switchEl = page.locator('[role="switch"]').first();
    if (await switchEl.count() > 0) {
      await switchEl.click();
      console.log('Clicked switch element on canvas');
    }
  }

  await page.waitForTimeout(1000);

  // 2. Check PropertyPanel for labelPosition select options
  const labelPosSelect = page.locator('select').filter({ has: page.locator('option[value="right"]') }).first();
  const hasRightOption = await labelPosSelect.count() > 0;
  console.log('Found select with option "right":', hasRightOption);

  if (hasRightOption) {
    const options = await labelPosSelect.evaluate(el => Array.from(el.options).map(o => o.value));
    console.log('labelPosition options in designer:', options);

    // Select "right"
    console.log('Setting labelPosition to "right"...');
    await labelPosSelect.selectOption('right');
    await page.waitForTimeout(1000);
  }

  // 3. Take screenshot of designer with label on right
  const screenshotPath = 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/designer_label_position_right.png';
  await page.screenshot({ path: screenshotPath });
  console.log('Saved screenshot to:', screenshotPath);

  // 4. Test actual rendered login page
  console.log('Navigating to runtime http://localhost:3000/#/login...');
  await page.goto('http://localhost:3000/#/login');
  await page.waitForTimeout(1500);

  // Check if we have fields with label positioning
  const fields = await page.evaluate(() => {
    return Array.from(document.querySelectorAll('.bos-field')).map(f => {
      const isHoriz = f.classList.contains('is-horizontal');
      const isRight = f.classList.contains('is-label-right');
      const caption = f.querySelector('.bos-caption')?.innerText?.trim();
      return { caption, isHoriz, isRight, classes: f.className };
    });
  });
  console.log('Rendered fields on LoginPage:', fields);

  await page.screenshot({ path: 'C:/Users/mades/.gemini/antigravity/brain/647f9332-5d72-4220-a31b-9886a57b7fc3/login_page_live.png' });

  await browser.close();
  console.log('TEST COMPLETE');
}

testLabelPositionRight().catch(console.error);
