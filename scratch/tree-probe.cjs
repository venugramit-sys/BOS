const { chromium } = require('playwright');
(async () => {
  const b = await chromium.launch({ channel: 'msedge' });
  const p = await b.newPage({ viewport: { width: 1600, height: 950 } });
  await p.goto('http://127.0.0.1:3100/dev-studio');
  await p.fill('input[type=text]', process.env.U);
  await p.fill('input[type=password]', process.env.P);
  await p.click('button[type=submit]');
  await p.waitForTimeout(5000);
  const items = await p.$$eval('[role=treeitem]', (els) => els.map((e) => `${e.getAttribute('aria-expanded')}|${e.getAttribute('aria-label')}|${e.textContent.trim().slice(0, 40)}`));
  console.log(items.slice(0, 60).join('\n'));
  await b.close();
})();
