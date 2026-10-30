#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::allow_attributes,
        clippy::missing_panics_doc
    )
)]

#[cfg(feature = "auth")]
pub mod auth;
mod boot;
mod config;
mod container;
mod error;
mod flash;
mod inject;
pub mod middleware;
pub mod prelude;
mod redirect;
mod session;
pub mod sse;
pub mod testing;
pub mod view;

/// Re-exports used by build-generated application code.
#[doc(hidden)]
pub mod __private {
    pub use jsxrs;
}
pub use boot::Boot;
#[doc(hidden)]
pub use config::__register_embedded_config;
pub use config::{AppConfig, Config, DatabaseConfig, ServerConfig, TelemetryConfig, ViewConfig};
pub use container::{Container, ContainerBuilder};
pub use error::{AppError, FieldError, Result};
pub use flash::Flash;
pub use gluon_macros::{Entity, gluon_test};
pub use inject::Inject;
pub use redirect::Redirect;
pub use session::{PostgresSessionStore, Session};
pub use view::View;

#[macro_export]
macro_rules! app {
    () => {
        include!(concat!(env!("OUT_DIR"), "/__gluon_app.rs"));
    };
}
