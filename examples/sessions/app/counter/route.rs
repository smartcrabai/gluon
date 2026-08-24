use axum::Json;
use gluon::{Result, Session};

async fn read_count(session: &Session) -> Result<u64> {
    session
        .get::<u64>("count")
        .await
        .map(|count| count.unwrap_or_default())
        .map_err(|error| gluon::AppError::Internal(Box::new(error)))
}

pub async fn get(session: Session) -> Result<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({"count": read_count(&session).await?})))
}

pub async fn post(session: Session) -> Result<Json<serde_json::Value>> {
    let count = read_count(&session).await? + 1;
    session
        .insert("count", count)
        .await
        .map_err(|error| gluon::AppError::Internal(Box::new(error)))?;
    Ok(Json(serde_json::json!({"count": count})))
}
