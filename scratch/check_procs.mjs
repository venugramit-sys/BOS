import pg from '../ui/node_modules/pg/lib/index.js';
const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const procs = await pool.query("SELECT proname, prosrc FROM pg_proc WHERE prosrc ILIKE '%vendor%'");
  console.log('Procs referencing vendor:', procs.rows.map(r => r.proname));
}
main().catch(console.error).finally(() => pool.end());
