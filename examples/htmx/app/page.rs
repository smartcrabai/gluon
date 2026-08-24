use gluon::{Result, View};
use serde::Serialize;

#[derive(Serialize)]
pub struct PageProps {
    marker: &'static str,
}

pub async fn get() -> Result<View<PageProps>> {
    Ok(View::new(PageProps {
        marker: "sample-htmx",
    }))
}
