//! Server-Sent Events responses and TSX fragment events.
//!
//! [`Sse`] configures Axum's event stream with keep-alive comments and marks
//! responses to bypass buffering by reverse proxies that honor
//! `X-Accel-Buffering: no`. The response captures `CURRENT_TEMPLATE`,
//! `CURRENT_SOURCE_MAP`, `CURRENT_RENDER_CONFIG`, and
//! `CURRENT_TAILWIND_STYLESHEET` for event rendering.

use std::error::Error;

use axum::http::{HeaderValue, header::HeaderName};
use axum::response::{IntoResponse, Response, sse::Sse as AxumSse};
use futures_util::{Stream, StreamExt, stream};
use serde::Serialize;

use crate::view::{
    CURRENT_RENDER_CONFIG, CURRENT_SOURCE_MAP, CURRENT_TAILWIND_STYLESHEET, CURRENT_TEMPLATE,
};
use crate::{Result, View};

pub use axum::response::sse::{Event, KeepAlive};

/// An SSE response that enables Axum's default keep-alive comment interval.
///
/// Streams are polled within the template, source-map, rendering-config, and
/// stylesheet contexts captured for the request, so [`view_event`] can render
/// TSX fragments as events are produced after the handler has returned.
pub struct Sse<S> {
    stream: S,
    keep_alive: KeepAlive,
}

impl<S> Sse<S> {
    /// Create a streaming response with Axum's default keep-alive comments.
    #[must_use]
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            keep_alive: KeepAlive::default(),
        }
    }

    /// Replace the default keep-alive configuration.
    #[must_use]
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = keep_alive;
        self
    }
}

impl<S, E> IntoResponse for Sse<S>
where
    S: Stream<Item = std::result::Result<Event, E>> + Send + 'static,
    E: Error + Send + Sync + 'static,
{
    fn into_response(self) -> Response {
        let template = CURRENT_TEMPLATE.try_with(Clone::clone).ok().flatten();
        let render_config = CURRENT_RENDER_CONFIG.try_with(Clone::clone).ok();
        let source_map = CURRENT_SOURCE_MAP.try_with(Clone::clone).ok().flatten();
        let stylesheet_url = CURRENT_TAILWIND_STYLESHEET
            .try_with(|url| *url)
            .ok()
            .flatten();
        let stream = stream::unfold(Box::pin(self.stream), move |mut stream| {
            let template = template.clone();
            let render_config = render_config.clone();
            let source_map = source_map.clone();
            let stylesheet_url = stylesheet_url;
            async move {
                let item = CURRENT_TAILWIND_STYLESHEET
                    .scope(
                        stylesheet_url,
                        CURRENT_TEMPLATE.scope(template, async {
                            CURRENT_SOURCE_MAP
                                .scope(source_map, async {
                                    match render_config {
                                        Some(config) => {
                                            CURRENT_RENDER_CONFIG.scope(config, stream.next()).await
                                        }
                                        None => stream.next().await,
                                    }
                                })
                                .await
                        }),
                    )
                    .await?;
                Some((item, stream))
            }
        });

        let mut response = AxumSse::new(stream)
            .keep_alive(self.keep_alive)
            .into_response();
        response.headers_mut().insert(
            HeaderName::from_static("x-accel-buffering"),
            HeaderValue::from_static("no"),
        );
        response
    }
}

/// Render a [`View`] as a fragment and set its named SSE event type.
///
/// The view uses the current request's build-discovered TSX template. The
/// fragment is rendered when this helper is called, so invoke it from the
/// stream that produces the event if rendering should happen incrementally.
///
/// # Errors
///
/// Returns an application error if the view cannot be rendered.
pub fn view_event<P: Serialize>(event: impl Into<String>, view: View<P>) -> Result<Event> {
    Ok(Event::default()
        .event(event.into())
        .data(view.render_fragment()?))
}
