import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import {
  FIELD_TYPES,
  addDocumentStatus,
  applyBaseType,
  applyLength,
  applyPrecision,
  bosTypeFromSql,
  createField,
  documentFromDetails,
  emitDocument,
  ensureEvent,
  ensureValidate,
  eventLine,
  parseTableDocument,
  syncPrimaryKey,
  typeParts,
} from "./tableModel.js";

const root = new URL("../../../../../../packages/", import.meta.url);

function readPackage(path) {
  return readFileSync(new URL(path, root), "utf8");
}

test("type list is unique and length stays out of the type name", () => {
  const names = FIELD_TYPES.map((type) => type.id);
  assert.equal(new Set(names).size, names.length);
  assert.equal(names.includes("String(20)"), false);
  const field = createField({ name: "code", dataType: "String(20)" });
  assert.equal(typeParts(field.dataType).base, "String");
  assert.equal(typeParts(field.dataType).length, "20");
  applyBaseType(field, "Decimal");
  assert.equal(field.dataType, "Decimal(18, 2)");
  applyPrecision(field, "10", "4");
  assert.equal(field.dataType, "Decimal(10, 4)");
  applyBaseType(field, "String");
  applyLength(field, "30");
  assert.equal(field.dataType, "String(30)");
  assert.equal(typeParts(field.dataType).base, "String");
});

test("an event builder lands on that event and does not duplicate it", () => {
  const model = parseTableDocument("table Note {\n    code: String(20);\n    onInsert(rec, ctx) {\n        rec.content = 1;\n    }\n}\n");
  ensureEvent(model, "onInsert");
  ensureEvent(model, "beforeModify");
  assert.equal(model.triggers.filter((trigger) => trigger.name === "onInsert").length, 1);
  const script = emitDocument(model).combined;
  assert.match(script, /beforeModify\(rec, ctx\)/);
  assert.ok(eventLine(script, "beforeModify") > eventLine(script, "onInsert"));
});

test("before and after events are written to the logic companion", () => {
  const model = parseTableDocument("table Note {\n    code: String(20);\n    onInsert(rec, ctx) { rec.code = rec.code; }\n}\n");
  model.triggers.push({
    id: "t-before",
    name: "beforeInsert",
    params: ["rec", "ctx"],
    body: "rec.code = rec.code.trim();",
    home: "table",
  });
  const doc = emitDocument(model);
  assert.doesNotMatch(doc.tableScript, /onInsert|beforeInsert/);
  assert.match(doc.logicScript, /tablelogic NoteLogic for Note/);
  assert.match(doc.logicScript, /onInsert\(rec, ctx\)/);
  assert.match(doc.logicScript, /beforeInsert\(rec, ctx\)/);
  assert.equal(model.triggers.find((trigger) => trigger.name === "beforeInsert").home, "logic");
});

test("bos types round-trip from PostgreSQL type names", () => {
  assert.equal(bosTypeFromSql("VARCHAR(20)"), "String(20)");
  assert.equal(bosTypeFromSql("NUMERIC(18, 2)"), "Decimal(18, 2)");
  assert.equal(bosTypeFromSql("TIMESTAMPTZ"), "DateTime");
  assert.equal(bosTypeFromSql("BOOLEAN"), "Boolean");
});

test("Note.bos keeps its key, fields, and inline trigger", () => {
  const model = parseTableDocument(readPackage("bos.simple_test/tables/Note.bos"));
  assert.equal(model.name, "Note");
  assert.deepEqual(model.fields.map((field) => field.name), ["code", "content"]);
  assert.equal(model.fields[0].dataType, "String(20)");
  assert.equal(model.fields[0].notNull, true);
  assert.equal(model.fields[0].unique, true);
  assert.equal(model.fields[0].isPrimaryKey, true);
  assert.equal(model.triggers[0].name, "onInsert");
  assert.equal(model.triggers[0].home, "table");
  assert.match(model.triggers[0].body, /rec\.content = text\.trim\(\)/);

  const emitted = emitDocument(model);
  assert.doesNotMatch(emitted.tableScript, /onInsert/);
  assert.match(emitted.logicScript, /tablelogic NoteLogic for Note/);
  assert.match(emitted.logicScript, /text\.trim\(\)/);
  const again = parseTableDocument(emitted.combined);
  assert.equal(again.fields[0].dataType, "String(20)");
  assert.equal(again.triggers[0].home, "logic");
  assert.match(again.triggers[0].body, /text\.trim\(\)/);
});

test("User.bos keeps table properties and template-literal triggers", () => {
  const model = parseTableDocument(readPackage("bos.security/tables/User.bos"));
  assert.equal(model.caption, "Security User Master");
  assert.equal(model.tableType, "Normal");
  assert.equal(model.dataPerCompany, false);
  assert.equal(model.fields.find((field) => field.name === "isActive").defaultVal, "true");
  assert.equal(model.triggers.map((trigger) => trigger.name).join(","), "onInsert,onModify,onDelete");
  assert.match(model.triggers[0].body, /ctx\.crypto\.sha256/);
  const emitted = emitDocument(model);
  assert.match(emitted.tableScript, /dataPerCompany = false;/);
  assert.doesNotMatch(emitted.tableScript, /onInsert|tablelogic/);
  assert.match(emitted.logicScript, /onInsert\(rec, ctx\)/);
  assert.match(emitted.logicScript, /ctx\.crypto\.sha256/);
});

test("Item.bos field validate is emitted on the logic companion", () => {
  const model = parseTableDocument(readPackage("bos.typescript_test/tables/Item.bos"));
  const price = model.fields.find((field) => field.name === "unitPrice");
  assert.equal(price.defaultVal, "0.00");
  assert.equal(price.validate.home, "table");
  assert.match(price.validate.body, /Unit price cannot be negative/);

  const scripts = emitDocument(model);
  assert.doesNotMatch(scripts.tableScript, /validate\(|onInsert/);
  assert.doesNotMatch(scripts.tableScript, /validate\(/);
  assert.match(scripts.logicScript, /tablelogic ItemLogic for Item/);
  assert.match(scripts.logicScript, /validate unitPrice\(val\)/);
  assert.match(scripts.logicScript, /onInsert\(rec, ctx\)/);
  const parsed = parseTableDocument(scripts.combined);
  assert.equal(parsed.fields.find((field) => field.name === "unitPrice").validate.home, "logic");
  assert.equal(parsed.triggers[0].home, "logic");
});

test("tablelogic companion binds validates, lifecycle hooks, and methods", () => {
  const source = `
    table Customer {
        code: String(20) { notNull = true; };
        creditLimit: Decimal(18, 2);
        primaryKey(code);
    }
    tablelogic CustomerLogic for Customer {
        validate creditLimit(val, oldVal) {
            if (val < 0) throw new Error("negative");
        }
        beforeInsert(rec, ctx) {
            rec.code = rec.code.trim();
        }
        public calculateRiskScore(rec) {
            return "STANDARD";
        }
    }
  `;
  const model = parseTableDocument(source);
  assert.equal(model.logicName, "CustomerLogic");
  assert.equal(model.fields[1].validate.home, "logic");
  assert.equal(model.fields[1].validate.oldParam, "oldVal");
  assert.equal(model.triggers[0].name, "beforeInsert");
  assert.equal(model.methods[0].name, "calculateRiskScore");
  const again = parseTableDocument(emitDocument(model).combined);
  assert.equal(again.methods[0].name, "calculateRiskScore");
  assert.match(again.triggers[0].body, /rec\.code\.trim\(\)/);
});

test("definition JSON becomes a table document and a logic companion", () => {
  const model = documentFromDetails("Note", {
    summary: { object_name: "Note", package_name: "bos.simple_test", caption: "Note" },
    definition: {
      caption: "Note",
      table_type: "Normal",
      data_per_company: true,
      primary_key: ["code"],
      columns: [
        { name: "code", sql_type: "VARCHAR(20)", is_nullable: false, is_unique: true, caption: "code" },
        { name: "content", sql_type: "VARCHAR(100)", is_nullable: true, caption: "Content" },
      ],
      indexes: [{ name: "note_code", columns: ["code"] }],
      triggers: [{ name: "onInsert", params: ["rec", "ctx"], script: "rec.content = (rec.content || \"\").trim();" }],
    },
    relations: [],
  }, [{
    kind: "tablelogic",
    details: {
      definition: {
        name: "NoteLogic",
        target_table: "Note",
        field_validations: [{ field_name: "content", param_name: "val", script: "if (!val) throw new Error(\"empty\");" }],
        triggers: [],
        methods: [],
      },
    },
  }]);
  assert.equal(model.packageName, "bos.simple_test");
  assert.equal(model.fields[0].dataType, "String(20)");
  assert.equal(model.fields[0].isPrimaryKey, true);
  assert.equal(model.fields[1].caption, "Content");
  assert.equal(model.fields[1].validate.home, "logic");
  assert.equal(model.keys.length, 0);
  assert.match(emitDocument(model).logicScript, /tablelogic NoteLogic for Note/);
});

test("primary key forces notNull and document status is explicit", () => {
  const model = parseTableDocument("table Order {\n    no: String(20);\n}\n");
  model.tableType = "Document";
  model.fields[0].isPrimaryKey = true;
  syncPrimaryKey(model);
  assert.deepEqual(model.primaryKey, ["no"]);
  assert.equal(model.fields[0].notNull, true);
  addDocumentStatus(model);
  assert.equal(model.fields[1].name, "status");
  assert.equal(model.fields[1].defaultVal, '"Draft"');
  ensureValidate(model.fields[1]);
  assert.equal(model.fields[1].validate.home, "logic");
  assert.match(emitDocument(model).logicScript, /validate status\(val\)/);
});
