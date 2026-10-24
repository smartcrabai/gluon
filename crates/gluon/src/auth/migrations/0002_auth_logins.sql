-- Separate from cookie-session storage: cached cookies cannot recreate authority.
CREATE TABLE gluon_auth_logins (id TEXT PRIMARY KEY, data JSONB NOT NULL);
