use std::fmt::{self, Display, Formatter};
use std::default::Default;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::id::WeakId;
use super::poll::Poll;
use super::user::{User, PossibleUser};

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Ballot {
    pub poll: Option<Poll>,
    pub voter: Option<User>,
    pub ranked_preferences: Vec<WeakId>,
    pub created_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(ranked_preferences: Vec<WeakId>) -> Ballot {
        Ballot {
            poll: None,
            voter: None,
            ranked_preferences,
            created_at: Utc::now(),
        }
    }
}

impl Display for Ballot {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}: {:?})", PossibleUser(&self.voter), self.ranked_preferences)
    }
}
