use axum::Json;
use gluon::Result;

pub async fn get() -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({
        "service": "sample-api",
        "ok": true,
    })))
}
