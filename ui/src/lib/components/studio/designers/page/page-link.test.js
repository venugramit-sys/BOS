import test from "node:test";
import assert from "node:assert/strict";

function parseLinkString(str) {
  if (!str || typeof str !== "string") return [];
  const items = str.split(",").map((s) => s.trim()).filter(Boolean);
  const parsed = [];
  for (const item of items) {
    const mField = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*FIELD\(([^)]+)\)/i);
    const mConst = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*CONST\(([^)]+)\)/i);
    const mFilter = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*FILTER\(([^)]+)\)/i);
    const mDirect = item.match(/^["']?([^"'=\s]+)["']?\s*=\s*(?:rec\.)?([^;\s]+)/i);

    if (mField) {
      parsed.push({ targetField: mField[1], type: "FIELD", sourceValue: mField[2] });
    } else if (mConst) {
      parsed.push({ targetField: mConst[1], type: "CONST", sourceValue: mConst[2] });
    } else if (mFilter) {
      parsed.push({ targetField: mFilter[1], type: "FILTER", sourceValue: mFilter[2] });
    } else if (mDirect) {
      parsed.push({ targetField: mDirect[1], type: "FIELD", sourceValue: mDirect[2] });
    } else {
      parsed.push({ targetField: item, type: "FIELD", sourceValue: "" });
    }
  }
  return parsed;
}

function serializeRows(rowsList) {
  const valid = rowsList.filter((r) => r.targetField.trim() && r.sourceValue.trim());
  if (valid.length === 0) return "";
  return valid
    .map((r) => {
      const tf = r.targetField.trim();
      const sv = r.sourceValue.trim();
      if (r.type === "CONST") return `"${tf}" = CONST(${sv})`;
      if (r.type === "FILTER") return `"${tf}" = FILTER(${sv})`;
      return `"${tf}" = FIELD(${sv})`;
    })
    .join(", ");
}

test("parses runPageLink expressions correctly", () => {
  const str = '"CustomerNo" = FIELD(No), "Type" = CONST(Invoice), "Status" = FILTER(Open|Pending)';
  const parsed = parseLinkString(str);
  assert.equal(parsed.length, 3);
  assert.deepEqual(parsed[0], { targetField: "CustomerNo", type: "FIELD", sourceValue: "No" });
  assert.deepEqual(parsed[1], { targetField: "Type", type: "CONST", sourceValue: "Invoice" });
  assert.deepEqual(parsed[2], { targetField: "Status", type: "FILTER", sourceValue: "Open|Pending" });
});

test("serializes link rows back into valid BOS syntax", () => {
  const rows = [
    { targetField: "CustomerNo", type: "FIELD", sourceValue: "No" },
    { targetField: "Amount", type: "CONST", sourceValue: "100" }
  ];
  const serialized = serializeRows(rows);
  assert.equal(serialized, '"CustomerNo" = FIELD(No), "Amount" = CONST(100)');
});
