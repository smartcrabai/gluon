# Screenless authentication server

See [the authentication guide](../../docs/auth.md) for configuration and protocol contracts. This example has no login page or browser SDK.

```sh
# PostgreSQL is the default. Provide a disposable database and TLS SMTP account.
export DATABASE_URL='postgres://user:password@localhost/example'
export AUTH_SMTP_SERVER='smtp.example.test'
export AUTH_SMTP_PORT=587
export AUTH_SMTP_USERNAME='your-user'
export AUTH_SMTP_PASSWORD='your-password'
export AUTH_SMTP_FROM='auth@example.test'
# Generate independent development secrets. Persist them securely for stable deployments.
export AUTH_OTP_SECRET="$(openssl rand -hex 32)"
export SECRET_KEY_BASE="$(openssl rand -hex 64)"
export AUTH_MIGRATE_ON_START=1 # explicit schema migration opt-in
export GLUON_BIND=127.0.0.1:3000
export GLUON_INSECURE_COOKIE=1 # local HTTP only
export GLUON_TELEMETRY_DISABLED=1
cargo run --manifest-path examples/Cargo.toml -p sample-auth
```

Run that command from the repository root. For disposable local development without PostgreSQL, explicitly set `AUTH_STORE=memory-development` and unset `DATABASE_URL`. Memory loses all auth records on process restart and is not multi-instance storage. SMTP is still TLS-only. Never log OTPs or expose a public test inbox.

Optional JWT configuration (no implicit temporary key):

```sh
openssl genpkey -algorithm Ed25519 -out jwt-private.pem
chmod 600 jwt-private.pem
export AUTH_JWT_PRIVATE_KEY_FILE="$PWD/jwt-private.pem"
export AUTH_JWT_KID='development-1'
export AUTH_JWT_ISSUER='https://auth.example.test'
export AUTH_JWT_AUDIENCE='external-api'
```

Set all JWT settings before starting. Keep the private key out of source control. This example loads one signing key; coordinated multi-key rotation belongs in application configuration. Logout stops new JWT issuance but does not revoke tokens already accepted by offline verifiers before expiration.

For Passkeys, set `AUTH_ORIGIN` and `AUTH_RP_ID` consistently. Application browser code must implement `navigator.credentials.create/get` and base64url/buffer conversion. OAuth providers are not configured by this example.

Start with `GET /auth/csrf` and retain the Cookie. Send the returned token in `x-csrf-token` for unsafe methods. OTP request/verification live at `/auth/email-otp/request` and `/auth/email-otp/verify`. Fetch a new CSRF token after successful login before requesting `/auth/token` or logout. `/auth/jwks` publishes public keys only when JWT is configured. `/health` provides process readiness.

This example allows first-time verified identities to provision auth users. It is not an invite-only or business-domain authorization policy. For restrictions, install `RegistrationPolicy` on a custom shared `Auth` service as described in the guide.

The CLI example smoke test launches the actual executable and checks configuration refusal, CSRF, anonymous token refusal and public JWKS without SMTP network access. Additionally, `cargo test --manifest-path examples/Cargo.toml -p sample-auth --test process` launches a separate integration-test process calling the same `sample_auth::run(config)` assembly used by main. A test-only `EmailSender` delivers into a private temporary mailbox outside the production HTTP routes. The test covers OTP verification, restored session, stale/fresh CSRF, JWT issuance and public-JWKS signature/issuer/audience verification, logout, new-token denial and continued offline validity of the issued token. There are no production OTP logging routes or plaintext SMTP exceptions. This does not prove real SMTP delivery, external OAuth interoperability or real-browser WebAuthn; those require configured services and separate integration checks.
