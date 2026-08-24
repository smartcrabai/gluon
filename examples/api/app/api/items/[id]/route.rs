use axum::{Json, extract::{Path, Query}};
use gluon::Result;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ItemQuery {
    label: String,
}

pub async fn get(
    Path(id): Path<String>,
    Query(query): Query<ItemQuery>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "id": id,
        "label": query.label,
    })))
}
