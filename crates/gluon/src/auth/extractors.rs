//! Cookie-session authentication extractors.
use super::{Auth, AuthContext, AuthError, AuthUser, random_id};
use crate::{Container, Session};
use axum::extract::{ConnectInfo, FromRef, FromRequestParts, OptionalFromRequestParts};
use http::request::Parts;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

const BROWSER_KEY: &str = "_gluon_auth_browser_binding";
fn internal() -> AuthError {
    AuthError {
        code: "internal_error",
    }
}

impl<S: Send + Sync> FromRequestParts<S> for AuthContext {
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, AuthError> {
        let session = parts
            .extensions
            .get::<Session>()
            .cloned()
            .ok_or_else(internal)?;
        let browser_binding = if let Some(binding) = session
            .get::<String>(BROWSER_KEY)
            .await
            .map_err(|_| internal())?
        {
            binding
        } else {
            let binding = random_id();
            session
                .insert(BROWSER_KEY, &binding)
                .await
                .map_err(|_| internal())?;
            // Save before returning flow options so another worker can restore the binding.
            session.save().await.map_err(|_| internal())?;
            binding
        };
        // Forwarded headers are deliberately not trusted. In-process routers have
        // no socket peer and use one shared loopback rate-limit bucket.
        let source = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |peer| peer.0.ip());
        Ok(Self {
            session,
            source,
            browser_binding,
        })
    }
}

/// A user authenticated exclusively by the application's cookie session.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub AuthUser);

impl<S> OptionalFromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    Arc<Container>: FromRef<S>,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, AuthError> {
        let auth = Arc::<Container>::from_ref(state)
            .try_resolve::<Auth>()
            .ok_or_else(internal)?;
        let context = AuthContext::from_request_parts(parts, state).await?;
        auth.current_user(&context).await.map(|user| user.map(Self))
    }
}
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
    Arc<Container>: FromRef<S>,
{
    type Rejection = AuthError;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, AuthError> {
        <Self as OptionalFromRequestParts<S>>::from_request_parts(parts, state)
            .await?
            .ok_or(AuthError {
                code: "unauthorized",
            })
    }
}
