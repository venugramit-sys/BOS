//! Declarative table relations (`relation: Customer.no`) compiled to real PostgreSQL foreign keys.
//!
//! Every relation is installed through the kernel function `bos_ensure_relation`, which resolves the
//! physical source/target tables for the active storage strategy, checks that the target field is
//! unique, and creates, replaces or keeps the constraint so repeated deployments are idempotent.

use bosc_ir::{StorageStrategyIR, TableIR};

/// Kernel objects required by schema synchronization: the migration journal and the relation installer.
pub const SCHEMA_SUPPORT_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS "_bos_schema_migrations" (
    "migration_id" UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    "package_name" VARCHAR(200) NOT NULL,
    "source" VARCHAR(50) NOT NULL,
    "checksum" VARCHAR(64) NOT NULL,
    "steps" JSONB NOT NULL DEFAULT '[]',
    "relations" JSONB NOT NULL DEFAULT '[]',
    "destructive" BOOLEAN NOT NULL DEFAULT FALSE,
    "applied_by" UUID,
    "applied_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS "idx_bos_schema_migrations_pkg" ON "_bos_schema_migrations" ("package_name", "applied_at" DESC);

CREATE OR REPLACE FUNCTION bos_ensure_relation(
    p_schema text,
    p_table text,
    p_column text,
    p_target_table text,
    p_target_column text,
    p_on_delete text,
    p_prefix text DEFAULT ''
) RETURNS text
LANGUAGE plpgsql AS $bos_rel$
DECLARE
    v_action "char";
    v_on_delete text := upper(btrim(p_on_delete));
    v_name text := left('fk_' || p_table || '_' || p_column, 63);
    v_source regclass;
    v_target regclass;
    v_target_col text := p_target_column;
    v_src_cols text[];
    v_tgt_cols text[];
    v_existing record;
    v_set_null_cols text := '';
    v_not_valid text := ' NOT VALID';
BEGIN
    v_action := CASE v_on_delete
        WHEN 'RESTRICT' THEN 'r'
        WHEN 'CASCADE' THEN 'c'
        WHEN 'SET NULL' THEN 'n'
        WHEN 'NO ACTION' THEN 'a'
        ELSE NULL
    END;
    IF v_action IS NULL THEN
        RAISE EXCEPTION 'BOS relation %.%: unsupported onDelete action "%" (use restrict, cascade, setNull or noAction)',
            p_table, p_column, p_on_delete;
    END IF;

    v_source := to_regclass(format('%I.%I', p_schema, p_prefix || p_table));
    IF v_source IS NULL THEN
        RAISE EXCEPTION 'BOS relation %.%: source table %.% does not exist', p_table, p_column, p_schema, p_prefix || p_table;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_attribute
        WHERE attrelid = v_source AND attname = p_column AND attnum > 0 AND NOT attisdropped
    ) THEN
        RAISE EXCEPTION 'BOS relation %.%: field "%" does not exist on %', p_table, p_column, p_column, v_source;
    END IF;

    v_target := COALESCE(
        to_regclass(format('%I.%I', p_schema, p_prefix || p_target_table)),
        to_regclass(format('%I.%I', p_schema, p_target_table)),
        to_regclass(format('%I.%I', 'public', p_prefix || p_target_table)),
        to_regclass(format('%I.%I', 'public', p_target_table))
    );
    IF v_target IS NULL THEN
        RAISE EXCEPTION 'BOS relation %.% -> %.%: target table "%" does not exist (deploy it first)',
            p_table, p_column, p_target_table, p_target_column, p_target_table;
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_attribute
        WHERE attrelid = v_target AND attname = v_target_col AND attnum > 0 AND NOT attisdropped
    ) THEN
        IF v_target_col = 'id' AND EXISTS (
            SELECT 1 FROM pg_attribute WHERE attrelid = v_target AND attname = '_id' AND attnum > 0 AND NOT attisdropped
        ) THEN
            v_target_col := '_id';
        ELSE
            RAISE EXCEPTION 'BOS relation %.% -> %.%: target field "%" does not exist on %',
                p_table, p_column, p_target_table, p_target_column, v_target_col, v_target;
        END IF;
    END IF;

    IF EXISTS (
        SELECT 1 FROM pg_index i
        WHERE i.indrelid = v_target AND i.indisunique AND i.indpred IS NULL AND i.indexprs IS NULL
          AND (SELECT array_agg(a.attname::text ORDER BY a.attname::text)
               FROM pg_attribute a
               WHERE a.attrelid = v_target AND a.attnum = ANY (i.indkey::int2[])) = ARRAY[v_target_col]
    ) THEN
        v_src_cols := ARRAY[p_column];
        v_tgt_cols := ARRAY[v_target_col];
    ELSIF EXISTS (
        SELECT 1 FROM pg_attribute WHERE attrelid = v_source AND attname = '_tenant_id' AND attnum > 0 AND NOT attisdropped
    ) AND EXISTS (
        SELECT 1 FROM pg_index i
        WHERE i.indrelid = v_target AND i.indisunique AND i.indpred IS NULL AND i.indexprs IS NULL
          AND (SELECT array_agg(a.attname::text ORDER BY a.attname::text)
               FROM pg_attribute a
               WHERE a.attrelid = v_target AND a.attnum = ANY (i.indkey::int2[]))
              = (SELECT array_agg(x ORDER BY x) FROM unnest(ARRAY['_tenant_id', v_target_col]) AS x)
    ) THEN
        v_src_cols := ARRAY['_tenant_id', p_column];
        v_tgt_cols := ARRAY['_tenant_id', v_target_col];
        IF v_action = 'n' THEN
            v_set_null_cols := format(' (%I)', p_column);
        END IF;
    ELSE
        RAISE EXCEPTION 'BOS relation %.% -> %.%: the target field is not unique; make it the primary key or mark it unique',
            p_table, p_column, p_target_table, v_target_col;
    END IF;

    SELECT c.confrelid,
           c.confdeltype,
           ARRAY(SELECT a.attname::text FROM unnest(c.conkey) WITH ORDINALITY AS k(attnum, ord)
                 JOIN pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum ORDER BY k.ord) AS src_cols,
           ARRAY(SELECT a.attname::text FROM unnest(c.confkey) WITH ORDINALITY AS k(attnum, ord)
                 JOIN pg_attribute a ON a.attrelid = c.confrelid AND a.attnum = k.attnum ORDER BY k.ord) AS tgt_cols
      INTO v_existing
      FROM pg_constraint c
     WHERE c.conrelid = v_source AND c.conname = v_name AND c.contype = 'f';
    IF FOUND THEN
        IF v_existing.confrelid = v_target::oid
           AND v_existing.confdeltype = v_action
           AND v_existing.src_cols = v_src_cols
           AND v_existing.tgt_cols = v_tgt_cols THEN
            RETURN 'unchanged';
        END IF;
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', v_source, v_name);
    END IF;

    IF (SELECT relkind FROM pg_class WHERE oid = v_source) = 'p' THEN
        v_not_valid := '';
    END IF;

    EXECUTE format(
        'ALTER TABLE %s ADD CONSTRAINT %I FOREIGN KEY (%s) REFERENCES %s (%s) ON DELETE %s%s%s',
        v_source,
        v_name,
        array_to_string(ARRAY(SELECT quote_ident(x) FROM unnest(v_src_cols) WITH ORDINALITY AS u(x, o) ORDER BY o), ', '),
        v_target,
        array_to_string(ARRAY(SELECT quote_ident(x) FROM unnest(v_tgt_cols) WITH ORDINALITY AS u(x, o) ORDER BY o), ', '),
        v_on_delete,
        v_set_null_cols,
        v_not_valid
    );

    IF v_not_valid = '' THEN
        RETURN 'created';
    END IF;

    BEGIN
        EXECUTE format('ALTER TABLE %s VALIDATE CONSTRAINT %I', v_source, v_name);
    EXCEPTION WHEN foreign_key_violation THEN
        RAISE WARNING 'BOS relation %: existing rows in % point to missing % records; the relation is enforced for new and changed rows until those rows are fixed',
            v_name, v_source, v_target;
        RETURN 'created_not_validated';
    END;
    RETURN 'created';
END;
$bos_rel$;
"#;

/// Name of the constraint `bos_ensure_relation` installs for `table.column` (PostgreSQL's 63-byte limit applied).
pub fn relation_constraint_name(table: &str, column: &str) -> String {
    let full = format!("fk_{}_{}", table, column);
    let mut end = full.len().min(63);
    while !full.is_char_boundary(end) {
        end -= 1;
    }
    full[..end].to_string()
}

fn sql_text(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Physical copies of a table for a strategy, as `(schema SQL expression, table-name prefix)`.
pub fn physical_targets(strategy: StorageStrategyIR, data_per_company: bool) -> Vec<(String, String)> {
    let current = "current_schema()".to_string();
    match strategy {
        StorageStrategyIR::SharedRLS | StorageStrategyIR::Partitioned => vec![(current, String::new())],
        StorageStrategyIR::SchemaPerCompany if data_per_company => vec![
            (sql_text("comp_default"), String::new()),
            (sql_text("public"), String::new()),
        ],
        StorageStrategyIR::SchemaPerCompany => vec![(sql_text("public"), String::new())],
        StorageStrategyIR::TablePerCompany if data_per_company => vec![
            (current.clone(), "default$".to_string()),
            (current, String::new()),
        ],
        StorageStrategyIR::TablePerCompany => vec![(current, String::new())],
    }
}

/// `SELECT bos_ensure_relation(...)` statements for every relation declared on `table`.
pub fn generate_relations(strategy: StorageStrategyIR, table: &TableIR) -> String {
    let mut sql = String::new();
    for col in &table.columns {
        let Some(fk) = &col.foreign_key else { continue };
        for (schema, prefix) in physical_targets(strategy, table.data_per_company) {
            sql.push_str(&format!(
                "SELECT bos_ensure_relation({}, {}, {}, {}, {}, {}, {});\n",
                schema,
                sql_text(&table.sql_table_name),
                sql_text(&col.sql_name),
                sql_text(&fk.target_table),
                sql_text(&fk.target_column),
                sql_text(&fk.on_delete),
                sql_text(&prefix),
            ));
        }
    }
    sql
}
