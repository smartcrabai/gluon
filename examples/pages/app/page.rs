use gluon::{Result, View};
use serde::Serialize;

#[derive(Serialize)]
pub struct HomeProps {
    marker: &'static str,
}

pub async fn get() -> Result<View<HomeProps>> {
    Ok(View::new(HomeProps {
        marker: "sample-pages-home",
    }))
}
