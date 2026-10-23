//! Executable, provisional reconstructed CSRF requirements against public middleware.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use axum::{
    Extension, Router,
    body::Bytes,
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware,
    routing::any,
};
use axum_test::{TestResponse, TestServer};
use cucumber::{World, WriterExt, given, then, when};
use gluon::middleware::{CsrfToken, csrf::csrf_middleware};
use tower_sessions::{
    MemoryStore, Session, SessionManagerLayer, SessionStore,
    session::{Id, Record},
    session_store,
};

const COOKIE: &str = "csrf_spec_sid";

/// Fault injection delegates all healthy operations to the real `MemoryStore`.
#[derive(Clone, Debug, Default)]
struct Store {
    memory: MemoryStore,
    fault: Arc<AtomicUsize>,
    failures: Arc<AtomicUsize>,
}
impl Store {
    fn check(&self, operation: usize) -> session_store::Result<()> {
        if self.fault.load(Ordering::SeqCst) == operation {
            self.failures.fetch_add(1, Ordering::SeqCst);
            Err(session_store::Error::Backend(
                "injected CSRF lifecycle failure".into(),
            ))
        } else {
            Ok(())
        }
    }
}
#[async_trait::async_trait]
impl SessionStore for Store {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        self.check(2)?;
        self.memory.create(record).await
    }
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.check(3)?;
        self.memory.save(record).await
    }
    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        self.check(1)?;
        self.memory.load(id).await
    }
    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        self.memory.delete(id).await
    }
}

#[derive(Default, World)]
struct CsrfWorld {
    server: Option<TestServer>,
    store: Store,
    effects: Arc<AtomicUsize>,
    observed_body: Arc<std::sync::Mutex<Vec<u8>>>,
    method: Option<Method>,
    token: String,
    session_cookie: Option<tower_sessions::cookie::Cookie<'static>>,
    response: Option<TestResponse>,
    body: Vec<u8>,
    baseline: usize,
    expected_effects: usize,
}
impl fmt::Debug for CsrfWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CsrfWorld")
            .field("handler_effects", &self.effects.load(Ordering::SeqCst))
            .field(
                "status",
                &self.response.as_ref().map(TestResponse::status_code),
            )
            .field("body_len", &self.body.len())
            .finish_non_exhaustive()
    }
}
impl CsrfWorld {
    fn client(&self) -> &TestServer {
        self.server.as_ref().unwrap()
    }
    fn count(&self) -> usize {
        self.effects.load(Ordering::SeqCst)
    }
    fn response(&self) -> &TestResponse {
        self.response.as_ref().unwrap()
    }
    async fn token(&self) -> String {
        let response = self.client().get("/").await;
        assert_eq!(response.status_code(), StatusCode::OK);
        response.headers()["x-observed-csrf"]
            .to_str()
            .unwrap()
            .to_owned()
    }
    async fn submit(
        &mut self,
        method: Method,
        header: Option<HeaderValue>,
        mime: &str,
        body: Vec<u8>,
    ) {
        self.baseline = self.count();
        self.method = Some(method.clone());
        self.body = body;
        let mut request = self
            .client()
            .method(method, "/")
            .content_type(mime)
            .bytes(Bytes::copy_from_slice(&self.body));
        if let Some(value) = header {
            request = request.add_header("x-csrf-token", value);
        }
        self.response = Some(request.await);
    }
    fn assert_result(&self, status: u16) {
        let response = self.response();
        assert_eq!(response.status_code().as_u16(), status, "{self:?}");
        if status == 200 {
            assert_eq!(self.count(), self.baseline + 1);
            assert_eq!(response.headers()["x-observed-csrf"], self.token);
            assert_eq!(
                *self.observed_body.lock().unwrap(),
                self.body,
                "downstream body must be byte-exact"
            );
            // Axum intentionally strips successful CONNECT response bodies.
            if self.method.as_ref() != Some(&Method::CONNECT) {
                assert_eq!(
                    response.as_bytes().as_ref(),
                    self.body,
                    "echo must be byte-exact"
                );
            }
        } else {
            assert_eq!(
                self.count(),
                self.baseline,
                "rejected request invoked handler"
            );
            assert!(!response.headers().contains_key("x-observed-csrf"));
        }
    }
}

#[given("an isolated production CSRF application")]
fn application(world: &mut CsrfWorld) {
    let effects = world.effects.clone();
    let observed_body = world.observed_body.clone();
    let app = Router::new()
        .route(
            "/",
            any(
                move |Extension(token): Extension<CsrfToken>,
                      session: Session,
                      headers: HeaderMap,
                      body: Bytes| {
                    let effects = effects.clone();
                    let observed_body = observed_body.clone();
                    async move {
                        *observed_body.lock().unwrap() = body.to_vec();
                        effects.fetch_add(1, Ordering::SeqCst);
                        if headers.contains_key("x-touch-session") {
                            session.insert("handler-write", true).await.unwrap();
                        }
                        ([("x-observed-csrf", token.as_str().to_owned())], body)
                    }
                },
            ),
        )
        // The harness extractor must not impose Axum's unrelated 2 MiB limit.
        .layer(axum::extract::DefaultBodyLimit::disable())
        .layer(middleware::from_fn(csrf_middleware))
        .layer(
            SessionManagerLayer::new(world.store.clone())
                .with_name(COOKIE)
                .with_secure(false),
        );
    let mut server = TestServer::new(app);
    server.save_cookies();
    world.server = Some(server);
}

#[given("a persisted session token")]
async fn persisted(world: &mut CsrfWorld) {
    let response = world.client().get("/").await;
    assert_eq!(response.status_code(), StatusCode::OK);
    world.session_cookie = response.cookies().get(COOKIE).cloned();
    response.headers()["x-observed-csrf"]
        .to_str()
        .unwrap()
        .clone_into(&mut world.token);
    assert_eq!(world.token.len(), 64);
    assert!(
        world
            .token
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    );
    assert_eq!(world.token().await, world.token);
}

#[when(regex = r"^a fresh client requests (\S+)$")]
async fn fresh(world: &mut CsrfWorld, method: String) {
    world.baseline = world.count();
    world.response = Some(world.client().method(method.parse().unwrap(), "/").await);
    assert!(
        world.response().cookies().get(COOKIE).is_some(),
        "session must persist even on first unsafe rejection"
    );
}

#[then("a persisted token is issued and reused")]
async fn issued(world: &mut CsrfWorld) {
    assert_eq!(world.response().status_code(), StatusCode::OK);
    assert_eq!(world.count(), world.baseline + 1);
    let token = world.response().headers()["x-observed-csrf"]
        .to_str()
        .unwrap()
        .to_owned();
    persisted(world).await;
    assert_eq!(token, world.token);
    let value = world.token.parse().unwrap();
    world
        .submit(
            Method::POST,
            Some(value),
            "application/octet-stream",
            vec![],
        )
        .await;
    world.assert_result(200);
}

#[then("the fresh unsafe request is rejected and its token can be reused")]
async fn fresh_rejected(world: &mut CsrfWorld) {
    world.assert_result(403);
    persisted(world).await;
    world
        .submit(
            Method::POST,
            Some(world.token.parse().unwrap()),
            "text/plain",
            b"after first rejection".to_vec(),
        )
        .await;
    world.assert_result(200);
}

#[when(regex = r"^the client exercises missing wrong and valid tokens with (\S+)$")]
async fn methods(world: &mut CsrfWorld, method: String) {
    let method: Method = method.parse().unwrap();
    let initial = world.count();
    for header in [
        None,
        Some(HeaderValue::from_static("wrong")),
        Some(world.token.parse().unwrap()),
    ] {
        let expected = if header.as_ref().is_some_and(|h| h == world.token.as_str()) {
            200
        } else {
            403
        };
        world
            .submit(
                method.clone(),
                header,
                "application/octet-stream",
                b"method payload".to_vec(),
            )
            .await;
        world.assert_result(expected);
    }
    world.expected_effects = initial + 1;
}

#[then("only the valid request reaches the handler")]
fn admitted_only(world: &mut CsrfWorld) {
    assert_eq!(world.count(), world.expected_effects);
}

#[when(regex = r"^the client submits the (\S+) token selection case$")]
async fn selection(world: &mut CsrfWorld, case: String) {
    let token = &world.token;
    let mut body = format!("payload=a+b%00%FF&csrf=ignored&_csrf={token}");
    let mut header = None;
    let mut mime = " Application/X-WWW-Form-Urlencoded ; charset=UTF-8";
    match case.as_str() {
        "trimmed-header" => header = Some(format!(" \t{token}\t ").parse().unwrap()),
        "wrong-header-wins" => header = Some(HeaderValue::from_static("wrong")),
        "valid-header-wins" => {
            header = Some(token.parse().unwrap());
            body = "_csrf=wrong".into();
        }
        "empty-header-fallback" => header = Some(HeaderValue::from_static(" \t ")),
        "invalid-header-fallback" => header = Some(HeaderValue::from_bytes(&[0xff]).unwrap()),
        "encoded-form" => {
            use std::fmt::Write as _;
            let encoded = token.bytes().fold(String::new(), |mut encoded, byte| {
                write!(encoded, "%{byte:02X}").unwrap();
                encoded
            });
            body = format!("%5Fcsrf={encoded}&payload=a+b%2B%00%FF");
        }
        "first-valid-duplicate" => body = format!("_csrf={token}&_csrf=wrong"),
        "first-wrong-duplicate" => body = format!("_csrf=wrong&_csrf={token}"),
        "first-empty-duplicate" => body = format!("_csrf=&_csrf={token}"),
        "non-form" => mime = "application/json",
        "form-token-not-trimmed" => body = format!("_csrf=+{token}+"),
        _ => panic!("unknown selection case: {case}"),
    }
    world
        .submit(Method::POST, header, mime, body.into_bytes())
        .await;
}

#[then(regex = r"^the selection result is (\d+)$")]
fn selection_result(world: &mut CsrfWorld, status: u16) {
    world.assert_result(status);
}

#[when(regex = r"^a body of (\d+) bytes is sent with a (\S+) token$")]
async fn limit(world: &mut CsrfWorld, size: usize, token: String) {
    let header = match token.as_str() {
        "valid" => Some(world.token.parse().unwrap()),
        "wrong" => Some(HeaderValue::from_static("wrong")),
        "missing" => None,
        _ => panic!("unknown token case"),
    };
    world
        .submit(
            Method::POST,
            header,
            "application/octet-stream",
            vec![0xa5; size],
        )
        .await;
}

#[then("the safe request preserves its payload and session token")]
async fn safe_preserved(world: &mut CsrfWorld) {
    world.assert_result(200);
    assert_eq!(world.token().await, world.token);
}

#[when("binary requests are admitted and rejected")]
async fn binary(world: &mut CsrfWorld) {
    let initial = world.count();
    for (header, status) in [
        (Some(world.token.parse().unwrap()), 200),
        (None, 403),
        (Some(HeaderValue::from_static("wrong")), 403),
    ] {
        world
            .submit(
                Method::POST,
                header,
                "application/octet-stream",
                vec![0, 255, 128, 13, 10, b'&', b'+', b'%', 0],
            )
            .await;
        world.assert_result(status);
    }
    world.expected_effects = initial + 1;
}

#[when("a safe GET sends 4194305 bytes with a wrong header token")]
async fn safe_oversized(world: &mut CsrfWorld) {
    world
        .submit(
            Method::GET,
            Some(HeaderValue::from_static("wrong")),
            "application/octet-stream",
            vec![0xa5; 4 * 1024 * 1024 + 1],
        )
        .await;
}

#[when("a second session attempts to use the first session token")]
async fn cross_session(world: &mut CsrfWorld) {
    // Same app and store, different cookie jar, not a different backend.
    let first_cookie = world.session_cookie.clone().unwrap();
    world.server.as_mut().unwrap().clear_cookies();
    let second = world.token().await;
    assert_ne!(world.token, second);
    world
        .submit(
            Method::POST,
            Some(world.token.parse().unwrap()),
            "text/plain",
            b"cross-session".to_vec(),
        )
        .await;
    world.assert_result(403);
    let first = std::mem::replace(&mut world.token, second);
    world
        .submit(
            Method::POST,
            Some(world.token.parse().unwrap()),
            "text/plain",
            b"own-session".to_vec(),
        )
        .await;
    world.assert_result(200);
    assert_eq!(world.token().await, world.token);
    assert_ne!(first, world.token);
    let second_token = world.token.clone();
    let server = world.server.as_mut().unwrap();
    server.clear_cookies();
    server.add_cookie(first_cookie);
    world
        .submit(
            Method::POST,
            Some(second_token.parse().unwrap()),
            "text/plain",
            b"reverse-cross-session".to_vec(),
        )
        .await;
    world.assert_result(403);
    world.token = first;
    assert_eq!(
        world.token().await,
        world.token,
        "original session must persist independently"
    );
    world
        .submit(
            Method::POST,
            Some(world.token.parse().unwrap()),
            "text/plain",
            b"original-session".to_vec(),
        )
        .await;
    world.assert_result(200);
}

#[then("the clients remain isolated")]
fn isolated(world: &mut CsrfWorld) {
    assert_eq!(world.store.failures.load(Ordering::SeqCst), 0);
}

#[when(regex = r"^the session store fails during (\S+)$")]
async fn failure(world: &mut CsrfWorld, operation: String) {
    let fault = match operation.as_str() {
        "load" => 1,
        "create" => 2,
        "save" => 3,
        _ => panic!("unknown store operation"),
    };
    if fault == 2 {
        world.server.as_mut().unwrap().clear_cookies();
    }
    world.store.fault.store(fault, Ordering::SeqCst);
    world.baseline = world.count();
    // Load fails within CSRF before admission. Create/save are deferred by
    // SessionManagerLayer until after the handler, so they cannot roll it back.
    world.expected_effects = world.baseline + usize::from(fault != 1);
    let method = if fault == 2 {
        Method::GET
    } else {
        Method::POST
    };
    world.response = Some(
        world
            .client()
            .method(method, "/")
            .add_header("x-csrf-token", world.token.clone())
            .add_header("x-touch-session", "yes")
            .await,
    );
}

#[then("the backend failure returns an internal error")]
fn backend_error(world: &mut CsrfWorld) {
    assert_eq!(
        world.response().status_code(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        world.store.failures.load(Ordering::SeqCst),
        1,
        "must hit actual store failure"
    );
    assert_eq!(world.count(), world.expected_effects);
    assert!(
        world.response().cookies().get(COOKIE).is_none(),
        "failed persistence must not publish a cookie"
    );
}

#[when("the persisted CSRF token has a non-string value")]
async fn malformed_token(world: &mut CsrfWorld) {
    let id: Id = world
        .session_cookie
        .as_ref()
        .unwrap()
        .value()
        .parse()
        .unwrap();
    let mut record = world.store.memory.load(&id).await.unwrap().unwrap();
    record
        .data
        .insert("_gluon_csrf_token".into(), serde_json::json!(42));
    world.store.memory.save(&record).await.unwrap();
    world
        .submit(
            Method::POST,
            Some(world.token.parse().unwrap()),
            "text/plain",
            b"must not reach the handler".to_vec(),
        )
        .await;
    world.assert_result(500);
    assert_eq!(world.store.failures.load(Ordering::SeqCst), 0);
    let unchanged = world.store.memory.load(&id).await.unwrap().unwrap();
    assert_eq!(
        unchanged.data, record.data,
        "read error must not rotate state"
    );
}

#[tokio::main]
async fn main() {
    let report_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/csrf-spec");
    std::fs::create_dir_all(&report_dir).unwrap();
    let report = std::fs::File::create(report_dir.join("junit.xml")).unwrap();
    let writer = cucumber::writer::Basic::stdout().summarized().tee(
        cucumber::writer::JUnit::new(report, cucumber::writer::Verbosity::ShowWorld)
            .discard_stats_writes(),
    );
    CsrfWorld::cucumber()
        .with_writer(writer)
        // This is a mandatory regression suite, not a filterable CLI. Ignore
        // libtest arguments and prevent --dry-run/name/tag filters skipping it.
        .with_default_cli()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/features/csrf.feature"
        ))
        .await;
}
