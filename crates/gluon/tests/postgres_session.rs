#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use axum::{Router, routing::get};
use axum_test::TestServer;
use gluon::PostgresSessionStore;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use time::{Duration, OffsetDateTime};
use tower_sessions::cookie::Key;
use tower_sessions::session::{Id, Record};
use tower_sessions::{Session, SessionManagerLayer, SessionStore};

async fn counter(session: Session) -> String {
    let count: usize = session.get("count").await.unwrap().unwrap_or_default();
    session.insert("count", count + 1).await.unwrap();
    count.to_string()
}

fn server(store: PostgresSessionStore, key: Key) -> TestServer {
    TestServer::new(
        Router::new().route("/", get(counter)).layer(
            SessionManagerLayer::new(store)
                .with_name("gluon_sid")
                .with_secure(false)
                .with_signed(key),
        ),
    )
}

#[tokio::test]
async fn postgres_sessions_persist_across_instances() {
    let container = Postgres::default().start().await.expect("start postgres");
    let port = container
        .get_host_port_ipv4(5432)
        .await
        .expect("postgres port");
    let pool = sqlx::PgPool::connect(&format!(
        "postgres://postgres:postgres@127.0.0.1:{port}/postgres"
    ))
    .await
    .expect("connect postgres");
    let store = PostgresSessionStore::new(pool.clone());
    store.migrate().await.expect("migrate session table");

    let expired_id = Id::default();
    sqlx::query("INSERT INTO gluon_sessions (id, data, expiry) VALUES ($1, $2, $3)")
        .bind(expired_id.to_string())
        .bind("{}")
        .bind(OffsetDateTime::now_utc() - Duration::seconds(1))
        .execute(&pool)
        .await
        .expect("insert expired session");
    assert!(store.load(&expired_id).await.unwrap().is_none());

    let mut deletable = Record {
        id: Id::default(),
        data: std::collections::HashMap::default(),
        expiry_date: OffsetDateTime::now_utc() + Duration::minutes(5),
    };
    store.create(&mut deletable).await.unwrap();
    let expired_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM gluon_sessions WHERE id = $1")
            .bind(expired_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(expired_count, 0);
    assert!(store.load(&deletable.id).await.unwrap().is_some());
    store.delete(&deletable.id).await.unwrap();
    assert!(store.load(&deletable.id).await.unwrap().is_none());

    let key = Key::generate();
    let first_server = server(store.clone(), key.clone());
    let second_server = server(store, key);

    let first = first_server.get("/").await;
    first.assert_text("0");
    let cookie = first.cookies().get("gluon_sid").unwrap().value().to_owned();

    let second = second_server
        .get("/")
        .add_header("cookie", format!("gluon_sid={cookie}"))
        .await;
    second.assert_text("1");

    let third = first_server
        .get("/")
        .add_header("cookie", format!("gluon_sid={cookie}"))
        .await;
    third.assert_text("2");
}
