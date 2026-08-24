use gluon::Boot;

mod wiring;

gluon::app!();

#[tokio::main(flavor = "multi_thread")]
async fn main() -> gluon::Result<()> {
    Boot::new()
        .with_container(wiring::build_container)
        .with_router(__gluon_router())
        .run()
        .await
}
