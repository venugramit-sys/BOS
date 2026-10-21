/**
 * Real Live BOS Studio API Client (Zero Mocks)
 * All operations map 1:1 to production Axum endpoints.
 */

/**
 * Error carrying the HTTP status so callers can react to 409 (conflict / blocked migration)
 * and 428 (record version required).
 * @param {Response} res
 * @param {any} data
 * @param {string} fallback
 * @returns {Error & { status: number, body: any }}
 */
function apiError(res, data, fallback) {
  const err = /** @type {Error & { status: number, body: any }} */ (new Error(data?.message || fallback));
  err.status = res.status;
  err.body = data;
  return err;
}

export const StudioApi = {
  /**
   * Fetches the registered meta-objects catalog.
   */
  async listMetaObjects(type = null) {
    const url = type ? `/api/meta/objects?type=${encodeURIComponent(type)}` : '/api/meta/objects';
    const res = await fetch(url);
    if (!res.ok) throw new Error(`Failed to fetch catalog: ${res.statusText}`);
    return await res.json();
  },

  /**
   * Fetches details of a specific meta-object.
   */
  async getMetaObject(name) {
    const res = await fetch(`/api/meta/objects/${encodeURIComponent(name)}`);
    if (!res.ok) throw new Error(`Meta-object '${name}' not found`);
    return await res.json();
  },

  /**
   * Compiles a BOS-Script without applying it to the database.
   */
  async compileScript(script, packageName = 'bos.app') {
    const res = await fetch('/api/script/compile', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ script, package_name: packageName, apply: false }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.message || 'Script compilation failed');
    return data;
  },

  /**
   * Compiles and atomically applies migrations to PostgreSQL.
   */
  async applyScript(script, packageName = 'bos.app', { allowDestructive = false } = {}) {
    const res = await fetch('/api/script/apply', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ script, package_name: packageName, apply: true, allow_destructive: allowDestructive }),
    });
    const data = await res.json();
    if (!res.ok) throw apiError(res, data, 'Failed to apply schema changes to PostgreSQL');
    return data;
  },

  /**
   * Fetches physical table schema for Datasheet / Table Designer.
   */
  async getEntitySchema(table) {
    const res = await fetch(`/api/entities/${encodeURIComponent(table)}/schema`);
    if (!res.ok) throw new Error(`Failed to load schema for '${table}'`);
    return await res.json();
  },

  /**
   * Fetches table field schema for Monaco IntelliSense (via meta-objects catalog or entity schema).
   */
  async getTableSchema(table) {
    try {
      const meta = await this.getMetaObject(table);
      if (meta && Array.isArray(meta.fields) && meta.fields.length > 0) {
        return {
          fields: meta.fields.map((f) => ({
            name: f.field_name || f.name || f.column_name,
            data_type: f.field_type || f.data_type || f.sql_type || 'any',
            is_primary_key: Boolean(f.is_primary_key),
            is_flow_field: Boolean(f.is_flowfield || f.is_flow_field),
          })),
        };
      }
    } catch {
      // Fall back to entity schema
    }
    return await this.getEntitySchema(table);
  },

  /**
   * Fetches live table records for the Access Datasheet grid.
   */
  async getEntities(table, { limit = 50, offset = 0, filter = null, sort = null } = {}) {
    const params = new URLSearchParams();
    params.set('limit', limit.toString());
    params.set('offset', offset.toString());
    if (filter) params.set('filter', filter);
    if (sort) params.set('sort', sort);

    const res = await fetch(`/api/entities/${encodeURIComponent(table)}?${params.toString()}`);
    if (!res.ok) throw new Error(`Failed to fetch records for '${table}'`);
    const totalCount = res.headers.get('x-total-count');
    const data = await res.json();

    return {
      records: Array.isArray(data) ? data : data.records || [],
      total: totalCount ? parseInt(totalCount, 10) : (Array.isArray(data) ? data.length : 0),
    };
  },

  /**
   * Inserts a record in the Datasheet grid.
   */
  async createEntity(table, payload) {
    const res = await fetch(`/api/entities/${encodeURIComponent(table)}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.message || 'Insert failed');
    return data;
  },

  /**
   * Updates an existing record in the Datasheet grid.
   */
  async updateEntity(table, id, payload) {
    const res = await fetch(`/api/entities/${encodeURIComponent(table)}/${encodeURIComponent(id)}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const data = await res.json();
    if (!res.ok) throw apiError(res, data, 'Update failed');
    return data;
  },

  /**
   * Deletes a record from the Datasheet grid.
   */
  async deleteEntity(table, id) {
    const res = await fetch(`/api/entities/${encodeURIComponent(table)}/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    });
    if (!res.ok) {
      const data = await res.json().catch(() => ({}));
      throw new Error(data.message || 'Delete failed');
    }
    return true;
  },

  /**
   * Executes a registered QBE query against live PostgreSQL tables.
   */
  async runQuery(name, limit = 100) {
    const res = await fetch('/api/queries/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ query_name: name, limit }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.message || data.error || 'Query execution failed');
    return data;
  },

  /**
   * Executes a codeunit procedure inside the QuickJS sandbox.
   */
  async runCodeunit(name, method, params = {}) {
    const res = await fetch(`/api/codeunits/${encodeURIComponent(name)}/run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ method, params }),
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data.message || 'Procedure execution failed');
    return data;
  },
};
