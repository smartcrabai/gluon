//! Plan §5.1: applications can prohibit registration or choose their auth subject.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod common;
use gluon::auth::{Auth, AuthError, RegistrationPolicy, UserId, VerifiedIdentity};
use std::sync::Arc;

struct Reject;
impl RegistrationPolicy for Reject {
    fn register(&self, _identity: &VerifiedIdentity) -> Result<Option<UserId>, AuthError> {
        Err(AuthError {
            code: "registration_disabled",
        })
    }
}
struct ExistingSubject;
impl RegistrationPolicy for ExistingSubject {
    fn register(&self, identity: &VerifiedIdentity) -> Result<Option<UserId>, AuthError> {
        assert_eq!(identity.method, "email-otp");
        assert_eq!(identity.subject, "business@example.test");
        Ok(Some(UserId("business-subject".into())))
    }
}
#[tokio::test]
async fn registration_policy_rejects_only_new_verified_identities() {
    let f = common::Fixture::new().await;
    let owner = f.login(&common::context(), "existing@example.test").await;
    f.clock.advance(time::Duration::seconds(60));
    let auth = Auth::new(f.config.clone(), None)
        .await
        .unwrap()
        .with_registration_policy(Arc::new(Reject));
    for (email, accepted) in [("existing@example.test", true), ("new@example.test", false)] {
        let context = common::context();
        let challenge = auth.request_email_otp(&context, email).await.unwrap();
        let code = f.mail.last().code;
        let result = auth
            .verify_email_otp(&context, &challenge.challenge_id, &code)
            .await;
        if accepted {
            assert_eq!(result.unwrap(), owner);
        } else {
            assert_eq!(result.unwrap_err().code, "registration_disabled");
            assert!(auth.current_user(&context).await.unwrap().is_none());
            assert!(
                auth.verify_email_otp(&context, &challenge.challenge_id, &code)
                    .await
                    .is_err()
            );
        }
    }
}
#[tokio::test]
async fn registration_policy_selects_a_stable_business_owned_subject() {
    let f = common::Fixture::new().await;
    let auth = Auth::new(f.config.clone(), None)
        .await
        .unwrap()
        .with_registration_policy(Arc::new(ExistingSubject));
    let context = common::context();
    let challenge = auth
        .request_email_otp(&context, "business@example.test")
        .await
        .unwrap();
    let user = auth
        .verify_email_otp(&context, &challenge.challenge_id, &f.mail.last().code)
        .await
        .unwrap();
    assert_eq!(user.id.0, "business-subject");
    assert_eq!(auth.current_user(&context).await.unwrap(), Some(user));
}

#[tokio::test]
async fn first_verification_records_creation_time_without_rewriting_it_on_login() {
    use gluon::auth::Clock as _;
    let f = common::Fixture::new().await;
    let created_at = f.clock.now();
    let user = f.login(&common::context(), "created@example.test").await;
    f.clock.advance(time::Duration::seconds(60));
    assert_eq!(
        f.login(&common::context(), "created@example.test").await,
        user
    );
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            assert_eq!(data.users[&user.id.0].created_at, created_at);
            Ok(())
        })
        .await
        .unwrap();
}
