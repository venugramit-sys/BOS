import { chromium } from '../ui/node_modules/playwright/index.mjs';

async function main() {
  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.goto('http://localhost:3000/#/studio');
  await page.waitForTimeout(1000);

  // Type credentials
  await page.locator('input[name="username"]').fill('admin');
  await page.locator('input[type="password"]').fill('Boss@123');
  await page.locator('button:has-text("Sign In")').click();
  
  // Wait for catalog to load
  await page.waitForSelector('text=bos.core', { timeout: 10000 });
  await page.waitForTimeout(1000);

  const data = await page.evaluate(async () => {
    const res = await fetch('/api/meta/objects');
    const all = await res.json();
    return {
      total: all.length,
      features: all.filter(o => o.object_type === 'feature'),
      withFeature: all.filter(o => o.feature).length
    };
  });

  console.log('Total objects:', data.total);
  console.log('Features registered:', data.features.length);
  console.log('Feature names:', data.features.map(f => f.name));
  console.log('Objects tagged with feature:', data.withFeature);

  await page.screenshot({ path: 'scratch/features_live_verification.png' });
  console.log('Saved screenshot to scratch/features_live_verification.png');

  await browser.close();
}

main().catch(console.error);
