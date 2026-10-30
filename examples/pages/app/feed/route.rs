use gluon::View;
use serde::Serialize;

#[derive(Serialize)]
pub struct FeedProps {
    marker: &'static str,
}

pub async fn get() -> View<FeedProps> {
    View::new(FeedProps {
        marker: "sample-pages-route",
    })
}
