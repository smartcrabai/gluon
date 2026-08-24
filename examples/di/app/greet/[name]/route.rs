use axum::{Json, extract::Path};
use gluon::{Inject, Result};

use crate::wiring::GreetingService;

pub async fn get(
    Path(name): Path<String>,
    Inject(service): Inject<dyn GreetingService>,
) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"greeting": service.greet(&name)})))
}
