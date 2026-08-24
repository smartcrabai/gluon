# Testing Help

## 3-Layer Test Conventions

| Level | Location | What it does |
|---|---|---|
| Domain unit | `tests/domain/<name>.rs` | Entity equality and Value Object validation |
| UseCase unit | `tests/usecases/<name>.rs` | Assemble a Mock Repository, call `execute(input)`, and assert output / error |
| Controller integration | `tests/controllers/<route>.rs` | Boot the entire axum Router with `gluon::testing::TestClient` and verify HTTP request -> response |

Note: `gluon g` does not generate test files yet (MVP). Create test scaffolding by hand alongside each generate.

## `gluon::testing::TestClient`

A thin wrapper around `axum-test::TestServer`. Boots the whole application in-memory with a Container and Router.

```rust
use gluon::testing::TestClient;
use gluon::ContainerBuilder;
use std::sync::Arc;

#[gluon::gluon_test]
async fn lists_users() {
    let container = ContainerBuilder::new()
        // bind a mock
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

## The `#[gluon::gluon_test]` Attribute Macro

A proc-macro that wraps `#[tokio::test]` and optionally adds tracing init. Just decorate an `async fn` with `#[gluon::gluon_test]`.

```rust
#[gluon::gluon_test]
async fn flash_round_trip() { /* ... */ }
```

## Tests Using the DB

- Point `DATABASE_URL` at a test database
- The current recommendation is to provide your own helper that opens a transaction per test and rolls back on exit (the framework-provided `with_db` helper is not implemented yet)

## Testing with CSRF

- `TestClient` keeps session cookies, so the flow is: GET -> fetch the token from the session -> POST with the same client
- `tower-sessions`' MemoryStore lives entirely within the test process, so to isolate sessions between tests, create a new TestClient each time

## Container Override

```rust
let mut container = ContainerBuilder::new()
    .bind::<dyn UserRepository, _>(|_| Arc::new(PostgresUserRepository::new(...)))
    .build();
// swap only for this test
container.override_with::<dyn UserRepository>(Arc::new(MockUserRepository::new()));
```

Because `override_with` mutates the built Container via `&mut`, you need to rebuild the Arc<Container> visible to handlers for each test (`Arc::make_mut` cannot be used).
