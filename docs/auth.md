# Authentication

Authentication is opt-in: enable `gluon = { features = ["auth"], ... }`, build an `AuthConfig` with `AuthBuilder`, and pass it to `Boot::with_auth(config)`. Existing applications need no auth configuration. This framework remains under development, not security-certified for production.

## Minimal integration

```rust,ignore
use std::sync::Arc;
use gluon::{Boot, auth::{AuthBuilder, SmtpConfig, SmtpEmailSender}};
let sender = SmtpEmailSender::new(SmtpConfig {
    server: smtp_host, port: 587, username, password, from,
})?;
let config = AuthBuilder::new()
    .email_otp(Arc::new(sender), separate_otp_secret)
    .build()?;
Boot::new().with_auth(config).run().await?;
```

The builder returns configuration, not a running service. Boot shares one `Arc<Auth>` between its HTTP adapter and DI. Direct APIs use `AuthContext` with the real server session, trusted peer IP and browser binding. Never accept a user ID, forwarded IP or binding supplied by an untrusted client as authenticated identity. `CurrentUser` requires a valid Cookie session and rejects anonymous requests with 401. `Option<CurrentUser>` permits anonymity but must still surface backend/configuration failures. Bearer JWTs do not authenticate these extractors.

## Storage and migrations

PostgreSQL is the default and requires `DATABASE_URL`. There is no fallback to Memory when auth is enabled without a database. The existing pool/session store are reused. Auth schema history is separate from application `_sqlx_migrations`. Apply `PostgresAuthStore::migrate()` explicitly, or opt into `config.migrate_on_start = true`; otherwise startup checks an already-applied schema. Back up and review migrations before deployment. Expired flows/rate counters need periodic `AuthStore::cleanup_expired(now)` maintenance.

For disposable development/tests only, explicitly set `config.store = Some(Arc::new(MemoryAuthStore::new(10_000)))` or use `AuthBuilder::store`. Memory is process-local, bounded and lost on restart; it cannot coordinate multiple instances. Cookie sessions also require durable storage for multi-instance deployments.

## Screenless example

`examples/auth` has no login UI, templates or frontend SDK. Run from the repository root with `cargo run --manifest-path examples/Cargo.toml -p sample-auth`. It exposes `/health` and the configured auth routes.

`cargo test --manifest-path examples/Cargo.toml -p sample-auth --test process` runs the same application assembly in a guarded subprocess with a test-only sender and a private temporary mailbox. It checks OTP login, restored session, CSRF rotation, JWT verification through public JWKS and logout semantics. The fixture does not add a production OTP route, weaken SMTP TLS, or demonstrate real SMTP delivery. The separate CLI example smoke launches the production executable and checks environment configuration and anonymous/public endpoints.

Required environment:

| Variable | Meaning |
| --- | --- |
| `DATABASE_URL` | PostgreSQL connection, required by default |
| `AUTH_STORE` | `postgres` (default), or explicit `memory-development` |
| `AUTH_MIGRATE_ON_START=1` | Explicit opt-in to auth migrations |
| `AUTH_SMTP_SERVER`, `AUTH_SMTP_PORT` | TLS SMTP host and port (587 default) |
| `AUTH_SMTP_USERNAME`, `AUTH_SMTP_PASSWORD`, `AUTH_SMTP_FROM` | SMTP credentials and sender mailbox |
| `AUTH_OTP_SECRET` | Independent high-entropy OTP HMAC secret, at least 32 bytes |
| `SECRET_KEY_BASE` | Separate Cookie signing secret, at least 64 bytes |
| `AUTH_ORIGIN` | Explicit permitted application origin |
| `AUTH_RP_ID` | Optional Passkey RP ID, requires `AUTH_ORIGIN` |
| `AUTH_JWT_PRIVATE_KEY_FILE` | Optional Ed25519 PEM private key file |
| `AUTH_JWT_KID`, `AUTH_JWT_ISSUER`, `AUTH_JWT_AUDIENCE` | Required together with JWT key file |

For local HTTP only, set `GLUON_INSECURE_COOKIE=1`; `memory-development` requires this explicit development setting. Do not deploy it publicly. `GLUON_BIND=127.0.0.1:3000` limits local exposure. SMTP remains TLS-only even in Memory mode. Use a TLS-capable test mailbox, not a plaintext SMTP sink. OTPs are delivered only through the sender, never returned by HTTP or printed to logs. The example does not configure OAuth providers; use the Rust provider configuration below in your application.

To generate a development JWT key: `openssl genpkey -algorithm Ed25519 -out jwt-private.pem`. Restrict file permissions and never commit the file. No temporary signing key is implicitly generated. JWT is disabled unless configured, and partial JWT environment configuration fails startup. `JwtConfig::new(issuer, audience, keys, active_kid)` requires explicit signing material and defaults to a five-minute TTL with zero clock skew. The example uses that default TTL and explicitly opts into a 30-second clock tolerance.

## Registration policy and business users

By default, a first verified OTP/OAuth identity provisions a minimal auth user. This is not a generated business-domain user model or an invite-only policy. Applications that prohibit public signup or attach verified identities to business-owned IDs must explicitly install a `RegistrationPolicy` on the `Auth` service with `Auth::with_registration_policy` before sharing it in `Arc`. `VerifiedIdentity` contains verified `method`, `issuer` and stable `subject`. The synchronous `register` hook returns `Ok(None)` to allocate an auth ID, `Ok(Some(UserId(...)))` to select an application-approved auth ID, or an `AuthError` to deny first-time registration. Treat the hook as trusted application code, not an endpoint accepting client-selected IDs. Preserve explicit linking and never merge users based on unverified or matching email strings.

The policy runs only for first-time identity creation, not every existing-user login and not explicit linking to an already authenticated user. Returning an ID does not itself create or validate a business-domain record. Maintain that relationship in application code, and keep this synchronous hook fast and free of network I/O because identity resolution occurs inside the store's atomic operation. Disable an existing auth user through `AuthStore::set_user_enabled` when later login/session restoration must be denied.

`Boot::with_auth(config)` uses the default registration policy. To install a custom policy, construct `Auth::new(config, pool).await?`, call `.with_registration_policy(Arc::new(policy))`, then register that same `Arc<Auth>` through `Boot::with_container`. Boot mounts the final registered service's routers, so HTTP and direct calls must not be wired to separate instances. The screenless example intentionally uses the default open-registration behavior; do not treat it as an invite-only deployment recipe.

Credential changes and explicit OAuth linking require recent authentication; an old but still valid session is not equivalent to fresh proof. On `reauthentication_required`, complete a new OTP/OAuth/Passkey authentication ceremony before restarting the operation. This does not provide an application-specific authorization policy for business resources.

## HTTP contract and CSRF

The default mount is `/auth`; `config.mount_path` changes it and `config.http_enabled = false` disables HTTP while retaining Rust APIs. `gluon routes` only scans application routes and does not enumerate these runtime routes. Unconfigured methods are not mounted.

| Method | Relative path | Purpose |
| --- | --- | --- |
| GET | `/csrf` | Get `{ "token": "..." }` and session Cookie |
| POST | `/email-otp/request` | `{ "email": "..." }`, returns opaque `challenge_id` |
| POST | `/email-otp/verify` | `{ "challenge_id": "...", "code": "..." }`, establishes session |
| GET | `/session` | Current principal, including explicit anonymous state |
| POST | `/logout` | Destroy server-side session |
| POST | `/token` | Issue external API JWT from authenticated session |
| GET | `/jwks` | Public signing keys, no session Cookie |
| POST | `/passkeys/registration/start`, `/passkeys/registration/finish` | Register for current principal |
| POST | `/passkeys/authentication/start`, `/passkeys/authentication/finish` | Authenticate |
| GET | `/passkeys` | List own credentials |
| DELETE | `/passkeys/{id}` | Remove own credential subject to policy |
| POST | `/oauth/{provider}/start`, `/oauth/{provider}/link` | Start sign-in or explicit linking |
| GET | `/oauth/{provider}/callback` | Provider callback with browser-bound state |

Keep Cookies in the client. First fetch `/auth/csrf`, then send its token as `x-csrf-token` on every unsafe request, including logout and token issuance. A JWT or Authorization header never bypasses CSRF. Login rotates the session and discards the old CSRF state: fetch a new `/auth/csrf` immediately after login. Callback GET uses single-use state and browser binding instead of a normal POST CSRF token. Auth responses are `no-store`; JWKS uses finite caching and does not create sessions. Configure explicit origins, never credentialed wildcard CORS.

OTP defaults are eight digits, five minutes, five attempts and a 60-second resend interval. Preserve leading zeros. Delivery acceptance is not proof a user exists or that login succeeded. Resends invalidate old challenges. An incorrect OTP can be retried on the same challenge while attempts remain and the challenge has not expired. Expired, consumed or exhausted challenges require a fresh start; failed OAuth and Passkey ceremonies also require a fresh start. Email OTP is not strong MFA.

## OAuth and Passkeys

Configure trusted Google/GitHub providers or a generic OIDC provider through `config.oauth`; custom integrations implement `OAuthProvider`. For example, before passing configuration to Boot:

```rust,ignore
use gluon::auth::oauth::{GitHubProvider, OAuthClientConfig, ReqwestOAuthClient};
let transport = Arc::new(ReqwestOAuthClient::new(
    std::time::Duration::from_secs(10), 1024 * 1024,
));
let client = OAuthClientConfig {
    client_id, client_secret,
    callback_url: "https://app.example.test/auth/oauth/github/callback".into(),
    clock: config.clock.clone(),
};
config.oauth.push(Arc::new(GitHubProvider::new(client, transport)?));
```

For a private-CA OIDC issuer, opt in to extra roots with a PEM bundle; these are added to platform roots:

```rust,ignore
use std::sync::Arc;
use gluon::auth::oauth::{OAuthClientConfig, OidcProvider, ReqwestOAuthClient};

let ca_pem = std::fs::read("/run/secrets/idp-ca.pem")?;
let transport = Arc::new(ReqwestOAuthClient::new_with_root_certificates(
    std::time::Duration::from_secs(10),
    1024 * 1024,
    &ca_pem,
)?);
let client = OAuthClientConfig {
    client_id,
    client_secret,
    callback_url: "https://app.example.test/auth/oauth/enterprise/callback".into(),
    clock: config.clock.clone(),
};
let provider = OidcProvider::discover(
    "enterprise",
    "https://idp.example.test",
    client,
    transport,
)
.await?;
config.oauth.push(Arc::new(provider));

```

The PEM bundle is used for OIDC discovery and all later token, userinfo, and JWKS requests through that transport. HTTPS-only endpoint checks remain enabled.

`GoogleProvider::new(client, transport).await` performs Google OIDC discovery. `OidcProvider::discover("enterprise", trusted_issuer, client, transport).await` adds another trusted issuer. Give each provider its own registered callback URL and credentials; register the exact origin/path with the provider. These discovery constructors require network access at configuration time. The transport bounds response size/deadlines and refuses redirects. No real provider is contacted by the example smoke test.

Custom `OAuthProvider` implementations should implement `callback_url()` to expose their exact trusted HTTPS callback to the shared flow service, in addition to constructing authorization URLs and exchanging codes. Do not derive callbacks from untrusted request hosts or forwarded headers. Keep the token exchange callback identical to the registered authorization callback.

Server configuration, not requests, supplies issuer, client credentials and callbacks. Google identity is issuer plus stable `sub`, GitHub identity uses stable user ID. Matching emails never auto-link accounts. Link only from an existing authenticated session. Provider access/refresh tokens are not gluon JWTs. Authorization URLs are returned as data; do not relax the local-only `gluon::Redirect` restriction.

Passkeys require `PasskeyConfig { rp_id, origin }`, a valid secure browser context (localhost development is browser-dependent), and a frontend calling `navigator.credentials.create/get`. Start returns opaque `flow_id` and public `options`. Convert binary challenge/user/credential fields from base64url JSON to browser buffers and encode response buffers back to base64url JSON for finish, passing `flow_id` and `response`. Do not send server verification state to the browser. No browser SDK or UI is provided here. Initial registration requires an existing OTP/OAuth login. Discoverable/conditional authentication additionally requires compatible discoverable credentials and browser support; it is not a promise that every credential supports username-less login. Real browser/OS authenticator and origin testing remains necessary even when software-authenticator tests pass.

## JWT lifetime and operations

JWT is a short-lived external API credential issued only from a valid Cookie session, not a session replacement. Verify EdDSA signature, configured issuer/audience, required claims, expiration and known `kid`. Do not use unverified decode for authorization. `Auth::verify_token` provides configured verification; external consumers use public JWKS. JWKS contains no private material. Never interpret `kid` as a path or URL.

Logout destroys the Cookie session and blocks new JWT issuance. Already-issued JWTs verified offline remain valid until `exp` (with verifier clock tolerance). Logout is not immediate JWT revocation. Immediate revocation requires an application-specific online check/denylist, which this contract does not provide. No long-lived refresh token is issued.

Rotate to a new active `kid` and retain old verification keys for at least the maximum issued TTL plus clock skew; account for JWKS caches. The example only loads one key, so production rotation needs application configuration and consumer coordination. Keep Cookie, OTP and JWT secrets separate. External SMTP, real Google/GitHub and browser WebAuthn need their own end-to-end checks; local fixtures alone do not establish production interoperability.

## Development checks

Run `taplo fmt --check --option reorder_keys=true Cargo.toml`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --all-features -- -D warnings` before submitting authentication changes. The subprocess smoke check is `cargo test --manifest-path examples/Cargo.toml -p sample-auth --test process --locked`; PostgreSQL contracts additionally require Docker with published ports reachable from the test process.
