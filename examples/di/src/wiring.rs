use std::sync::Arc;

use gluon::ContainerBuilder;

pub trait GreetingService: Send + Sync {
    fn greet(&self, name: &str) -> String;
}

struct DefaultGreetingService;

impl GreetingService for DefaultGreetingService {
    fn greet(&self, name: &str) -> String {
        format!("Hello, {name}!")
    }
}

pub fn build_container(builder: ContainerBuilder) -> ContainerBuilder {
    builder.bind_instance::<dyn GreetingService>(Arc::new(DefaultGreetingService))
}
