use std::fmt::{self, Display, Formatter};
use std::default::Default;
use std::convert::{From, Into, TryInto};

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::id::WeakId;
use super::poll::Poll;
use super::user::{User, PossibleUser};
use crate::error;

#[derive(Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Ballot {
    pub poll: Option<Poll>,
    pub voter: Option<User>,
    pub ranked_preferences: Vec<WeakId>,
    pub created_at: DateTime<Utc>,
}

impl Ballot {
    pub fn new(poll: Poll, voter: User, CreateBallot { ranked_preferences }: CreateBallot) -> Ballot {
        Ballot {
            poll: Some(poll),
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




pub struct CreateBallot {
    pub ranked_preferences: Vec<WeakId>,
}

pub struct UnvalidatedCreateBallot {
    pub ranked_preferences: Vec<WeakId>,
}

impl TryInto<CreateBallot> for (UnvalidatedCreateBallot, &Poll) {
    type Error = error::ValidationError;
    fn try_into(self) -> Result<CreateBallot, Self::Error> {
        let (UnvalidatedCreateBallot { ranked_preferences }, poll) = self;

        if ranked_preferences.is_empty() {
            return Err(error::ballot_empty());
        }

        // todo: rewrite validation here, update model parsing to use this method

        for pref in ranked_preferences {
            if !poll.option_ids.contains(pref) {
                return Err(error::ballot_invalid_selection(id, preference_index, option_id))
            }
        }

        Ok(voting::Ballot {
            poll: Some(poll),
            voter: Some(voter.into()),
            ranked_preferences: ranked_prefs,
            created_at: db_ballot.created_at.and_utc(),
        })
    }
}
