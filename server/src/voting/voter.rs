use super::id::Id;

pub struct Voter {
    pub id: Id,
    pub display_name: String,
}

impl Voter {
    pub const fn new(id: Id, display_name: String) -> Voter {
        Voter {
            id,
            display_name,
        }
    }
}
