const base = 'http://127.0.0.1:3100/api/entities';
async function post(t, body) {
  const r = await fetch(`${base}/${t}`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
  const s = await r.text(); if (!r.ok) console.log('ERR', t, r.status, s.slice(0, 300)); return s;
}
await post('country', { code: 'IN', name: 'India' });
await post('country', { code: 'US', name: 'United States' });
for (const [no, name, cl, cc] of [['C1000','Adatum Corporation','5000','IN'],['C2000','Trey Research','12000.50','US'],['C3000','Fabrikam Inc','0','IN']])
  await post('customer', { no, name, email: `${no.toLowerCase()}@example.com`, phone_no: '+91 98450 00000', credit_limit: cl, country_code: cc, blocked: false });
await post('customer_ledger_entry', { customer_no: 'C1000', amount: '1250.75' });
await post('customer_ledger_entry', { customer_no: 'C1000', amount: '6000' });
for (const [no, c, st] of [['SO-1001','C1000','Open'],['SO-1002','C2000','Released'],['SO-1003','C3000','Shipped']])
  await post('sales_header', { no, customer_no: c, order_date: '2026-10-20', status: st, amount: '0' });
await post('sales_line', { document_no: 'SO-1001', line_no: 10000, item_no: 'ITEM-1', description: 'Bicycle', quantity: '2', unit_price: '450.00' });
await post('sales_line', { document_no: 'SO-1001', line_no: 20000, item_no: 'ITEM-2', description: 'Helmet', quantity: '1', unit_price: '35.50' });
await post('activity', { subject: 'Call Adatum', due_at: '2026-10-22T09:30:00Z', done: false });
await post('activity', { subject: 'Send quote', due_at: '2026-10-21T14:00:00Z', done: true });
const r = await fetch(`${base}/customer?limit=5`); console.log((await r.text()).slice(0, 600));
