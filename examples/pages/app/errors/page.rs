use gluon::{AppError, Result, View};

pub async fn get() -> Result<View<()>> {
    Err(AppError::NotFound)
}
