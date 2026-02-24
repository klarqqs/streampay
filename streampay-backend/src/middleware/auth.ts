import { Request, Response, NextFunction } from 'express';
import jwt from 'jsonwebtoken';
import { pool } from '../db/client';

export interface AuthRequest extends Request {
  user?: {
    userId: string;
    email: string;
  };
}

export async function requireAuth(req: AuthRequest, res: Response, next: NextFunction) {
  const header = req.headers.authorization;
  if (!header?.startsWith('Bearer ')) {
    return res.status(401).json({ error: 'Missing token' });
  }

  const token = header.slice(7);
  try {
    const payload = jwt.verify(token, process.env.JWT_SECRET!) as any;
    req.user = { userId: payload.userId, email: payload.email };
    next();
  } catch {
    return res.status(401).json({ error: 'Invalid or expired token' });
  }
}

export async function requireOrgRole(roles: string[]) {
  return async (req: AuthRequest, res: Response, next: NextFunction) => {
    const { orgId } = req.params;
    const userId = req.user?.userId;
    if (!userId) return res.status(401).json({ error: 'Unauthorized' });

    const result = await pool.query(
      `SELECT role FROM members WHERE org_id = $1 AND user_id = $2`,
      [orgId, userId]
    );

    if (!result.rows.length) {
      return res.status(403).json({ error: 'Not a member of this organization' });
    }

    const role = result.rows[0].role;
    if (!roles.includes(role)) {
      return res.status(403).json({ error: `Requires role: ${roles.join(' or ')}` });
    }

    next();
  };
}
