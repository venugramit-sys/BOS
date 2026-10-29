import pg from '../ui/node_modules/pg/lib/index.js';

const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function testQuery(name, fn) {
  const client = await pool.connect();
  try {
    await client.query("BEGIN;");
    await client.query("SET LOCAL bos.current_tenant_id = '254d6f53-0c80-473d-91b3-74b4bdcc1ad3';");
    await client.query("SET LOCAL bos.current_user_id = '00000000-0000-0000-0000-000000000000';");
    await client.query("SET LOCAL bos.current_company_id = '00000000-0000-0000-0000-000000000000';");
    
    console.log(`Running: ${name}`);
    await fn(client);
    console.log(`SUCCESS: ${name}`);
    await client.query("ROLLBACK;");
  } catch (err) {
    console.error(`FAILED: ${name}`, err.message);
    await client.query("ROLLBACK;");
  } finally {
    client.release();
  }
}

async function main() {
  const id = 'fc381b3d-740f-4ebf-9895-994d8af1e6f4';

  await testQuery('1. lock_for_write', async (c) => {
    const res = await c.query('SELECT to_jsonb(t.*) FROM "vendor" t WHERE t."_id" = $1 FOR UPDATE', [id]);
    console.log('x_rec found:', res.rows.length);
  });

  await testQuery('2. statemachine query', async (c) => {
    const res = await c.query(
      "SELECT definition FROM \"_bos_meta_objects\" WHERE object_type = 'statemachine' AND status = 'active' AND LOWER(definition->>'target_table') = LOWER($1) LIMIT 1",
      ['vendor']
    );
    console.log('sm_def:', res.rows.length);
  });

  await testQuery('3. Update SQL', async (c) => {
    const res = await c.query(
      `UPDATE "vendor" AS t SET "name" = 'Acme Test', "_updated_at" = NOW(), "_system_version" = COALESCE(t."_system_version", 0) + 1 WHERE t."_id" = $1 RETURNING to_jsonb(t.*)`,
      [id]
    );
    console.log('updated:', res.rows.length);
  });

  await testQuery('4. Outbox insert', async (c) => {
    const res = await c.query(
      `INSERT INTO "_bos_event_outbox" ("id", "tenant_id", "company_id", "publisher", "event_name", "payload", "status", "retry_count", "created_at") VALUES ($1, $2, $3, $4, $5, $6, $7, 0, NOW())`,
      ['a0000000-0000-0000-0000-000000000001', '254d6f53-0c80-473d-91b3-74b4bdcc1ad3', '00000000-0000-0000-0000-000000000000', 'Vendor', 'OnBeforeModify', '{}', 'pending']
    );
    console.log('outbox inserted');
  });

  await pool.end();
}

main().catch(console.error);
