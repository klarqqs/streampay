import { Router, Request, Response } from 'express';
import bcrypt from 'bcryptjs';
import jwt from 'jsonwebtoken';
import crypto from 'crypto';
import nacl from 'tweetnacl';
import { db as pool } from '../db/client';
import { requireAuth, AuthRequest } from '../middleware/auth';

const router = Router();
const JWT_SECRET = process.env.JWT_SECRET!;
const JWT_EXPIRES = '7d';

function signToken(userId: string, email: string) {
  return jwt.sign({ userId, email }, JWT_SECRET, { expiresIn: JWT_EXPIRES });
}

// ── REGISTER (email + password) ──────────────────────────
router.post('/register', async (req: Request, res: Response) => {
  const { email, password, display_name } = req.body;
  if (!email || !password) return res.status(400).json({ error: 'Email and password required' });

  try {
    const exists = await pool.query('SELECT id FROM users WHERE email = $1', [email]);
    if (exists.rows.length) return res.status(409).json({ error: 'Email already registered' });

    const password_hash = await bcrypt.hash(password, 12);
    const result = await pool.query(
      `INSERT INTO users (email, password_hash, display_name, email_verified)
       VALUES ($1, $2, $3, false) RETURNING id, email, display_name`,
      [email, password_hash, display_name || null]
    );

    const user = result.rows[0];
    const token = signToken(user.id, user.email);
    return res.status(201).json({ token, user });
  } catch (err: any) {
    console.error(err);
    return res.status(500).json({ error: 'Registration failed' });
  }
});

// ── LOGIN (email + password) ─────────────────────────────
router.post('/login', async (req: Request, res: Response) => {
  const { email, password } = req.body;
  if (!email || !password) return res.status(400).json({ error: 'Email and password required' });

  try {
    const result = await pool.query('SELECT * FROM users WHERE email = $1', [email]);
    const user = result.rows[0];
    if (!user || !user.password_hash) return res.status(401).json({ error: 'Invalid credentials' });

    const valid = await bcrypt.compare(password, user.password_hash);
    if (!valid) return res.status(401).json({ error: 'Invalid credentials' });

    await pool.query('UPDATE users SET last_seen_at = NOW() WHERE id = $1', [user.id]);
    const token = signToken(user.id, user.email);
    return res.json({ token, user: { id: user.id, email: user.email, display_name: user.display_name, wallet_address: user.wallet_address } });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Login failed' });
  }
});

// ── WALLET: REQUEST NONCE ────────────────────────────────
// Client calls this with their Stellar address to get a challenge to sign
router.post('/wallet/nonce', async (req: Request, res: Response) => {
  const { wallet_address } = req.body;
  if (!wallet_address) return res.status(400).json({ error: 'wallet_address required' });

  const nonce = crypto.randomBytes(32).toString('hex');
  const message = `Sign in to StreamPay: ${nonce}`;
  const expires_at = new Date(Date.now() + 5 * 60 * 1000); // 5 min

  try {
    // Upsert user by wallet address
    await pool.query(
      `INSERT INTO users (wallet_address) VALUES ($1)
       ON CONFLICT (wallet_address) DO NOTHING`,
      [wallet_address]
    );

    const userResult = await pool.query(
      'SELECT id FROM users WHERE wallet_address = $1', [wallet_address]
    );
    const userId = userResult.rows[0].id;

    // Store nonce in sessions table
    const tokenHash = crypto.createHash('sha256').update(nonce).digest('hex');
    await pool.query(
      `INSERT INTO sessions (user_id, token_hash, wallet_nonce, expires_at)
       VALUES ($1, $2, $3, $4)
       ON CONFLICT (token_hash) DO NOTHING`,
      [userId, tokenHash, nonce, expires_at]
    );

    return res.json({ nonce, message });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Failed to generate nonce' });
  }
});

// ── WALLET: VERIFY SIGNATURE ─────────────────────────────
router.post('/wallet/verify', async (req: Request, res: Response) => {
  const { wallet_address, nonce, signature } = req.body;
  if (!wallet_address || !nonce || !signature) {
    return res.status(400).json({ error: 'wallet_address, nonce, and signature required' });
  }

  try {
    const tokenHash = crypto.createHash('sha256').update(nonce).digest('hex');
    const sessionResult = await pool.query(
      `SELECT s.*, u.id as user_id, u.email FROM sessions s
       JOIN users u ON u.id = s.user_id
       WHERE s.token_hash = $1 AND s.expires_at > NOW()`,
      [tokenHash]
    );

    if (!sessionResult.rows.length) {
      return res.status(401).json({ error: 'Nonce expired or invalid' });
    }

    const session = sessionResult.rows[0];

    // Verify the Ed25519 signature from Freighter
    const message = `Sign in to StreamPay: ${nonce}`;
    const messageBytes = Buffer.from(message, 'utf8');
    const signatureBytes = Buffer.from(signature, 'base64');

    // Decode Stellar public key (base32 → raw bytes, strip checksum)
    const StrKey = require('@stellar/stellar-sdk').StrKey;
    const publicKeyBytes = StrKey.decodeEd25519PublicKey(wallet_address);
    const valid = nacl.sign.detached.verify(messageBytes, signatureBytes, publicKeyBytes);

    if (!valid) return res.status(401).json({ error: 'Invalid signature' });

    // Clean up nonce session
    await pool.query('DELETE FROM sessions WHERE token_hash = $1', [tokenHash]);

    // Issue JWT
    await pool.query('UPDATE users SET last_seen_at = NOW() WHERE id = $1', [session.user_id]);
    const token = signToken(session.user_id, session.email || wallet_address);

    const userResult = await pool.query(
      'SELECT id, email, display_name, wallet_address FROM users WHERE id = $1',
      [session.user_id]
    );

    return res.json({ token, user: userResult.rows[0] });
  } catch (err) {
    console.error(err);
    return res.status(500).json({ error: 'Verification failed' });
  }
});

// ── ME ───────────────────────────────────────────────────
router.get('/me', requireAuth, async (req: AuthRequest, res: Response) => {
  const result = await pool.query(
    `SELECT id, email, display_name, wallet_address, created_at FROM users WHERE id = $1`,
    [req.user!.userId]
  );
  return res.json(result.rows[0]);
});

export default router;
