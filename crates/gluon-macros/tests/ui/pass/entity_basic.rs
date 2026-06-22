use gluon_macros::Entity;

#[derive(Debug, Clone, Entity)]
pub struct User {
    #[id]
    pub id: u32,
    pub name: String,
}

fn main() {
    let a = User {
        id: 1,
        name: "a".into(),
    };
    let b = User {
        id: 1,
        name: "b".into(),
    };
    assert_eq!(a, b);
}
