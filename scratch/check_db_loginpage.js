const { Client } = require('pg');
(async () => {
  const client = new Client({ connectionString: 'postgres://postgres:postgres@localhost:5432/my_db' });
  await client.connect();
  const res = await client.query("SELECT object_name, definition->'actions' as actions FROM _bos_meta_objects WHERE object_name = 'LoginPage'");
  console.log('DB LoginPage actions:', JSON.stringify(res.rows[0]?.actions, null, 2));
  await client.end();
})().catch(err => { console.error(err); process.exit(1); });
