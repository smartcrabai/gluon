//! Plan §5.4: real signed OIDC responses through the public provider adapter.
//! HTTP is a deterministic fixture here, not an OIDC verification-success mock.
//! Real network transport safety is covered by auth_oauth_transport.rs.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod common;

use async_trait::async_trait;
use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use gluon::auth::{
    AuthError, Clock, OAuthExchange, OAuthProvider,
    oauth::{GitHubProvider, GoogleProvider, OAuthClientConfig, OAuthHttpClient, OidcProvider},
};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

const KEY: &[u8] = include_bytes!("common/auth/oidc_test_key.pem");
const ROTATED_KEY: &[u8] = include_bytes!("common/auth/oidc_rotated_test_key.pem");
const ROTATED_MODULUS: &str = "uj2h/Tr+OEQHPCtQcqIvabRbnOLVKleLx6vRsbsaY+eVYe+9fsgo5fD+1jmlRD4PnXpK8rffKo3A07TCn1tR1iHKZZ4Nb/KwUlLx3ikWAkRBtKjLpd4sqAJvMjHKROvdGBr36LRk5E0aeFljh5OX75QETwUho9j3u5ci5m0lnqIf7TFA/YfBIsJ2aVnSZDDnIYiAqdPktsbkcXi8RXcph6RxzTCcApISg495k2isOLyW+NUgl+evzuuySc6yOygpYtxeEgZx2OMDWLF6o6MO0j2nvBpLplh3C71lOwI2dgps1cnjXlLepwZtexbzPk2h9Oy3SfEMP4W66SlvCljH5Q==";
const MODULUS: &str = "sA0ifjIJAoYsWuSYYPzIK6rScwwWhgZydCcYCl9H+nR7v/SkKtppfNYWUmgyrrcoGxADhbleMy8iPSEO3tWIH8JxWZ0h1mmp3NmozIGmLXcy1DGnh6nO3EnEnn+1jdN26B6RtraYDcWCIk8OeCHelQviXXhYdZiQb3QZoJall2zBbARKcyUu/+hcjANNRfchfs82U68QYQWPpoU9tGXJOXfthnrq/4L2bRTvrTeZ7hXpjnIZEwF5AI9CG6n8c84WfyRA2z4c0aQtVHb3bHNiHR2sELjAGbajvOY7aH5dJqo3dTInkRDp+hV76+8CGtR3c089iWfTxw1HzznegPgYAw==";

fn client_config() -> OAuthClientConfig {
    OAuthClientConfig {
        client_id: "fixture-client".into(),
        client_secret: "fixture-secret".into(),
        callback_url: "https://app.example.test/auth/oauth/fixture/callback".into(),
        clock: Arc::new(common::TestClock::new()),
    }
}
fn claims(issuer: &str) -> Value {
    let now = common::TestClock::new().now().unix_timestamp();
    json!({"iss":issuer, "aud":"fixture-client", "sub":"stable-subject", "iat":now, "exp":now+300, "nonce":"expected-nonce", "email":"same@example.test", "email_verified":true})
}
fn sign(claims: &Value) -> String {
    sign_with(claims, KEY, "fixture-rsa")
}
fn sign_with(claims: &Value, key: &[u8], kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    encode(&header, claims, &EncodingKey::from_rsa_pem(key).unwrap()).unwrap()
}
fn jwks(modulus: &str, kid: &str) -> Value {
    json!({"keys":[{"kty":"RSA", "kid":kid, "use":"sig", "alg":"RS256",
        "n":URL_SAFE_NO_PAD.encode(STANDARD.decode(modulus).unwrap()), "e":"AQAB"}]})
}
fn exchange() -> OAuthExchange {
    OAuthExchange {
        code: "authorization-code".into(),
        pkce_verifier: "server-only-verifier".into(),
        nonce: "expected-nonce".into(),
        callback: client_config().callback_url,
    }
}

struct Idp {
    issuer: String,
    discovery_issuer: Mutex<String>,
    token: Mutex<String>,
    jwks: Mutex<Value>,
    fail_jwks: Mutex<bool>,
    github_user: Mutex<Value>,
    requests: Mutex<Vec<(String, Vec<u8>)>>,
}
impl Idp {
    fn new(issuer: &str, token_claims: &Value) -> Self {
        Self {
            issuer: issuer.into(),
            discovery_issuer: Mutex::new(issuer.into()),
            token: Mutex::new(sign(token_claims)),
            jwks: Mutex::new(jwks(MODULUS, "fixture-rsa")),
            fail_jwks: Mutex::new(false),
            github_user: Mutex::new(
                json!({"id":123456, "login":"initial-name", "email":"same@example.test"}),
            ),
            requests: Mutex::new(vec![]),
        }
    }

    fn request_count(&self, suffix: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(url, _)| url.ends_with(suffix))
            .count()
    }

    fn rotate(&self, token_claims: &Value, kid: &str) {
        *self.jwks.lock().unwrap() = jwks(ROTATED_MODULUS, kid);
        *self.token.lock().unwrap() = sign_with(token_claims, ROTATED_KEY, kid);
    }
}
#[async_trait]
impl OAuthHttpClient for Idp {
    async fn execute(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, AuthError> {
        let url = request.uri().to_string();
        self.requests
            .lock()
            .unwrap()
            .push((url.clone(), request.body().clone()));
        let body = if url.ends_with("/.well-known/openid-configuration") {
            json!({"issuer":self.discovery_issuer.lock().unwrap().clone(),
                "authorization_endpoint":format!("{}/authorize", self.issuer),
                "token_endpoint":format!("{}/token", self.issuer),
                "jwks_uri":format!("{}/jwks", self.issuer),
                "response_types_supported":["code"], "subject_types_supported":["public"],
                "id_token_signing_alg_values_supported":["RS256"],
                "token_endpoint_auth_methods_supported":["client_secret_basic", "client_secret_post"],
                "code_challenge_methods_supported":["S256"]})
        } else if url.ends_with("/jwks") || url.ends_with("/certs") {
            if *self.fail_jwks.lock().unwrap() {
                return Err(AuthError {
                    code: "fixture_jwks_unavailable",
                });
            }
            self.jwks.lock().unwrap().clone()
        } else if url.ends_with("/token") || url.ends_with("/access_token") {
            assert_eq!(request.method(), http::Method::POST);
            json!({"access_token":"fixture-provider-access-token", "token_type":"Bearer", "expires_in":300,
                "id_token":self.token.lock().unwrap().clone()})
        } else if url == "https://api.github.com/user" {
            assert_eq!(
                request.headers()["authorization"]
                    .to_str()
                    .unwrap()
                    .split_whitespace()
                    .nth(1),
                Some("fixture-provider-access-token")
            );
            self.github_user.lock().unwrap().clone()
        } else if url.ends_with("/user/emails") {
            json!([{"email":"same@example.test", "primary":true, "verified":true}])
        } else {
            return Err(AuthError {
                code: "unexpected_fixture_endpoint",
            });
        };
        Ok(http::Response::builder()
            .status(200)
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&body).unwrap())
            .unwrap())
    }
}

async fn oidc(idp: Arc<Idp>) -> OidcProvider {
    OidcProvider::discover("fixture", &idp.issuer.clone(), client_config(), idp)
        .await
        .unwrap()
}

async fn rotation_fixture(
    google: bool,
) -> (Arc<Idp>, Arc<common::TestClock>, Box<dyn OAuthProvider>) {
    let issuer = if google {
        "https://accounts.google.com"
    } else {
        "https://idp.example.test"
    };
    let idp = Arc::new(Idp::new(issuer, &claims(issuer)));
    let clock = Arc::new(common::TestClock::new());
    let mut config = client_config();
    config.clock = clock.clone();
    let provider: Box<dyn OAuthProvider> = if google {
        Box::new(GoogleProvider::new(config, idp.clone()).await.unwrap())
    } else {
        Box::new(
            OidcProvider::discover("fixture", issuer, config, idp.clone())
                .await
                .unwrap(),
        )
    };
    (idp, clock, provider)
}

fn current_claims(idp: &Idp, clock: &common::TestClock) -> Value {
    let mut c = claims(&idp.issuer);
    let now = clock.now().unix_timestamp();
    c["iat"] = json!(now);
    c["exp"] = json!(now + 300);
    c
}

#[tokio::test]
async fn signing_key_rotation_refreshes_unknown_kid_and_shares_updated_cache() {
    for google in [false, true] {
        let (idp, clock, provider) = rotation_fixture(google).await;
        assert_eq!(
            provider.exchange(exchange()).await.unwrap().subject,
            "stable-subject"
        );
        assert_eq!(idp.request_count("/jwks"), 1);
        assert_eq!(idp.request_count("/token"), 1);

        // The same provider instance must accept an independently generated K2.
        idp.rotate(&current_claims(&idp, &clock), "rotated-rsa");
        assert_eq!(
            provider.exchange(exchange()).await.unwrap().subject,
            "stable-subject"
        );
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 2);
        assert_eq!(idp.request_count("/.well-known/openid-configuration"), 1);

        assert!(provider.exchange(exchange()).await.is_ok());
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 3);
    }
}

#[tokio::test]
async fn signing_key_cache_expires_at_fixed_injected_clock_deadline() {
    for google in [false, true] {
        let (idp, clock, provider) = rotation_fixture(google).await;
        assert!(provider.exchange(exchange()).await.is_ok());
        clock.advance(time::Duration::seconds(299));
        *idp.token.lock().unwrap() = sign(&current_claims(&idp, &clock));
        assert!(provider.exchange(exchange()).await.is_ok());
        assert_eq!(idp.request_count("/jwks"), 1);

        // Reuse the known kid so only expiry, not an unknown kid, triggers fetch.
        clock.advance(time::Duration::seconds(1));
        idp.rotate(&current_claims(&idp, &clock), "fixture-rsa");
        assert!(provider.exchange(exchange()).await.is_ok());
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 3);

        clock.advance(time::Duration::seconds(299));
        assert!(provider.exchange(exchange()).await.is_ok());
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 4);
    }
}

#[tokio::test]
async fn signing_key_refresh_failure_rejects_without_stale_fallback_or_code_retry() {
    for google in [false, true] {
        for expired in [false, true] {
            let (idp, clock, provider) = rotation_fixture(google).await;
            assert!(provider.exchange(exchange()).await.is_ok());
            if expired {
                clock.advance(time::Duration::minutes(5));
                // K1 is still a valid signature, but an expired cache must not be
                // used when its refresh fails. Keep the token itself unexpired.
                *idp.token.lock().unwrap() = sign(&current_claims(&idp, &clock));
            } else {
                idp.rotate(&current_claims(&idp, &clock), "rotated-rsa");
            }
            *idp.fail_jwks.lock().unwrap() = true;
            assert!(provider.exchange(exchange()).await.is_err());
            assert_eq!(idp.request_count("/jwks"), 2);
            assert_eq!(idp.request_count("/token"), 2);

            // A failed refresh does not mark the cache fresh or poison recovery.
            *idp.fail_jwks.lock().unwrap() = false;
            assert!(provider.exchange(exchange()).await.is_ok());
            assert_eq!(idp.request_count("/jwks"), 3);
            assert_eq!(idp.request_count("/token"), 3);
        }
    }
}

#[tokio::test]
async fn signing_key_refresh_still_rejects_forged_signature() {
    for google in [false, true] {
        let (idp, clock, provider) = rotation_fixture(google).await;
        assert!(provider.exchange(exchange()).await.is_ok());
        let c = current_claims(&idp, &clock);
        idp.rotate(&c, "rotated-rsa");
        // Claim K2 in the header, but sign with K1. A successful JWKS fetch is
        // not itself proof of the received token's authenticity.
        *idp.token.lock().unwrap() = sign_with(&c, KEY, "rotated-rsa");
        assert!(provider.exchange(exchange()).await.is_err());
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 2);
    }
}

#[tokio::test]
async fn signing_key_still_missing_after_refresh_rejects_without_second_retry() {
    for google in [false, true] {
        let (idp, clock, provider) = rotation_fixture(google).await;
        assert!(provider.exchange(exchange()).await.is_ok());
        *idp.token.lock().unwrap() =
            sign_with(&current_claims(&idp, &clock), ROTATED_KEY, "rotated-rsa");
        // Trusted JWKS still only publishes K1 after refresh.
        assert!(provider.exchange(exchange()).await.is_err());
        assert_eq!(idp.request_count("/jwks"), 2);
        assert_eq!(idp.request_count("/token"), 2);
    }
}

#[tokio::test]
async fn signing_key_refresh_preserves_all_identity_and_token_claim_checks() {
    for google in [false, true] {
        for (claim, value) in [
            ("iss", json!("https://attacker.example.test")),
            ("aud", json!("another-client")),
            ("nonce", json!("attacker-nonce")),
            ("exp", json!(1)),
            ("iat", json!(1_800_000_001)),
            ("sub", json!("")),
            ("at_hash", json!("incorrect-access-token-hash")),
        ] {
            let (idp, clock, provider) = rotation_fixture(google).await;
            assert!(provider.exchange(exchange()).await.is_ok());
            let mut c = current_claims(&idp, &clock);
            c[claim] = value;
            idp.rotate(&c, "rotated-rsa");
            assert!(
                provider.exchange(exchange()).await.is_err(),
                "accepted {claim}"
            );
            assert_eq!(idp.request_count("/jwks"), 2);
            assert_eq!(idp.request_count("/token"), 2);
        }
    }
}

#[tokio::test]
async fn generic_oidc_discovers_trusted_issuer_and_resolves_verified_subject() {
    let idp = Arc::new(Idp::new(
        "https://idp.example.test",
        &claims("https://idp.example.test"),
    ));
    let provider = oidc(idp.clone()).await;
    let identity = provider.exchange(exchange()).await.unwrap();
    assert_eq!(identity.issuer, "https://idp.example.test");
    assert_eq!(identity.subject, "stable-subject");
    let requests = idp.requests.lock().unwrap();
    let token = requests
        .iter()
        .find(|(url, _)| url.ends_with("/token"))
        .unwrap();
    let form: HashMap<_, _> = form_urlencoded::parse(&token.1).into_owned().collect();
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "authorization-code");
    assert_eq!(form["code_verifier"], "server-only-verifier");
    assert_eq!(form["redirect_uri"], exchange().callback);
}

#[tokio::test]
async fn discovery_cannot_substitute_a_different_issuer() {
    let idp = Arc::new(Idp::new(
        "https://idp.example.test",
        &claims("https://idp.example.test"),
    ));
    *idp.discovery_issuer.lock().unwrap() = "https://attacker.example.test".into();
    assert!(
        OidcProvider::discover("fixture", "https://idp.example.test", client_config(), idp)
            .await
            .is_err()
    );
}

macro_rules! invalid_oidc_claim {
    ($name:ident, $claim:literal, $value:expr) => {
        #[tokio::test]
        async fn $name() {
            let mut c = claims("https://idp.example.test");
            c[$claim] = json!($value);
            let provider = oidc(Arc::new(Idp::new("https://idp.example.test", &c))).await;
            assert!(provider.exchange(exchange()).await.is_err());
        }
    };
}
invalid_oidc_claim!(oidc_rejects_wrong_nonce, "nonce", "attacker-nonce");
invalid_oidc_claim!(
    oidc_rejects_wrong_issuer,
    "iss",
    "https://attacker.example.test"
);
invalid_oidc_claim!(oidc_rejects_wrong_audience, "aud", "another-client");
invalid_oidc_claim!(oidc_rejects_expired_id_token, "exp", 1);

macro_rules! invalid_google_claim {
    ($name:ident, $claim:literal, $value:expr) => {
        #[tokio::test]
        async fn $name() {
            let mut c = claims("https://accounts.google.com");
            c[$claim] = json!($value);
            let idp = Arc::new(Idp::new("https://accounts.google.com", &c));
            let provider = GoogleProvider::new(client_config(), idp).await.unwrap();
            assert!(provider.exchange(exchange()).await.is_err());
        }
    };
}
invalid_google_claim!(google_rejects_wrong_nonce, "nonce", "attacker-nonce");
invalid_google_claim!(
    google_rejects_wrong_issuer,
    "iss",
    "https://attacker.example.test"
);
invalid_google_claim!(google_rejects_wrong_audience, "aud", "another-client");
invalid_google_claim!(google_rejects_expired_id_token, "exp", 1);

#[tokio::test]
async fn google_rejects_invalid_id_token_signature() {
    let idp = Arc::new(Idp::new(
        "https://accounts.google.com",
        &claims("https://accounts.google.com"),
    ));
    let token = idp.token.lock().unwrap().clone();
    let mut parts: Vec<String> = token.split('.').map(str::to_owned).collect();
    let mut forged = claims("https://accounts.google.com");
    forged["sub"] = json!("attacker-subject");
    parts[1] = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged).unwrap());
    *idp.token.lock().unwrap() = parts.join(".");
    let provider = GoogleProvider::new(client_config(), idp).await.unwrap();
    assert!(provider.exchange(exchange()).await.is_err());
}

#[tokio::test]
async fn oidc_rejects_modified_signature() {
    let idp = Arc::new(Idp::new(
        "https://idp.example.test",
        &claims("https://idp.example.test"),
    ));
    let valid = idp.token.lock().unwrap().clone();
    let mut parts: Vec<String> = valid.split('.').map(str::to_owned).collect();
    let mut forged = claims("https://idp.example.test");
    forged["sub"] = json!("attacker-subject");
    parts[1] = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged).unwrap());
    *idp.token.lock().unwrap() = parts.join(".");
    assert!(oidc(idp).await.exchange(exchange()).await.is_err());
}

#[tokio::test]
async fn oidc_requires_a_stable_subject() {
    let mut c = claims("https://idp.example.test");
    c.as_object_mut().unwrap().remove("sub");
    assert!(
        oidc(Arc::new(Idp::new("https://idp.example.test", &c)))
            .await
            .exchange(exchange())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn google_authorization_uses_oidc_code_flow_and_pkce() {
    let idp = Arc::new(Idp::new(
        "https://accounts.google.com",
        &claims("https://accounts.google.com"),
    ));
    let provider = GoogleProvider::new(client_config(), idp.clone())
        .await
        .unwrap();
    let url = reqwest::Url::parse(&provider.authorization_url(
        "opaque-state",
        "s256-challenge",
        "expected-nonce",
    ))
    .unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("accounts.google.com"));
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["nonce"], "expected-nonce");
    assert!(
        query["scope"]
            .split_whitespace()
            .any(|scope| scope == "openid")
    );
    assert!(!url.as_str().contains("fixture-secret"));
}

#[tokio::test]
async fn google_identity_uses_subject_not_email() {
    let idp = Arc::new(Idp::new(
        "https://accounts.google.com",
        &claims("https://accounts.google.com"),
    ));
    let provider = GoogleProvider::new(client_config(), idp.clone())
        .await
        .unwrap();
    let first = provider.exchange(exchange()).await.unwrap();
    let mut c = claims("https://accounts.google.com");
    c["email"] = json!("renamed@example.test");
    *idp.token.lock().unwrap() = sign(&c);
    let second = provider.exchange(exchange()).await.unwrap();
    assert_eq!(first.subject, "stable-subject");
    assert_eq!(first.subject, second.subject);
    assert_eq!(first.issuer, second.issuer);
}

#[tokio::test]
async fn github_authorization_uses_configured_callback_state_and_pkce() {
    let idp = Arc::new(Idp::new(
        "https://github.com",
        &claims("https://github.com"),
    ));
    let provider = GitHubProvider::new(client_config(), idp.clone()).unwrap();
    let url = reqwest::Url::parse(&provider.authorization_url(
        "opaque-state",
        "s256-challenge",
        "unused-nonce",
    ))
    .unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("github.com"));
    assert_eq!(url.path(), "/login/oauth/authorize");
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["state"], "opaque-state");
    assert_eq!(query["redirect_uri"], client_config().callback_url);
    assert!(!url.as_str().contains("fixture-secret"));
}

#[tokio::test]
async fn github_identity_survives_login_name_and_email_changes() {
    let idp = Arc::new(Idp::new(
        "https://github.com",
        &claims("https://github.com"),
    ));
    let provider = GitHubProvider::new(client_config(), idp.clone()).unwrap();
    let first = provider.exchange(exchange()).await.unwrap();
    *idp.github_user.lock().unwrap() =
        json!({"id":123456, "login":"renamed-user", "email":"renamed@example.test"});
    let second = provider.exchange(exchange()).await.unwrap();
    assert_eq!(first.subject, "123456");
    assert_eq!(first.subject, second.subject);
    assert_eq!(first.issuer, second.issuer);
}
