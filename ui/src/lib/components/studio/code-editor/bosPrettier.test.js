import assert from "node:assert/strict";
import test from "node:test";
import { formatBosScriptSource, formatTypeScriptSource } from "./bosPrettier.js";
import { documentFromDetails, emitDocument } from "../designers/tableModel.js";

test("prettier formats trigger typescript and leaves the table structural", async () => {
  const source = [
    "table Note {",
    "code: String(20) {",
    "notNull = true;",
    "};",
    "primaryKey(code);",
    "}",
    "tablelogic NoteLogic for Note {",
    "onInsert(rec, ctx) {",
    "const text=rec.content||\"\";",
    "if(text){rec.content=text.trim()}",
    "}",
    "}",
    "",
  ].join("\n");

  const formatted = await formatBosScriptSource(source);
  assert.match(formatted, /table Note \{/);
  assert.match(formatted, /code: String\(20\) \{/);
  assert.match(formatted, /notNull = true;/);
  assert.match(formatted, /tablelogic NoteLogic for Note \{/);
  assert.match(formatted, /const text = rec\.content \|\| "";/);
  assert.match(formatted, /if \(text\) \{\n\s+rec\.content = text\.trim\(\);\n\s+\}/);
  assert.doesNotMatch(formatted, /__BOS_SCRIPT_/);
});

test("field validate is formatted inside the logic companion", async () => {
  const source = "tablelogic ItemLogic for Item {\nvalidate unitPrice(val) {\nif(val<0){throw new Error(\"Unit price cannot be negative\");}\n}\n}\n";
  const formatted = await formatBosScriptSource(source);
  assert.match(formatted, /validate unitPrice\(val\) \{/);
  assert.match(formatted, /if \(val < 0\) \{/);
  assert.match(formatted, /throw new Error\("Unit price cannot be negative"\);/);
});

test("typescript mode formats with prettier", async () => {
  const formatted = await formatTypeScriptSource("const text=rec.content||\"\"\n");
  assert.match(formatted, /const text = rec\.content \|\| "";/);
});

test("reloaded table + tablelogic round-trip normalizes legacy unevenly indented trigger scripts", async () => {
  const details = {
    summary: { object_name: "Note", package_name: "bos.simple_test" },
    definition: {
      columns: [
        { name: "code", sql_type: "VARCHAR(20)", is_nullable: false, is_unique: true, is_primary_key: true },
        { name: "content", sql_type: "VARCHAR(100)", is_nullable: true },
        { name: "content_2", sql_type: "VARCHAR(100)", is_nullable: true },
      ],
      primary_key: ["code"],
    },
  };
  const companions = [
    {
      kind: "tablelogic",
      details: {
        summary: { object_name: "NoteLogic" },
        definition: {
          name: "NoteLogic",
          target_table: "Note",
          triggers: [
            {
              name: "onInsert",
              params: ["rec", "ctx"],
              // Test legacy DB script where line 1 had 0 indent and line 2 had 8/16 spaces
              script: 'const text = rec.content || "";\n                rec.content = text.trim();',
            },
          ],
        },
      },
    },
  ];

  const model = documentFromDetails("Note", details, companions);
  const emitted = emitDocument(model).combined;
  const formatted = await formatBosScriptSource(emitted);

  assert.ok(
    formatted.includes(
      "    onInsert(rec, ctx) {\n        const text = rec.content || \"\";\n        rec.content = text.trim();\n    }",
    ),
    `Expected uniform 8-space indentation inside onInsert, got:\n${formatted}`,
  );
});
