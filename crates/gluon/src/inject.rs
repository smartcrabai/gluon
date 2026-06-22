//! `Inject<T>` axum extractor that resolves a dependency from the
//! application [`Container`](crate::Container) stored in router state.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use http::request::Parts;

use crate::Container;

/// Extractor that resolves an `Arc<T>` from the application container.
///
/// The router state must expose `Arc<Container>` via [`FromRef`].
pub struct Inject<T: ?Sized + Send + Sync + 'static>(pub Arc<T>);

impl<T, S> FromRequestParts<S> for Inject<T>
where
    T: ?Sized + Send + Sync + 'static,
    S: Send + Sync,
    Arc<Container>: FromRef<S>,
{
    type Rejection = Infallible;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let container: Arc<Container> = Arc::<Container>::from_ref(state);
        Ok(Self(container.resolve::<T>()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContainerBuilder;
    use axum::extract::FromRequestParts;
    use http::Request;

    trait TestTrait: Send + Sync {
        fn name(&self) -> &'static str;
    }

    struct TestImpl;
    impl TestTrait for TestImpl {
        fn name(&self) -> &'static str {
            "test-impl"
        }
    }

    fn empty_parts() -> Parts {
        let (parts, _body) = Request::builder().uri("/").body(()).unwrap().into_parts();
        parts
    }

    // Compile-time assertion that `Inject<T>` returns `Infallible` as its
    // rejection. If this ever changes the test below would fail to compile.
    #[allow(dead_code)]
    fn _rejection_is_infallible() {
        fn assert_infallible<T>()
        where
            T: ?Sized + Send + Sync + 'static,
            Inject<T>: FromRequestParts<Arc<Container>, Rejection = Infallible>,
        {
        }
        assert_infallible::<dyn TestTrait>();
    }

    #[tokio::test]
    async fn resolves_arc_when_bound() {
        let container = ContainerBuilder::new()
            .bind::<dyn TestTrait, _>(|_| Arc::new(TestImpl))
            .build();
        let state = Arc::new(container);
        let mut parts = empty_parts();

        let Inject(arc) =
            <Inject<dyn TestTrait> as FromRequestParts<Arc<Container>>>::from_request_parts(
                &mut parts, &state,
            )
            .await
            .unwrap();
        assert_eq!(arc.name(), "test-impl");
    }

    #[tokio::test]
    #[should_panic(expected = "no binding registered")]
    async fn panics_when_unbound() {
        let container = ContainerBuilder::new().build();
        let state = Arc::new(container);
        let mut parts = empty_parts();

        let _ = <Inject<dyn TestTrait> as FromRequestParts<Arc<Container>>>::from_request_parts(
            &mut parts, &state,
        )
        .await;
    }
}
