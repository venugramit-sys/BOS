import pg from '../ui/node_modules/pg/lib/index.js';
const pool = new pg.Pool({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });

async function main() {
  const policies = await pool.query("SELECT tablename, policyname, qual, with_check FROM pg_policies");
  for (const p of policies.rows) {
    if (p.qual && p.qual.includes('company_id')) {
      console.log(`Table: ${p.tablename}, Policy: ${p.policyname}\nQual: ${p.qual}\n`);
    }
  }
}
main().catch(console.error).finally(() => pool.end());
