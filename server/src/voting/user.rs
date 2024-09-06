use serde::Serialize;
use super::id::Id;

#[derive(Clone, Serialize)]
pub struct User {
    pub id: Id,
    pub display_name: String,
}

impl User {
    pub const fn new(id: Id, display_name: String) -> User {
        User {
            id,
            display_name,
        }
    }
}
