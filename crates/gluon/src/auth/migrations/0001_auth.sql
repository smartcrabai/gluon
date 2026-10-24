-- Authentication owns its own migration history, separate from application SQLx migrations.
CREATE TABLE gluon_auth_users (id TEXT PRIMARY KEY, data JSONB NOT NULL);
CREATE TABLE gluon_auth_identities (id TEXT PRIMARY KEY, data JSONB NOT NULL);
CREATE TABLE gluon_auth_challenges (id TEXT PRIMARY KEY, data JSONB NOT NULL);
CREATE TABLE gluon_auth_passkeys (id TEXT PRIMARY KEY, data JSONB NOT NULL);
CREATE TABLE gluon_auth_rate_limits (id TEXT PRIMARY KEY, data JSONB NOT NULL);
