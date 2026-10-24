//! Plan §5.5 / §10.1: session-gated JWT issuance, strict verification, and public-key rotation.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/auth/mod.rs"]
mod common;
#[path = "common/auth/jwt.rs"]
mod keys;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{Fixture, Mailbox, TestClock, config, context};
use gluon::auth::{Auth, Clock, JwtKey};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
    jwk::JwkSet,
};
use serde_json::{Value, json};
use std::sync::Arc;
use time::Duration;

async fn fixture() -> Fixture {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut config = config(clock.clone(), mail.clone());
    config.jwt = Some(keys::jwt_config());
    Fixture::with_config(config, clock, mail).await
}

fn valid_claims(f: &Fixture) -> Value {
    let now = f.clock.now().unix_timestamp();
    json!({"iss":"https://auth.example.test", "aud":"external-api", "sub":"subject-1", "iat":now, "exp":now + 300, "jti":"jwt-contract-1"})
}

fn signed(claims: &Value, kid: &str) -> String {
    let mut header = Header::new(Algorithm::EdDSA);
    header.kid = Some(kid.into());
    encode(
        &header,
        claims,
        &EncodingKey::from_ed_pem(keys::OLD_KEY).unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn anonymous_session_cannot_mint_tokens() {
    let f = fixture().await;
    assert!(f.auth.token(&context()).await.is_err());
}

#[tokio::test]
async fn issued_token_is_verifiable_using_only_public_jwks_and_session_subject() {
    let f = fixture().await;
    let ctx = context();
    let user = f.login(&ctx, "jwt@example.test").await;
    let token = f.auth.token(&ctx).await.unwrap();
    let header = decode_header(&token).unwrap();
    assert_eq!(header.alg, Algorithm::EdDSA);
    let jwks: JwkSet = serde_json::from_value(f.auth.jwks()).unwrap();
    let jwk = jwks.find(header.kid.as_deref().unwrap()).unwrap();
    let mut validation = Validation::new(Algorithm::EdDSA);
    validation.set_issuer(&["https://auth.example.test"]);
    validation.set_audience(&["external-api"]);
    // The service's injected clock is independent of the verifier's wall clock.
    // Expiration is asserted below against that clock instead of wall time.
    validation.validate_exp = false;
    let decoded =
        decode::<Value>(&token, &DecodingKey::from_jwk(jwk).unwrap(), &validation).unwrap();
    assert_eq!(decoded.claims["sub"], user.id.0);
    assert_eq!(decoded.claims["iss"], "https://auth.example.test");
    assert_eq!(decoded.claims["aud"], "external-api");
    assert_eq!(decoded.claims["iat"], f.clock.now().unix_timestamp());
    assert_eq!(
        decoded.claims["exp"].as_i64().unwrap() - decoded.claims["iat"].as_i64().unwrap(),
        300
    );
    assert!(!decoded.claims["jti"].as_str().unwrap().is_empty());
    assert_eq!(f.auth.verify_token(&token).unwrap().sub, user.id.0);
}

#[tokio::test]
async fn separate_issuances_have_distinct_token_ids() {
    let f = fixture().await;
    let ctx = context();
    f.login(&ctx, "unique@example.test").await;
    let first = f
        .auth
        .verify_token(&f.auth.token(&ctx).await.unwrap())
        .unwrap();
    let second = f
        .auth
        .verify_token(&f.auth.token(&ctx).await.unwrap())
        .unwrap();
    assert_ne!(first.jti, second.jti);
}

#[tokio::test]
async fn logout_prevents_new_tokens_but_does_not_revoke_offline_tokens() {
    let f = fixture().await;
    let ctx = context();
    f.login(&ctx, "logout@example.test").await;
    let token = f.auth.token(&ctx).await.unwrap();
    f.auth.logout(&ctx).await.unwrap();
    assert!(f.auth.token(&ctx).await.is_err());
    assert!(
        f.auth.verify_token(&token).is_ok(),
        "issued tokens remain valid to exp"
    );
    f.clock.advance(Duration::minutes(5));
    assert!(f.auth.verify_token(&token).is_err());
}

#[tokio::test]
async fn modified_signed_payload_is_rejected() {
    let f = fixture().await;
    let token = signed(&valid_claims(&f), "old-key");
    assert!(
        f.auth.verify_token(&token).is_ok(),
        "control token must be valid"
    );
    let parts: Vec<_> = token.split('.').collect();
    let mut claims = valid_claims(&f);
    claims["sub"] = json!("attacker");
    let tampered = format!(
        "{}.{}.{}",
        parts[0],
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap()),
        parts[2]
    );
    assert!(f.auth.verify_token(&tampered).is_err());
}

macro_rules! rejected_claim {
    ($name:ident, $field:literal, $value:expr) => {
        #[tokio::test]
        async fn $name() {
            let f = fixture().await;
            let mut claims = valid_claims(&f);
            claims[$field] = json!($value);
            assert!(f.auth.verify_token(&signed(&claims, "old-key")).is_err());
        }
    };
}
rejected_claim!(
    wrong_issuer_is_rejected,
    "iss",
    "https://attacker.example.test"
);
rejected_claim!(wrong_audience_is_rejected, "aud", "other-api");
rejected_claim!(expired_token_is_rejected, "exp", 1);

macro_rules! required_claim {
    ($name:ident, $field:literal) => {
        #[tokio::test]
        async fn $name() {
            let f = fixture().await;
            let mut claims = valid_claims(&f);
            claims.as_object_mut().unwrap().remove($field);
            assert!(f.auth.verify_token(&signed(&claims, "old-key")).is_err());
        }
    };
}
required_claim!(issuer_is_required, "iss");
required_claim!(audience_is_required, "aud");
required_claim!(subject_is_required, "sub");
required_claim!(issued_at_is_required, "iat");
required_claim!(expiration_is_required, "exp");
required_claim!(token_id_is_required, "jti");

#[tokio::test]
async fn unrecognized_kid_is_not_interpreted_as_a_path_or_url() {
    let f = fixture().await;
    for kid in [
        "unknown-key",
        "../../test.pem",
        "https://attacker.example.test/key",
    ] {
        assert!(
            f.auth
                .verify_token(&signed(&valid_claims(&f), kid))
                .is_err()
        );
    }
}

#[tokio::test]
async fn symmetric_algorithm_is_rejected_even_when_the_signature_is_valid() {
    let f = fixture().await;
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("old-key".into());
    let token = encode(
        &header,
        &valid_claims(&f),
        &EncodingKey::from_secret(b"not-an-ed25519-key"),
    )
    .unwrap();
    assert!(f.auth.verify_token(&token).is_err());
}

#[tokio::test]
async fn unsigned_algorithm_none_is_rejected() {
    let f = fixture().await;
    let token = format!(
        "{}.{}.",
        URL_SAFE_NO_PAD.encode(br#"{"alg":"none","kid":"old-key"}"#),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&valid_claims(&f)).unwrap())
    );
    assert!(f.auth.verify_token(&token).is_err());
}

#[tokio::test]
async fn jwks_contains_public_ed25519_keys_but_no_private_material() {
    let f = fixture().await;
    let jwks = f.auth.jwks();
    let keys = jwks["keys"].as_array().unwrap();
    assert!(!keys.is_empty());
    for key in keys {
        assert_eq!(key["kty"], "OKP");
        assert_eq!(key["crv"], "Ed25519");
        assert!(key["x"].as_str().is_some());
        for private in ["d", "p", "q", "dp", "dq", "qi", "k"] {
            assert!(
                key.get(private).is_none(),
                "private key component {private} exposed"
            );
        }
    }
}

#[tokio::test]
async fn rotation_signs_with_new_kid_and_keeps_old_tokens_verifiable() {
    let f = fixture().await;
    let ctx = context();
    f.login(&ctx, "rotation@example.test").await;
    let old = f.auth.token(&ctx).await.unwrap();
    let mut rotated = f.config.clone();
    let jwt = rotated.jwt.as_mut().unwrap();
    jwt.keys.push(JwtKey {
        kid: "new-key".into(),
        private_key_pem: keys::NEW_KEY.to_vec(),
    });
    jwt.active_kid = "new-key".into();
    let rotated = Auth::new(rotated, None).await.unwrap();
    assert!(rotated.verify_token(&old).is_ok());
    let token = rotated.token(&ctx).await.unwrap();
    assert_eq!(
        decode_header(&token).unwrap().kid.as_deref(),
        Some("new-key")
    );
    let jwks: JwkSet = serde_json::from_value(rotated.jwks()).unwrap();
    assert!(jwks.find("old-key").is_some());
    assert!(jwks.find("new-key").is_some());
}
