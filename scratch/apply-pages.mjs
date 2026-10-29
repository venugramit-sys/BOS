import fs from 'fs';
const [file, pkg, port = '3100'] = process.argv.slice(2);
const script = fs.readFileSync(file, 'utf8');
const res = await fetch(`http://127.0.0.1:${port}/api/script/apply`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ script, package_name: pkg }) });
const text = await res.text();
console.log(res.status, text.slice(0, 2000));
