---
name: gluon
description: Procedures for scaffolding and developing applications with the gluon CLI.
---

# gluon application

This project is a gluon application. Use the `gluon` CLI for scaffolding and code generation; keep generated marker regions intact.

## CLI

```text
gluon new <name> [--no-git] [--no-install] [--claude] [--agents]
gluon generate (g) controller <route> [--api]
gluon generate (g) resource <name>
gluon generate (g) usecase <name>
gluon generate (g) domain <name> [--field NAME:TYPE]*
gluon generate (g) dto <name>
gluon generate (g) migration <name>
gluon destroy (d) <kind> <name>
gluon db <create|drop|migrate|rollback|prepare|seed>
gluon dev | gluon build | gluon run [--release] | gluon routes
```

## Conventions

- `src/wiring.rs` and each generated layer `mod.rs` contain machine-edited marker regions. Preserve the markers.
- `gluon g domain` does not create a migration; create one separately with `gluon g migration`.
- Generated `Cargo.toml` dependencies use the GitHub repository and are pinned to the exact framework revision used to build the CLI, or to the matching `v<version>` tag when Git metadata is unavailable. For local framework development, rewrite them to local paths.
- In zsh, quote routes containing bracket segments, for example `gluon g controller 'users/[id]'`.

## Authentication (opt-in)

- Enable gluon's `auth` feature, create configuration with `gluon::auth::AuthBuilder`, and pass it to `Boot::with_auth`. HTTP and direct Rust APIs share the same service in DI.
- PostgreSQL is the default and requires `DATABASE_URL` and an applied auth schema. Opt into migrations explicitly with `config.migrate_on_start` or `PostgresAuthStore::migrate()`. Do not mix auth migrations into application `_sqlx_migrations`.
- Use `MemoryAuthStore` only through explicit development/test configuration. Never silently fall back to it on missing/failed PostgreSQL.
- Supply `EmailSender` or TLS `SmtpEmailSender` and a separate high-entropy OTP HMAC secret. Never return or log OTPs. Keep OTP, Cookie and JWT keys independent.
- Default HTTP routes live under `/auth`, configurable with `mount_path`; `http_enabled = false` keeps only Rust APIs. `gluon routes` does not enumerate runtime auth routes.
- Keep session Cookies and fetch `GET /auth/csrf`; send `x-csrf-token` on every unsafe auth request. Fetch a new token after login/session rotation. Bearer JWTs never bypass CSRF.
- Use `CurrentUser` for required Cookie authentication or `Option<CurrentUser>` for anonymous access. Optional authentication must not hide backend failures. Never trust a client user ID as the principal.
- JWTs are short-lived external API credentials issued from valid sessions, not Cookie replacements. Explicitly configure Ed25519 PEM keys, `kid`, issuer and audience. Logout prevents new issuance but does not revoke already-issued offline JWTs before expiration. Publish only public JWKS and retain old keys through TTL plus clock skew/cache overlap.
- Default first-time OTP/OAuth signup provisions an auth user. For invite-only signup or business-owned IDs, install a synchronous `RegistrationPolicy` with `Auth::with_registration_policy` before registering the shared `Arc<Auth>` via `Boot::with_container`. The hook is not run for existing identities or explicit linking and must not perform network I/O inside atomic identity resolution. A returned auth ID does not validate a business-domain record.
- OAuth providers use trusted server configuration and explicit linking, never automatic email-based account merging. Keep `Redirect` local-only.
- Passkeys require RP ID/origin configuration and application browser code calling `navigator.credentials.create/get` with base64url/binary conversion. No login UI or frontend SDK is generated. Conditional/discoverable login needs compatible browser and credentials, not just a missing username.
- In the gluon framework source repository, see `docs/auth.md` and the screenless `examples/auth` for environment configuration. These files are not included in generated applications. Its `AUTH_STORE=memory-development` requires explicit insecure local Cookies, while SMTP remains TLS-only. Real SMTP, OAuth providers and browser authenticators require independent interoperability checks.

## Verification

After generation, run `gluon routes` and `cargo check`. For runtime checks, use `GLUON_INSECURE_COOKIE=1 cargo run` locally.
