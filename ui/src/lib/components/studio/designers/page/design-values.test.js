import test from "node:test";
import assert from "node:assert/strict";
import {
  toBosValue,
  fromBosValue,
  parentPath,
  lastSegment,
  dropLocation,
  isWithin,
  emptyChanges,
  patchControl,
  moveControl,
  addField,
  removeAddedField,
  patchAction,
  customizationPlacement,
  changeCount
} from "./design-values.js";

test("values are written as BOS literals", () => {
  assert.equal(toBosValue("text", 'Say "hi"'), '"Say \\"hi\\""');
  assert.equal(toBosValue("bool", true), "true");
  assert.equal(toBosValue("int", "3"), "3");
  assert.equal(toBosValue("color", "#1E40AF"), '"#1e40af"');
  assert.equal(toBosValue("color", "primary"), "primary");
  assert.equal(toBosValue("enum:importance", "Promoted"), "Promoted");
  assert.equal(toBosValue("expr", 'rec.status == "Open"'), 'rec.status == "Open"');
  assert.equal(toBosValue("fields", "no, name"), "[no, name]");
  assert.throws(() => toBosValue("expr", "x; table"));
  assert.throws(() => toBosValue("color", "red; x"));
  assert.throws(() => toBosValue("enum:x", "a b"));
  assert.equal(fromBosValue("text", '"A \\"b\\""'), 'A "b"');
  assert.equal(fromBosValue("bool", "false"), false);
  assert.deepEqual(fromBosValue("fields", "[no, name]"), ["no", "name"]);
});

const NODES = [
  {
    path: "area:Content",
    children: [
      { path: "area:Content/group:General", children: [{ path: "area:Content/group:General/field:no", children: [] }, { path: "area:Content/group:General/field:name", children: [] }] },
      { path: "area:Content/group:Status", children: [] }
    ]
  }
];

test("paths and drop locations", () => {
  assert.equal(parentPath("area:Content/group:General"), "area:Content");
  assert.equal(parentPath("area:Content"), "layout");
  assert.deepEqual(lastSegment("area:Content/group:General/field:no"), { kind: "field", name: "no" });
  const no = NODES[0].children[0].children[0];
  assert.deepEqual(dropLocation(NODES, no, "before"), { parent: "area:Content/group:General", before: no.path });
  assert.deepEqual(dropLocation(NODES, no, "after"), { parent: "area:Content/group:General", before: "area:Content/group:General/field:name" });
  assert.deepEqual(dropLocation(NODES, NODES[0].children[0].children[1], "after"), { parent: "area:Content/group:General", before: null });
  assert.deepEqual(dropLocation(NODES, NODES[0].children[1], "inside"), { parent: "area:Content/group:Status", before: null });
  assert.equal(isWithin("area:Content/group:General/field:no", "area:Content/group:General"), true);
  assert.equal(isWithin("area:Content/group:Generalx", "area:Content/group:General"), false);
});

test("customization changes merge per control", () => {
  let c = emptyChanges();
  c = patchControl(c, "email", { caption: "Mail" });
  c = patchControl(c, "email", { style: { background: "primary" } });
  c = patchControl(c, "email", { style: { icon: "mail" } });
  assert.deepEqual(c.controls, [{ name: "email", caption: "Mail", style: { background: "primary", icon: "mail" } }]);
  c = patchControl(c, "email", { caption: null, style: { background: null, icon: null } });
  assert.deepEqual(c.controls, [], "clearing everything removes the entry");
  c = patchControl(c, "name", { responsive: { span: { md: 4 } } });
  assert.deepEqual(c.controls[0].responsive, { span: { md: 4 } });

  c = moveControl(c, "email", "no", "after");
  c = moveControl(c, "email", "name", "before");
  assert.deepEqual(c.moves, [{ name: "email", anchor: "name", position: "before" }]);

  c = addField(c, "rating", "General", "last");
  c = addField(c, "rating", "General", "last");
  assert.equal(c.adds.length, 1);
  c = moveControl(c, "rating", "no", "after");
  assert.deepEqual(c.adds[0], { field: "rating", anchor: "no", position: "after" }, "moving an added field re-anchors it");
  c = removeAddedField(c, "rating");
  assert.equal(c.adds.length, 0);

  c = patchAction(c, "Post", { visible: false });
  assert.deepEqual(c.actions, [{ name: "Post", visible: false }]);
  c = patchAction(c, "Post", { visible: null });
  assert.deepEqual(c.actions, []);
  assert.deepEqual(customizationPlacement({ path: "area:Content/group:General" }, "inside"), { anchor: "General", position: "last" });
  assert.equal(changeCount(c), 2);
});
