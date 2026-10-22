// Page source health check (designer + code editor input).
//
// For every page / pageextension in a node's catalog: fetch the source the Studio shows
// (`GET /api/meta/objects/{name}` → source_script), run it through the Studio formatter
// (ui/src/lib/components/studio/code-editor/bosPrettier.js), then make sure the designer can work with it:
//   - `POST /api/v1/pages/parse` parses it and finds the object
//   - the lowered IR of the object matches the catalog definition (the source is faithful)
//   - a no-op designer edit round trip succeeds (`POST /api/v1/pages/edit`)
//
// Usage (from the repo root): BOS_NODE_URL=http://127.0.0.1:3000 node scripts/test-agent/page_sources_check.mjs

import path from 'path';
import { fileURLToPath, pathToFileURL } from 'url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../');
const NODE = process.env.BOS_NODE_URL || 'http://127.0.0.1:3000';
const { formatBosScriptSource } = await import(pathToFileURL(path.join(root, 'ui/src/lib/components/studio/code-editor/bosPrettier.js')).href);

const json = async (res) => {
  const text = await res.text();
  try {
    return JSON.parse(text);
  } catch {
    return { error: text };
  }
};
const get = (p) => fetch(`${NODE}${p}`).then(json);
const post = (p, body) =>
  fetch(`${NODE}${p}`, { method: 'POST', headers: { 'Content-Type': 'application/json', 'x-bos-roles': 'SUPER' }, body: JSON.stringify(body) }).then(async (r) => ({ status: r.status, body: await json(r) }));

// Catalog bookkeeping stored next to the IR by registration; not part of what the source expresses.
const CATALOG_KEYS = new Set(['source_script', 'ir_version', 'object_type', 'package_name', 'sql_table_name', 'sub_type', 'version', 'data_per_company']);
const strip = (v) => JSON.parse(JSON.stringify(v, (k, x) => (CATALOG_KEYS.has(k) ? undefined : x)));

const objects = (await get('/api/meta/objects')).filter?.((o) => o.object_type === 'page' || o.object_type === 'pageextension') ?? [];
let failures = 0;

// The same page registered by two packages (e.g. an installed page re-applied under the default package by
// an older Studio) is ambiguous: the node serves whichever row was updated last.
const byName = new Map();
for (const o of objects) byName.set(o.object_name, [...(byName.get(o.object_name) ?? []), o.package_name]);
const duplicates = [...byName].filter(([, pkgs]) => pkgs.length > 1);
for (const [name, pkgs] of duplicates) console.log(`! ${name} is registered by several packages: ${pkgs.join(', ')}`);
for (const o of objects) {
  const name = o.object_name;
  const details = await get(`/api/meta/objects/${encodeURIComponent(name)}`);
  const raw = details.source_script ?? '';
  let source;
  try {
    source = await formatBosScriptSource(raw);
  } catch (e) {
    console.log(`✗ ${name}: formatter failed: ${e.message}`);
    failures++;
    continue;
  }
  const parsed = await post('/api/v1/pages/parse', { source, package_name: o.package_name });
  const lowered = parsed.body?.model && [...(parsed.body.model.pages ?? []), ...(parsed.body.model.page_extensions ?? [])].find((p) => p.name === name);
  if (parsed.status !== 200 || parsed.body.error || !lowered) {
    console.log(`✗ ${name}: ${parsed.body?.error ?? `HTTP ${parsed.status}`}`);
    failures++;
    continue;
  }
  // Faithfulness is checked on the unformatted source (the formatter reflows script bodies).
  const exact = await post('/api/v1/pages/parse', { source: raw, package_name: o.package_name });
  const exactIr = exact.body?.model && [...(exact.body.model.pages ?? []), ...(exact.body.model.page_extensions ?? [])].find((p) => p.name === name);
  if (JSON.stringify(strip(exactIr)) !== JSON.stringify(strip(details.definition))) {
    console.log(`✗ ${name}: source does not reproduce the stored definition`);
    failures++;
    continue;
  }
  const first = parsed.body.outline.objects.find((x) => x.name === name)?.layout?.nodes?.[0];
  if (first) {
    const edit = await post('/api/v1/pages/edit', { source, page: name, ops: [{ op: 'setProperty', path: first.path, name: 'caption', value: '"Designer check"' }], package_name: o.package_name });
    if (edit.status !== 200 || edit.body.error) {
      console.log(`✗ ${name}: designer edit failed: ${edit.body?.error ?? edit.status}`);
      failures++;
      continue;
    }
  }
  console.log(`✓ ${name} (${o.package_name})`);
}
console.log(`\n${objects.length} page objects, ${failures} failure(s)`);
process.exit(failures ? 1 : 0);
