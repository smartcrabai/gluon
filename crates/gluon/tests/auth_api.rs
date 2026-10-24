//! Plan §4.3 / §5.5: direct composition obeys HTTP disabling and explicit JWT defaults.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod common;
#[path = "common/auth/jwt.rs"]
mod keys;
use gluon::{
    ContainerBuilder,
    auth::{Auth, JwtConfig},
};
use std::sync::Arc;

#[test]
fn explicit_jwt_configuration_defaults_to_five_minutes_without_clock_tolerance() {
    let configured = keys::jwt_config();
    let default = JwtConfig::new(
        configured.issuer,
        configured.audience,
        configured.keys,
        configured.active_kid,
    );
    assert_eq!(default.ttl, time::Duration::minutes(5));
    assert_eq!(default.clock_skew, time::Duration::ZERO);
    assert_eq!(default.keys.len(), 1);
}

#[tokio::test]
async fn disabled_http_routers_stay_empty_while_direct_api_remains_available() {
    let clock = Arc::new(common::TestClock::new());
    let mailbox = Arc::new(common::Mailbox::default());
    let mut config = common::config(clock, mailbox.clone());
    config.http_enabled = false;
    config.jwt = Some(keys::jwt_config());
    let auth = Arc::new(Auth::new(config, None).await.unwrap());
    let container = Arc::new(ContainerBuilder::new().bind_instance(auth.clone()).build());
    let server = axum_test::TestServer::new(
        auth.router()
            .merge(auth.public_router())
            .with_state(container),
    );
    server.get("/csrf").await.assert_status_not_found();
    server.get("/jwks").await.assert_status_not_found();
    let context = common::context();
    let challenge = auth
        .request_email_otp(&context, "rust-api@example.test")
        .await
        .unwrap();
    let user = auth
        .verify_email_otp(&context, &challenge.challenge_id, &mailbox.last().code)
        .await
        .unwrap();
    let token = auth.token(&context).await.unwrap();
    assert_eq!(auth.verify_token(&token).unwrap().sub, user.id.0);
}
