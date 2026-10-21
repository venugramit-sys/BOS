import test from "node:test";
import assert from "node:assert/strict";
import { shiftIsoDate, shiftIsoMonth, weekEdgeIso } from "./evaluator.js";

test("calendar arrows move one day or one week", () => {
  assert.equal(shiftIsoDate("2026-10-16", 1), "2026-10-17");
  assert.equal(shiftIsoDate("2026-10-16", -1), "2026-10-15");
  assert.equal(shiftIsoDate("2026-10-16", 7), "2026-10-23");
  assert.equal(shiftIsoDate("2026-10-16", -7), "2026-10-09");
  assert.equal(shiftIsoDate("2026-10-31", 1), "2026-11-01");
});

test("page up and page down move a month and keep the day", () => {
  assert.equal(shiftIsoMonth("2026-10-16", 1), "2026-11-16");
  assert.equal(shiftIsoMonth("2026-10-16", -1), "2026-09-16");
  assert.equal(shiftIsoMonth("2026-03-31", 1), "2026-04-30");
  assert.equal(shiftIsoMonth("2026-10-16", 12), "2027-10-16");
});

test("home and end jump to the edges of the Monday week", () => {
  assert.equal(weekEdgeIso("2026-10-16", false), "2026-10-12");
  assert.equal(weekEdgeIso("2026-10-16", true), "2026-10-18");
});
