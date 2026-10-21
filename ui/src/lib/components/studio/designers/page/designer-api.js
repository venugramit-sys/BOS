/**
 * HTTP access of the page designer (`bosc-runtime::page_designer` and the page service routes).
 */

async function call(method, url, body) {
  const init = { method, headers: {} };
  if (body !== undefined) {
    init.headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }
  const res = await fetch(url, init);
  const text = await res.text();
  let data = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {
    data = { error: text };
  }
  if (!res.ok) {
    const err = new Error(data?.error || data?.message || `Request failed (${res.status})`);
    err.status = res.status;
    err.diagnostics = data?.diagnostics ?? [];
    throw err;
  }
  return data;
}

const enc = encodeURIComponent;

export const designerApi = {
  propertySchema: () => call("GET", "/api/v1/pages/property-schema"),
  parse: (source, packageName) => call("POST", "/api/v1/pages/parse", { source, package_name: packageName }),
  edit: (source, page, ops, packageName) => call("POST", "/api/v1/pages/edit", { source, page, ops, package_name: packageName }),
  compiled: (page) => call("GET", `/api/v1/pages/${enc(page)}/compiled`),
  preview: (page, extensions) => call("POST", "/api/v1/pages/preview", { page, extensions, design: true }),
  customization: (page) => call("GET", `/api/v1/pages/${enc(page)}/customization`),
  saveCustomization: (page, changes) => call("PUT", `/api/v1/pages/${enc(page)}/customization`, { changes }),
  previewCustomization: (page, changes) => call("POST", `/api/v1/pages/${enc(page)}/customization/preview`, { changes }),
  /** Authored table definition (columns + FlowFields) for the toolbox. */
  async tableFields(table) {
    const meta = await call("GET", `/api/meta/objects/${enc(table)}`);
    const def = meta?.definition ?? {};
    const columns = (def.columns ?? []).filter((c) => !String(c.sql_name ?? c.name).startsWith("_"));
    const flows = def.flow_fields ?? [];
    return [
      ...columns.map((c) => ({ name: c.name, sql: c.sql_name ?? c.name, type: c.data_type ?? c.sql_type ?? "", caption: c.caption ?? null, flow: false })),
      ...flows.map((f) => ({ name: f.name, sql: f.name, type: "FlowField", caption: null, flow: true }))
    ];
  }
};
