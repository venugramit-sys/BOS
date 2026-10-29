import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage();
  await page.goto('http://localhost:3000');
  await page.waitForTimeout(1000);

  const res = await page.evaluate(() => {
    const isControlVisible = (el) => {
      if (!el) return false;
      const style = window.getComputedStyle(el);
      if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') return false;
      const rect = el.getBoundingClientRect();
      return rect.width > 0 && rect.height > 0;
    };
    const selector = [
      'input:not([type="hidden"]):not([disabled]):not([readonly])',
      'select:not([disabled]):not([readonly])',
      'textarea:not([disabled]):not([readonly])',
      '[role="combobox"]:not([aria-disabled="true"])',
      '[role="switch"]:not([disabled]):not([aria-disabled="true"])',
      '[role="checkbox"]:not([disabled]):not([aria-disabled="true"])',
      '[data-bos-field]:not([disabled])'
    ].join(', ');
    const all = Array.from(document.querySelectorAll(selector));
    const editables = all.filter(isControlVisible).map(el => ({
      tag: el.tagName,
      id: el.id,
      name: el.getAttribute('name'),
      role: el.getAttribute('role'),
      type: el.getAttribute('type')
    }));

    const footer = document.querySelector('.bos-page-footer');
    const buttons = Array.from(footer ? footer.querySelectorAll('button') : []).map(b => ({
      text: b.innerText,
      classes: b.className,
      disabled: b.disabled
    }));

    return { editables, buttons };
  });

  console.log('Result:', JSON.stringify(res, null, 2));

  // Now simulate pressing enter in password field
  const pass = page.locator('#bos-f-LoginPage-2');
  await pass.focus();
  await page.keyboard.press('Enter');
  await page.waitForTimeout(1000);

  const activeElement = await page.evaluate(() => {
    const el = document.activeElement;
    return el ? { tag: el.tagName, id: el.id, name: el.getAttribute('name'), role: el.getAttribute('role'), text: el.innerText } : null;
  });
  console.log('Active element after pressing Enter in password field:', activeElement);

  await browser.close();
}

main().catch(console.error);
