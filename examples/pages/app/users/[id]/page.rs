use axum::extract::Path;
use gluon::{Result, View};
use serde::Serialize;

#[derive(Serialize)]
pub struct UserProps {
    marker: String,
}

pub async fn get(Path(id): Path<String>) -> Result<View<UserProps>> {
    Ok(View::new(UserProps {
        marker: format!("sample-pages-user:{id}"),
    }))
}
