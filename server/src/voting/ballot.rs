use chrono::{DateTime, Utc};
use std::fmt::{self, Display, Formatter};
use super::id::{Id, WeakId};
use super::poll::Poll;
use super::user::User;

#[derive(Debug)]
pub struct Ballot {
    pub poll_id: Id,
    pub voter_id: Id,
    pub selection_ids: Vec<WeakId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(poll: &Poll, voter: &User, selections: Vec<WeakId>) -> Ballot {
        Ballot {
            poll_id: poll.id.clone(),
            voter_id: voter.id.clone(),
            selection_ids: selections,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Display for Ballot {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}: {:?})", self.voter_id, self.selection_ids)
    }
}
