#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::allow_attributes,
        clippy::missing_panics_doc
    )
)]

mod boot;
mod container;
mod error;
mod flash;
mod inject;
pub mod middleware;
pub mod prelude;
mod redirect;
mod session;
pub mod testing;
pub mod view;

pub use boot::Boot;
pub use container::{Container, ContainerBuilder};
pub use error::{AppError, FieldError, Result};
pub use flash::Flash;
pub use gluon_macros::{Entity, gluon_test};
pub use inject::Inject;
pub use redirect::Redirect;
pub use session::Session;
pub use view::View;

#[macro_export]
macro_rules! app {
    () => {
        include!(concat!(env!("OUT_DIR"), "/__gluon_app.rs"));
    };
}
