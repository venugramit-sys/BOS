/**
 * Live entity endpoints for the API studio.
 * Create and update bodies come from GET /api/entities/{table}/schema
 * (sample_create_payload / sample_update_payload), not from a fixed example table.
 */

export function sqlTableName(objectName) {
  let result = '';
  const name = String(objectName || '');
  for (let i = 0; i < name.length; i++) {
    const c = name[i];
    if (c >= 'A' && c <= 'Z') {
      if (i > 0) result += '_';
      result += c.toLowerCase();
    } else {
      result += c;
    }
  }
  return result;
}

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export function isUuid(value) {
  return UUID_RE.test(String(value || ''));
}

/**
 * @returns {{ table: string, action: 'collection'|'schema'|'batch'|'record'|'other', id?: string } | null}
 */
export function parseEntityRequest(path) {
  const clean = String(path || '').split('?')[0];
  const match = clean.match(/^\/api\/entities\/([^/]+)(?:\/([^/]+))?\/?$/);
  if (!match) return null;
  const table = decodeURIComponent(match[1]);
  const rest = match[2];
  if (!rest) return { table, action: 'collection' };
  if (rest === 'schema' || rest === 'batch' || rest === 'lock') {
    return { table, action: rest };
  }
  return { table, action: 'record', id: decodeURIComponent(rest) };
}

/** Which schema sample belongs on this request, if any. */
export function fillKindFor(method, parsed) {
  if (!parsed) return null;
  if (method === 'POST' && parsed.action === 'batch') return 'batch';
  if (method === 'POST' && parsed.action === 'collection') return 'create';
  if ((method === 'PUT' || method === 'PATCH') && (parsed.action === 'record' || parsed.action === 'collection')) {
    return 'update';
  }
  return null;
}

function uniqueStamp() {
  return String(Date.now()).slice(-6);
}

function stampUniqueStrings(payload, columns, stamp) {
  const next = { ...payload };
  for (const col of columns || []) {
    if (!col.is_unique && !col.is_primary_key) continue;
    const key = Object.prototype.hasOwnProperty.call(next, col.name)
      ? col.name
      : col.sql_name;
    if (typeof next[key] === 'string') {
      next[key] = `${next[key]} ${stamp}`;
    }
  }
  return next;
}

export function createPayloadFromSchema(schema) {
  return stampUniqueStrings(
    schema?.sample_create_payload || {},
    schema?.columns,
    uniqueStamp()
  );
}

export function batchPayloadFromSchema(schema) {
  const columns = schema?.columns || [];
  const base = schema?.sample_batch_payload;
  const rows = Array.isArray(base) && base.length > 0
    ? base
    : [schema?.sample_create_payload || {}, schema?.sample_create_payload || {}];
  return rows.map((row, index) => stampUniqueStrings(row, columns, `${uniqueStamp()}${index + 1}`));
}

export function updatePayloadFromSchema(schema, row) {
  const payload = { ...(schema?.sample_update_payload || {}) };
  if (row && typeof row === 'object') {
    for (const col of schema?.columns || []) {
      if (col.is_primary_key) continue;
      const current = row[col.name] !== undefined ? row[col.name] : row[col.sql_name];
      if (current === undefined) continue;
      if (Object.prototype.hasOwnProperty.call(payload, col.name)) payload[col.name] = current;
      else if (Object.prototype.hasOwnProperty.call(payload, col.sql_name)) payload[col.sql_name] = current;
    }
    if (row._system_version !== undefined && row._system_version !== null) {
      payload._system_version = row._system_version;
    }
  }
  return payload;
}

export function liveEntityEndpoints(tables) {
  return tables.flatMap((table) => {
    const label = table.caption || table.name;
    return [
      {
        id: `live-create-${table.sql}`,
        category: 'entities',
        method: 'POST',
        path: `/api/entities/${table.sql}`,
        name: `Create ${label}`,
        description: `Insert one ${label} record. The body is built from the live table schema.`,
        sampleBody: '',
        live: true,
        fill: 'create',
        table: table.sql
      },
      {
        id: `live-update-${table.sql}`,
        category: 'entities',
        method: 'PUT',
        path: `/api/entities/${table.sql}/{id}`,
        name: `Update ${label}`,
        description: `Update one ${label} record. The id and _system_version are read from the table, and the fields come from its schema.`,
        sampleBody: '',
        live: true,
        fill: 'update',
        table: table.sql
      }
    ];
  });
}
