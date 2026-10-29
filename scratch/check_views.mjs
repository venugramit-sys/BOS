import pg from '../ui/node_modules/pg/lib/index.js';
const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const views = await pool.query("SELECT table_name, view_definition FROM information_schema.views WHERE view_definition ILIKE '%vendor%'");
  console.log('Views referencing vendor:', views.rows);
}
main().catch(console.error).finally(() => pool.end());
