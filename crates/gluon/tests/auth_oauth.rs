#![cfg(feature = "auth")]
#[path = "common/auth/mod.rs"]
mod common;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{Fixture, Mailbox, TestClock, config, context};
use gluon::auth::{AuthContext, AuthError, AuthUser, OAuthExchange, OAuthIdentity, OAuthProvider};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use time::Duration;

#[derive(Default)]
struct Provider {
    requests: Mutex<Vec<OAuthExchange>>,
    other_requests: Mutex<Vec<OAuthExchange>>,
    exchange_gate: Mutex<Option<Arc<tokio::sync::Barrier>>>,
}
#[async_trait]
impl OAuthProvider for Provider {
    fn name(&self) -> &str {
        "fixture"
    }
    fn authorization_url(&self, state: &str, challenge: &str, nonce: &str) -> String {
        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("state", state),
                ("code_challenge", challenge),
                ("code_challenge_method", "S256"),
                ("nonce", nonce),
            ])
            .finish();
        format!("https://provider.example.test/authorize?{query}")
    }
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
        let code = request.code.clone();
        self.requests.lock().unwrap().push(request);
        let gate = self.exchange_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        if code == "exchange-fails" {
            return Err(AuthError {
                code: "provider_failed",
            });
        }
        Ok(OAuthIdentity {
            issuer: "https://provider.example.test".into(),
            subject: code,
            email: Some("same@example.test".into()),
        })
    }
}
struct OtherProvider(Arc<Provider>);
#[async_trait]
impl OAuthProvider for OtherProvider {
    fn name(&self) -> &str {
        "other-provider"
    }
    fn authorization_url(&self, state: &str, challenge: &str, nonce: &str) -> String {
        self.0.authorization_url(state, challenge, nonce)
    }
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
        let subject = request.code.clone();
        self.0.other_requests.lock().unwrap().push(request);
        Ok(OAuthIdentity {
            issuer: "https://other.example.test".into(),
            subject,
            email: None,
        })
    }
}
async fn fixture() -> (Fixture, Arc<Provider>) {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let provider = Arc::new(Provider::default());
    let mut cfg = config(clock.clone(), mail.clone());
    cfg.oauth.push(provider.clone());
    cfg.oauth.push(Arc::new(OtherProvider(provider.clone())));
    cfg.flow_ttl = Duration::seconds(1);
    cfg.idle_timeout = Duration::days(7);
    cfg.absolute_timeout = Duration::days(7);
    (Fixture::with_config(cfg, clock, mail).await, provider)
}
fn params(start: &gluon::auth::OAuthStart) -> HashMap<String, String> {
    reqwest::Url::parse(&start.authorization_url)
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect()
}
async fn state(f: &Fixture, ctx: &AuthContext, provider: &str, link: bool) -> String {
    params(&f.auth.start_oauth(ctx, provider, link).await.unwrap())["state"].clone()
}
async fn signin(f: &Fixture, provider: &str, subject: &str) -> AuthUser {
    let ctx = context();
    let state = state(f, &ctx, provider, false).await;
    let user = f
        .auth
        .finish_oauth(&ctx, provider, &state, subject)
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user.clone()));
    user
}
fn assert_no_exchange(p: &Provider) {
    assert!(p.requests.lock().unwrap().is_empty());
    assert!(p.other_requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn start_has_unique_unpredictable_state_nonce_and_challenge() {
    let (f, _) = fixture().await;
    let ctx = context();
    let a = params(&f.auth.start_oauth(&ctx, "fixture", false).await.unwrap());
    let b = params(&f.auth.start_oauth(&ctx, "fixture", false).await.unwrap());
    for key in ["state", "nonce", "code_challenge"] {
        assert!(a[key].len() >= 32, "missing or weak {key}");
        assert_ne!(a[key], b[key], "{key} reused across flows");
    }
}
#[tokio::test]
async fn pkce_is_the_s256_digest_of_the_actual_exchange_verifier() {
    let (f, p) = fixture().await;
    let ctx = context();
    let start = f.auth.start_oauth(&ctx, "fixture", false).await.unwrap();
    let query = params(&start);
    assert_eq!(query["code_challenge_method"], "S256");
    f.auth
        .finish_oauth(&ctx, "fixture", &query["state"], "subject")
        .await
        .unwrap();
    let calls = p.requests.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let verifier = &calls[0].pkce_verifier;
    assert!((43..=128).contains(&verifier.len()));
    assert!(
        verifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~".contains(&b))
    );
    assert_eq!(
        query["code_challenge"],
        URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
    );
    assert!(!start.authorization_url.contains(verifier));
}
#[tokio::test]
async fn exchange_receives_saved_nonce_and_trusted_callback() {
    let (f, p) = fixture().await;
    let ctx = context();
    let query = params(&f.auth.start_oauth(&ctx, "fixture", false).await.unwrap());
    f.auth
        .finish_oauth(&ctx, "fixture", &query["state"], "subject")
        .await
        .unwrap();
    let calls = p.requests.lock().unwrap();
    assert_eq!(calls[0].nonce, query["nonce"]);
    assert_eq!(calls[0].code, "subject");
    assert_eq!(
        calls[0].callback,
        "https://app.example.test/auth/oauth/fixture/callback"
    );
}
#[tokio::test]
async fn callbackless_provider_uses_first_https_origin_and_custom_mount_for_exchange() {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let provider = Arc::new(Provider::default());
    let mut cfg = config(clock.clone(), mail.clone());
    cfg.mount_path = "/identity".into();
    cfg.allowed_origins = vec!["https://login.example.test/".into(), "https://other.example.test".into()];
    cfg.oauth.push(provider.clone());
    let f = Fixture::with_config(cfg, clock, mail).await;
    let ctx = context();
    let start = f.auth.start_oauth(&ctx, "fixture", false).await.unwrap();
    f.auth.finish_oauth(&ctx, "fixture", &start.flow_id, "subject").await.unwrap();
    let calls = provider.requests.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].callback, "https://login.example.test/identity/oauth/fixture/callback");
}

#[tokio::test]
async fn wrong_browser_binding_rejects_before_exchange() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = state(&f, &ctx, "fixture", false).await;
    let mut caller = ctx;
    caller.browser_binding = "other-browser".into();
    assert!(
        f.auth
            .finish_oauth(&caller, "fixture", &state, "subject")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn wrong_configured_provider_rejects_before_exchange() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = state(&f, &ctx, "fixture", false).await;
    assert!(
        f.auth
            .finish_oauth(&ctx, "other-provider", &state, "subject")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn tampered_state_rejects_before_exchange() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = format!("{}tampered", state(&f, &ctx, "fixture", false).await);
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "subject")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn expired_flow_rejects_before_exchange_while_session_remains_valid() {
    let (f, p) = fixture().await;
    let ctx = context();
    let user = f.login(&ctx, "owner@example.test").await;
    let state = state(&f, &ctx, "fixture", false).await;
    f.clock.advance(Duration::seconds(2));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "subject")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn successful_callback_cannot_be_exchanged_twice() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = state(&f, &ctx, "fixture", false).await;
    let user = f
        .auth
        .finish_oauth(&ctx, "fixture", &state, "subject")
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "subject")
            .await
            .is_err()
    );
    assert_eq!(p.requests.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn failed_exchange_still_consumes_flow_without_login() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = state(&f, &ctx, "fixture", false).await;
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "exchange-fails")
            .await
            .is_err()
    );
    assert_eq!(p.requests.lock().unwrap().len(), 1);
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "subject")
            .await
            .is_err()
    );
    assert_eq!(p.requests.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn concurrent_callbacks_exchange_and_login_only_once() {
    let (f, p) = fixture().await;
    let ctx = context();
    let state = state(&f, &ctx, "fixture", false).await;
    let (a, b) = tokio::join!(
        f.auth.finish_oauth(&ctx, "fixture", &state, "subject"),
        f.auth.finish_oauth(&ctx, "fixture", &state, "subject")
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(p.requests.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn concurrent_first_logins_for_same_identity_resolve_one_user() {
    let (f, p) = fixture().await;
    *p.exchange_gate.lock().unwrap() = Some(Arc::new(tokio::sync::Barrier::new(2)));
    let a = context();
    let mut b = context();
    b.browser_binding = "browser-b".into();
    let first = state(&f, &a, "fixture", false).await;
    let second = state(&f, &b, "fixture", false).await;
    let (a_user, b_user) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            f.auth.finish_oauth(&a, "fixture", &first, "new-subject"),
            f.auth.finish_oauth(&b, "fixture", &second, "new-subject")
        )
    })
    .await
    .expect("independent OAuth exchanges must progress concurrently");
    let user = a_user.unwrap();
    assert_eq!(user, b_user.unwrap());
    assert_eq!(f.auth.current_user(&a).await.unwrap(), Some(user.clone()));
    assert_eq!(f.auth.current_user(&b).await.unwrap(), Some(user));
    assert_eq!(p.requests.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn stable_subject_resolves_same_user_on_later_login() {
    let (f, _) = fixture().await;
    assert_eq!(
        signin(&f, "fixture", "subject").await,
        signin(&f, "fixture", "subject").await
    );
}
#[tokio::test]
async fn matching_provider_emails_do_not_link_distinct_subjects() {
    let (f, _) = fixture().await;
    assert_ne!(
        signin(&f, "fixture", "a").await,
        signin(&f, "fixture", "b").await
    );
}
#[tokio::test]
async fn matching_email_otp_identity_is_not_automatically_linked() {
    let (f, _) = fixture().await;
    let email_user = f.login(&context(), "same@example.test").await;
    assert_ne!(email_user, signin(&f, "fixture", "subject").await);
}
#[tokio::test]
async fn identical_subjects_from_distinct_issuers_remain_distinct() {
    let (f, _) = fixture().await;
    assert_ne!(
        signin(&f, "fixture", "subject").await,
        signin(&f, "other-provider", "subject").await
    );
}
#[tokio::test]
async fn anonymous_explicit_link_start_rejects() {
    let (f, p) = fixture().await;
    assert!(
        f.auth
            .start_oauth(&context(), "fixture", true)
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn explicit_link_adds_identity_to_authenticated_owner() {
    let (f, _) = fixture().await;
    let ctx = context();
    let owner = f.login(&ctx, "owner@example.test").await;
    let state = state(&f, &ctx, "fixture", true).await;
    assert_eq!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "linked")
            .await
            .unwrap(),
        owner
    );
    assert_eq!(signin(&f, "fixture", "linked").await, owner);
}
#[tokio::test]
async fn explicit_link_rejects_identity_already_owned_by_another_user() {
    let (f, _) = fixture().await;
    let original = signin(&f, "fixture", "linked").await;
    let ctx = context();
    let other = f.login(&ctx, "other@example.test").await;
    let state = state(&f, &ctx, "fixture", true).await;
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "linked")
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(other));
    assert_eq!(signin(&f, "fixture", "linked").await, original);
}
#[tokio::test]
async fn link_flow_rejects_changed_authenticated_target_before_exchange() {
    let (f, p) = fixture().await;
    let ctx = context();
    f.login(&ctx, "owner@example.test").await;
    let state = state(&f, &ctx, "fixture", true).await;
    f.auth.logout(&ctx).await.unwrap();
    f.login(&ctx, "replacement@example.test").await;
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "linked")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
#[tokio::test]
async fn link_flow_rejects_lost_authenticated_session_before_exchange() {
    let (f, p) = fixture().await;
    let ctx = context();
    f.login(&ctx, "owner@example.test").await;
    let state = state(&f, &ctx, "fixture", true).await;
    f.auth.logout(&ctx).await.unwrap();
    assert!(
        f.auth
            .finish_oauth(&ctx, "fixture", &state, "linked")
            .await
            .is_err()
    );
    assert_no_exchange(&p);
}
