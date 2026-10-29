import pg from '../ui/node_modules/pg/lib/index.js';

const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const cols = await pool.query(`
    SELECT column_name, data_type 
    FROM information_schema.columns 
    WHERE table_name = 'vendor'
  `);
  console.log('Columns of vendor:', cols.rows.map(r => r.column_name));

  const triggers = await pool.query(`
    SELECT tgname, pg_get_triggerdef(oid) 
    FROM pg_trigger 
    WHERE tgrelid = 'vendor'::regclass
  `);
  console.log('Triggers on vendor:', triggers.rows);

  const policies = await pool.query(`
    SELECT policyname, qual, with_check 
    FROM pg_policies 
    WHERE tablename = 'vendor'
  `);
  console.log('Policies on vendor:', policies.rows);
}

main().catch(console.error).finally(() => pool.end());
