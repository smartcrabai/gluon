use std::time::Duration;

use axum::response::IntoResponse;
use futures_util::{StreamExt, stream};
use gluon::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct EventProps {
    message: &'static str,
}

pub async fn get() -> Result<impl IntoResponse> {
    let updates = stream::iter([
        ("query-progress", "Executing a sample query..."),
        ("query-complete", "Sample query complete."),
    ])
    .then(|(event, message)| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;
        sse::view_event(event, View::new(EventProps { message }))
    })
    .chain(stream::pending());

    Ok(sse::Sse::new(updates))
}
