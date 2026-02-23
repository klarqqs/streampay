CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE IF NOT EXISTS escrows (
  id                UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  contract_id       VARCHAR(56)  NOT NULL UNIQUE,
  token_id          VARCHAR(56)  NOT NULL,
  client_address    VARCHAR(56)  NOT NULL,
  developer_address VARCHAR(56)  NOT NULL,
  total_amount      BIGINT       NOT NULL,
  status            VARCHAR(20)  NOT NULL DEFAULT 'active',
  platform          VARCHAR(20)  NOT NULL,
  repo_or_board     VARCHAR(255) NOT NULL,
  created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS milestones (
  id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  escrow_id       UUID REFERENCES escrows(id) ON DELETE CASCADE,
  index           INTEGER      NOT NULL,
  title           VARCHAR(255) NOT NULL,
  trigger_keyword VARCHAR(255) NOT NULL,
  bps             INTEGER      NOT NULL,
  status          VARCHAR(20)  NOT NULL DEFAULT 'pending',
  completed_at    TIMESTAMPTZ,
  task_url        TEXT,
  UNIQUE(escrow_id, index)
);

CREATE TABLE IF NOT EXISTS platform_connections (
  id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
  escrow_id      UUID REFERENCES escrows(id) ON DELETE CASCADE,
  platform       VARCHAR(20)  NOT NULL,
  external_id    VARCHAR(255) NOT NULL,
  webhook_secret VARCHAR(255),
  access_token   TEXT,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  UNIQUE(platform, external_id)
);
