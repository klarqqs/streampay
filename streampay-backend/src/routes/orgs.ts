import { Router, Response } from 'express';
import crypto from 'crypto';
import { db as pool } from '../db/client';
import { requireAuth, requireOrgRole, AuthRequest } from '../middleware/auth';

const router = Router();
router.use(requireAuth);

// ── CREATE ORG ───────────────────────────────────────────
router.post('/', async (req: AuthRequest, res: Response) => {
  const { name, treasury_wallet } = req.body;
  if (!name) return res.status(400).json({ error: 'name required' });

  const slug = name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '')
    + '-' + crypto.randomBytes(3).toString('hex');

  try {
    const org = await pool.query(
      `INSERT INTO organizations (name, slug, treasury_wallet, created_by)
       VALUES ($1, $2, $3, $4) RETURNING *`,
      [name, slug, treasury_wallet || null, req.user!.userId]
    );

    // Creator becomes admin automatically
    await pool.query(
      `INSERT INTO members (org_id, user_id, role) VALUES ($1, $2, 'admin')`,
      [org.rows[0].id, req.user!.userId]
    );

    return res.status(201).json(org.rows[0]);
  } catch (err: any) {
    if (err.code === '23505') return res.status(409).json({ error: 'Org name taken' });
    console.error(err);
    return res.status(500).json({ error: 'Failed to create org' });
  }
});

// ── LIST MY ORGS ─────────────────────────────────────────
router.get('/', async (req: AuthRequest, res: Response) => {
  const result = await pool.query(
    `SELECT o.*, m.role FROM organizations o
     JOIN members m ON m.org_id = o.id
     WHERE m.user_id = $1
     ORDER BY o.created_at DESC`,
    [req.user!.userId]
  );
  return res.json(result.rows);
});

// ── GET ORG DASHBOARD ────────────────────────────────────
router.get('/:orgId/dashboard', async (req: AuthRequest, res: Response) => {
  const { orgId } = req.params;
  const userId = req.user!.userId;

  // Check membership
  const membership = await pool.query(
    `SELECT role FROM members WHERE org_id = $1 AND user_id = $2`,
    [orgId, userId]
  );
  if (!membership.rows.length) return res.status(403).json({ error: 'Not a member' });

  try {
    const [org, members, fundedEscrows, receivingEscrows, recentActivity] = await Promise.all([
      // Org info
      pool.query(`SELECT * FROM organizations WHERE id = $1`, [orgId]),

      // Members
      pool.query(
        `SELECT m.id, m.role, m.joined_at, u.email, u.display_name, u.wallet_address
         FROM members m JOIN users u ON u.id = m.user_id
         WHERE m.org_id = $1 ORDER BY m.joined_at`,
        [orgId]
      ),

      // Escrows where this org is client (funding)
      pool.query(
        `SELECT e.*, COUNT(mil.id) as milestone_count,
                SUM(CASE WHEN mil.status = 'released' THEN 1 ELSE 0 END) as completed_milestones
         FROM escrows e
         LEFT JOIN milestones mil ON mil.escrow_id = e.id
         WHERE e.client_org_id = $1
         GROUP BY e.id ORDER BY e.created_at DESC LIMIT 10`,
        [orgId]
      ),

      // Escrows where this org is developer (receiving)
      pool.query(
        `SELECT e.*, COUNT(mil.id) as milestone_count,
                SUM(CASE WHEN mil.status = 'released' THEN 1 ELSE 0 END) as completed_milestones
         FROM escrows e
         LEFT JOIN milestones mil ON mil.escrow_id = e.id
         WHERE e.developer_org_id = $1
         GROUP BY e.id ORDER BY e.created_at DESC LIMIT 10`,
        [orgId]
      ),

      // Recent approvals activity
      pool.query(
        `SELECT a.*, mil.title as milestone_title, e.contract_id,
                u.display_name as approved_by_name
         FROM approvals a
         JOIN milestones mil ON mil.escrow_id = a.escrow_id AND mil.index = a.milestone_index
         JOIN escrows e ON e.id = a.escrow_id
         JOIN members m ON m.id = a.member_id
         JOIN users u ON u.id = m.user_id
         WHERE e.client_org_id = $1 OR e.developer_org_id = $1
         ORDER BY a.created_at DESC LIMIT 20`,
        [orgId]
      ),
    ]);

    return res.json({
      org: org.rows[0],
      role: membership.rows[0].role,
      members: members.rows,
      funded_escrows: fundedEscrows.rows,
      receiving_escrows: receivingEscrows.rows,
      recent_activity: recentActivity.rows,
    });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Failed to load dashboard' });
  }
});

// ── INVITE MEMBER ────────────────────────────────────────
router.post('/:orgId/invite', async (req: AuthRequest, res: Response) => {
  const { orgId } = req.params;
  const { email, role = 'contributor' } = req.body;
  if (!email) return res.status(400).json({ error: 'email required' });

  // Only admins can invite
  const membership = await pool.query(
    `SELECT role FROM members WHERE org_id = $1 AND user_id = $2`,
    [orgId, req.user!.userId]
  );
  if (!membership.rows.length || membership.rows[0].role !== 'admin') {
    return res.status(403).json({ error: 'Only admins can invite members' });
  }

  try {
    const token = crypto.randomBytes(32).toString('hex');
    const result = await pool.query(
      `INSERT INTO invitations (org_id, email, role, invited_by, token)
       VALUES ($1, $2, $3, $4, $5)
       ON CONFLICT (org_id, email) DO UPDATE SET token = EXCLUDED.token, expires_at = NOW() + INTERVAL '7 days'
       RETURNING *`,
      [orgId, email, role, req.user!.userId, token]
    );

    // TODO: send invite email — for now return the token
    const inviteUrl = `${process.env.FRONTEND_URL}/invite/${token}`;
    return res.status(201).json({ invitation: result.rows[0], invite_url: inviteUrl });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Failed to create invitation' });
  }
});

// ── ACCEPT INVITE ────────────────────────────────────────
router.post('/invitations/:token/accept', async (req: AuthRequest, res: Response) => {
  const { token } = req.params;

  try {
    const inv = await pool.query(
      `SELECT * FROM invitations WHERE token = $1 AND accepted_at IS NULL AND expires_at > NOW()`,
      [token]
    );
    if (!inv.rows.length) return res.status(404).json({ error: 'Invitation not found or expired' });

    const invitation = inv.rows[0];

    // Add member
    await pool.query(
      `INSERT INTO members (org_id, user_id, role, invited_by)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT (org_id, user_id) DO NOTHING`,
      [invitation.org_id, req.user!.userId, invitation.role, invitation.invited_by]
    );

    await pool.query(
      `UPDATE invitations SET accepted_at = NOW() WHERE token = $1`, [token]
    );

    return res.json({ message: 'Joined organization', org_id: invitation.org_id, role: invitation.role });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Failed to accept invitation' });
  }
});

// ── APPROVE MILESTONE ────────────────────────────────────
router.post('/:orgId/escrows/:escrowId/approve', async (req: AuthRequest, res: Response) => {
  const { orgId, escrowId } = req.params;
  const { milestone_index, action = 'approve', note } = req.body;

  if (milestone_index === undefined) return res.status(400).json({ error: 'milestone_index required' });

  // Must be admin or finance
  const membership = await pool.query(
    `SELECT id, role FROM members WHERE org_id = $1 AND user_id = $2`,
    [orgId, req.user!.userId]
  );
  if (!membership.rows.length || !['admin', 'finance'].includes(membership.rows[0].role)) {
    return res.status(403).json({ error: 'Requires admin or finance role' });
  }

  try {
    const memberId = membership.rows[0].id;

    await pool.query(
      `INSERT INTO approvals (escrow_id, milestone_index, member_id, action, note)
       VALUES ($1, $2, $3, $4, $5)`,
      [escrowId, milestone_index, memberId, action, note || null]
    );

    // Check if approval threshold is met
    const org = await pool.query(`SELECT approval_threshold FROM organizations WHERE id = $1`, [orgId]);
    const threshold = org.rows[0].approval_threshold;

    const approvalCount = await pool.query(
      `SELECT COUNT(*) FROM approvals
       WHERE escrow_id = $1 AND milestone_index = $2 AND action = 'approve'`,
      [escrowId, milestone_index]
    );

    const count = parseInt(approvalCount.rows[0].count);
    const thresholdMet = count >= threshold;

    if (thresholdMet) {
      await pool.query(
        `UPDATE milestones SET status = 'released' WHERE escrow_id = $1 AND index = $2`,
        [escrowId, milestone_index]
      );
      // TODO: trigger on-chain release transaction
    }

    return res.json({
      recorded: true,
      approvals: count,
      threshold,
      threshold_met: thresholdMet,
      message: thresholdMet ? 'Threshold met — payment releasing' : `${count}/${threshold} approvals`
    });
  } catch (err: any) {
    if (err.code === '23505') return res.status(409).json({ error: 'Already voted on this milestone' });
    console.error(err);
    return res.status(500).json({ error: 'Failed to record approval' });
  }
});

export default router;
