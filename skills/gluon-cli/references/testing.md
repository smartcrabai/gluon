# テスト助け

## 3 層のテスト規約

| レベル | 場所 | 何をするか |
|---|---|---|
| Domain unit | `tests/domain/<name>.rs` | Entity の等価性・Value Object のバリデーション |
| UseCase unit | `tests/usecases/<name>.rs` | Mock Repository を組み立てて `execute(input)` を呼び、output / error を assert |
| Controller integration | `tests/controllers/<route>.rs` | `gluon::testing::TestClient` で axum Router 全体をブートし HTTP request → response 検証 |

注: 現状 `gluon g` はテストファイルを生成しない(MVP)。各 generate と一緒に手でテスト雛形を作る。

## `gluon::testing::TestClient`

`axum-test::TestServer` の薄いラッパー。Container と Router を持ってアプリ全体をインメモリで起動する。

```rust
use gluon::testing::TestClient;
use gluon::ContainerBuilder;
use std::sync::Arc;

#[gluon::gluon_test]
async fn lists_users() {
    let container = ContainerBuilder::new()
        // mock を bind
        .bind::<dyn crate::domain::user::UserRepository, _>(|_| {
            Arc::new(crate::infrastructure::mocks::user_repository::MockUserRepository::new())
        })
        .build();
    let router = my_app::__gluon_router();
    let client = TestClient::new(router, container).expect("test server");

    let resp = client.server().get("/users").await;
    assert_eq!(resp.status_code(), 200);
}
```

## `#[gluon::gluon_test]` 属性マクロ

`#[tokio::test]` を巻きつつ、必要に応じて tracing init を追加できる proc-macro。`#[gluon::gluon_test]` で `async fn` を装飾するだけ。

```rust
#[gluon::gluon_test]
async fn flash_round_trip() { /* ... */ }
```

## DB を使うテスト

- `DATABASE_URL` を test 用 DB に向ける
- 各テストで transaction を貼り、終了時に rollback するヘルパを自前で用意するのが現状の推奨(framework 提供の `with_db` ヘルパは未実装)

## CSRF を伴うテスト

- `TestClient` は session cookie を維持するため、GET → token を session から取得 → 同じ client で POST する流れ
- `tower-sessions` の MemoryStore は test プロセス内で完結するので、複数テスト間で session を分けるには TestClient を都度 new する

## Container override

```rust
let mut container = ContainerBuilder::new()
    .bind::<dyn UserRepository, _>(|_| Arc::new(PostgresUserRepository::new(...)))
    .build();
// テスト中だけ差し替えたい場合
container.override_with::<dyn UserRepository>(Arc::new(MockUserRepository::new()));
```

`override_with` は build 後の Container を `&mut` で変更するため、ハンドラから見える Arc<Container> をテスト毎に作り直す必要がある(`Arc::make_mut` は不可)。
