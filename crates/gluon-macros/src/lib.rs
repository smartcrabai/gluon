//! Procedural macros for the gluon framework.
//!
//! Provides:
//! - `#[derive(Entity)]` -- implements `PartialEq` / `Eq` / `Hash` based on the
//!   field annotated with `#[id]`.
//! - `#[gluon_test]` -- wraps an async function with `#[tokio::test]` so it can
//!   be used as `#[gluon::gluon_test]` in user code.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::missing_panics_doc,
        clippy::allow_attributes
    )
)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, ItemFn, parse_macro_input, spanned::Spanned};

/// Derives `PartialEq`, `Eq`, and `Hash` for a struct based on the field
/// annotated with `#[id]`.
///
/// # Example
/// ```ignore
/// #[derive(Entity)]
/// struct Foo {
///     #[id]
///     id: FooId,
///     name: FooName,
/// }
/// ```
#[proc_macro_derive(Entity, attributes(id))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_entity(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

pub(crate) fn expand_entity(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let Data::Struct(data_struct) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "#[derive(Entity)] can only be applied to structs",
        ));
    };

    let fields = match &data_struct.fields {
        Fields::Named(named) => &named.named,
        Fields::Unnamed(_) | Fields::Unit => {
            return Err(syn::Error::new(
                data_struct.fields.span(),
                "#[derive(Entity)] requires a struct with named fields and one #[id] field",
            ));
        }
    };

    let mut id_field = None;
    for field in fields {
        if field.attrs.iter().any(|attr| attr.path().is_ident("id")) {
            if id_field.is_some() {
                return Err(syn::Error::new(
                    field.span(),
                    "#[derive(Entity)] requires exactly one #[id] field, but multiple were found",
                ));
            }
            id_field = Some(field);
        }
    }

    let Some(id_field) = id_field else {
        return Err(syn::Error::new(
            input.span(),
            "#[derive(Entity)] requires exactly one field annotated with #[id]",
        ));
    };

    let Some(id_ident) = id_field.ident.as_ref() else {
        return Err(syn::Error::new(
            id_field.span(),
            "#[derive(Entity)] requires a named #[id] field",
        ));
    };

    Ok(quote! {
        impl #impl_generics ::core::cmp::PartialEq for #struct_name #ty_generics #where_clause {
            fn eq(&self, other: &Self) -> bool {
                self.#id_ident == other.#id_ident
            }
        }

        impl #impl_generics ::core::cmp::Eq for #struct_name #ty_generics #where_clause {}

        impl #impl_generics ::core::hash::Hash for #struct_name #ty_generics #where_clause {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.#id_ident, state);
            }
        }
    })
}

/// Wraps an async function with `#[tokio::test]`.
///
/// Intended to be re-exported by the `gluon` crate so users can write
/// `#[gluon::gluon_test]` on async test functions.
#[proc_macro_attribute]
pub fn gluon_test(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    expand_gluon_test(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

pub(crate) fn expand_gluon_test(input: &ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    if input.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            input.sig.span(),
            "#[gluon_test] requires an async function",
        ));
    }

    let attrs = &input.attrs;
    let vis = &input.vis;
    let sig = &input.sig;
    let block = &input.block;

    Ok(quote! {
        #[::tokio::test]
        #(#attrs)*
        #vis #sig #block
    })
}

#[cfg(test)]
mod tests {
    use super::{expand_entity, expand_gluon_test};
    use syn::{DeriveInput, ItemFn, parse_quote};

    #[test]
    fn expand_entity_succeeds_with_id_field() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[id]
                id: UserId,
                name: String,
            }
        };
        let tokens = expand_entity(&input).unwrap().to_string();
        assert!(tokens.contains("PartialEq for User"));
        assert!(tokens.contains("Eq for User"));
        assert!(tokens.contains("Hash for User"));
        assert!(tokens.contains("self . id == other . id"));
    }

    #[test]
    fn expand_entity_fails_without_id_attribute() {
        let input: DeriveInput = parse_quote! {
            struct NoId {
                name: String,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(err.to_string().contains("#[id]"), "unexpected error: {err}");
    }

    #[test]
    fn expand_entity_fails_with_multiple_id_attributes() {
        let input: DeriveInput = parse_quote! {
            struct Multi {
                #[id]
                a: u32,
                #[id]
                b: u32,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(
            err.to_string().contains("multiple"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn expand_entity_fails_on_enum() {
        let input: DeriveInput = parse_quote! {
            enum E {
                A,
            }
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(
            err.to_string().contains("structs"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn expand_entity_fails_on_tuple_struct() {
        let input: DeriveInput = parse_quote! {
            struct T(u32);
        };
        let err = expand_entity(&input).unwrap_err();
        assert!(
            err.to_string().contains("named fields"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn expand_entity_preserves_generics_and_where_clause() {
        let input: DeriveInput = parse_quote! {
            struct Foo<T: Clone> where T: Send {
                #[id]
                id: T,
            }
        };
        let tokens = expand_entity(&input).unwrap().to_string();
        assert!(
            tokens.contains("impl < T : Clone >") || tokens.contains("impl<T: Clone>"),
            "missing generics in: {tokens}"
        );
        assert!(
            tokens.contains("where"),
            "missing where clause in: {tokens}"
        );
    }

    #[test]
    fn expand_gluon_test_wraps_async_fn() {
        let input: ItemFn = parse_quote! {
            async fn t() {}
        };
        let tokens = expand_gluon_test(&input).unwrap().to_string();
        assert!(
            tokens.contains("tokio :: test") || tokens.contains("tokio::test"),
            "missing tokio::test in: {tokens}"
        );
        assert!(
            tokens.contains("async fn t"),
            "missing async fn in: {tokens}"
        );
    }

    #[test]
    fn expand_gluon_test_rejects_non_async_fn() {
        let input: ItemFn = parse_quote! {
            fn t() {}
        };
        let err = expand_gluon_test(&input).unwrap_err();
        assert!(err.to_string().contains("async"), "unexpected error: {err}");
    }

    #[test]
    fn expand_gluon_test_preserves_existing_attrs() {
        let input: ItemFn = parse_quote! {
            #[serial]
            async fn t() {}
        };
        let tokens = expand_gluon_test(&input).unwrap().to_string();
        assert!(
            tokens.contains("serial"),
            "missing serial attr in: {tokens}"
        );
    }
}
