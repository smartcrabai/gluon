#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod common;
#[path = "common/auth/passkey_device.rs"]
mod device;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use common::{Fixture, Mailbox, TestClock, config, context};
use device::{Device, ORIGIN, client_data, corrupt_signature};
use gluon::auth::{AuthContext, AuthFlow, AuthUser, PasskeyConfig};
use serde_json::Value;
use std::sync::Arc;
use time::Duration;

async fn fixture() -> Fixture {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut cfg = config(clock.clone(), mail.clone());
    cfg.passkey = Some(PasskeyConfig {
        rp_id: "app.example.test".into(),
        origin: ORIGIN.into(),
    });
    // Flow-expiry tests must not be accidentally satisfied by session expiry.
    cfg.flow_ttl = Duration::seconds(1);
    cfg.idle_timeout = Duration::days(7);
    cfg.absolute_timeout = Duration::days(7);
    Fixture::with_config(cfg, clock, mail).await
}
async fn owner(f: &Fixture) -> (AuthContext, AuthUser) {
    let ctx = context();
    let user = f.login(&ctx, "passkey-owner@example.test").await;
    (ctx, user)
}
async fn enrolled(f: &Fixture) -> (AuthContext, AuthUser, Device) {
    let (ctx, user) = owner(f).await;
    let mut device = Device::new();
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    let response = device.register(flow.options);
    f.auth
        .finish_passkey_registration(&ctx, &flow.flow_id, response)
        .await
        .unwrap();
    assert_eq!(f.auth.passkeys(&ctx).await.unwrap().len(), 1);
    (ctx, user, device)
}
async fn assertion(
    f: &Fixture,
    user: &AuthUser,
    device: &mut Device,
) -> (AuthContext, AuthFlow, Value) {
    let ctx = context();
    let flow = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    let response = device.authenticate(flow.options.clone());
    (ctx, flow, response)
}
fn assert_option_schema(options: &Value) {
    for key in options
        .as_object()
        .expect("public WebAuthn option object")
        .keys()
    {
        assert!(
            ["publicKey", "mediation"].contains(&key.as_str()),
            "non-public wrapper field {key}"
        );
    }
    for key in options["publicKey"]
        .as_object()
        .expect("publicKey options")
        .keys()
    {
        assert!(
            [
                "rp",
                "user",
                "challenge",
                "pubKeyCredParams",
                "timeout",
                "excludeCredentials",
                "authenticatorSelection",
                "hints",
                "attestation",
                "attestationFormats",
                "extensions",
                "rpId",
                "allowCredentials",
                "userVerification"
            ]
            .contains(&key.as_str()),
            "non-public option field {key}"
        );
    }
}
fn assert_public_only(value: &Value) {
    if let Some(options) = value.get("options") {
        assert_option_schema(options);
    }
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                assert!(
                    ![
                        "state",
                        "server_state",
                        "registration_state",
                        "authentication_state",
                        "browser_binding",
                        "expires_at"
                    ]
                    .contains(&key.as_str()),
                    "server-only field leaked: {key}"
                );
                assert_public_only(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                assert_public_only(value);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn anonymous_registration_start_rejects() {
    let f = fixture().await;
    assert!(f.auth.start_passkey_registration(&context()).await.is_err());
}
#[tokio::test]
async fn anonymous_registration_finish_cannot_choose_a_victim_user() {
    let f = fixture().await;
    let (owner, _) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&owner).await.unwrap();
    let mut response = Device::new().register(flow.options);
    response["user_id"] = "victim".into();
    let ctx = context();
    assert!(
        f.auth
            .finish_passkey_registration(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}
#[tokio::test]
async fn registration_start_exposes_only_public_options_and_opaque_flow_id() {
    let f = fixture().await;
    let (ctx, _) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    assert_ne!(flow.flow_id, "");
    let public = serde_json::to_value(&flow).unwrap();
    assert_public_only(&public);
    assert_eq!(flow.options["publicKey"]["rp"]["id"], "app.example.test");
    assert!(
        flow.options["publicKey"]["challenge"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(
        !serde_json::to_string(&public)
            .unwrap()
            .contains(&ctx.browser_binding)
    );
}
#[tokio::test]
async fn genuine_registration_and_authentication_establish_the_same_user() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, response) = assertion(&f, &user, &mut device).await;
    assert_eq!(
        reqwest::Url::parse(client_data(&response)["origin"].as_str().unwrap()).unwrap(),
        reqwest::Url::parse(ORIGIN).unwrap()
    );
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .unwrap(),
        user
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
}
#[tokio::test]
async fn failed_signature_consumes_flow_but_fresh_valid_flow_still_succeeds() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, valid) = assertion(&f, &user, &mut device).await;
    let mut invalid = valid.clone();
    corrupt_signature(&mut invalid);
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, invalid)
            .await
            .is_err()
    );
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, valid)
            .await
            .is_err()
    );
    let fresh = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    let response = device.authenticate(fresh.options);
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&ctx, &fresh.flow_id, response)
            .await
            .unwrap(),
        user
    );
}

#[tokio::test]
async fn signed_assertion_for_wrong_rp_id_rejects() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let ctx = context();
    let mut flow = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    flow.options["publicKey"]["rpId"] = "other.example.test".into();
    let response = device.authenticate(flow.options);
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn valid_credential_cannot_authenticate_a_different_user_flow() {
    let f = fixture().await;
    let (_, first, mut device) = enrolled(&f).await;
    let other = context();
    let second = f.login(&other, "second@example.test").await;
    let registration = f.auth.start_passkey_registration(&other).await.unwrap();
    let response = Device::new().register(registration.options);
    f.auth
        .finish_passkey_registration(&other, &registration.flow_id, response)
        .await
        .unwrap();
    let ctx = context();
    let first_options = f
        .auth
        .start_passkey_authentication(&ctx, Some(&first.id))
        .await
        .unwrap()
        .options;
    let mut second_flow = f
        .auth
        .start_passkey_authentication(&ctx, Some(&second.id))
        .await
        .unwrap();
    second_flow.options["publicKey"]["allowCredentials"] =
        first_options["publicKey"]["allowCredentials"].clone();
    let response = device.authenticate(second_flow.options);
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &second_flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}

#[tokio::test]
async fn discoverable_start_returns_public_challenge_without_server_state() {
    let f = fixture().await;
    let flow = f
        .auth
        .start_passkey_authentication(&context(), None)
        .await
        .unwrap();
    assert_public_only(&serde_json::to_value(&flow).unwrap());
    assert!(
        flow.options["publicKey"]["challenge"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    let allow = &flow.options["publicKey"]["allowCredentials"];
    assert!(allow.is_null() || allow.as_array().is_some_and(std::vec::Vec::is_empty));
}

#[tokio::test]
async fn discoverable_signed_assertion_restores_the_credential_owner() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let sessions = Arc::new(tower_sessions::MemoryStore::default());
    let mut ctx = context();
    ctx.session = tower_sessions::Session::new(None, sessions.clone(), None);
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    let flow = f
        .auth
        .start_passkey_authentication(&ctx, None)
        .await
        .unwrap();
    let allow = &flow.options["publicKey"]["allowCredentials"];
    assert!(allow.is_null() || allow.as_array().is_some_and(std::vec::Vec::is_empty));
    let response = device.authenticate(flow.options.clone());
    assert_eq!(
        client_data(&response)["challenge"],
        flow.options["publicKey"]["challenge"]
    );
    assert!(response["response"]["userHandle"].is_string());
    // A blanket rejection must fail here: every Err panics at unwrap, rather
    // than letting a negative-only discoverable test silently pass.
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .unwrap(),
        user
    );
    let restored = AuthContext {
        session: tower_sessions::Session::new(ctx.session.id(), sessions, None),
        ..ctx
    };
    assert_eq!(f.auth.current_user(&restored).await.unwrap(), Some(user));
}

#[tokio::test]
async fn discoverable_signed_assertion_rejects_mismatched_user_handle_without_login() {
    let f = fixture().await;
    let (_, _, mut device) = enrolled(&f).await;
    let ctx = context();
    let flow = f
        .auth
        .start_passkey_authentication(&ctx, None)
        .await
        .unwrap();
    let mut response = device.authenticate(flow.options);
    let mut handle = URL_SAFE_NO_PAD
        .decode(response["response"]["userHandle"].as_str().unwrap())
        .unwrap();
    assert_eq!(handle.len(), 16);
    // Keep a well-formed UUID handle and the genuine signature, but guarantee
    // the handle differs from the one retained at registration.
    handle[0] ^= 1;
    response["response"]["userHandle"] = URL_SAFE_NO_PAD.encode(handle).into();
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}

#[tokio::test]
async fn successful_discoverable_signed_assertion_cannot_authenticate_replay_context() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let ctx = context();
    let flow = f
        .auth
        .start_passkey_authentication(&ctx, None)
        .await
        .unwrap();
    let response = device.authenticate(flow.options);
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response.clone())
            .await
            .unwrap(),
        user
    );
    let replay = context();
    // Same browser binding, independent anonymous session: rejection must not
    // rely on a binding mismatch or hide behind the successful caller's login.
    assert_eq!(replay.browser_binding, ctx.browser_binding);
    assert_eq!(f.auth.current_user(&replay).await.unwrap(), None);
    assert!(
        f.auth
            .finish_passkey_authentication(&replay, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&replay).await.unwrap(), None);
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
}

#[tokio::test]
async fn authentication_options_do_not_expose_verification_state() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (_, flow, _) = assertion(&f, &user, &mut device).await;
    assert_public_only(&serde_json::to_value(flow).unwrap());
}
#[tokio::test]
async fn corrupted_real_signature_rejects_without_login() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, mut response) = assertion(&f, &user, &mut device).await;
    corrupt_signature(&mut response);
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}
#[tokio::test]
async fn successful_registration_response_cannot_be_replayed() {
    let f = fixture().await;
    let (ctx, _) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    let response = Device::new().register(flow.options);
    f.auth
        .finish_passkey_registration(&ctx, &flow.flow_id, response.clone())
        .await
        .unwrap();
    assert!(
        f.auth
            .finish_passkey_registration(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.passkeys(&ctx).await.unwrap().len(), 1);
}
#[tokio::test]
async fn successful_signed_assertion_cannot_be_replayed() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, response) = assertion(&f, &user, &mut device).await;
    f.auth
        .finish_passkey_authentication(&ctx, &flow.flow_id, response.clone())
        .await
        .unwrap();
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
}

async fn stored_data(f: &Fixture) -> gluon::auth::StoreData {
    let mut snapshot = gluon::auth::StoreData::default();
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            snapshot = data.clone();
            Ok(())
        })
        .await
        .unwrap();
    snapshot
}

fn verifier() -> webauthn_rs::prelude::Webauthn {
    webauthn_rs::prelude::WebauthnBuilder::new(
        "app.example.test",
        &reqwest::Url::parse(ORIGIN).unwrap(),
    )
    .unwrap()
    .build()
    .unwrap()
}

#[tokio::test]
async fn fresh_signed_registration_collision_preserves_existing_owner_and_record() {
    let f = fixture().await;
    let (owner, user) = owner(&f).await;
    let (_, id) = enroll_device(&f, &owner).await;
    let before = stored_data(&f).await.passkeys[&id].clone();
    assert_eq!(before.user, user.id.0);
    assert_eq!(before.version, 1);
    let other = context();
    f.login(&other, "collision@example.test").await;
    let flow = f.auth.start_passkey_registration(&other).await.unwrap();
    let response = device::register_with_id(&flow.options, &id);
    assert_eq!(
        client_data(&response)["challenge"],
        flow.options["publicKey"]["challenge"]
    );
    let snapshot = stored_data(&f).await;
    let state = serde_json::from_value::<webauthn_rs::prelude::PasskeyRegistration>(
        snapshot.challenges[&flow.flow_id].data["state"].clone(),
    )
    .unwrap();
    // Independently verify the packed signature and this fresh ceremony before
    // testing the global-ID guard, so cryptographic rejection cannot satisfy it.
    verifier()
        .finish_passkey_registration(&serde_json::from_value(response.clone()).unwrap(), &state)
        .unwrap();
    assert_eq!(
        f.auth
            .finish_passkey_registration(&other, &flow.flow_id, response)
            .await
            .unwrap_err()
            .code,
        "invalid_passkey"
    );
    let after = stored_data(&f).await;
    assert_eq!(after.passkeys.len(), 1);
    assert_eq!(after.passkeys[&id].user, before.user);
    assert_eq!(after.passkeys[&id].data, before.data);
    assert_eq!(after.passkeys[&id].version, before.version);
    assert!(!after.challenges.contains_key(&flow.flow_id));
    assert_eq!(
        f.auth.passkeys(&other).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
}

#[tokio::test]
async fn signed_authentication_advances_state_and_fences_a_distinct_live_flow() {
    let f = fixture().await;
    let (owner, user) = owner(&f).await;
    let (mut device, id) = enroll_device(&f, &owner).await;
    let before = stored_data(&f).await.passkeys[&id].clone();
    assert_eq!(before.user, user.id.0);
    assert_eq!(before.version, 1);
    let (first_ctx, first, first_response) = assertion(&f, &user, &mut device).await;
    let (second_ctx, second, second_response) = assertion(&f, &user, &mut device).await;
    assert_ne!(first.flow_id, second.flow_id);
    assert_ne!(
        first.options["publicKey"]["challenge"],
        second.options["publicKey"]["challenge"]
    );
    let pending = stored_data(&f).await;
    for flow in [&first, &second] {
        assert_eq!(
            pending.challenges[&flow.flow_id].data["versions"][&id],
            before.version
        );
    }
    let state = serde_json::from_value::<webauthn_rs::prelude::PasskeyAuthentication>(
        pending.challenges[&second.flow_id].data["state"].clone(),
    )
    .unwrap();
    verifier()
        .finish_passkey_authentication(
            &serde_json::from_value(second_response.clone()).unwrap(),
            &state,
        )
        .unwrap();
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&first_ctx, &first.flow_id, first_response)
            .await
            .unwrap(),
        user
    );
    let advanced = stored_data(&f).await;
    let current = &advanced.passkeys[&id];
    assert_eq!(current.user, before.user);
    assert_eq!(current.version, before.version + 1);
    assert_ne!(current.data, before.data);
    assert!(advanced.challenges.contains_key(&second.flow_id));
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&second_ctx, &second.flow_id, second_response)
            .await
            .unwrap_err()
            .code,
        "invalid_passkey"
    );
    let after = stored_data(&f).await;
    assert_eq!(after.passkeys[&id].user, current.user);
    assert_eq!(after.passkeys[&id].data, current.data);
    assert_eq!(after.passkeys[&id].version, current.version);
    assert!(!after.challenges.contains_key(&second.flow_id));
    assert_eq!(f.auth.current_user(&second_ctx).await.unwrap(), None);
}

#[tokio::test]
async fn concurrent_signed_assertion_is_consumed_once() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, response) = assertion(&f, &user, &mut device).await;
    let (a, b) = tokio::join!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response.clone()),
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response.clone())
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
}

#[tokio::test]
async fn malformed_stored_credentials_and_both_ceremony_states_consume_flow_without_updates() {
    for (discoverable, corrupt_key) in [(false, true), (false, false), (true, false)] {
        let f = fixture().await;
        let (owner, user) = owner(&f).await;
        let (mut device, id) = enroll_device(&f, &owner).await;
        let ctx = context();
        let flow = f
            .auth
            .start_passkey_authentication(&ctx, if discoverable { None } else { Some(&user.id) })
            .await
            .unwrap();
        let response = device.authenticate(flow.options);
        f.config
            .store
            .as_ref()
            .unwrap()
            .transact(&mut |data| {
                if corrupt_key {
                    data.passkeys.get_mut(&id).unwrap().data = Value::Null;
                } else {
                    data.challenges.get_mut(&flow.flow_id).unwrap().data["state"] = Value::Null;
                }
                Ok(())
            })
            .await
            .unwrap();
        let before = stored_data(&f).await.passkeys[&id].clone();
        assert_eq!(
            f.auth
                .finish_passkey_authentication(&ctx, &flow.flow_id, response.clone())
                .await
                .unwrap_err()
                .code,
            "storage_error"
        );
        let after = stored_data(&f).await;
        assert!(!after.challenges.contains_key(&flow.flow_id));
        assert_eq!(after.passkeys[&id].user, before.user);
        assert_eq!(after.passkeys[&id].data, before.data);
        assert_eq!(after.passkeys[&id].version, before.version);
        assert_eq!(
            f.auth
                .finish_passkey_authentication(&ctx, &flow.flow_id, response)
                .await
                .unwrap_err()
                .code,
            "invalid_passkey"
        );
        assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    }
}
#[tokio::test]
async fn registration_valid_response_rejects_wrong_browser_binding() {
    let f = fixture().await;
    let (ctx, _) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    let response = Device::new().register(flow.options);
    let mut caller = ctx.clone();
    caller.browser_binding = "other-browser".into();
    assert!(
        f.auth
            .finish_passkey_registration(&caller, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(
        f.auth.passkeys(&ctx).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
}
#[tokio::test]
async fn registration_valid_response_rejects_different_authenticated_user() {
    let f = fixture().await;
    let (ctx, _) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    let response = Device::new().register(flow.options);
    let other = context();
    f.login(&other, "other@example.test").await;
    assert!(
        f.auth
            .finish_passkey_registration(&other, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(
        f.auth.passkeys(&other).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
    assert_eq!(
        f.auth.passkeys(&ctx).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
}
#[tokio::test]
async fn registration_valid_response_rejects_expired_flow_not_expired_session() {
    let f = fixture().await;
    let (ctx, user) = owner(&f).await;
    let flow = f.auth.start_passkey_registration(&ctx).await.unwrap();
    let response = Device::new().register(flow.options);
    f.clock.advance(Duration::seconds(2));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
    assert!(
        f.auth
            .finish_passkey_registration(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn authentication_valid_signature_rejects_wrong_browser_binding() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (mut ctx, flow, response) = assertion(&f, &user, &mut device).await;
    ctx.browser_binding = "other-browser".into();
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}
#[tokio::test]
async fn authentication_valid_signature_rejects_expired_flow() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, flow, response) = assertion(&f, &user, &mut device).await;
    f.clock.advance(Duration::seconds(2));
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}
#[tokio::test]
async fn assertion_for_one_challenge_rejects_another_live_flow() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let (ctx, _flow, response) = assertion(&f, &user, &mut device).await;
    let different = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &different.flow_id, response)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn authentication_signature_for_other_origin_rejects() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let ctx = context();
    let flow = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    // Same RP, different port: the device signs for an origin outside the
    // explicitly trusted origin without corrupting the signature.
    let response = device.authenticate_at(flow.options, "https://app.example.test:8443");
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn authentication_without_user_verification_rejects() {
    let f = fixture().await;
    let (_, user, mut device) = enrolled(&f).await;
    let ctx = context();
    let mut flow = f
        .auth
        .start_passkey_authentication(&ctx, Some(&user.id))
        .await
        .unwrap();
    flow.options["publicKey"]["userVerification"] = "discouraged".into();
    let response = device.authenticate(flow.options);
    assert!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .is_err()
    );
}
async fn enroll_device(f: &Fixture, ctx: &AuthContext) -> (Device, String) {
    let mut device = Device::new();
    let flow = f.auth.start_passkey_registration(ctx).await.unwrap();
    let response = device.register(flow.options);
    let credential_id = response["id"]
        .as_str()
        .expect("public credential id")
        .to_owned();
    f.auth
        .finish_passkey_registration(ctx, &flow.flow_id, response)
        .await
        .unwrap();
    (device, credential_id)
}

#[tokio::test]
async fn anonymous_cannot_list_credentials() {
    let f = fixture().await;
    assert!(f.auth.passkeys(&context()).await.is_err());
}
#[tokio::test]
async fn anonymous_cannot_delete_an_existing_credential() {
    let f = fixture().await;
    let (owner, _) = owner(&f).await;
    let (_, id) = enroll_device(&f, &owner).await;
    assert!(f.auth.delete_passkey(&context(), &id).await.is_err());
    assert_eq!(f.auth.passkeys(&owner).await.unwrap().len(), 1);
}
#[tokio::test]
async fn another_user_cannot_delete_owner_credential() {
    let f = fixture().await;
    let (owner, user) = owner(&f).await;
    let (mut device, id) = enroll_device(&f, &owner).await;
    let other = context();
    f.login(&other, "other@example.test").await;
    assert!(f.auth.delete_passkey(&other, &id).await.is_err());
    assert_eq!(f.auth.passkeys(&owner).await.unwrap().len(), 1);
    assert_eq!(
        f.auth.passkeys(&other).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
    let (ctx, flow, response) = assertion(&f, &user, &mut device).await;
    assert_eq!(
        f.auth
            .finish_passkey_authentication(&ctx, &flow.flow_id, response)
            .await
            .unwrap(),
        user
    );
}
#[tokio::test]
async fn owner_can_delete_credential_when_email_identity_remains() {
    let f = fixture().await;
    let (owner, user) = owner(&f).await;
    let (_, id) = enroll_device(&f, &owner).await;
    f.auth.delete_passkey(&owner, &id).await.unwrap();
    assert_eq!(
        f.auth.passkeys(&owner).await.unwrap(),
        [] as [serde_json::Value; 0]
    );
    assert_eq!(f.auth.current_user(&owner).await.unwrap(), Some(user));
}

#[tokio::test]
async fn registration_flow_cannot_be_used_for_valid_authentication_assertion() {
    let f = fixture().await;
    let (owner, user, mut device) = enrolled(&f).await;
    let registration = f.auth.start_passkey_registration(&owner).await.unwrap();
    let mut options = f
        .auth
        .start_passkey_authentication(&owner, Some(&user.id))
        .await
        .unwrap()
        .options;
    options["publicKey"]["challenge"] = registration.options["publicKey"]["challenge"].clone();
    let response = device.authenticate(options);
    assert!(
        f.auth
            .finish_passkey_authentication(&owner, &registration.flow_id, response)
            .await
            .is_err()
    );
}
