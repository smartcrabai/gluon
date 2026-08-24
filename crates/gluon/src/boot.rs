//! Application bootstrap: OpenTelemetry initialization, middleware wiring, and
//! axum server startup.

use std::sync::Arc;
use std::{future::Future, io};

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tower_sessions::cookie::{Key, SameSite};
use tower_sessions::service::SignedCookie;
use tower_sessions::{MemoryStore, SessionManagerLayer, SessionStore};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::PostgresSessionStore;
use crate::container::{Container, ContainerBuilder};
use crate::error::{AppError, Result};
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::htmx::htmx_middleware;
use crate::middleware::static_files;

type ContainerFactory = Box<dyn FnOnce(ContainerBuilder) -> ContainerBuilder + Send + 'static>;
type RouterFactory =
    Box<dyn FnOnce(axum::Router<Arc<Container>>) -> axum::Router<Arc<Container>> + Send + 'static>;

/// Bootstraps a gluon application: initializes telemetry, builds the
/// dependency-injection container, wires the standard middleware stack, and
/// serves the axum router.
pub struct Boot {
    container_factory: Option<ContainerFactory>,
    router: Option<axum::Router<Arc<Container>>>,
    router_factory: Option<RouterFactory>,
}

impl Boot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            container_factory: None,
            router: None,
            router_factory: None,
        }
    }

    /// Registers a closure that mutates the [`ContainerBuilder`] before the
    /// container is finalized.
    #[must_use]
    pub fn with_container<F>(mut self, builder: F) -> Self
    where
        F: FnOnce(ContainerBuilder) -> ContainerBuilder + Send + 'static,
    {
        self.container_factory = Some(Box::new(builder));
        self
    }

    /// Registers the application's [`axum::Router`]. Typically this is the
    /// `__gluon_router()` produced by `gluon::app!()`.
    #[must_use]
    pub fn with_router(mut self, router: axum::Router<Arc<Container>>) -> Self {
        self.router = Some(router);
        self
    }

    /// Applies application-specific middleware outside gluon's standard
    /// session, CSRF, and HTMX layers.
    #[must_use]
    pub fn with_middleware<F>(mut self, middleware: F) -> Self
    where
        F: FnOnce(axum::Router<Arc<Container>>) -> axum::Router<Arc<Container>> + Send + 'static,
    {
        self.router_factory = Some(Box::new(middleware));
        self
    }

    /// Initializes tracing (+ optional OpenTelemetry), builds the container,
    /// mounts the session/CSRF/HTMX middleware stack and the `public/` static
    /// asset service, then serves the axum router.
    ///
    /// Environment variables:
    /// - `GLUON_TELEMETRY_DISABLED=1` -- skip OpenTelemetry, fmt subscriber only.
    /// - `GLUON_BIND` -- bind address (default `0.0.0.0:3000`).
    /// - `GLUON_INSECURE_COOKIE=1` -- drop `Secure` on the session cookie (dev only).
    /// - `SECRET_KEY_BASE` -- at least 64 bytes; required unless insecure-cookie
    ///   development mode is enabled.
    /// - `DATABASE_URL` -- when present, a `PgPool` and persistent PostgreSQL
    ///   session store are configured; otherwise sessions use memory storage.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when binding the TCP listener, serving
    /// the axum router, or constructing the `PgPool` from `DATABASE_URL`
    /// fails.
    pub async fn run(self) -> Result<()> {
        init_tracing()?;

        let mut builder = ContainerBuilder::new();

        let database_pool = match std::env::var("DATABASE_URL") {
            Ok(database_url) => {
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .connect_lazy(&database_url)
                    .map_err(|e| AppError::Internal(Box::new(e)))?;
                builder = builder.bind_instance::<sqlx::PgPool>(Arc::new(pool.clone()));
                Some(pool)
            }
            Err(std::env::VarError::NotPresent) => {
                tracing::warn!("DATABASE_URL not set; no PgPool registered in container");
                None
            }
            Err(error) => return Err(AppError::Internal(Box::new(error))),
        };

        if let Some(factory) = self.container_factory {
            builder = factory(builder);
        }
        let container = Arc::new(builder.build());

        let mut router = self.router.unwrap_or_default();

        if let Ok(cwd) = std::env::current_dir() {
            let public_dir = cwd.join("public");
            if public_dir.is_dir() {
                router = router.nest_service("/public", static_files::service(public_dir));
            }
        }

        let secure_cookie = read_secure_cookie_env();
        let session_key = read_session_key(secure_cookie)?;

        let bind_addr = std::env::var("GLUON_BIND").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| AppError::Internal(Box::new(e)))?;

        tracing::info!("gluon listening on {}", bind_addr);

        if let Some(pool) = database_pool {
            let store = PostgresSessionStore::new(pool);
            store
                .migrate()
                .await
                .map_err(|error| AppError::Internal(Box::new(error)))?;
            serve_with_shutdown(
                router,
                container,
                listener,
                store,
                (secure_cookie, session_key),
                self.router_factory,
                shutdown_signal(),
            )
            .await
        } else {
            serve_with_shutdown(
                router,
                container,
                listener,
                MemoryStore::default(),
                (secure_cookie, session_key),
                self.router_factory,
                shutdown_signal(),
            )
            .await
        }
    }
}

impl Default for Boot {
    fn default() -> Self {
        Self::new()
    }
}

/// Initializes the global tracing subscriber. Adds an OpenTelemetry layer when
/// telemetry is enabled, otherwise installs an fmt-only subscriber so log
/// macros still reach stderr.
fn init_tracing() -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer();

    if telemetry_disabled() {
        return tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|e| AppError::Internal(Box::new(e)));
    }

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
        )
        .build()
        .map_err(|e| AppError::Internal(Box::new(e)))?;

    let service_name = std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "gluon".to_string());
    let resource = Resource::builder()
        .with_attribute(KeyValue::new("service.name", service_name))
        .build();

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer("gluon");

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(OpenTelemetryLayer::new(tracer))
        .try_init()
        .map_err(|e| AppError::Internal(Box::new(e)))?;

    opentelemetry::global::set_tracer_provider(provider);
    Ok(())
}

/// Reads `GLUON_INSECURE_COOKIE` and decides whether the session cookie should
/// be marked `Secure`. Returns `true` (secure) when the env var is unset or
/// holds any value other than `1` / `true` (case-insensitive).
pub(crate) fn read_secure_cookie_env() -> bool {
    !std::env::var("GLUON_INSECURE_COOKIE")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

/// Reads `GLUON_TELEMETRY_DISABLED` and decides whether OpenTelemetry should
/// be skipped. Returns `true` when the env var is set to `1` or `true`
/// (case-insensitive).
pub(crate) fn telemetry_disabled() -> bool {
    std::env::var("GLUON_TELEMETRY_DISABLED")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn read_session_key(secure_cookie: bool) -> Result<Key> {
    match std::env::var("SECRET_KEY_BASE") {
        Ok(secret) => {
            Key::try_from(secret.as_bytes()).map_err(|error| AppError::Internal(Box::new(error)))
        }
        Err(std::env::VarError::NotPresent) if secure_cookie => Err(AppError::Internal(Box::new(
            io::Error::other("SECRET_KEY_BASE is required when secure cookies are enabled"),
        ))),
        Err(std::env::VarError::NotPresent) => {
            tracing::warn!("SECRET_KEY_BASE not set; using ephemeral development key");
            Ok(Key::generate())
        }
        Err(error) => Err(AppError::Internal(Box::new(error))),
    }
}

/// Builds the signed session cookie middleware layer used by `Boot::run`.
fn build_session_layer<Store: SessionStore + Clone>(
    store: Store,
    secure: bool,
    key: Key,
) -> SessionManagerLayer<Store, SignedCookie> {
    SessionManagerLayer::new(store)
        .with_name("gluon_sid")
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_secure(secure)
        .with_signed(key)
}

async fn serve_with_shutdown<Store, Shutdown>(
    router: axum::Router<Arc<Container>>,
    container: Arc<Container>,
    listener: tokio::net::TcpListener,
    store: Store,
    session: (bool, Key),
    router_factory: Option<RouterFactory>,
    shutdown: Shutdown,
) -> Result<()>
where
    Store: SessionStore + Clone,
    Shutdown: Future<Output = ()> + Send + 'static,
{
    let mut router = router
        .layer(axum::middleware::from_fn(htmx_middleware))
        .layer(axum::middleware::from_fn(csrf_middleware))
        .layer(build_session_layer(store, session.0, session.1));
    if let Some(factory) = router_factory {
        router = factory(router);
    }
    let app = router.with_state(container);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|error| AppError::Internal(Box::new(error)))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to install SIGTERM handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header};
    use axum::routing::get;
    use axum_test::TestServer;
    use serial_test::serial;
    use tower_sessions::Session;

    const SECURE_KEY: &str = "GLUON_INSECURE_COOKIE";
    const SESSION_KEY: &str = "SECRET_KEY_BASE";
    const TELEMETRY_KEY: &str = "GLUON_TELEMETRY_DISABLED";

    fn clear(key: &str) {
        // SAFETY: tests touching env vars are gated by `#[serial]`, so no
        // other thread is reading/writing the process environment.
        unsafe { std::env::remove_var(key) };
    }

    fn set(key: &str, value: &str) {
        // SAFETY: see `clear`.
        unsafe { std::env::set_var(key, value) };
    }

    #[test]
    #[serial]
    fn read_secure_cookie_env_default_is_true() {
        clear(SECURE_KEY);
        assert!(read_secure_cookie_env());
    }

    #[test]
    #[serial]
    fn read_secure_cookie_env_disabled_when_1() {
        set(SECURE_KEY, "1");
        assert!(!read_secure_cookie_env());
        clear(SECURE_KEY);
    }

    #[test]
    #[serial]
    fn read_secure_cookie_env_disabled_when_true_caseless() {
        set(SECURE_KEY, "TRUE");
        assert!(!read_secure_cookie_env());
        set(SECURE_KEY, "True");
        assert!(!read_secure_cookie_env());
        set(SECURE_KEY, "true");
        assert!(!read_secure_cookie_env());
        clear(SECURE_KEY);
    }

    #[test]
    #[serial]
    fn read_secure_cookie_env_enabled_for_other_values() {
        set(SECURE_KEY, "yes");
        assert!(read_secure_cookie_env());
        set(SECURE_KEY, "");
        assert!(read_secure_cookie_env());
        set(SECURE_KEY, "0");
        assert!(read_secure_cookie_env());
        clear(SECURE_KEY);
    }

    #[test]
    #[serial]
    fn telemetry_disabled_default_false() {
        clear(TELEMETRY_KEY);
        assert!(!telemetry_disabled());
    }

    #[test]
    #[serial]
    fn telemetry_disabled_when_set() {
        set(TELEMETRY_KEY, "1");
        assert!(telemetry_disabled());
        set(TELEMETRY_KEY, "true");
        assert!(telemetry_disabled());
        set(TELEMETRY_KEY, "True");
        assert!(telemetry_disabled());
        set(TELEMETRY_KEY, "no");
        assert!(!telemetry_disabled());
        clear(TELEMETRY_KEY);
    }

    #[test]
    #[serial]
    fn secure_cookie_requires_long_secret() {
        clear(SESSION_KEY);
        assert!(read_session_key(true).is_err());
        set(SESSION_KEY, "short");
        assert!(read_session_key(true).is_err());
        set(
            SESSION_KEY,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        assert!(read_session_key(true).is_ok());
        clear(SESSION_KEY);
    }

    #[test]
    #[serial]
    fn insecure_development_mode_allows_ephemeral_secret() {
        clear(SESSION_KEY);
        assert!(read_session_key(false).is_ok());
    }

    async fn counter(session: Session) -> String {
        let count: usize = session.get("count").await.unwrap().unwrap_or_default();
        session.insert("count", count + 1).await.unwrap();
        count.to_string()
    }

    fn session_server(store: MemoryStore, secure: bool, key: Key) -> TestServer {
        TestServer::new(
            axum::Router::new()
                .route("/", get(counter))
                .layer(build_session_layer(store, secure, key)),
        )
    }

    #[tokio::test]
    async fn session_cookie_is_signed_and_hardened() {
        let server = session_server(MemoryStore::default(), true, Key::generate());
        let first = server.get("/").await;
        first.assert_text("0");

        let set_cookie = first
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("HttpOnly"), "{set_cookie}");
        assert!(set_cookie.contains("Secure"), "{set_cookie}");
        assert!(set_cookie.contains("SameSite=Lax"), "{set_cookie}");

        let cookie = first.cookies().get("gluon_sid").unwrap().value().to_owned();
        let valid = server
            .get("/")
            .add_header(header::COOKIE, format!("gluon_sid={cookie}"))
            .await;
        valid.assert_text("1");

        let mut tampered = cookie.into_bytes();
        let last = tampered.last_mut().unwrap();
        *last = if *last == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(tampered).unwrap();
        let rejected = server
            .get("/")
            .add_header(header::COOKIE, format!("gluon_sid={tampered}"))
            .await;
        rejected.assert_text("0");
    }

    #[tokio::test]
    async fn insecure_development_cookie_omits_secure_attribute() {
        let server = session_server(MemoryStore::default(), false, Key::generate());
        let response = server.get("/").await;
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(!set_cookie.contains("Secure"), "{set_cookie}");
    }

    async fn add_test_header(
        request: axum::extract::Request,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        let mut response = next.run(request).await;
        response
            .headers_mut()
            .insert("x-test-middleware", HeaderValue::from_static("active"));
        response
    }

    #[tokio::test]
    async fn server_applies_custom_middleware_and_shuts_down() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = axum::Router::new().route("/", get(|| async { "ok" }));
        let container = Arc::new(ContainerBuilder::new().build());
        let middleware: RouterFactory =
            Box::new(|router| router.layer(axum::middleware::from_fn(add_test_header)));
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let task = tokio::spawn(serve_with_shutdown(
            router,
            container,
            listener,
            MemoryStore::default(),
            (false, Key::generate()),
            Some(middleware),
            async move {
                let _ = shutdown_rx.await;
            },
        ));

        let response = reqwest::get(format!("http://{address}/")).await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.headers()["x-test-middleware"], "active");

        shutdown_tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .expect("server did not shut down")
            .unwrap()
            .unwrap();
    }
}
