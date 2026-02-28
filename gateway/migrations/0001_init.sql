-- =============================================================================
-- OpenClaw Gateway -- Initial Schema
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email           VARCHAR(255) NOT NULL UNIQUE,
    name            VARCHAR(255),
    password_hash   VARCHAR(255),
    google_id       VARCHAR(255),
    role            VARCHAR(50) NOT NULL DEFAULT 'user',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- api_keys
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_keys (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    key_hash        VARCHAR(128) NOT NULL UNIQUE,
    name            VARCHAR(255) NOT NULL,
    user_id         UUID REFERENCES users(id) ON DELETE CASCADE,
    cost_profile    VARCHAR(50) NOT NULL DEFAULT 'balanced',
    rate_limit_rpm  INTEGER NOT NULL DEFAULT 60,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- providers
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS providers (
    id                          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name                        VARCHAR(100) NOT NULL UNIQUE,
    base_url                    VARCHAR(500) NOT NULL,
    api_key_env                 VARCHAR(100) NOT NULL,
    is_enabled                  BOOLEAN NOT NULL DEFAULT TRUE,
    priority                    INTEGER NOT NULL DEFAULT 0,
    max_rpm                     INTEGER NOT NULL DEFAULT 1000,
    circuit_breaker_threshold   INTEGER NOT NULL DEFAULT 5,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ---------------------------------------------------------------------------
-- models
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS models (
    id                      UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    provider_id             UUID NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    model_id                VARCHAR(255) NOT NULL,
    display_name            VARCHAR(255) NOT NULL,
    tier                    VARCHAR(50) NOT NULL DEFAULT 'standard',
    input_cost_per_million  NUMERIC(12, 6) NOT NULL DEFAULT 0,
    output_cost_per_million NUMERIC(12, 6) NOT NULL DEFAULT 0,
    max_context_length      INTEGER NOT NULL DEFAULT 4096,
    supports_vision         BOOLEAN NOT NULL DEFAULT FALSE,
    supports_tools          BOOLEAN NOT NULL DEFAULT FALSE,
    supports_streaming      BOOLEAN NOT NULL DEFAULT TRUE,
    is_enabled              BOOLEAN NOT NULL DEFAULT TRUE,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (provider_id, model_id)
);

-- ---------------------------------------------------------------------------
-- request_logs
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS request_logs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    api_key_id      UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    request_id      VARCHAR(255) NOT NULL,
    model_used      VARCHAR(255) NOT NULL,
    provider        VARCHAR(100) NOT NULL,
    tier            VARCHAR(50) NOT NULL DEFAULT 'standard',
    cost_profile    VARCHAR(50) NOT NULL DEFAULT 'balanced',
    input_tokens    INTEGER NOT NULL DEFAULT 0,
    output_tokens   INTEGER NOT NULL DEFAULT 0,
    total_tokens    INTEGER NOT NULL DEFAULT 0,
    cost_usd        NUMERIC(12, 8) NOT NULL DEFAULT 0,
    latency_ms      INTEGER NOT NULL DEFAULT 0,
    status          VARCHAR(50) NOT NULL DEFAULT 'success',
    cache_hit       BOOLEAN NOT NULL DEFAULT FALSE,
    error_message   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_request_logs_api_key_id ON request_logs(api_key_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_created_at ON request_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_request_logs_provider   ON request_logs(provider);
CREATE INDEX IF NOT EXISTS idx_request_logs_model      ON request_logs(model_used);

-- ---------------------------------------------------------------------------
-- settings (key-value store for runtime configuration)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS settings (
    key         VARCHAR(255) PRIMARY KEY,
    value       JSONB NOT NULL DEFAULT '{}',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
