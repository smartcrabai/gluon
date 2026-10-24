//! Plan §6 / §10.2: real PostgreSQL, never a mock or a silent Docker skip.
//! Run with Docker reachable: cargo test -p gluon --features auth --test `auth_postgres`.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "common/auth/mod.rs"]
mod common;
#[path = "common/auth/jwt.rs"]
mod keys;

use common::{Mailbox, TestClock, config};
use gluon::{
    PostgresSessionStore,
    auth::{Auth, AuthConfig, AuthContext, Clock, PostgresAuthStore},
};
use std::sync::Arc;
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{ContainerAsync, runners::AsyncRunner},
};
use time::Duration;
use tower_sessions::Session;

struct PausedSender {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

#[async_trait::async_trait]
impl gluon::auth::EmailSender for PausedSender {
    async fn send(&self, _email: gluon::auth::OtpEmail) -> Result<(), gluon::auth::AuthError> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(())
    }
}

struct Database {
    _container: ContainerAsync<Postgres>,
    pool: sqlx::PgPool,
}
impl Database {
    async fn new() -> Self {
        let container = Postgres::default()
            .start()
            .await
            .expect("start postgres, Docker is required");
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(&format!(
                "postgres://postgres:postgres@127.0.0.1:{port}/postgres"
            ))
            .await
            .expect("PostgreSQL container published port must be reachable");
        Self {
            _container: container,
            pool,
        }
    }
    async fn config(&self, clock: Arc<TestClock>, mail: Arc<Mailbox>) -> AuthConfig {
        let store = Arc::new(PostgresAuthStore::new(self.pool.clone()));
        store.migrate().await.unwrap();
        let mut config = config(clock, mail);
        config.store = Some(store);
        config
    }
    async fn context(&self) -> AuthContext {
        let sessions = PostgresSessionStore::new(self.pool.clone());
        sessions.migrate().await.unwrap();
        AuthContext {
            session: Session::new(None, Arc::new(sessions), None),
            source: "192.0.2.11".parse().unwrap(),
            browser_binding: "database-browser".into(),
        }
    }
}

#[tokio::test]
async fn postgres_auth_store_contracts_share_flow_and_login_across_instances() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut config = db.config(clock, mail.clone()).await;
    let first = Auth::new(config.clone(), None).await.unwrap();
    let ctx = db.context().await;
    let challenge = first
        .request_email_otp(&ctx, "persist@example.test")
        .await
        .unwrap();
    drop(first);
    config.store = Some(Arc::new(PostgresAuthStore::new(db.pool.clone())));
    let second = Auth::new(config.clone(), None).await.unwrap();
    let user = second
        .verify_email_otp(&ctx, &challenge.challenge_id, &mail.last().code)
        .await
        .unwrap();
    ctx.session.save().await.unwrap();
    let restored = AuthContext {
        session: Session::new(
            ctx.session.id(),
            Arc::new(PostgresSessionStore::new(db.pool.clone())),
            None,
        ),
        ..ctx.clone()
    };
    config.store = Some(Arc::new(PostgresAuthStore::new(db.pool.clone())));
    let third = Auth::new(config, None).await.unwrap();
    assert_eq!(third.current_user(&restored).await.unwrap(), Some(user));
    third.logout(&restored).await.unwrap();
    let replay = AuthContext {
        session: Session::new(
            ctx.session.id(),
            Arc::new(PostgresSessionStore::new(db.pool.clone())),
            None,
        ),
        ..ctx
    };
    assert!(second.current_user(&replay).await.unwrap().is_none());
}

#[tokio::test]
async fn postgres_auth_store_contracts_consume_challenge_once_across_independent_pools() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let first_config = db.config(clock, mail.clone()).await;
    let other_pool = sqlx::PgPool::connect_with(db.pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let mut second_config = first_config.clone();
    second_config.store = Some(Arc::new(PostgresAuthStore::new(other_pool)));
    let first = Auth::new(first_config, None).await.unwrap();
    let second = Auth::new(second_config, None).await.unwrap();
    let ctx = db.context().await;
    let challenge = first
        .request_email_otp(&ctx, "once@example.test")
        .await
        .unwrap();
    let email = mail.last();
    let other = db.context().await;
    let (a, b) = tokio::join!(
        first.verify_email_otp(&ctx, &challenge.challenge_id, &email.code),
        second.verify_email_otp(&other, &challenge.challenge_id, &email.code),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
}

#[tokio::test]
async fn postgres_auth_store_contracts_rate_limit_is_shared_across_instances() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut config = db.config(clock, mail).await;
    config.email_otp.as_mut().unwrap().source_limit = 1;
    let first = Auth::new(config.clone(), None).await.unwrap();
    config.store = Some(Arc::new(PostgresAuthStore::new(db.pool.clone())));
    let second = Auth::new(config, None).await.unwrap();
    first
        .request_email_otp(&db.context().await, "first@example.test")
        .await
        .unwrap();
    let result = second
        .request_email_otp(&db.context().await, "second@example.test")
        .await;
    assert_eq!(result.unwrap_err().code, "rate_limited");
}

#[tokio::test]
async fn postgres_auth_store_contracts_parallel_requests_cannot_overspend_source_budget() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let mut config = db.config(clock, mail.clone()).await;
    config.email_otp.as_mut().unwrap().source_limit = 1;
    let first = Auth::new(config.clone(), None).await.unwrap();
    config.store = Some(Arc::new(PostgresAuthStore::new(db.pool.clone())));
    let second = Auth::new(config, None).await.unwrap();
    let a = db.context().await;
    let b = db.context().await;
    let (a, b) = tokio::join!(
        first.request_email_otp(&a, "budget-a@example.test"),
        second.request_email_otp(&b, "budget-b@example.test"),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert_eq!(
        if a.is_err() { a } else { b }.unwrap_err().code,
        "rate_limited"
    );
    assert_eq!(mail.messages.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn postgres_auth_store_contracts_do_not_persist_plaintext_otp() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let auth = Auth::new(db.config(clock, mail.clone()).await, None)
        .await
        .unwrap();
    auth.request_email_otp(&db.context().await, "secret@example.test")
        .await
        .unwrap();
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(challenge)::text FROM gluon_auth_challenges AS challenge",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert!(!rows.is_empty(), "flow must actually be persisted");
    assert!(
        rows.iter().all(|row| !row.contains(&mail.last().code)),
        "plaintext OTP leaked into persisted challenge"
    );
}

#[tokio::test]
async fn postgres_auth_store_contracts_cleanup_removes_expired_flows_only() {
    let db = Database::new().await;
    let clock = Arc::new(TestClock::new());
    let mail = Arc::new(Mailbox::default());
    let config = db.config(clock.clone(), mail.clone()).await;
    let store = config.store.clone().unwrap();
    let auth = Auth::new(config, None).await.unwrap();
    let ctx = db.context().await;
    let old = auth
        .request_email_otp(&ctx, "old@example.test")
        .await
        .unwrap();
    let old_code = mail.last().code;
    clock.advance(Duration::minutes(5));
    let fresh = auth
        .request_email_otp(&ctx, "fresh@example.test")
        .await
        .unwrap();
    let fresh_code = mail.last().code;
    assert!(store.cleanup_expired(clock.now()).await.unwrap() >= 1);
    assert!(
        auth.verify_email_otp(&ctx, &old.challenge_id, &old_code)
            .await
            .is_err()
    );
    assert!(
        auth.verify_email_otp(&ctx, &fresh.challenge_id, &fresh_code)
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn postgres_auth_store_contracts_concurrent_migration_preserves_application_history() {
    let db = Database::new().await;
    sqlx::query(
        "CREATE TABLE _sqlx_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL)",
    )
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO _sqlx_migrations VALUES (9001, 'application sentinel')")
        .execute(&db.pool)
        .await
        .unwrap();
    let first = PostgresAuthStore::new(db.pool.clone());
    let second = PostgresAuthStore::new(db.pool.clone());
    let (a, b) = tokio::join!(first.migrate(), second.migrate());
    a.unwrap();
    b.unwrap();
    first.check_schema().await.unwrap();
    let app_history: Vec<(i64, String)> =
        sqlx::query_as("SELECT version, description FROM _sqlx_migrations")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(app_history, vec![(9001, "application sentinel".into())]);
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gluon_auth_schema_migrations")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(history > 0);
    let duplicate_versions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) - COUNT(DISTINCT version) FROM gluon_auth_schema_migrations",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(duplicate_versions, 0);
}

#[tokio::test]
async fn postgres_auth_store_contracts_schema_check_never_applies_migrations() {
    let db = Database::new().await;
    let store = PostgresAuthStore::new(db.pool.clone());
    assert!(store.check_schema().await.is_err());
    let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('gluon_auth_users')::text")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(table.is_none());
}

#[tokio::test]
async fn postgres_auth_store_contracts_delivery_wait_does_not_hold_flow_locks() {
    let db = Database::new().await;
    let mut config = db
        .config(Arc::new(TestClock::new()), Arc::new(Mailbox::default()))
        .await;
    let sender = Arc::new(PausedSender {
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    config.email_otp.as_mut().unwrap().sender = sender.clone();
    let auth = Arc::new(Auth::new(config, None).await.unwrap());
    let ctx = db.context().await;
    let pending = tokio::spawn(async move {
        auth.request_email_otp(&ctx, "slow-delivery@example.test")
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), sender.entered.notified())
        .await
        .expect("sender reached without sleeps");
    // A different connection can see and lock the pending flow while delivery
    // is still blocked. An uncommitted transaction would hide or lock the row.
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT row_to_json(challenge)::text FROM gluon_auth_challenges AS challenge FOR UPDATE NOWAIT"
    ).fetch_all(&db.pool).await.unwrap();
    sender.release.notify_one();
    pending.await.unwrap().unwrap();
    assert!(
        !rows.is_empty(),
        "challenge must be committed before external delivery waits"
    );
}

#[tokio::test]
async fn postgres_auth_store_contracts_failed_migration_rolls_back_and_is_retryable() {
    let db = Database::new().await;
    sqlx::query("CREATE TABLE application_data (id BIGINT PRIMARY KEY)")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO application_data VALUES (42)")
        .execute(&db.pool)
        .await
        .unwrap();
    // Inject a database failure, not a migration implementation or call-order mock.
    sqlx::raw_sql("CREATE SEQUENCE auth_test_ddl_counter; CREATE FUNCTION reject_auth_ddl() RETURNS event_trigger LANGUAGE plpgsql AS $$ BEGIN IF EXISTS (SELECT 1 FROM pg_event_trigger_ddl_commands() WHERE object_identity LIKE '%gluon_auth_%') AND nextval('auth_test_ddl_counter') > 1 THEN RAISE EXCEPTION 'injected authentication DDL failure'; END IF; END $$; CREATE EVENT TRIGGER reject_auth_ddl ON ddl_command_end WHEN TAG IN ('CREATE TABLE') EXECUTE FUNCTION reject_auth_ddl();")
        .execute(&db.pool).await.unwrap();
    let store = PostgresAuthStore::new(db.pool.clone());
    assert!(store.migrate().await.is_err());
    let auth_tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pg_tables WHERE schemaname = 'public' AND tablename LIKE 'gluon_auth_%'")
        .fetch_one(&db.pool).await.unwrap();
    assert_eq!(
        auth_tables, 0,
        "DDL and migration history must roll back together"
    );
    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM application_data")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(ids, vec![42]);
    sqlx::query("DROP EVENT TRIGGER reject_auth_ddl")
        .execute(&db.pool)
        .await
        .unwrap();
    store.migrate().await.unwrap();
    store.check_schema().await.unwrap();
}

#[tokio::test]
async fn postgres_preloaded_independent_sessions_cannot_restore_revoked_logins() {
    let db = Database::new().await;
    let mail = Arc::new(Mailbox::default());
    let mut cfg = db.config(Arc::new(TestClock::new()), mail.clone()).await;
    cfg.jwt = Some(keys::jwt_config());
    let first = Auth::new(cfg.clone(), None).await.unwrap();
    let other_pool = sqlx::PgPool::connect_with(db.pool.connect_options().as_ref().clone())
        .await
        .unwrap();
    let mut other_cfg = cfg;
    other_cfg.store = Some(Arc::new(PostgresAuthStore::new(other_pool.clone())));
    let other = Auth::new(other_cfg, None).await.unwrap();
    for replacement in [false, true] {
        let ctx = db.context().await;
        let challenge = first
            .request_email_otp(
                &ctx,
                if replacement {
                    "pg-old-replace@example.test"
                } else {
                    "pg-old-logout@example.test"
                },
            )
            .await
            .unwrap();
        let old_user = first
            .verify_email_otp(&ctx, &challenge.challenge_id, &mail.last().code)
            .await
            .unwrap();
        let stale = AuthContext {
            session: Session::new(
                ctx.session.id(),
                Arc::new(PostgresSessionStore::new(other_pool.clone())),
                None,
            ),
            ..ctx.clone()
        };
        let stale_writer = AuthContext {
            session: Session::new(
                ctx.session.id(),
                Arc::new(PostgresSessionStore::new(other_pool.clone())),
                None,
            ),
            ..ctx.clone()
        };
        assert_eq!(
            other.current_user(&stale).await.unwrap(),
            Some(old_user.clone())
        );
        assert_eq!(
            other.current_user(&stale_writer).await.unwrap(),
            Some(old_user)
        );
        let next = if replacement {
            let challenge = first
                .request_email_otp(&ctx, "pg-new@example.test")
                .await
                .unwrap();
            Some(
                first
                    .verify_email_otp(&ctx, &challenge.challenge_id, &mail.last().code)
                    .await
                    .unwrap(),
            )
        } else {
            first.logout(&ctx).await.unwrap();
            None
        };
        stale_writer
            .session
            .insert("late_write", true)
            .await
            .unwrap();
        stale_writer.session.save().await.unwrap();
        let replay = AuthContext {
            session: Session::new(
                stale_writer.session.id(),
                Arc::new(PostgresSessionStore::new(db.pool.clone())),
                None,
            ),
            ..ctx.clone()
        };
        for cached in [&replay, &stale, &stale_writer] {
            assert_eq!(other.token(cached).await.unwrap_err().code, "unauthorized");
            assert_eq!(other.current_user(cached).await.unwrap(), None);
        }
        assert_eq!(first.current_user(&ctx).await.unwrap(), next);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gluon_auth_logins")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, i64::from(replacement));
    }
}

#[tokio::test]
async fn postgres_login_authority_failure_is_not_anonymous_or_successful_logout() {
    let db = Database::new().await;
    let mail = Arc::new(Mailbox::default());
    let auth = Auth::new(
        db.config(Arc::new(TestClock::new()), mail.clone()).await,
        None,
    )
    .await
    .unwrap();
    let ctx = db.context().await;
    let challenge = auth
        .request_email_otp(&ctx, "pg-failure@example.test")
        .await
        .unwrap();
    let user = auth
        .verify_email_otp(&ctx, &challenge.challenge_id, &mail.last().code)
        .await
        .unwrap();
    let cookie_id = ctx.session.id();
    sqlx::query("ALTER TABLE gluon_auth_logins RENAME TO unavailable_auth_logins")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        auth.current_user(&ctx).await.unwrap_err().code,
        "storage_error"
    );
    assert_eq!(auth.logout(&ctx).await.unwrap_err().code, "storage_error");
    assert_eq!(
        ctx.session.id(),
        cookie_id,
        "failed revocation must not flush the cookie"
    );
    sqlx::query("ALTER TABLE unavailable_auth_logins RENAME TO gluon_auth_logins")
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(auth.current_user(&ctx).await.unwrap(), Some(user));
    auth.logout(&ctx).await.unwrap();
}

#[tokio::test]
async fn postgres_v1_upgrade_adds_login_authority_without_changing_existing_data() {
    let db = Database::new().await;
    sqlx::raw_sql(include_str!("../src/auth/migrations/0001_auth.sql"))
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::raw_sql("CREATE TABLE gluon_auth_schema_migrations (version BIGINT PRIMARY KEY, description TEXT NOT NULL); INSERT INTO gluon_auth_schema_migrations VALUES (1, 'authentication storage v1')")
        .execute(&db.pool).await.unwrap();
    let sentinel = serde_json::json!({"count": 1, "expires": TestClock::new().now()});
    sqlx::query("INSERT INTO gluon_auth_rate_limits VALUES ('sentinel', $1::jsonb)")
        .bind(sentinel.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    let store = PostgresAuthStore::new(db.pool.clone());
    assert!(store.check_schema().await.is_err());
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();
    store.check_schema().await.unwrap();
    let sentinel: String = sqlx::query_scalar("SELECT id FROM gluon_auth_rate_limits")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(sentinel, "sentinel");
    let versions: Vec<i64> =
        sqlx::query_scalar("SELECT version FROM gluon_auth_schema_migrations ORDER BY version")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    assert_eq!(versions, vec![1, 2]);
}

#[tokio::test]
async fn postgres_login_authority_write_failure_cannot_leave_an_authenticated_cookie() {
    let db = Database::new().await;
    let mail = Arc::new(Mailbox::default());
    let auth = Auth::new(
        db.config(Arc::new(TestClock::new()), mail.clone()).await,
        None,
    )
    .await
    .unwrap();
    let ctx = db.context().await;
    let challenge = auth
        .request_email_otp(&ctx, "pg-authority-write@example.test")
        .await
        .unwrap();
    let code = mail.last().code;
    sqlx::raw_sql("CREATE FUNCTION reject_login_write() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected login authority write failure'; END $$; CREATE TRIGGER reject_login_write BEFORE INSERT ON gluon_auth_logins FOR EACH ROW EXECUTE FUNCTION reject_login_write()")
        .execute(&db.pool).await.unwrap();
    assert_eq!(
        auth.verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await
            .unwrap_err()
            .code,
        "storage_error"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gluon_auth_logins")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(auth.current_user(&ctx).await.unwrap(), None);
    assert_eq!(
        auth.verify_email_otp(&ctx, &challenge.challenge_id, &code)
            .await
            .unwrap_err()
            .code,
        "invalid_otp"
    );
}

#[tokio::test]
async fn postgres_login_cleanup_enforces_idle_and_absolute_deadlines_for_cached_cookies() {
    let db = Database::new().await;
    for absolute in [false, true] {
        let clock = Arc::new(TestClock::new());
        let mail = Arc::new(Mailbox::default());
        let mut cfg = db.config(clock.clone(), mail.clone()).await;
        cfg.idle_timeout = Duration::minutes(5);
        cfg.absolute_timeout = Duration::minutes(12);
        let store = cfg.store.clone().unwrap();
        let auth = Auth::new(cfg, None).await.unwrap();
        let ctx = db.context().await;
        let challenge = auth
            .request_email_otp(
                &ctx,
                if absolute {
                    "pg-absolute@example.test"
                } else {
                    "pg-idle@example.test"
                },
            )
            .await
            .unwrap();
        let user = auth
            .verify_email_otp(&ctx, &challenge.challenge_id, &mail.last().code)
            .await
            .unwrap();
        let cached = AuthContext {
            session: Session::new(
                ctx.session.id(),
                Arc::new(PostgresSessionStore::new(db.pool.clone())),
                None,
            ),
            ..ctx.clone()
        };
        assert_eq!(
            auth.current_user(&cached).await.unwrap(),
            Some(user.clone())
        );
        if absolute {
            for _ in 0..2 {
                clock.advance(Duration::minutes(4));
                assert_eq!(auth.current_user(&ctx).await.unwrap(), Some(user.clone()));
            }
            clock.advance(Duration::minutes(4));
        } else {
            clock.advance(Duration::minutes(5));
        }
        assert!(store.cleanup_expired(clock.now()).await.unwrap() >= 1);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gluon_auth_logins")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(auth.current_user(&cached).await.unwrap(), None);
        assert_eq!(auth.current_user(&ctx).await.unwrap(), None);
    }
}
