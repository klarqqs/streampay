import { Router, Request, Response } from 'express';
import { db } from '../db/client';

export const escrowRouter = Router();

// Create escrow
escrowRouter.post('/', async (req: Request, res: Response) => {
  const {
    contract_id, token_id, client_address,
    developer_address, total_amount, platform,
    repo_or_board, milestones
  } = req.body;

  const client = await db.connect();
  try {
    await client.query('BEGIN');

    const escrow = await client.query(
      `INSERT INTO escrows
        (contract_id, token_id, client_address, developer_address, total_amount, platform, repo_or_board)
       VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *`,
      [contract_id, token_id, client_address, developer_address, total_amount, platform, repo_or_board]
    );

    for (const m of milestones) {
      await client.query(
        `INSERT INTO milestones (escrow_id, index, title, trigger_keyword, bps)
         VALUES ($1,$2,$3,$4,$5)`,
        [escrow.rows[0].id, m.index, m.title, m.trigger_keyword, m.bps]
      );
    }

    await client.query('COMMIT');
    res.status(201).json(escrow.rows[0]);
  } catch (err) {
    await client.query('ROLLBACK');
    console.error(err);
    res.status(500).json({ error: 'Failed to create escrow' });
  } finally {
    client.release();
  }
});

// Get escrow
escrowRouter.get('/:id', async (req: Request, res: Response) => {
  const escrow = await db.query('SELECT * FROM escrows WHERE id = $1', [req.params.id]);
  if (!escrow.rows[0]) return res.status(404).json({ error: 'Not found' });

  const milestones = await db.query(
    'SELECT * FROM milestones WHERE escrow_id = $1 ORDER BY index',
    [req.params.id]
  );

  res.json({ ...escrow.rows[0], milestones: milestones.rows });
});
