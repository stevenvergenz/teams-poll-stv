use std::clone::Clone;
use std::cmp::{Eq, PartialEq};
use std::fmt::{self, Display, Formatter};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Id(pub Uuid);
impl Id {
    pub const fn nil() -> Id {
        Id(Uuid::nil())
    }
    pub fn new() -> Id {
        Id(Uuid::new_v4())
    }
}
impl Display for Id {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct WeakId(pub u32);
impl WeakId {
    pub const fn nil() -> WeakId {
        WeakId(0)
    }
}
impl Display for WeakId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl PartialEq<u32> for WeakId {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}
