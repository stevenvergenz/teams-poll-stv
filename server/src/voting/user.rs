use std::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use super::id::Id;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
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

impl Display for User {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

pub struct PossibleUser<'a>(pub &'a Option<User>);
impl<'a> Display for PossibleUser<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(u) => write!(f, "{}", u.display_name),
            None => write!(f, "{}", "???"),
        }
    }
}
