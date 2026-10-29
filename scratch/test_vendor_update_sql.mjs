import pg from '../ui/node_modules/pg/lib/index.js';

const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const client = await pool.connect();
  try {
    await client.query("BEGIN;");
    await client.query("SET LOCAL bos.current_tenant_id = '254d6f53-0c80-473d-91b3-74b4bdcc1ad3';");
    await client.query("SET LOCAL bos.current_user_id = '00000000-0000-0000-0000-000000000000';");
    await client.query("SET LOCAL bos.current_company_id = '00000000-0000-0000-0000-000000000000';");

    const res = await client.query(`
      UPDATE "vendor" AS t 
      SET "name" = 'Acme Industrial Supplies Ltd', "_updated_at" = NOW(), "_system_version" = COALESCE(t."_system_version", 0) + 1 
      WHERE t."_id" = 'fc381b3d-740f-4ebf-9895-994d8af1e6f4'::uuid 
      RETURNING to_jsonb(t.*)
    `);
    console.log('Update result:', res.rows);
    await client.query("ROLLBACK;");
  } catch (err) {
    console.error('SQL ERROR:', err);
    await client.query("ROLLBACK;");
  } finally {
    client.release();
    pool.end();
  }
}

main().catch(console.error);
