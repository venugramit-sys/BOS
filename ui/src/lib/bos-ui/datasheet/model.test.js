import test from "node:test";
import assert from "node:assert/strict";
import { formatISODate } from "../date-picker/evaluator.js";
import {
  applyColumnLayout,
  checkCell,
  columnLocked,
  coerceCell,
  columnsFromFields,
  compareCells,
  draftPayload,
  extendFill,
  fieldProblem,
  matchOption,
  moveCell,
  nextTypeQuery,
  parseTsv,
  pastePlan,
  revealColumnScroll,
  rowMatches,
  selectionStats,
  toTsv
} from "./model.js";

test("table fields become locked calculated columns and editable entry columns", () => {
  const columns = columnsFromFields([
    { name: "id", dataType: "UUID", isPrimaryKey: true },
    { name: "quantity", caption: "Quantity", dataType: "Decimal(18, 2)" },
    { name: "amount", dataType: "Decimal(18, 2)", flowField: { method: "sum" } },
    { name: "posted", dataType: "Boolean" }
  ]);
  assert.equal(columns[0].editable, false);
  assert.equal(columns[1].label, "Quantity");
  assert.equal(columns[1].editor, "number");
  assert.equal(columns[1].decimals, 2);
  assert.equal(columns[2].editable, false);
  assert.equal(columns[2].hint, "Calculated");
  assert.equal(columns[3].editor, "checkbox");
});

test("posting date shorthand t becomes today", () => {
  const column = { editor: "date" };
  const today = formatISODate(new Date());
  assert.deepEqual(coerceCell(column, "t"), { value: today, error: "" });
  assert.deepEqual(coerceCell(column, "T"), { value: today, error: "" });
  assert.equal(coerceCell(column, "cm").value.endsWith("-31") || coerceCell(column, "cm").value.endsWith("-30") || coerceCell(column, "cm").value.endsWith("-28") || coerceCell(column, "cm").value.endsWith("-29"), true);
  assert.equal(coerceCell(column, "not-a-date").error, "Enter a date");
});

test("quantity typed as pocket math becomes a number", () => {
  const column = { editor: "number" };
  assert.deepEqual(coerceCell(column, "50*12"), { value: 600, error: "" });
  assert.equal(coerceCell(column, "abc").error, "Enter a number");
  assert.equal(checkCell({ required: true, editor: "number" }, "", null), "");
  assert.equal(fieldProblem({ required: true, label: "Quantity", editor: "number" }, "", null), "Quantity is required");
});

test("Enter skips locked columns and wraps to the next row", () => {
  const columns = [
    { editable: false, quickEntry: false },
    { editable: true, quickEntry: true },
    { editable: false, quickEntry: false },
    { editable: true, quickEntry: true }
  ];
  assert.deepEqual(moveCell(columns, 0, 1, 3, 1, "entry"), { row: 0, col: 3 });
  assert.deepEqual(moveCell(columns, 0, 3, 3, 1, "entry"), { row: 1, col: 1 });
  assert.deepEqual(moveCell(columns, 2, 3, 3, 1, "entry"), { row: 2, col: 3 });
});

test("copy and paste keep a block and skip locked cells", () => {
  const matrix = [
    ["DRILL", 50],
    ["SAW", 12]
  ];
  assert.deepEqual(parseTsv(toTsv(matrix)), [
    ["DRILL", "50"],
    ["SAW", "12"]
  ]);
  const plan = pastePlan(2, [{ key: "item", editable: false }, { key: "quantity", editable: true }], { row: 0, col: 0 }, matrix);
  assert.deepEqual(plan, [
    { rowIndex: 0, colIndex: 1, key: "quantity", raw: 50 },
    { rowIndex: 1, colIndex: 1, key: "quantity", raw: 12 }
  ]);
});

test("fill continues a numeric step and copies text", () => {
  assert.deepEqual(extendFill([10, 20], 2), [30, 40]);
  assert.deepEqual(extendFill(["MAIN"], 2), ["MAIN", "MAIN"]);
});

test("search, column filter, and sort read the record values", () => {
  const columns = [
    { key: "item", editor: "text" },
    { key: "quantity", editor: "number" }
  ];
  const rows = [
    { item: "Drill", quantity: 5 },
    { item: "Saw", quantity: 40 }
  ];
  assert.equal(rowMatches(rows[0], columns, {}, "drill"), true);
  assert.equal(rowMatches(rows[0], columns, { quantity: "40" }, ""), false);
  assert.ok(compareCells(rows[0], rows[1], columns[1], "asc") < 0);
  assert.deepEqual(draftPayload({ item: "Saw", quantity: "", location: null }), { item: "Saw" });
});

test("selection status sums only numeric cells", () => {
  const stats = selectionStats([
    ["Drill", 10],
    ["Saw", 2.5]
  ]);
  assert.equal(stats.count, 4);
  assert.equal(stats.numeric, 2);
  assert.equal(stats.sum, 12.5);
  assert.equal(stats.average, 6.25);
});

test("typed letters build one search and a pause starts another", () => {
  assert.equal(nextTypeQuery("", "5", 0), "5");
  assert.equal(nextTypeQuery("5", "0", 40), "50");
  assert.equal(nextTypeQuery("50", "9", 900), "9");
});

test("work date w and a day number use the company work date", () => {
  const column = { editor: "date" };
  assert.deepEqual(coerceCell(column, "w", "2026-09-30"), { value: "2026-09-30", error: "" });
  assert.deepEqual(coerceCell(column, "15", "2026-09-30"), { value: "2026-09-15", error: "" });
  assert.equal(coerceCell(column, "t", "2026-09-30").value, formatISODate(new Date()));
});

test("filters accept an exact value, a range, and either-or", () => {
  const columns = [{ key: "quantity", editor: "number" }, { key: "location", editor: "text" }];
  const row = { quantity: 12, location: "EAST" };
  assert.equal(rowMatches(row, columns, { quantity: "10..20" }, ""), true);
  assert.equal(rowMatches(row, columns, { quantity: "=12" }, ""), true);
  assert.equal(rowMatches(row, columns, { quantity: "<>12" }, ""), false);
  assert.equal(rowMatches(row, columns, { location: "EAST|WEST" }, ""), true);
  assert.equal(rowMatches(row, columns, { location: "E*" }, ""), true);
});

test("column layout hides, reorders, and keeps a posting date visible", () => {
  const columns = [
    { key: "item", label: "Item", quickEntry: true },
    { key: "postingDate", label: "Posting date", keepVisible: true, quickEntry: true }
  ];
  const laid = applyColumnLayout(columns, {
    order: ["postingDate", "item"],
    hidden: ["postingDate", "item"],
    quickEntry: { item: false }
  });
  assert.deepEqual(laid.map((column) => column.key), ["postingDate"]);
  assert.equal(laid[0].quickEntry, true);
});

test("Enter skips a cell locked for this row", () => {
  const columns = [
    { key: "name", editable: true },
    { key: "length", editable: (row) => row?.typeName === "String" },
    { key: "caption", editable: true }
  ];
  const open = (column) => !columnLocked(column, { typeName: "Integer" }) && column.quickEntry !== false;
  assert.deepEqual(moveCell(columns, 0, 0, 2, 1, "entry", open), { row: 0, col: 2 });
});

test("a column can be locked for one row and open for another", () => {
  const column = { editable: (row) => row?.typeName === "String" };
  assert.equal(columnLocked(column, { typeName: "String" }), false);
  assert.equal(columnLocked(column, { typeName: "Integer" }), true);
  assert.equal(columnLocked({ editable: false }, null), true);
});

test("moving to the last column scrolls that column fully into view", () => {
  const gutter = 36;
  const widths = [64, 180, 140, 88, 96, 80, 92, 84, 160, 220];
  let left = gutter;
  for (let index = 0; index < widths.length - 1; index += 1) left += widths[index];
  const frozen = gutter + widths[0] + widths[1];
  const next = revealColumnScroll(0, 887, left, widths[9], frozen, 8);
  assert.ok(next > 0);
  assert.ok(left >= next + frozen);
  assert.ok(left + widths[9] + 8 <= next + 887);
});

test("a column already on screen does not scroll", () => {
  assert.equal(revealColumnScroll(0, 900, 400, 140, 280, 8), 0);
});

test("scrolling back uncovers a column hidden under the frozen columns", () => {
  const next = revealColumnScroll(500, 900, 300, 140, 280, 8);
  assert.equal(next, 20);
  assert.ok(300 >= next + 280);
});

test("typed text highlights the option that starts with it", () => {
  const locations = [
    { value: "MAIN", label: "MAIN" },
    { value: "EAST", label: "EAST" },
    { value: "WEST", label: "WEST" }
  ];
  const items = [
    { value: "DRILL-10", label: "Cordless drill" },
    { value: "SAW-02", label: "Circular saw" },
    { value: "BIT-04", label: "Driver bit" }
  ];
  assert.equal(matchOption(locations, "e"), "EAST");
  assert.equal(matchOption(locations, "ea"), "EAST");
  assert.equal(matchOption(items, "c"), "DRILL-10");
  assert.equal(matchOption(items, "dr"), "BIT-04");
  assert.equal(matchOption(items, "drill"), "DRILL-10");
  assert.equal(matchOption(locations, "z"), "");
});
