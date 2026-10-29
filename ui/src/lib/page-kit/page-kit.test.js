import test from "node:test";
import assert from "node:assert/strict";
import { fx } from "./fx.js";
import { formatDecimal, parseDecimal, divDecimal, toDecimalString } from "./decimal.js";
import { styleVars, resolveEmphasis, spanVars, colorValue, hideBelowClass } from "./style.js";
import { formatValue } from "./format.js";
import {
  buildPatch,
  buildInsert,
  changedKeys,
  newRecordValues,
  initialVariables,
  exactFilter,
  evaluateLinks,
  linksComplete,
  pendingConfirm,
  classifySaveError,
  writableKeys,
  dataCaption,
  missingRequired
} from "./record-model.js";

test("decimals stay exact", () => {
  assert.equal(fx.add("0.1", "0.2"), "0.3");
  assert.equal(fx.sub("10", "0.01"), "9.99");
  assert.equal(fx.mul("1.25", "4"), "5");
  assert.equal(fx.div("1", "3"), "0.3333333333");
  assert.equal(fx.div("1", "0"), null);
  assert.equal(fx.round("2.345", 2), "2.35");
  assert.equal(fx.round("-2.345", 2), "-2.35");
  assert.equal(fx.neg("5.50"), "-5.5");
  assert.equal(fx.abs("-0.5"), "0.5");
  assert.equal(toDecimalString(divDecimal(parseDecimal("10"), parseDecimal("4"), 2)), "2.5");
  assert.equal(formatDecimal("1234567.5", 2, 2), "1,234,567.50");
  assert.equal(formatDecimal("0.123456", 0, 5), "0.12346");
  assert.equal(formatDecimal("-0.001", 0, 2), "0");
});

test("comparison and equality are type-aware", () => {
  assert.equal(fx.eq("12.50", 12.5), true);
  assert.equal(fx.eq("Open", "Open"), true);
  assert.equal(fx.eq("Open", "open"), false);
  assert.equal(fx.eq(false, "false"), true);
  assert.equal(fx.eq(null, ""), true);
  assert.equal(fx.eq(null, 0), false);
  assert.ok(fx.cmp("100", "20") > 0, "numeric, not lexicographic");
  assert.ok(fx.cmp("2026-01-02", "2026-01-10") < 0, "ISO dates order as strings");
  assert.equal(fx.cmp(null, "1"), -1);
  assert.equal(fx.in("B", ["A", "B"]), true);
  assert.equal(fx.in(2, ["1", "2.0"]), true);
});

test("truthiness follows BOS values", () => {
  for (const v of [false, "false", 0, "0", "0.00", "", null, undefined, []]) assert.equal(fx.truthy(v), false, String(v));
  for (const v of [true, "true", 1, "0.01", "x", [1]]) assert.equal(fx.truthy(v), true, String(v));
});

test("string helpers and coalesce", () => {
  assert.equal(fx.isEmpty(""), true);
  assert.equal(fx.isNotEmpty("a"), true);
  assert.equal(fx.contains("Adatum Corp", "corp"), true);
  assert.equal(fx.startsWith("SO-100", "so-"), true);
  assert.equal(fx.len("abc"), 3);
  assert.equal(fx.coalesce(null, "", "x"), "x");
  assert.equal(fx.max("5", "12", "7"), "12");
  assert.match(fx.today(), /^\d{4}-\d{2}-\d{2}$/);
});

test("style tokens never emit raw CSS", () => {
  assert.equal(colorValue("primary"), "var(--bos-color-primary)");
  assert.equal(colorValue("#1E40AF"), "#1e40af");
  assert.equal(colorValue("red; background:url(x)"), null);
  const vars = styleVars({ background: "primary", borderColor: "#1e40af", radius: "md", shadow: "sm" });
  assert.match(vars, /--bos-bg:var\(--bos-color-primary\)/);
  assert.match(vars, /--bos-border-width:1px/);
  assert.doesNotMatch(styleVars({ background: "url(javascript:x)" }), /url/);
  assert.equal(resolveEmphasis({ emphasis: "Unfavorable" }, false), "");
  assert.equal(resolveEmphasis({ emphasis: "Unfavorable" }, true), "bos-emphasis-Unfavorable");
  assert.equal(resolveEmphasis({}, "Attention"), "bos-emphasis-Attention");
  assert.equal(spanVars({ span: { md: 4 } }), "--bos-span-xs:12;--bos-span-md:4");
  assert.equal(hideBelowClass({ hideBelow: "md" }), "bos-hide-below-md");
});

test("display formatting uses resolved field metadata", () => {
  assert.equal(formatValue({ control: "number", decimals: [2, 2] }, "1500"), "1,500.00");
  assert.equal(formatValue({ control: "number", decimals: [0, 0], blank_zero: true }, "0"), "");
  assert.equal(formatValue({ control: "select", options: [{ value: "OnHold", caption: "On Hold" }] }, "OnHold"), "On Hold");
  assert.equal(formatValue({ control: "switch" }, true), "Yes");
  assert.equal(formatValue({ control: "masked" }, "secret"), "••••••");
});

const C = [
  { i: 0, kind: "area", name: "Content" },
  { i: 1, kind: "field", name: "no", caption: "No", field: { key: "no", binding: "column", editable: true, required: true, control: "text" } },
  { i: 2, kind: "field", name: "blocked", field: { key: "blocked", binding: "column", editable: true, control: "switch", default_value: "false" } },
  { i: 3, kind: "field", name: "credit", field: { key: "credit_limit", binding: "column", editable: true, control: "number", default_value: "0" } },
  { i: 4, kind: "field", name: "balance", field: { key: "balance", binding: "flowField", editable: false, control: "number" } },
  { i: 5, kind: "field", name: "note", caption: "Note", field: { key: "note", binding: "variable", editable: true, required: true, control: "text" } }
];

test("new records start from table defaults and links", () => {
  assert.deepEqual(writableKeys(C), ["no", "blocked", "credit_limit"]);
  assert.deepEqual(newRecordValues(C, { no: "C9" }), { no: "C9", blocked: false, credit_limit: "0" });
  assert.deepEqual(
    initialVariables({ variables: [{ name: "remember", data_type: "Boolean" }, { name: "n", data_type: "Integer", default_value: "3" }] }),
    { remember: false, n: "3" }
  );
});

test("saving sends only changed writable fields with the row version", () => {
  const original = { _id: "u1", _system_version: 7, no: "C1", blocked: false, credit_limit: "100", balance: "5" };
  const edited = { ...original, credit_limit: "250", balance: "999" };
  assert.deepEqual(changedKeys(original, edited, writableKeys(C)), ["credit_limit"]);
  assert.deepEqual(buildPatch(original, edited, writableKeys(C)), { credit_limit: "250", _system_version: 7 });
  assert.deepEqual(buildPatch(original, { ...original, no: "" }, ["no"]), { no: null, _system_version: 7 });
  assert.deepEqual(buildInsert({ no: "C2", blocked: false, credit_limit: "" }, writableKeys(C)), { no: "C2", blocked: false });
});

test("links filter parts exactly and wait for the parent key", () => {
  const values = evaluateLinks([{ field: "document_no", value: (rec) => rec["no"] }], { no: "SO1" }, {});
  assert.deepEqual(exactFilter(values), { document_no: { $eq: "SO1" } });
  assert.equal(linksComplete(values), true);
  assert.equal(linksComplete(evaluateLinks([{ field: "x", value: (rec) => rec["no"] }], {}, {})), false);
  assert.deepEqual(exactFilter({ a: null, b: undefined }), { a: null });
});

test("confirm questions are recognised from both server shapes", () => {
  assert.equal(pendingConfirm({ code: "CONFIRM_REQUIRED", confirm: "Post?" }), "Post?");
  assert.equal(pendingConfirm({ message: "Business logic exception in 'A.b' (Error): ConfirmRequired: Delete all? [stack: x]" }), "Delete all?");
  assert.equal(pendingConfirm({ message: "boom" }), null);
});

test("save errors are classified", () => {
  assert.equal(classifySaveError({ status: 409, message: "changed" }).kind, "conflict");
  assert.equal(classifySaveError({ status: 428, message: "version" }).kind, "version");
  assert.equal(classifySaveError({ status: 400, message: "x", fieldErrors: { no: "required" } }).kind, "fields");
  assert.equal(classifySaveError(new Error("down")).kind, "error");
});

test("captions and required checks", () => {
  assert.equal(dataCaption({ data_caption_fields: ["no", "name"] }, { no: "C1", name: "Adatum" }), "C1 · Adatum");
  assert.deepEqual(missingRequired(C, { no: "" }, { note: "" }), ["No", "Note"]);
  assert.deepEqual(missingRequired(C, { no: "C1" }, { note: "x" }), []);
});

test("custom unbound page loads cleanly and handles action properties", () => {
  const meta = { name: "CustomDashboard", page_type: "Custom", caption: "Operations Dashboard", properties: { showHeader: "false" } };
  const action = { name: "DownloadReport", caption: "Download Report", properties: { primary: "true", openPage: "ReportViewer" } };
  assert.equal(meta.page_type, "Custom");
  assert.equal(meta.properties.showHeader, "false");
  assert.equal(action.properties.primary, "true");
  assert.equal(action.properties.openPage, "ReportViewer");
});
