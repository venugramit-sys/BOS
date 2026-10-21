<script>
  /**
   * Field list for the table designer.
   * One row is one field. Length, precision, and scale follow the data type.
   */
  import { ArrowDown, ArrowUp, Plus, Trash2 } from "lucide-svelte";
  import { Datasheet, Button, NEW_ROW_KEY } from "$lib/bos-ui/index.js";
  import { FIELD_TYPES, applyBaseType, applyLength, applyPrecision, createField, typeParts, bosTypeFromSql } from "./tableModel.js";

  let {
    fields = $bindable([]),
    selectedField = $bindable(null),
    onFieldSelect = null
  } = $props();

  let focusKey = $state("");
  let opened = false;

  $effect(() => {
    if (opened || !fields.length) return;
    const current = fields.find((field) => field.id === focusKey);
    opened = true;
    if (current) {
      choose(current);
      return;
    }
    if (focusKey && focusKey !== NEW_ROW_KEY) return;
    const first = fields[0];
    focusKey = first.id;
    choose(first);
  });

  const typeChoices = FIELD_TYPES.map((type) => ({ value: type.id, label: type.name }));

  const SQL_KEYWORDS = new Set([
    "VARCHAR", "CHARACTERVARYING", "CHAR", "TEXT", "BPCHAR",
    "INTEGER", "INT", "INT4", "SMALLINT", "INT2", "SERIAL",
    "BIGINT", "INT8", "BIGSERIAL",
    "BOOLEAN", "BOOL",
    "DATE", "TIMESTAMP", "TIMESTAMPTZ", "TIMESTAMPWITHTIMEZONE", "TIMESTAMPWITHOUTTIMEZONE",
    "JSON", "JSONB", "BYTEA", "BLOB", "UUID", "NUMERIC", "DECIMAL", "FLOAT", "REAL"
  ]);

  function project(field) {
    const canonical = bosTypeFromSql(field.dataType);
    if (field.dataType !== canonical) {
      field.dataType = canonical;
    }
    const parts = typeParts(field.dataType);
    if (field.typeName !== parts.base) field.typeName = parts.base;
    if (field.charLength !== parts.length) field.charLength = parts.length;
    if (field.numPrecision !== parts.precision) field.numPrecision = parts.precision;
    if (field.numScale !== parts.scale) field.numScale = parts.scale;
  }

  $effect(() => {
    for (const field of fields) project(field);
  });

  let columns = $derived.by(() => {
    const known = new Set(typeChoices.map((type) => type.value));
    const extras = [];
    for (const field of fields) {
      const base = typeParts(field.dataType).base;
      if (base && !known.has(base) && !SQL_KEYWORDS.has(base.toUpperCase()) && !extras.some((type) => type.value === base)) {
        extras.push({ value: base, label: base });
      }
    }
    const types = [...extras, ...typeChoices];
    return [
      {
        key: "isPrimaryKey",
        label: "PK",
        editor: "checkbox",
        width: 64,
        quickEntry: false,
        keepVisible: true,
        hint: "Primary key"
      },
      {
        key: "name",
        label: "Field name",
        editor: "text",
        width: 180,
        required: true,
        keepVisible: true,
        validate: (value, row) => {
          const name = String(value ?? "").trim();
          if (!name) return "A field needs a name";
          if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) return "Use a letter, then letters, digits, or underscores";
          if (fields.some((field) => field !== row && field.name === name)) return "Another field already uses this name";
          return "";
        }
      },
      {
        key: "typeName",
        label: "Data type",
        editor: "select",
        width: 140,
        options: types,
        keepVisible: true,
        quickEntry: true
      },
      {
        key: "charLength",
        label: "Length",
        editor: "code",
        width: 88,
        keepVisible: true,
        hint: "String length",
        editable: (row) => row?.typeName === "String",
        validate: (value, row) => {
          if (row?.typeName !== "String") return "";
          const text = String(value ?? "").trim();
          if (!text) return "A string needs a length";
          if (!/^\d+$/.test(text) || Number(text) < 1) return "Length is a whole number of 1 or more";
          return "";
        }
      },
      {
        key: "numPrecision",
        label: "Precision",
        editor: "code",
        width: 96,
        keepVisible: true,
        editable: (row) => row?.typeName === "Decimal",
        validate: (value, row) => {
          if (row?.typeName !== "Decimal") return "";
          const text = String(value ?? "").trim();
          if (!text) return "A decimal needs a precision";
          if (!/^\d+$/.test(text) || Number(text) < 1) return "Precision is a whole number of 1 or more";
          return "";
        }
      },
      {
        key: "numScale",
        label: "Scale",
        editor: "code",
        width: 80,
        keepVisible: true,
        editable: (row) => row?.typeName === "Decimal",
        validate: (value, row) => {
          if (row?.typeName !== "Decimal") return "";
          const text = String(value ?? "").trim();
          if (text === "") return "A decimal needs a scale";
          if (!/^\d+$/.test(text)) return "Scale is a whole number";
          const precision = Number(String(row?.numPrecision ?? "").replace(/\D/g, ""));
          if (precision && Number(text) > precision) return "Scale cannot be larger than precision";
          return "";
        }
      },
      { key: "notNull", label: "Required", editor: "checkbox", width: 92, quickEntry: false, keepVisible: true },
      { key: "unique", label: "Unique", editor: "checkbox", width: 84, quickEntry: false, keepVisible: true },
      { key: "defaultVal", label: "Default value", editor: "text", width: 160, keepVisible: true },
      { key: "caption", label: "Caption", editor: "text", width: 220, keepVisible: true }
    ];
  });

  let selectedIndex = $derived(fields.findIndex((field) => field === selectedField || (selectedField && field.id === selectedField.id)));

  function choose(row) {
    if (!row) return;
    selectedField = row;
    onFieldSelect?.(row);
  }

  function dataTypeFrom(payload) {
    const base = payload.typeName || "String";
    if (base === "String") return `String(${payload.charLength || "50"})`;
    if (base === "Decimal") return `Decimal(${payload.numPrecision || "18"}, ${payload.numScale || "2"})`;
    return base;
  }

  function insertField(payload = {}) {
    const field = createField({
      name: String(payload.name || `field_${fields.length + 1}`).trim() || `field_${fields.length + 1}`,
      dataType: dataTypeFrom(payload),
      notNull: Boolean(payload.notNull),
      unique: Boolean(payload.unique),
      isPrimaryKey: Boolean(payload.isPrimaryKey),
      defaultVal: payload.defaultVal || "",
      caption: payload.caption || ""
    });
    if (field.isPrimaryKey) field.notNull = true;
    project(field);
    fields = [...fields, field];
    selectedField = field;
    focusKey = field.id;
    onFieldSelect?.(field);
    return field;
  }

  async function commitField(field, key, value) {
    if (key === "typeName") applyBaseType(field, value);
    else if (key === "charLength") {
      if (typeParts(field.dataType).base !== "String") return { ok: false, message: "Length applies to a string" };
      applyLength(field, value);
    } else if (key === "numPrecision") {
      if (typeParts(field.dataType).base !== "Decimal") return { ok: false, message: "Precision applies to a decimal" };
      applyPrecision(field, value, typeParts(field.dataType).scale || "2");
    } else if (key === "numScale") {
      if (typeParts(field.dataType).base !== "Decimal") return { ok: false, message: "Scale applies to a decimal" };
      applyPrecision(field, typeParts(field.dataType).precision || "18", value);
    } else if (key === "isPrimaryKey" && value) field.notNull = true;
    else if (key === "name") {
      const name = String(value ?? "").trim();
      if (fields.some((item) => item !== field && item.name === name)) return { ok: false, message: "Another field already uses this name" };
    }
    project(field);
    return { ok: true };
  }

  function deleteField(record) {
    if (fields.length <= 1) return { ok: false, message: "A table keeps at least one field" };
    const next = fields.filter((field) => field !== record && field.id !== record?.id);
    if (next.length === fields.length) return { ok: false, message: "That field is already gone" };
    fields = next;
    if (selectedField === record || selectedField?.id === record?.id) {
      selectedField = fields[0] ?? null;
      focusKey = selectedField?.id ?? "";
      onFieldSelect?.(selectedField);
    }
    return { ok: true };
  }

  function moveField(direction) {
    const index = selectedIndex;
    const target = index + direction;
    if (index < 0 || target < 0 || target >= fields.length) return;
    const copy = [...fields];
    const [moved] = copy.splice(index, 1);
    copy.splice(target, 0, moved);
    fields = copy;
    focusKey = moved.id;
  }
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col bg-white dark:bg-slate-950">
  <Datasheet
    class="min-h-0 flex-1 rounded-none border-x-0 border-t-0"
    label="Table fields"
    layoutKey="studio-table-fields"
    rowKey="id"
    bind:rows={fields}
    bind:focusKey
    columns={columns}
    freezeCount={2}
    sortable={false}
    emptyTitle="No fields"
    emptyDescription="Insert a field to start the table."
    onActiveRow={choose}
    onCommitCell={commitField}
    onInsertRow={insertField}
    onDeleteRow={deleteField}
  />
  <div class="flex h-10 shrink-0 items-center justify-between gap-1 sm:gap-2 border-t border-slate-200 bg-slate-50/70 px-2 sm:px-3 dark:border-slate-800 dark:bg-slate-900/60 overflow-x-auto no-scrollbar">
    <div class="flex items-center gap-1 shrink-0">
      <Button variant="secondary" size="xs" icon={Plus} onclick={() => insertField()}>
        <span class="hidden sm:inline">Insert row</span>
        <span class="sm:hidden text-xs">Add</span>
      </Button>
      <Button variant="ghost" size="xs" icon={ArrowUp} iconOnly aria-label="Move up" tooltip="Move up" disabled={selectedIndex <= 0} onclick={() => moveField(-1)} />
      <Button variant="ghost" size="xs" icon={ArrowDown} iconOnly aria-label="Move down" tooltip="Move down" disabled={selectedIndex < 0 || selectedIndex >= fields.length - 1} onclick={() => moveField(1)} />
      <Button variant="ghost" size="xs" icon={Trash2} iconOnly aria-label="Delete field" tooltip="Delete field" disabled={fields.length <= 1 || selectedIndex < 0} onclick={() => selectedIndex >= 0 && deleteField(fields[selectedIndex])} />
    </div>
    <span class="font-mono text-[10px] sm:text-[11px] text-slate-400 shrink-0">{fields.length} {fields.length === 1 ? "col" : "cols"}</span>
  </div>
</div>
