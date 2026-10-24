#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use axum::{Router, routing::get};
use axum_test::TestServer;
use gluon::PostgresSessionStore;
use serde_json::json;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use time::{Duration, OffsetDateTime};
use tower_sessions::cookie::Key;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::Error as SessionStoreError;
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

    assert_snapshot_save_is_last_write_wins(&store).await;

    let corrupt_id = Id::default();
    sqlx::query("INSERT INTO gluon_sessions (id, data, expiry) VALUES ($1, $2, $3)")
        .bind(corrupt_id.to_string())
        .bind("{not-json")
        .bind(OffsetDateTime::now_utc() + Duration::minutes(5))
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        store.load(&corrupt_id).await,
        Err(SessionStoreError::Decode(_))
    ));

    let key = Key::generate();
    let first_server = server(store.clone(), key.clone());
    let second_server = server(store.clone(), key);

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

    pool.close().await;
    assert!(matches!(
        store.load(&deletable.id).await,
        Err(SessionStoreError::Backend(_))
    ));
}

async fn assert_snapshot_save_is_last_write_wins(store: &PostgresSessionStore) {
    let mut conflicted = Record {
        id: Id::default(),
        data: std::collections::HashMap::from([("value".to_owned(), json!("initial"))]),
        expiry_date: OffsetDateTime::now_utc() + Duration::minutes(5),
    };
    store.create(&mut conflicted).await.unwrap();
    let mut isolated = Record {
        id: Id::default(),
        data: std::collections::HashMap::from([("value".to_owned(), json!("isolated"))]),
        expiry_date: OffsetDateTime::now_utc() + Duration::minutes(5),
    };
    store.create(&mut isolated).await.unwrap();

    let mut snapshot_a = store.load(&conflicted.id).await.unwrap().unwrap();
    let mut snapshot_b = store.load(&conflicted.id).await.unwrap().unwrap();
    snapshot_a.data.insert("value".to_owned(), json!("save A"));
    snapshot_a.data.insert("only_a".to_owned(), json!(true));
    snapshot_b.data.insert("value".to_owned(), json!("save B"));
    snapshot_b.data.insert("only_b".to_owned(), json!(true));
    store.save(&snapshot_a).await.unwrap();
    store.save(&snapshot_b).await.unwrap();

    let saved = store.load(&conflicted.id).await.unwrap().unwrap();
    assert_eq!(
        saved.data.get("value").and_then(serde_json::Value::as_str),
        Some("save B")
    );
    assert_eq!(saved.data.get("only_b"), Some(&json!(true)));
    assert!(!saved.data.contains_key("only_a"));
    let isolated_after = store.load(&isolated.id).await.unwrap().unwrap();
    assert_eq!(
        isolated_after
            .data
            .get("value")
            .and_then(serde_json::Value::as_str),
        Some("isolated")
    );
}
