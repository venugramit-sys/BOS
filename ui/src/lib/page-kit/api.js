/**
 * HTTP access used by compiled pages. Authentication and tenant headers are added by the app's
 * global fetch wrapper (main.js), so every call here is a plain relative request.
 */

export class PageApiError extends Error {
  /**
   * @param {number} status
   * @param {{ error?: string, message?: string, code?: string, confirm?: string, field_errors?: Record<string,string>, notifications?: any[] }} body
   */
  constructor(status, body) {
    super(body?.error || body?.message || `Request failed (${status})`);
    this.name = "PageApiError";
    this.status = status;
    this.code = body?.code || "";
    this.confirm = body?.confirm || null;
    this.fieldErrors = body?.field_errors || {};
    this.notifications = body?.notifications || [];
  }
}

async function request(method, url, body, { signal } = {}) {
  const init = { method, signal, headers: {} };
  if (body !== undefined) {
    init.headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  const res = await fetch(url, init);
  const text = await res.text();
  let data = null;
  if (text) {
    try {
      data = JSON.parse(text);
    } catch {
      data = { error: text };
    }
  }
  if (!res.ok) throw new PageApiError(res.status, data);
  return data;
}

const enc = encodeURIComponent;

/** Entity list. `filter` values: plain strings match by substring; `{ $eq: v }` matches exactly. */
export function listRecords(table, { page = 1, limit = 50, sort = "", filter = null, signal } = {}) {
  const q = new URLSearchParams({ page: String(page), limit: String(limit) });
  if (sort) q.set("sort", sort);
  if (filter && Object.keys(filter).length) q.set("filter", JSON.stringify(filter));
  return request("GET", `/api/entities/${enc(table)}?${q}`, undefined, { signal });
}

/**
 * Query protocol (`POST /api/query`): computes FlowFields (SIFT pushdown) for the selected keys.
 * `filter` supports exact values and `{ $in: [...] }`.
 */
export async function queryRecords(entity, { select, filter, limit = 1000 } = {}) {
  const res = await request("POST", "/api/query", { entity, select, filter, limit });
  return res?.data ?? [];
}

export function getRecord(table, id, { signal } = {}) {
  return request("GET", `/api/entities/${enc(table)}/${enc(id)}`, undefined, { signal });
}

export function createRecord(table, data) {
  return request("POST", `/api/entities/${enc(table)}`, data);
}

export function updateRecord(table, id, data) {
  return request("PATCH", `/api/entities/${enc(table)}/${enc(id)}`, data);
}

export function deleteRecord(table, id) {
  return request("DELETE", `/api/entities/${enc(table)}/${enc(id)}`);
}

/** Server-side page trigger / action script (`POST /api/v1/pages/{page}/triggers/{ref}`). */
export function runPageTrigger(page, ref, { record, variables, uiAnswers } = {}) {
  return request("POST", `/api/v1/pages/${enc(page)}/triggers/${enc(ref)}`, {
    record: record ?? {},
    variables: variables ?? {},
    ui_answers: uiAnswers ?? {}
  });
}

/** Codeunit procedure; `target` is `Codeunit` or `Codeunit.Method`. */
export function runCodeunit(target, params, uiAnswers) {
  const [codeunit, method = "Run"] = String(target).split(".");
  return request("POST", `/api/codeunits/${enc(codeunit)}/run`, { method, params: params ?? {}, ui_answers: uiAnswers ?? {} });
}

export async function fetchManifest(etag) {
  const headers = etag ? { "If-None-Match": `"${etag}"` } : {};
  const res = await fetch("/api/v1/pages/manifest", { headers });
  if (res.status === 304) return null;
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new PageApiError(res.status, data);
  return data;
}
