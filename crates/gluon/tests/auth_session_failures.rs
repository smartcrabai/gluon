//! Plan section 5.1 / section 6.1: success must not escape before required session persistence.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "common/auth/mod.rs"]
mod common;

use async_trait::async_trait;
use std::sync::Arc;
use tower_sessions::{
    MemoryStore, Session, SessionStore,
    session::{Id, Record},
    session_store,
};

#[derive(Clone, Debug)]
struct FaultStore {
    inner: MemoryStore,
    fail_write: bool,
    fail_delete: bool,
}
#[async_trait]
impl SessionStore for FaultStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        if self.fail_write {
            return Err(session_store::Error::Backend(
                "injected session write failure".into(),
            ));
        }
        self.inner.create(record).await
    }
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        if self.fail_write {
            return Err(session_store::Error::Backend(
                "injected session write failure".into(),
            ));
        }
        self.inner.save(record).await
    }
    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        self.inner.load(id).await
    }
    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        if self.fail_delete {
            return Err(session_store::Error::Backend(
                "injected session delete failure".into(),
            ));
        }
        self.inner.delete(id).await
    }
}

#[tokio::test]
async fn authentication_does_not_succeed_when_new_session_cannot_be_persisted() {
    let f = common::Fixture::new().await;
    let mut ctx = common::context();
    let challenge = f
        .auth
        .request_email_otp(&ctx, "write-failure@example.test")
        .await
        .unwrap();
    let code = f.mail.last().code;
    ctx.session = Session::new(
        None,
        Arc::new(FaultStore {
            inner: MemoryStore::default(),
            fail_write: true,
            fail_delete: false,
        }),
        None,
    );
    assert!(
        f.auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await
            .is_err(),
        "authentication must persist the new session before reporting success"
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            assert!(
                data.logins.is_empty(),
                "failed cookie writes must not grant authority"
            );
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        f.auth
            .verify_email_otp(&common::context(), &challenge.challenge_id, &code)
            .await
            .is_err(),
        "consumed challenges are not reusable after session persistence fails"
    );
}

#[tokio::test]
async fn logout_does_not_succeed_when_server_side_deletion_fails() {
    let f = common::Fixture::new().await;
    let store = MemoryStore::default();
    let mut ctx = common::context();
    ctx.session = Session::new(None, Arc::new(store.clone()), None);
    f.login(&ctx, "delete-failure@example.test").await;
    let failing = gluon::auth::AuthContext {
        session: Session::new(
            ctx.session.id(),
            Arc::new(FaultStore {
                inner: store,
                fail_write: false,
                fail_delete: true,
            }),
            None,
        ),
        ..ctx.clone()
    };
    assert!(
        f.auth.logout(&failing).await.is_err(),
        "clearing a client cookie alone must not report a completed logout"
    );
    assert_eq!(
        f.auth.current_user(&ctx).await.unwrap(),
        None,
        "authority must be revoked even when cookie deletion fails"
    );
}

#[tokio::test]
async fn replacement_revokes_old_authority_even_when_cookie_flush_fails() {
    let f = common::Fixture::new().await;
    let store = MemoryStore::default();
    let mut ctx = common::context();
    ctx.session = Session::new(None, Arc::new(store.clone()), None);
    f.login(&ctx, "old-flush-failure@example.test").await;
    let failing = gluon::auth::AuthContext {
        session: Session::new(
            ctx.session.id(),
            Arc::new(FaultStore {
                inner: store,
                fail_write: false,
                fail_delete: true,
            }),
            None,
        ),
        ..ctx.clone()
    };
    assert!(f.auth.current_user(&failing).await.unwrap().is_some());
    let challenge = f
        .auth
        .request_email_otp(&failing, "new-flush-failure@example.test")
        .await
        .unwrap();
    assert_eq!(
        f.auth
            .verify_email_otp(&failing, &challenge.challenge_id, &f.mail.last().code)
            .await
            .unwrap_err()
            .code,
        "storage_error"
    );
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            assert!(data.logins.is_empty());
            Ok(())
        })
        .await
        .unwrap();
}
