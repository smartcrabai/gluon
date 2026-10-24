#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/auth/mod.rs"]
mod auth_fixture;
#[path = "common/auth/jwt.rs"]
mod keys;
use auth_fixture::{Fixture, Mailbox, TestClock, config, context};
use gluon::auth::{Auth, AuthContext, AuthStore, MemoryAuthStore};
use std::sync::Arc;
use time::Duration;
use tower_sessions::{MemoryStore, Session};

// Reopen the persisted cookie ID rather than cloning Session's in-memory cache.
fn reopened(ctx: &AuthContext, store: Arc<MemoryStore>) -> AuthContext {
    AuthContext {
        session: Session::new(ctx.session.id(), store, None),
        source: ctx.source,
        browser_binding: ctx.browser_binding.clone(),
    }
}

#[tokio::test]
async fn current_user_is_none_for_anonymous_session() {
    let f = Fixture::new().await;
    assert_eq!(f.auth.current_user(&context()).await.unwrap(), None);
}

#[tokio::test]
async fn login_rotates_persisted_id_revokes_old_id_and_discards_anonymous_data() {
    let f = Fixture::new().await;
    let store = Arc::new(MemoryStore::default());
    let mut ctx = context();
    ctx.session = Session::new(None, store.clone(), None);
    ctx.session
        .insert("anonymous_cart", "must not survive")
        .await
        .unwrap();
    ctx.session.save().await.unwrap();
    let old_id = ctx.session.id().expect("anonymous session persisted");
    let old_cookie = reopened(&ctx, store.clone());
    let user = f.login(&ctx, "alice@example.test").await;
    let new_id = ctx
        .session
        .id()
        .expect("login must persist session before success");
    assert_ne!(old_id, new_id);
    assert_eq!(
        ctx.session.get::<String>("anonymous_cart").await.unwrap(),
        None
    );
    let new_cookie = reopened(&ctx, store);
    assert_eq!(f.auth.current_user(&new_cookie).await.unwrap(), Some(user));
    assert_eq!(
        new_cookie
            .session
            .get::<String>("anonymous_cart")
            .await
            .unwrap(),
        None
    );
    assert_eq!(f.auth.current_user(&old_cookie).await.unwrap(), None);
    assert_eq!(
        old_cookie
            .session
            .get::<String>("anonymous_cart")
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn logout_revokes_persisted_session_across_auth_instances_and_cookie_replay() {
    let f = Fixture::new().await;
    let store = Arc::new(MemoryStore::default());
    let mut ctx = context();
    ctx.session = Session::new(None, store.clone(), None);
    let user = f.login(&ctx, "alice@example.test").await;
    let id = ctx.session.id().expect("login persisted session");
    let other = Auth::new(f.config.clone(), None).await.unwrap();
    assert_eq!(
        other
            .current_user(&reopened(&ctx, store.clone()))
            .await
            .unwrap(),
        Some(user)
    );
    f.auth.logout(&ctx).await.unwrap();
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    let replay = AuthContext {
        session: Session::new(Some(id), store, None),
        source: ctx.source,
        browser_binding: ctx.browser_binding.clone(),
    };
    assert_eq!(other.current_user(&replay).await.unwrap(), None);
    // Repeated logout must remain safe and cannot restore the revoked record.
    other.logout(&replay).await.unwrap();
    assert_eq!(other.current_user(&replay).await.unwrap(), None);
}

async fn short_lived_fixture() -> Fixture {
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut cfg = config(clock.clone(), mail.clone());
    cfg.idle_timeout = Duration::minutes(5);
    cfg.absolute_timeout = Duration::minutes(12);
    Fixture::with_config(cfg, clock, mail).await
}

#[tokio::test]
async fn idle_session_is_valid_just_before_expiry_and_invalid_at_exact_expiry() {
    for elapsed in [
        Duration::minutes(5) - Duration::seconds(1),
        Duration::minutes(5),
    ] {
        let f = short_lived_fixture().await;
        let ctx = context();
        let user = f.login(&ctx, "alice@example.test").await;
        f.clock.advance(elapsed);
        let expected = if elapsed < Duration::minutes(5) {
            Some(user)
        } else {
            None
        };
        assert_eq!(f.auth.current_user(&ctx).await.unwrap(), expected);
    }
}

#[tokio::test]
async fn activity_extends_idle_expiry_but_never_absolute_expiry() {
    let f = short_lived_fixture().await;
    let ctx = context();
    let user = f.login(&ctx, "alice@example.test").await;
    for _ in 0..2 {
        f.clock.advance(Duration::minutes(4));
        assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user.clone()));
    }
    // At 11:59 idle expiry is still in the future.
    f.clock.advance(Duration::minutes(4) - Duration::seconds(1));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user));
    f.clock.advance(Duration::seconds(1));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    f.clock.advance(Duration::minutes(1));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
}

#[tokio::test]
async fn preloaded_independent_sessions_cannot_restore_revoked_login_authority() {
    for replacement in [false, true] {
        let clock = Arc::new(TestClock::new());
        let mail = Arc::new(Mailbox::default());
        let mut cfg = config(clock.clone(), mail.clone());
        cfg.jwt = Some(keys::jwt_config());
        let f = Fixture::with_config(cfg, clock, mail).await;
        let sessions = Arc::new(MemoryStore::default());
        let mut ctx = context();
        ctx.session = Session::new(None, sessions.clone(), None);
        let old_user = f.login(&ctx, "cached-old@example.test").await;
        let stale = reopened(&ctx, sessions.clone());
        let stale_writer = reopened(&ctx, sessions.clone());
        let other = Auth::new(f.config.clone(), None).await.unwrap();
        // Independent Session instances have loaded the same cookie before revocation.
        assert_eq!(
            other.current_user(&stale).await.unwrap(),
            Some(old_user.clone())
        );
        assert_eq!(
            other.current_user(&stale_writer).await.unwrap(),
            Some(old_user)
        );
        let next = if replacement {
            Some(f.login(&ctx, "cached-new@example.test").await)
        } else {
            f.auth.logout(&ctx).await.unwrap();
            None
        };
        // Simulate a request that writes its already-cached cookie after revocation.
        stale_writer
            .session
            .insert("late_write", true)
            .await
            .unwrap();
        stale_writer.session.save().await.unwrap();
        let replay = reopened(&stale_writer, sessions);
        for cached in [&replay, &stale, &stale_writer] {
            assert_eq!(other.token(cached).await.unwrap_err().code, "unauthorized");
            assert_eq!(other.current_user(cached).await.unwrap(), None);
        }
        assert_eq!(f.auth.current_user(&ctx).await.unwrap(), next);
        f.config
            .store
            .as_ref()
            .unwrap()
            .transact(&mut |data| {
                assert_eq!(data.logins.len(), usize::from(replacement));
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn cached_cookie_uses_authoritative_idle_activity_and_cleanup_cannot_be_undone() {
    let f = short_lived_fixture().await;
    let sessions = Arc::new(MemoryStore::default());
    let mut ctx = context();
    ctx.session = Session::new(None, sessions.clone(), None);
    let user = f.login(&ctx, "shared-idle@example.test").await;
    let stale = reopened(&ctx, sessions);
    assert_eq!(
        f.auth.current_user(&stale).await.unwrap(),
        Some(user.clone())
    );
    f.clock.advance(Duration::minutes(4));
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), Some(user.clone()));
    f.clock.advance(Duration::minutes(4));
    assert_eq!(f.auth.current_user(&stale).await.unwrap(), Some(user));
    f.clock.advance(Duration::minutes(4));
    let store = f.config.store.as_ref().unwrap();
    assert!(store.cleanup_expired(f.config.clock.now()).await.unwrap() >= 1);
    assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    assert_eq!(f.auth.current_user(&stale).await.unwrap(), None);
    store
        .transact(&mut |data| {
            assert!(data.logins.is_empty());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn login_payload_must_match_authoritative_user_and_authentication_time() {
    for field in ["id", "user", "authenticated"] {
        let f = Fixture::new().await;
        let ctx = context();
        f.login(&ctx, "bound@example.test").await;
        let mut payload = ctx
            .session
            .get::<serde_json::Value>("__gluon_auth")
            .await
            .unwrap()
            .unwrap();
        match field {
            "id" => payload["id"] = serde_json::json!("missing-login"),
            "user" => payload["user"]["id"] = serde_json::json!("another-user"),
            _ => {
                let later = f.config.clock.now() + Duration::seconds(1);
                payload["authenticated"] = serde_json::to_value(later).unwrap();
            }
        }
        ctx.session.insert("__gluon_auth", payload).await.unwrap();
        assert_eq!(f.auth.current_user(&ctx).await.unwrap(), None);
    }
}

#[tokio::test]
async fn login_authority_counts_toward_memory_capacity_and_expiry_reclaims_it() {
    let f = short_lived_fixture().await;
    f.login(&context(), "capacity@example.test").await;
    let mut login = None;
    f.config
        .store
        .as_ref()
        .unwrap()
        .transact(&mut |data| {
            login = data.logins.values().next().cloned();
            Ok(())
        })
        .await
        .unwrap();
    let login = login.unwrap();
    let bounded = MemoryAuthStore::new(1);
    bounded
        .transact(&mut |data| {
            data.logins.insert("first".into(), login.clone());
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        bounded
            .transact(&mut |data| {
                data.logins.insert("second".into(), login.clone());
                Ok(())
            })
            .await
            .unwrap_err()
            .code,
        "storage_capacity"
    );
    bounded.prepare_capacity(login.idle_expires).await.unwrap();
    bounded
        .transact(&mut |data| {
            assert!(data.logins.is_empty());
            data.logins.insert("replacement".into(), login.clone());
            Ok(())
        })
        .await
        .unwrap();
}
