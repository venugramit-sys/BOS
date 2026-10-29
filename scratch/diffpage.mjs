import path from 'path';
import { pathToFileURL } from 'url';
const { formatBosScriptSource } = await import(pathToFileURL(path.resolve('ui/src/lib/components/studio/code-editor/bosPrettier.js')).href);
const name = process.argv[2];
const d = await (await fetch(`http://127.0.0.1:3100/api/meta/objects/${name}`)).json();
const src = await formatBosScriptSource(d.source_script);
const r = await (await fetch('http://127.0.0.1:3100/api/v1/pages/parse', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ source: src }) })).json();
const low = [...r.model.pages, ...r.model.page_extensions].find((p) => p.name === name);
const walk = (a, b, p = '') => {
  if (JSON.stringify(a) === JSON.stringify(b)) return;
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) walk(a[k], b[k], `${p}.${k}`);
  } else console.log(p, 'lowered=', JSON.stringify(a)?.slice(0, 160), ' stored=', JSON.stringify(b)?.slice(0, 160));
};
walk(low, d.definition);
if (process.argv[3]) console.log(src);
