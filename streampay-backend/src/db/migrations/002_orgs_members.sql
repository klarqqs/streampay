-- ============================================================
-- StreamPay Migration 002: Users, Organizations, Members, Approvals
-- ============================================================

-- ── USERS ──────────────────────────────────────────────────
CREATE TABLE users (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  email            TEXT UNIQUE NOT NULL,
  email_verified   BOOLEAN DEFAULT FALSE,
  password_hash    TEXT,                        -- null if wallet-only login
  wallet_address   TEXT UNIQUE,                 -- Stellar G... address
  display_name     TEXT,
  avatar_url       TEXT,
  created_at       TIMESTAMPTZ DEFAULT NOW(),
  last_seen_at     TIMESTAMPTZ
);

CREATE INDEX idx_users_email          ON users(email);
CREATE INDEX idx_users_wallet_address ON users(wallet_address);

-- ── ORGANIZATIONS ──────────────────────────────────────────
CREATE TABLE organizations (
  id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name                TEXT NOT NULL,
  slug                TEXT UNIQUE NOT NULL,     -- e.g. "acme-labs" for URLs
  logo_url            TEXT,
  treasury_wallet     TEXT,                     -- Stellar G... address
  approval_threshold  SMALLINT DEFAULT 1,       -- multisig ready: default 1
  created_by          UUID REFERENCES users(id) ON DELETE SET NULL,
  created_at          TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_orgs_slug ON organizations(slug);

-- ── MEMBERS ────────────────────────────────────────────────
-- One row per user per org. Role is scoped to the org.
CREATE TYPE member_role AS ENUM ('admin', 'finance', 'contributor');

CREATE TABLE members (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role        member_role NOT NULL DEFAULT 'contributor',
  invited_by  UUID REFERENCES users(id) ON DELETE SET NULL,
  joined_at   TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(org_id, user_id)                      -- one membership per org
);

CREATE INDEX idx_members_org_id  ON members(org_id);
CREATE INDEX idx_members_user_id ON members(user_id);

-- ── ESCROW: LINK TO ORGS ───────────────────────────────────
-- Add org references to existing escrows table.
-- client_org_id  = org funding the escrow
-- developer_org_id = org receiving payment
-- Both are nullable for backwards compat with existing rows.
ALTER TABLE escrows
  ADD COLUMN client_org_id    UUID REFERENCES organizations(id) ON DELETE SET NULL,
  ADD COLUMN developer_org_id UUID REFERENCES organizations(id) ON DELETE SET NULL;

CREATE INDEX idx_escrows_client_org    ON escrows(client_org_id);
CREATE INDEX idx_escrows_developer_org ON escrows(developer_org_id);

-- ── APPROVALS ──────────────────────────────────────────────
-- One row per approval action. Supports threshold logic later.
-- Right now approval_threshold = 1 so first approval triggers release.
CREATE TYPE approval_action AS ENUM ('approve', 'reject', 'dispute');

CREATE TABLE approvals (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  escrow_id   UUID NOT NULL REFERENCES escrows(id) ON DELETE CASCADE,
  milestone_index SMALLINT NOT NULL,
  member_id   UUID NOT NULL REFERENCES members(id) ON DELETE CASCADE,
  action      approval_action NOT NULL,
  note        TEXT,
  created_at  TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(escrow_id, milestone_index, member_id)  -- one vote per member per milestone
);

CREATE INDEX idx_approvals_escrow_id ON approvals(escrow_id);
CREATE INDEX idx_approvals_member_id ON approvals(member_id);

-- ── AUTH SESSIONS ──────────────────────────────────────────
-- Lightweight session table. JWT preferred but this covers
-- wallet-based nonce challenges too.
CREATE TABLE sessions (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token_hash    TEXT NOT NULL UNIQUE,           -- hashed JWT or session token
  wallet_nonce  TEXT,                           -- for wallet signature auth
  expires_at    TIMESTAMPTZ NOT NULL,
  created_at    TIMESTAMPTZ DEFAULT NOW(),
  ip_address    TEXT,
  user_agent    TEXT
);

CREATE INDEX idx_sessions_user_id    ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

-- ── INVITATIONS ────────────────────────────────────────────
-- Email invites to join an org before the user has an account.
CREATE TABLE invitations (
  id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  email       TEXT NOT NULL,
  role        member_role NOT NULL DEFAULT 'contributor',
  invited_by  UUID REFERENCES users(id) ON DELETE SET NULL,
  token       TEXT UNIQUE NOT NULL,             -- random invite token in URL
  accepted_at TIMESTAMPTZ,
  expires_at  TIMESTAMPTZ DEFAULT NOW() + INTERVAL '7 days',
  created_at  TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(org_id, email)
);

CREATE INDEX idx_invitations_token  ON invitations(token);
CREATE INDEX idx_invitations_email  ON invitations(email);
CREATE INDEX idx_invitations_org_id ON invitations(org_id);

-- ── HELPER VIEW ────────────────────────────────────────────
-- Convenient view: member with user + org details in one query
CREATE VIEW member_details AS
  SELECT
    m.id           AS member_id,
    m.role,
    m.joined_at,
    u.id           AS user_id,
    u.email,
    u.display_name,
    u.wallet_address,
    o.id           AS org_id,
    o.name         AS org_name,
    o.slug         AS org_slug,
    o.approval_threshold
  FROM members m
  JOIN users        u ON u.id = m.user_id
  JOIN organizations o ON o.id = m.org_id;
