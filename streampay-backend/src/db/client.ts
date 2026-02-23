import { Pool } from 'pg';

export const db = new Pool({
  connectionString: process.env.DATABASE_URL,
  ssl: process.env.NODE_ENV === 'production'
    ? { rejectUnauthorized: false }
    : false,
});

export async function testConnection() {
  const client = await db.connect();
  await client.query('SELECT 1');
  client.release();
  console.log('✅ Database connected');
}
