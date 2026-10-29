import pg from '../ui/node_modules/pg/lib/index.js';
const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const res = await pool.query("SELECT relname, relrowsecurity, relforcerowsecurity FROM pg_class WHERE relname = 'vendor'");
  console.log(res.rows);
}
main().catch(console.error).finally(() => pool.end());
