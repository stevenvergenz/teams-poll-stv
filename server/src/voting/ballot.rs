use std::fmt::{self, Display, Formatter};
use std::default::Default;
use std::convert::From;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::id::WeakId;
use super::poll::Poll;
use super::user::{User, PossibleUser};
use crate::error;

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Ballot {
    pub poll: Option<Poll>,
    pub voter: Option<User>,
    pub ranked_preferences: Vec<WeakId>,
    pub created_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(voter: User, CreateBallot { poll, ranked_preferences }: CreateBallot) -> Ballot {
        Ballot {
            poll,
            voter: Some(voter),
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

impl Default for Ballot {
    fn default() -> Self {
        Ballot {
            poll: None,
            voter: None,
            ranked_preferences: vec![],
            created_at: Utc::now(),
        }
    }
}

impl From<CreateBallot> for Ballot {
    fn from(CreateBallot { poll, ranked_preferences }: CreateBallot) -> Self {
        Self {
            poll,
            ranked_preferences,
            ..Ballot::default()
        }
    }
}


pub struct CreateBallot {
    pub poll: Option<Poll>,
    pub ranked_preferences: Vec<WeakId>,
}

impl Default for CreateBallot {
    fn default() -> Self {
        Self {
            poll: None,
            ranked_preferences: vec![],
        }
    }
}

#[derive(Deserialize)]
pub struct UnvalidatedCreateBallot {
    pub ranked_preferences: Vec<WeakId>,
}

impl UnvalidatedCreateBallot {
    pub fn new() -> Self {
        Self {
            ranked_preferences: vec![],
        }
    }

    pub fn validate(self, poll: Poll) -> Result<CreateBallot, error::ValidationError> {
        let Self { ranked_preferences, .. } = self;
        if ranked_preferences.is_empty() {
            return Err(error::ballot_empty());
        }

        for (i, pref) in ranked_preferences.iter().enumerate() {
            if !poll.option_ids.contains(&pref) {
                return Err(error::ballot_invalid_selection(i, pref.0));
            }
            if let Some((old_idx, _)) = ranked_preferences[0..i].iter().enumerate().find(|(_, p)| *p == pref) {
                return Err(error::ballot_duplicate_selection(pref.0, (old_idx, i)))
            }
        }

        Ok(CreateBallot {
            poll: Some(poll),
            ranked_preferences,
        })
    }
}
