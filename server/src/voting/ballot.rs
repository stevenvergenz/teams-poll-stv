use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::id::WeakId;
use super::poll::Poll;
use super::user::{User, PossibleUser};

#[derive(Deserialize, Serialize)]
pub struct Ballot {
    pub poll: Option<Poll>,
    pub voter: Option<User>,
    pub selection_ids: Vec<WeakId>,
    pub created_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(selections: Vec<WeakId>) -> Ballot {
        Ballot {
            poll: None,
            voter: None,
            selection_ids: selections,
            created_at: Utc::now(),
        }
    }
}

impl Display for Ballot {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}: {:?})", PossibleUser(&self.voter), self.selection_ids)
    }
}
