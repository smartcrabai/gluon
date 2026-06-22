//! Dependency injection container for the gluon framework.
//!
//! The [`Container`] stores `Arc<T>` values keyed by `TypeId`, supporting
//! both sized and `?Sized` (trait object) bindings. Use [`ContainerBuilder`]
//! to register bindings and produce a finalized container.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// A dependency injection container that resolves `Arc<T>` values by type.
pub struct Container {
    bindings: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Container {
    /// Resolve a binding for type `T`.
    ///
    /// # Panics
    ///
    /// Panics if no binding for `T` has been registered, or if the stored
    /// value cannot be downcast to `Arc<T>` (which would indicate an
    /// internal invariant violation).
    #[must_use]
    pub fn resolve<T: ?Sized + Send + Sync + 'static>(&self) -> Arc<T> {
        match self.try_resolve::<T>() {
            Some(arc) => arc,
            None => panic!(
                "gluon::Container: no binding registered for type {}",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Try to resolve a binding for type `T`, returning `None` if absent.
    #[must_use]
    pub fn try_resolve<T: ?Sized + Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let key = TypeId::of::<Arc<T>>();
        let entry = self.bindings.get(&key)?;
        // The `TypeId::of::<Arc<T>>` key and the `Box::new(arc)` insertion
        // are kept in sync inside this module, so downcast cannot fail in
        // well-formed code. Use `debug_assert!` instead of a runtime panic.
        let arc = entry.downcast_ref::<Arc<T>>();
        debug_assert!(arc.is_some(), "container storage type mismatch");
        arc.map(Arc::clone)
    }

    /// Override the binding for type `T`, replacing any prior value.
    ///
    /// Intended for tests that need to swap implementations after the
    /// container has been built.
    pub fn override_with<T: ?Sized + Send + Sync + 'static>(&mut self, value: Arc<T>) {
        let key = TypeId::of::<Arc<T>>();
        self.bindings.insert(key, Box::new(value));
    }
}

type Factory = Box<dyn FnOnce(&Container) -> InsertFn + Send + 'static>;
type InsertFn = Box<dyn FnOnce(&mut Container) + Send + 'static>;

/// Builder used to register bindings before producing a [`Container`].
pub struct ContainerBuilder {
    factories: Vec<Factory>,
}

impl ContainerBuilder {
    /// Create an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Register a factory that produces an `Arc<T>` given the in-progress container.
    ///
    /// Factories run in the order they were registered, and each one sees
    /// every binding registered before it.
    #[must_use]
    pub fn bind<T, F>(mut self, factory: F) -> Self
    where
        T: ?Sized + Send + Sync + 'static,
        F: FnOnce(&Container) -> Arc<T> + Send + 'static,
    {
        let wrapper: Factory = Box::new(move |container: &Container| {
            let value = factory(container);
            let insert: InsertFn = Box::new(move |c: &mut Container| {
                let key = TypeId::of::<Arc<T>>();
                c.bindings.insert(key, Box::new(value));
            });
            insert
        });
        self.factories.push(wrapper);
        self
    }

    /// Register a concrete `Arc<T>` instance directly.
    #[must_use]
    pub fn bind_instance<T: ?Sized + Send + Sync + 'static>(mut self, value: Arc<T>) -> Self {
        let wrapper: Factory = Box::new(move |_container: &Container| {
            let insert: InsertFn = Box::new(move |c: &mut Container| {
                let key = TypeId::of::<Arc<T>>();
                c.bindings.insert(key, Box::new(value));
            });
            insert
        });
        self.factories.push(wrapper);
        self
    }

    /// Consume the builder and produce the finalized [`Container`].
    #[must_use]
    pub fn build(self) -> Container {
        let mut container = Container {
            bindings: HashMap::new(),
        };
        for factory in self.factories {
            let insert = factory(&container);
            insert(&mut container);
        }
        container
    }
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    trait Greeter: Send + Sync {
        fn greet(&self) -> &'static str;
    }
    struct Hello;
    impl Greeter for Hello {
        fn greet(&self) -> &'static str {
            "hello"
        }
    }
    struct Aloha;
    impl Greeter for Aloha {
        fn greet(&self) -> &'static str {
            "aloha"
        }
    }

    #[test]
    fn resolve_returns_bound_arc() {
        let container = ContainerBuilder::new()
            .bind::<dyn Greeter, _>(|_| Arc::new(Hello))
            .build();
        assert_eq!(container.resolve::<dyn Greeter>().greet(), "hello");
    }

    #[test]
    fn try_resolve_returns_none_when_unbound() {
        let container = ContainerBuilder::new().build();
        assert!(container.try_resolve::<dyn Greeter>().is_none());
    }

    #[test]
    fn bind_instance_short_circuits_factory() {
        let container = ContainerBuilder::new()
            .bind_instance::<dyn Greeter>(Arc::new(Hello))
            .build();
        assert_eq!(container.resolve::<dyn Greeter>().greet(), "hello");
    }

    #[test]
    fn override_with_replaces_existing_binding() {
        let mut container = ContainerBuilder::new()
            .bind::<dyn Greeter, _>(|_| Arc::new(Hello))
            .build();
        container.override_with::<dyn Greeter>(Arc::new(Aloha));
        assert_eq!(container.resolve::<dyn Greeter>().greet(), "aloha");
    }

    #[test]
    fn factories_see_prior_bindings_during_build() {
        struct Counter(usize);
        let container = ContainerBuilder::new()
            .bind::<dyn Greeter, _>(|_| Arc::new(Hello))
            .bind::<Counter, _>(|c| {
                // resolve another binding inside a factory closure
                let g = c
                    .try_resolve::<dyn Greeter>()
                    .expect("Greeter must be bound");
                Arc::new(Counter(g.greet().len()))
            })
            .build();
        assert_eq!(container.resolve::<Counter>().0, 5);
    }

    #[test]
    #[should_panic(expected = "no binding registered")]
    fn resolve_panics_when_unbound() {
        let container = ContainerBuilder::new().build();
        let _ = container.resolve::<dyn Greeter>();
    }

    #[test]
    fn default_builder_builds_empty_container() {
        let container = ContainerBuilder::default().build();
        assert!(container.try_resolve::<dyn Greeter>().is_none());
    }

    #[test]
    fn sized_type_override() {
        struct MyStruct {
            value: u32,
        }

        let mut container = ContainerBuilder::new()
            .bind_instance::<MyStruct>(Arc::new(MyStruct { value: 1 }))
            .build();
        assert_eq!(container.resolve::<MyStruct>().value, 1);

        container.override_with::<MyStruct>(Arc::new(MyStruct { value: 42 }));
        assert_eq!(container.resolve::<MyStruct>().value, 42);
    }

    #[test]
    fn bind_same_type_twice_uses_last() {
        let container = ContainerBuilder::new()
            .bind::<dyn Greeter, _>(|_| Arc::new(Hello))
            .bind::<dyn Greeter, _>(|_| Arc::new(Aloha))
            .build();
        assert_eq!(container.resolve::<dyn Greeter>().greet(), "aloha");
    }

    #[test]
    fn factory_can_resolve_multiple_prior_bindings() {
        trait A: Send + Sync {
            fn a(&self) -> &'static str;
        }
        trait B: Send + Sync {
            fn b(&self) -> &'static str;
        }
        struct ImplA;
        impl A for ImplA {
            fn a(&self) -> &'static str {
                "a"
            }
        }
        struct ImplB;
        impl B for ImplB {
            fn b(&self) -> &'static str {
                "b"
            }
        }
        struct Combined(String);

        let container = ContainerBuilder::new()
            .bind::<dyn A, _>(|_| Arc::new(ImplA))
            .bind::<dyn B, _>(|_| Arc::new(ImplB))
            .bind::<Combined, _>(|c| {
                let a = c.try_resolve::<dyn A>().expect("A must be bound");
                let b = c.try_resolve::<dyn B>().expect("B must be bound");
                Arc::new(Combined(format!("{}{}", a.a(), b.b())))
            })
            .build();
        assert_eq!(container.resolve::<Combined>().0, "ab");
    }
}
