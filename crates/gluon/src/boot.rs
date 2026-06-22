//! Application bootstrap: OpenTelemetry initialization, middleware wiring, and
//! axum server startup.

use std::sync::Arc;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tower_sessions::cookie::SameSite;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::container::{Container, ContainerBuilder};
use crate::error::{AppError, Result};
use crate::middleware::csrf::csrf_middleware;
use crate::middleware::htmx::htmx_middleware;
use crate::middleware::static_files;

type ContainerFactory = Box<dyn FnOnce(ContainerBuilder) -> ContainerBuilder + Send + 'static>;

/// Bootstraps a gluon application: initializes telemetry, builds the
/// dependency-injection container, wires the standard middleware stack, and
/// serves the axum router.
pub struct Boot {
    container_factory: Option<ContainerFactory>,
    router: Option<axum::Router<Arc<Container>>>,
}

impl Boot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            container_factory: None,
            router: None,
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

    /// Initializes tracing (+ optional OpenTelemetry), builds the container,
    /// mounts the session/CSRF/HTMX middleware stack and the `public/` static
    /// asset service, then serves the axum router.
    ///
    /// Environment variables:
    /// - `GLUON_TELEMETRY_DISABLED=1` -- skip OpenTelemetry, fmt subscriber only.
    /// - `GLUON_BIND` -- bind address (default `0.0.0.0:3000`).
    /// - `GLUON_INSECURE_COOKIE=1` -- drop `Secure` on the session cookie (dev only).
    /// - `DATABASE_URL` -- when present, a lazy `PgPool` is registered in the
    ///   container; when absent, no pool is registered.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Internal`] when binding the TCP listener, serving
    /// the axum router, or constructing the `PgPool` from `DATABASE_URL`
    /// fails.
    pub async fn run(self) -> Result<()> {
        init_tracing()?;

        let mut builder = ContainerBuilder::new();

        if let Ok(database_url) = std::env::var("DATABASE_URL") {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .connect_lazy(&database_url)
                .map_err(|e| AppError::Internal(Box::new(e)))?;
            builder = builder.bind_instance::<sqlx::PgPool>(Arc::new(pool));
        } else {
            tracing::warn!("DATABASE_URL not set; no PgPool registered in container");
        }

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
        let session_layer = build_session_layer(secure_cookie);

        let app = router
            .layer(axum::middleware::from_fn(htmx_middleware))
            .layer(axum::middleware::from_fn(csrf_middleware))
            .layer(session_layer)
            .with_state(Arc::clone(&container));

        let bind_addr = std::env::var("GLUON_BIND").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| AppError::Internal(Box::new(e)))?;

        tracing::info!("gluon listening on {}", bind_addr);

        axum::serve(listener, app)
            .await
            .map_err(|e| AppError::Internal(Box::new(e)))?;

        Ok(())
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

/// Builds the session cookie middleware layer used by `Boot::run`.
pub(crate) fn build_session_layer(secure: bool) -> SessionManagerLayer<MemoryStore> {
    SessionManagerLayer::new(MemoryStore::default())
        .with_name("gluon_sid")
        .with_same_site(SameSite::Lax)
        .with_http_only(true)
        .with_secure(secure)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    const SECURE_KEY: &str = "GLUON_INSECURE_COOKIE";
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
    fn build_session_layer_uses_provided_secure() {
        // Black-box: we only ensure construction doesn't panic for both
        // values. The cookie attributes are an implementation detail of
        // `tower-sessions` and not introspectable without a real request.
        let _ = build_session_layer(true);
        let _ = build_session_layer(false);
    }
}
