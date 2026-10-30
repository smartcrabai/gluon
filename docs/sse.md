# Server-Sent Events

`gluon::sse::Sse<S>` is an Axum SSE response with keep-alive comments enabled by default. A route `get` handler can return it directly or wrap it in gluon's `Result`. The stream item type is `Result<gluon::sse::Event, E>`, where `E` is an error type that implements `std::error::Error + Send + Sync + 'static`.

```rust
use std::time::Duration;

use axum::response::IntoResponse;
use futures_util::{StreamExt, stream};
use gluon::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct QueryUpdate {
    message: &'static str,
}

pub async fn get() -> Result<impl IntoResponse> {
    let updates = stream::iter([
        ("query-progress", "Query started"),
        ("query-complete", "Query finished"),
    ])
    .then(|(event, message)| async move {
        sse::view_event(event, View::new(QueryUpdate { message }))
    });

    Ok(sse::Sse::new(updates).keep_alive(
        sse::KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("query is still running"),
    ))
}
```

`Sse::new(stream)` uses Axum's default keep-alive configuration; `.keep_alive(...)` replaces it. Each stream item can be a named Axum event created by `sse::view_event(name, view)`. The helper renders the view as an HTML fragment and sets that fragment as the event's `data` payload.

For a generated `route.rs` handler, add a sibling `route.tsx` containing the fragment to render. `gluon-build` injects that template path for the handler; the SSE stream re-enters the request's template scope while polling, so views created by later stream items still resolve the correct template. Fragment rendering is explicit and does not depend on the `HX-Request` header.

## HTMX integration

Load htmx and its SSE extension in the page, then connect the stream and listen for the named events:

```tsx
<Head>
  <script src="https://unpkg.com/htmx.org@2.0.4"></script>
  <script src="https://unpkg.com/htmx-ext-sse@2.2.2"></script>
</Head>
<section hx-ext="sse" sse-connect="/events">
  <p sse-swap="query-progress">Waiting for progress...</p>
  <p sse-swap="query-complete"></p>
</section>
```

The `sse-swap` values must match the event names passed to `view_event`. The rendered fragment is inserted by htmx; gluon does not wrap it in an HTML document. `examples/htmx` demonstrates this route and markup.

## Middleware and proxies

SSE responses are returned as streaming bodies and gluon's CSRF, session, and HTMX middleware do not collect or buffer response bodies. CSRF treats `GET` as safe, so event-stream handlers do not need a CSRF token. Responses set `X-Accel-Buffering: no` for proxies that support this convention. Configure any reverse proxy or app-added middleware to preserve streaming behavior; the header cannot disable buffering by a proxy that ignores it.
