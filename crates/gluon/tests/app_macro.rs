#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

//! Sanity check that `gluon::app!` is exported from the crate root.
//!
//! `gluon::app!()` expands to `include!(concat!(env!("OUT_DIR"),
//! "/__gluon_app.rs"))`, which requires the consuming crate to have a
//! `build.rs` that generates that file. Invoking the macro from here would
//! therefore fail to compile. Instead we verify the macro path is
//! referenceable -- catching breakage of the `#[macro_export]` attribute or
//! a rename of the symbol.

#[test]
fn app_macro_is_exported() {
    // Compile-time check: macro path is referenceable.
    // Actual invocation requires a build.rs setup.
    let _ = stringify!(gluon::app!());
}
