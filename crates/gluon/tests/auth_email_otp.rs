#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/auth/mod.rs"]
mod auth_fixture;
use auth_fixture::{Fixture, Mailbox, TestClock, assert_code, config, context};
use std::sync::{Arc, atomic::Ordering};
use time::Duration;

fn incorrect(code: &str) -> String {
    let mut bytes = code.as_bytes().to_vec();
    bytes[0] = if bytes[0] == b'0' { b'1' } else { b'0' };
    String::from_utf8(bytes).unwrap()
}

#[tokio::test]
async fn delivered_code_is_eight_decimal_characters_and_authenticates_once() {
    let f = Fixture::new().await;
    let ctx = context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let mail = f.mail.last();
    assert_eq!(mail.recipient, "alice@example.test");
    assert_eq!(mail.challenge_id, challenge.challenge_id);
    let public = serde_json::to_value(&challenge).unwrap();
    assert_eq!(
        public,
        serde_json::json!({ "challenge_id": challenge.challenge_id })
    );
    assert_eq!(mail.code.len(), 8);
    assert!(mail.code.bytes().all(|b| b.is_ascii_digit()));
    let user = f
        .auth
        .verify_email_otp(&ctx, &challenge.challenge_id, &mail.code)
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
    assert_code(
        f.auth
            .verify_email_otp(&context(), &challenge.challenge_id, &mail.code)
            .await,
        "invalid_otp",
    );
}

#[tokio::test]
async fn incorrect_code_does_not_authenticate_and_allows_retry_with_valid_code() {
    let f = Fixture::new().await;
    let ctx = context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    assert_code(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &incorrect(&code))
            .await,
        "invalid_otp",
    );
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            assert_eq!(data.challenges[&challenge.challenge_id].data["attempts"], 1);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    assert!(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn malformed_stored_otp_returns_storage_error_without_mutation() {
    let f = Fixture::new().await;
    let ctx = context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "malformed@example.test")
        .await
        .unwrap();
    let store = f.config.store.as_ref().unwrap();
    let mut before = gluon::auth::StoreData::default();
    store
        .transact(&mut |data| {
            data.challenges
                .get_mut(&challenge.challenge_id)
                .unwrap()
                .data = serde_json::json!({"version":1,"email":123,"tag":[],"attempts":0});
            before = data.clone();
            Ok(())
        })
        .await
        .unwrap();
    assert_code(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &f.mail.last().code)
            .await,
        "storage_error",
    );
    store
        .transact(&mut |data| {
            assert!(*data == before, "malformed OTP must not mutate storage");
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}

#[tokio::test]
async fn leading_zero_padding_is_not_accepted_as_numeric_equivalent_code() {
    let f = Fixture::new().await;
    let ctx = context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    // Prefixing zero preserves the integer value for every generated code.
    // This requires no random search for a code that happens to start with zero.
    let padded = format!("0{code}");
    assert_eq!(padded.parse::<u32>().unwrap(), code.parse::<u32>().unwrap());
    assert_code(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &padded)
            .await,
        "invalid_otp",
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    assert!(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn otp_expires_at_exact_ttl_but_not_before() {
    for (elapsed, valid) in [
        (Duration::minutes(5) - Duration::seconds(1), true),
        (Duration::minutes(5), false),
    ] {
        let f = Fixture::new().await;
        let ctx = context();
        let challenge = f
            .auth
            .request_email_otp(&ctx, "alice@example.test")
            .await
            .unwrap();
        let code = f.mail.last().code;
        f.clock.advance(elapsed);
        let result = f
            .auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await;
        if valid {
            assert!(result.is_ok());
        } else {
            assert_code(result, "invalid_otp");
        }
    }
}

#[tokio::test]
async fn configured_maximum_failed_attempts_permanently_exhausts_challenge() {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut cfg = config(clock.clone(), mail.clone());
    cfg.email_otp.as_mut().unwrap().max_attempts = 3;
    let f = Fixture::with_config(cfg, clock, mail).await;
    let ctx = context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    for _ in 0..3 {
        assert_code(
            f.auth
                .verify_email_otp(&ctx, &challenge.challenge_id, &incorrect(&code))
                .await,
            "invalid_otp",
        );
    }
    assert_code(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await,
        "invalid_otp",
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}

#[tokio::test]
async fn resend_cooldown_is_exact_and_successful_resend_invalidates_old_challenge() {
    let f = Fixture::new().await;
    let ctx = context();
    let old = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let old_code = f.mail.last().code;
    f.clock
        .advance(Duration::seconds(60) - Duration::seconds(1));
    assert_code(
        f.auth.request_email_otp(&ctx, "alice@example.test").await,
        "rate_limited",
    );
    assert_eq!(f.mail.messages.lock().unwrap().len(), 1);
    f.clock.advance(Duration::seconds(1));
    let new = f
        .auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    assert_ne!(old.challenge_id, new.challenge_id);
    assert_code(
        f.auth
            .verify_email_otp(&ctx, &old.challenge_id, &old_code)
            .await,
        "invalid_otp",
    );
    assert!(
        f.auth
            .verify_email_otp(&ctx, &new.challenge_id, &code)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn delivery_failure_invalidates_only_failed_challenge() {
    let f = Fixture::new().await;
    let ctx = context();
    let good = f
        .auth
        .request_email_otp(&ctx, "good@example.test")
        .await
        .unwrap();
    let good_code = f.mail.last().code;
    f.mail.fail.store(true, Ordering::SeqCst);
    assert_code(
        f.auth.request_email_otp(&ctx, "failed@example.test").await,
        "delivery_failed",
    );
    let failed = f.mail.last();
    f.mail.fail.store(false, Ordering::SeqCst);
    assert_code(
        f.auth
            .verify_email_otp(&context(), &failed.challenge_id, &failed.code)
            .await,
        "invalid_otp",
    );
    assert!(
        f.auth
            .verify_email_otp(&ctx, &good.challenge_id, &good_code)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn concurrent_verification_of_one_challenge_succeeds_once() {
    let f = Fixture::new().await;
    let a = context();
    let b = context();
    let challenge = f
        .auth
        .request_email_otp(&a, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    let (one, two) = tokio::join!(
        f.auth.verify_email_otp(&a, &challenge.challenge_id, &code),
        f.auth.verify_email_otp(&b, &challenge.challenge_id, &code)
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert_code(if one.is_err() { one } else { two }, "invalid_otp");
    let users = [
        f.auth.current_user(&a).await.unwrap(),
        f.auth.current_user(&b).await.unwrap(),
    ];
    assert_eq!(users.iter().filter(|u| u.is_some()).count(), 1);
}

#[tokio::test]
async fn source_limit_applies_across_destinations_and_resets_at_window_boundary() {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut cfg = config(clock.clone(), mail.clone());
    let otp = cfg.email_otp.as_mut().unwrap();
    otp.source_limit = 2;
    otp.destination_limit = 10;
    otp.rate_window = Duration::minutes(10);
    let f = Fixture::with_config(cfg, clock, mail).await;
    let ctx = context();
    for email in ["a@example.test", "b@example.test"] {
        f.auth.request_email_otp(&ctx, email).await.unwrap();
    }
    assert_code(
        f.auth.request_email_otp(&ctx, "c@example.test").await,
        "rate_limited",
    );
    assert_eq!(f.mail.messages.lock().unwrap().len(), 2);
    let mut other = context();
    other.source = "192.0.2.11".parse().unwrap();
    assert!(
        f.auth
            .request_email_otp(&other, "c@example.test")
            .await
            .is_ok()
    );
    f.clock.advance(Duration::minutes(10));
    assert!(
        f.auth
            .request_email_otp(&ctx, "d@example.test")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn destination_limit_is_shared_across_sources() {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut cfg = config(clock.clone(), mail.clone());
    let otp = cfg.email_otp.as_mut().unwrap();
    otp.destination_limit = 2;
    otp.source_limit = 10;
    otp.resend_interval = Duration::seconds(1);
    otp.rate_window = Duration::minutes(10);
    let f = Fixture::with_config(cfg, clock, mail).await;
    let mut ctx = context();
    f.auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    f.clock.advance(Duration::seconds(1));
    ctx.source = "192.0.2.11".parse().unwrap();
    f.auth
        .request_email_otp(&ctx, "alice@example.test")
        .await
        .unwrap();
    f.clock.advance(Duration::seconds(1));
    ctx.source = "192.0.2.12".parse().unwrap();
    assert_code(
        f.auth.request_email_otp(&ctx, "alice@example.test").await,
        "rate_limited",
    );
    assert_eq!(f.mail.messages.lock().unwrap().len(), 2);
    f.clock
        .advance(Duration::minutes(10) - Duration::seconds(2));
    assert!(
        f.auth
            .request_email_otp(&ctx, "alice@example.test")
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn normalization_does_not_apply_provider_specific_dot_or_plus_collapsing() {
    let f = Fixture::new().await;
    let base = f.login(&context(), "first.last+tag@gmail.com").await;
    assert_eq!(f.mail.last().recipient, "first.last+tag@gmail.com");
    f.clock.advance(Duration::seconds(60));
    let same = f.login(&context(), "first.last+tag@gmail.com").await;
    assert_eq!(base.id, same.id);
    let no_dots = f.login(&context(), "firstlast+tag@gmail.com").await;
    let no_tag = f.login(&context(), "first.last@gmail.com").await;
    assert_ne!(base.id, no_dots.id);
    assert_ne!(base.id, no_tag.id);
    assert_ne!(no_dots.id, no_tag.id);
}

#[tokio::test]
async fn identity_resolution_is_shared_across_auth_instances() {
    let f = Fixture::new().await;
    let a = context();
    let b = context();
    let first = f
        .auth
        .request_email_otp(&a, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    // A resend invalidates older challenges, so identity reuse is checked after
    // consuming the first challenge, through a second service sharing the store.
    let user = f
        .auth
        .verify_email_otp(&a, &first.challenge_id, &code)
        .await
        .unwrap();
    f.clock.advance(Duration::seconds(60));
    let second = f
        .auth
        .request_email_otp(&b, "alice@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    let other_auth = gluon::auth::Auth::new(f.config.clone(), None)
        .await
        .unwrap();
    let (resolved, current) = tokio::join!(
        other_auth.verify_email_otp(&b, &second.challenge_id, &code),
        f.auth.current_user(&a)
    );
    assert_eq!(resolved.unwrap().id, user.id);
    assert_eq!(current.unwrap(), Some(user));
}

#[tokio::test]
async fn disabled_user_cannot_login_or_remain_authenticated() {
    let f = Fixture::new().await;
    let ctx = context();
    let user = f.login(&ctx, "alice@example.test").await;
    f.config
        .store
        .as_ref()
        .unwrap()
        .set_user_enabled(&user.id, false)
        .await
        .unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    f.clock.advance(Duration::seconds(60));
    let anonymous = context();
    let challenge = f
        .auth
        .request_email_otp(&anonymous, "alice@example.test")
        .await
        .unwrap();
    assert_code(
        f.auth
            .verify_email_otp(&anonymous, &challenge.challenge_id, &f.mail.last().code)
            .await,
        "user_disabled",
    );
    assert_eq!(f.auth.current_user(&anonymous).await.unwrap(), None);
}
