use axum::{Extension, Json};
use gluon::{Result, middleware::CsrfToken};

pub async fn get(Extension(token): Extension<CsrfToken>) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"token": token.as_str()})))
}
