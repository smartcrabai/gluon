//! Plan-item → observable assertion map for the test-first authentication step.
//!
//! - §4.1, §4.3: this file rejects missing PostgreSQL, invalid mount paths,
//!   short OTP keys, inconsistent RP/origin, and absent JWT signing material.
//! - §4.2, §5.1: `auth_session.rs`, `auth_session_failures.rs`, and `auth_http.rs`
//!   cover current-user extraction, rotation/revocation, optional anonymity,
//!   session expiry, write failures, and failed server-side logout deletion.
//! - §5.2: `auth_email_otp.rs` covers delivery, verification, limits, resend,
//!   expiry, single consumption, and identity resolution.
//!   `auth_smtp.rs` checks that a real plaintext-only SMTP endpoint cannot
//!   receive credentials or OTP messages from the standard sender.
//! - §5.3: `auth_passkey.rs` exercises real authenticator registration and
//!   authentication as well as invalid and cross-browser flows.
//! - §5.4: `auth_oauth.rs` covers provider flows, identity and explicit linking.
//!   `auth_oauth_providers.rs` uses genuinely RSA-signed ID tokens for trusted
//!   OIDC discovery, nonce/issuer/audience/expiry/signature validation, and
//!   Google/GitHub stable identities. `auth_oauth_transport.rs` exercises real
//!   local HTTP redirect refusal, byte limits, and nonresponding-server timeout.
//! - §5.5: `auth_jwt.rs` verifies signature/claims, issuance gating, and rotation.
//! - §6: `auth_postgres.rs` exercises real PostgreSQL sharing, atomic consumption,
//!   rate limits, cleanup, migration isolation, and concurrent migrations.
//! - §4.3–4.4 / §10.2: `auth_boot.rs` drives Boot as a subprocess, and `auth_http.rs`
//!   drives the real router/Session/CSRF/Extractor boundaries.
//! - §3 / §10.1 compatibility: existing boot, `csrf_middleware`, `csrf_spec`,
//!   `postgres_session`, redirect, DI, and example tests remain unchanged.
//!
//! Boundaries: these tests do not claim real Google/GitHub account registration,
//! actual external SMTP delivery, a physical browser authenticator, statistical
//! RNG assurance, or constant-time cryptographic verification measurements.
//! Those are not replaced with success mocks or source-text assertions.
//!
//! Tests below do not infer configuration validity from incidental defaults.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/auth/mod.rs"]
mod common;
#[path = "common/auth/jwt.rs"]
mod keys;

use common::{Mailbox, TestClock, config};
use gluon::auth::{MemoryAuthStore, PasskeyConfig};
use gluon::prelude::{Auth, AuthBuilder, AuthConfig};
use std::sync::Arc;

fn base() -> AuthConfig {
    config(Arc::new(TestClock::new()), Arc::new(Mailbox::default()))
}

struct CallbackProvider(Option<&'static str>);
#[async_trait::async_trait]
impl gluon::auth::OAuthProvider for CallbackProvider {
    fn name(&self) -> &'static str {
        "fixture"
    }
    fn callback_url(&self) -> Option<&str> {
        self.0
    }
    fn authorization_url(&self, _: &str, _: &str, _: &str) -> String {
        "https://provider.example.test/authorize".into()
    }
    async fn exchange(
        &self,
        _: gluon::auth::OAuthExchange,
    ) -> Result<gluon::auth::OAuthIdentity, gluon::auth::AuthError> {
        Err(gluon::auth::AuthError {
            code: "unused_provider",
        })
    }
}

#[tokio::test]
async fn callbackless_provider_rejects_http_fallback_at_construction() {
    let mut cfg = base();
    cfg.allowed_origins = vec!["http://localhost:3000".into()];
    cfg.oauth.push(Arc::new(CallbackProvider(None)));
    assert!(matches!(Auth::new(cfg, None).await, Err(error) if error.code == "configuration"));
}

#[tokio::test]
async fn callbackless_provider_accepts_https_fallback_with_custom_mount() {
    let mut cfg = base();
    cfg.mount_path = "/identity".into();
    cfg.oauth.push(Arc::new(CallbackProvider(None)));
    assert!(Auth::new(cfg, None).await.is_ok());
}

#[tokio::test]
async fn explicit_oauth_callback_restrictions_remain_enforced() {
    for callback in [
        "http://localhost:3000/callback",
        "https://user@app.example.test/callback",
        "https://user:password@app.example.test/callback",
        "https://app.example.test/callback#fragment",
        "not-a-url",
    ] {
        let mut cfg = base();
        cfg.oauth.push(Arc::new(CallbackProvider(Some(callback))));
        assert!(matches!(Auth::new(cfg, None).await, Err(error) if error.code == "configuration"));
    }
    let mut cfg = base();
    cfg.allowed_origins = vec!["http://localhost:3000".into()];
    cfg.oauth.push(Arc::new(CallbackProvider(Some(
        "https://app.example.test/registered/callback",
    ))));
    assert!(Auth::new(cfg, None).await.is_ok());
}

#[tokio::test]
async fn absent_database_does_not_silently_select_memory_storage() {
    let mut config = base();
    config.store = None;
    assert!(Auth::new(config, None).await.is_err());
}

#[tokio::test]
async fn explicitly_selected_memory_storage_is_usable_without_postgres() {
    assert!(Auth::new(base(), None).await.is_ok());
}

macro_rules! invalid_mount {
    ($name:ident, $path:literal) => {
        #[tokio::test]
        async fn $name() {
            let mut config = base();
            config.mount_path = $path.into();
            assert_eq!(
                Auth::new(config, None).await.err().unwrap().code,
                "configuration"
            );
        }
    };
}
invalid_mount!(relative_mount_path_is_rejected, "auth");
invalid_mount!(parameter_mount_segment_is_rejected, "/auth/:tenant");
invalid_mount!(wildcard_mount_segment_is_rejected, "/auth/*tenant");
invalid_mount!(
    external_mount_url_is_rejected,
    "https://attacker.example.test/auth"
);
invalid_mount!(
    mount_path_with_query_is_rejected,
    "/auth?redirect=elsewhere"
);

#[tokio::test]
async fn short_otp_secret_is_rejected_during_construction() {
    let mut config = base();
    config.email_otp.as_mut().unwrap().secret = b"short".to_vec();
    assert!(Auth::new(config, None).await.is_err());
}

#[tokio::test]
async fn origin_canonicalization_does_not_relax_configuration_restrictions() {
    for origin in [
        "http://app.example.test",
        "https:///",
        "https://user@app.example.test",
        "https://user:password@app.example.test",
        "https://app.example.test/path",
        "https://app.example.test/?query=1",
        "https://app.example.test/#fragment",
        "ftp://app.example.test",
    ] {
        let mut cfg = base();
        cfg.allowed_origins = vec![origin.into()];
        assert_eq!(
            Auth::new(cfg, None).await.err().unwrap().code,
            "configuration",
            "{origin}"
        );
    }
}

#[tokio::test]
async fn passkey_rp_must_match_the_configured_origin() {
    let mut config = base();
    config.passkey = Some(PasskeyConfig {
        rp_id: "example.test".into(),
        origin: "https://attacker.test".into(),
    });
    assert!(Auth::new(config, None).await.is_err());
}

#[tokio::test]
async fn jwt_configuration_without_private_keys_does_not_create_ephemeral_keys() {
    let mut config = base();
    let mut jwt = keys::jwt_config();
    jwt.keys.clear();
    config.jwt = Some(jwt);
    assert!(Auth::new(config, None).await.is_err());
}

#[tokio::test]
async fn active_jwt_kid_must_reference_a_configured_key() {
    let mut config = base();
    let mut jwt = keys::jwt_config();
    jwt.active_kid = "missing".into();
    config.jwt = Some(jwt);
    assert!(Auth::new(config, None).await.is_err());
}

async fn default_otp() -> common::Fixture {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let config = AuthBuilder::new()
        .store(Arc::new(MemoryAuthStore::new(1024)))
        .clock(clock.clone())
        .email_otp(mail.clone(), vec![0x71; 32])
        .build()
        .unwrap();
    common::Fixture::with_config(config, clock, mail).await
}

#[tokio::test]
async fn builder_default_otp_is_eight_digits() {
    let f = default_otp().await;
    f.auth
        .request_email_otp(&common::context(), "defaults@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    assert_eq!(code.len(), 8);
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
}

#[tokio::test]
async fn builder_default_otp_lifetime_is_five_minutes() {
    for (seconds, accepted) in [(299, true), (300, false)] {
        let f = default_otp().await;
        let ctx = common::context();
        let challenge = f
            .auth
            .request_email_otp(&ctx, "defaults@example.test")
            .await
            .unwrap();
        f.clock.advance(time::Duration::seconds(seconds));
        assert_eq!(
            f.auth
                .verify_email_otp(&ctx, &challenge.challenge_id, &f.mail.last().code)
                .await
                .is_ok(),
            accepted
        );
    }
}

#[tokio::test]
async fn builder_default_resend_interval_is_sixty_seconds() {
    let f = default_otp().await;
    let ctx = common::context();
    f.auth
        .request_email_otp(&ctx, "defaults@example.test")
        .await
        .unwrap();
    f.clock.advance(time::Duration::seconds(59));
    assert_eq!(
        f.auth
            .request_email_otp(&ctx, "defaults@example.test")
            .await
            .unwrap_err()
            .code,
        "rate_limited"
    );
    f.clock.advance(time::Duration::seconds(1));
    assert!(
        f.auth
            .request_email_otp(&ctx, "defaults@example.test")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn builder_default_attempt_limit_is_five() {
    for failed_attempts in [4, 5] {
        let f = default_otp().await;
        let ctx = common::context();
        let challenge = f
            .auth
            .request_email_otp(&ctx, "defaults@example.test")
            .await
            .unwrap();
        let code = f.mail.last().code;
        let wrong = if code == "00000000" {
            "00000001"
        } else {
            "00000000"
        };
        for _ in 0..failed_attempts {
            assert!(
                f.auth
                    .verify_email_otp(&ctx, &challenge.challenge_id, wrong)
                    .await
                    .is_err()
            );
        }
        assert_eq!(
            f.auth
                .verify_email_otp(&ctx, &challenge.challenge_id, &code)
                .await
                .is_ok(),
            failed_attempts < 5
        );
    }
}
