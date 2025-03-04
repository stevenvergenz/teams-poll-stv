use std::default::Default;
use std::convert::{From, TryFrom};
use std::ops::RangeInclusive;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Serialize};
use uuid::Uuid;

use super::id::{Id, WeakId};
use super::user::User;
use crate::error;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Poll {
    pub id: Id,
    pub title: String,
    pub option_ids: Vec<WeakId>,
    pub options: Option<Vec<PollOption>>,

    pub winner_count: u8,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<DateTime<Utc>>,
    pub close_after_votes: Option<u32>,

    pub owner_id: Id,
    pub owner: Option<User>,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,

    #[serde(skip)]
    pub rng_seed: [u8; 32],
}

impl Poll {
    pub fn new(settings: CreatePollSettings, options: Vec<PollOption>, owner: User, rng_seed: Vec<u8>) -> Self {
        let mut poll = Self::from(settings);
        poll.option_ids = options.iter().map(|o| o.id).collect();
        poll.options = Some(options);
        poll.owner_id = owner.id.clone();
        poll.owner = Some(owner);
        poll.rng_seed.copy_from_slice(&rng_seed);
        poll
    }
}

impl From<CreatePollSettings> for Poll {
    fn from(CreatePollSettings {
        id,
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes: close_after_num_votes
    }: CreatePollSettings) -> Poll {
        let options: Vec<PollOption> = options.into_iter().enumerate().map(|(i, text)| {
            PollOption {
                id: WeakId(i as u32),
                description: text,
            }
        }).collect();

        Poll {
            id: match id {
                Some(uuid) => Id(uuid),
                None => Id::new(),
            },
            title,
            option_ids: options.iter().map(|o| o.id).collect(),
            options: Some(options),
            winner_count,
            write_ins_allowed,
            close_after_time,
            close_after_votes: close_after_num_votes,

            owner_id: Id::nil(),
            owner: None,
            created_at: Utc::now(),
            closed_at: None,
            rng_seed: [0u8; 32],
        }
    }
}


#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct PollOption {
    pub id: WeakId,
    pub description: String,
}


#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "UnvalidatedCreatePollSettings")]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,
    pub options: Vec<String>,

    pub winner_count: u8,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<DateTime<Utc>>,
    pub close_after_votes: Option<u32>,
}

impl Default for CreatePollSettings {
    fn default() -> Self {
        let unvalidated_default = UnvalidatedCreatePollSettings::default();
        Self {
            id: None,
            title: String::new(),
            options: vec![],
            winner_count: unvalidated_default.winner_count as u8,
            write_ins_allowed: unvalidated_default.write_ins_allowed,
            close_after_time: unvalidated_default.close_after_time,
            close_after_votes: unvalidated_default.close_after_votes.map(|v| v as u32),
        }
    }
}

impl CreatePollSettings {
    pub fn apply(&mut self, patch: &UpdatePollSettings, new_options: &Vec<String>) {
        if let Some(title) = &patch.title {
            self.title = title.clone();
        }

        self.options.append(&mut new_options.clone());

        if let Some(winner_count) = &patch.winner_count {
            self.winner_count = *winner_count;
        }

        if let Some(write_ins_allowed) = &patch.write_ins_allowed {
            self.write_ins_allowed = *write_ins_allowed;
        }

        if let Some(close_after_time) = &patch.close_after_time {
            self.close_after_time = *close_after_time;
        }

        if let Some(close_after_votes) = &patch.close_after_votes {
            self.close_after_votes = *close_after_votes;
        }
    }
}

const TITLE_LENGTH_BOUNDS: RangeInclusive<usize> = 3usize ..= i32::MAX as usize;
const OPTIONS_LENGTH_BOUNDS: RangeInclusive<usize> = 2usize ..= i32::MAX as usize;
const WINNERS_BOUNDS: RangeInclusive<i32> = 1 ..= u8::MAX as i32;
const VOTES_BOUNDS: RangeInclusive<i64> = 2i64 ..= i32::MAX as i64;

impl TryFrom<UnvalidatedCreatePollSettings> for CreatePollSettings {
    type Error = error::ValidationError;

    fn try_from(UnvalidatedCreatePollSettings {
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: UnvalidatedCreatePollSettings) -> Result<Self, Self::Error> {
        if !TITLE_LENGTH_BOUNDS.contains(&title.len()) {
            return Err(error::poll_title_invalid_size(TITLE_LENGTH_BOUNDS, title.len()));
        }
        if !OPTIONS_LENGTH_BOUNDS.contains(&options.len()) {
            return Err(error::poll_option_limit_exceeded(OPTIONS_LENGTH_BOUNDS, options.len()));
        }
        if !WINNERS_BOUNDS.contains(&winner_count) {
            return Err(error::poll_winners_limit_exceeded(WINNERS_BOUNDS, winner_count));
        }
        if let Some(time) = close_after_time {
            if time < Utc::now() + Duration::from_secs(60) {
                return Err(error::poll_duration_invalid(1, &time))
            }
        }
        if let Some(votes) = close_after_votes {
            if !VOTES_BOUNDS.contains(&(votes as i64)) {
                return Err(error::poll_votes_limit_exceeded(VOTES_BOUNDS, votes as i64));
            }
        }

        Ok(CreatePollSettings {
            id: None,
            title,
            options,
            winner_count: winner_count as u8,
            write_ins_allowed,
            close_after_time,
            close_after_votes: close_after_votes.map(|v| v as u32),
        })
    }
}


#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct UnvalidatedCreatePollSettings {
    pub title: String,
    pub options: Vec<String>,

    pub winner_count: i32,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<DateTime<Utc>>,
    pub close_after_votes: Option<i32>,
}

impl Default for UnvalidatedCreatePollSettings {
    fn default() -> Self {
        Self {
            title: String::new(),
            options: vec![],
            winner_count: 1,
            write_ins_allowed: false,
            close_after_time: None,
            close_after_votes: None,
        }
    }
}

impl From<CreatePollSettings> for UnvalidatedCreatePollSettings {
    fn from(CreatePollSettings {
        id: _,
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: CreatePollSettings) -> Self {
        Self {
            title,
            options,
            winner_count: winner_count as i32,
            write_ins_allowed,
            close_after_time,
            close_after_votes: close_after_votes.map(|v| v as i32),
        }
    }
}


#[derive(Debug, Deserialize)]
#[serde(try_from = "UnvalidatedUpdatePollSettings")]
pub struct UpdatePollSettings {
    pub title: Option<String>,
    pub winner_count: Option<u8>,
    pub write_ins_allowed: Option<bool>,
    pub close_after_time: Option<Option<DateTime<Utc>>>,
    pub close_after_votes: Option<Option<u32>>,
}

impl Default for UpdatePollSettings {
    fn default() -> Self {
        Self {
            title: None,
            winner_count: None,
            write_ins_allowed: None,
            close_after_time: None,
            close_after_votes: None,
        }
    }
}

impl TryFrom<UnvalidatedUpdatePollSettings> for UpdatePollSettings {
    type Error = error::ValidationError;

    fn try_from(UnvalidatedUpdatePollSettings {
        title,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: UnvalidatedUpdatePollSettings) -> Result<Self, Self::Error> {
        if let Some(title) = &title {
            if !TITLE_LENGTH_BOUNDS.contains(&title.len()) {
                return Err(error::poll_title_invalid_size(TITLE_LENGTH_BOUNDS, title.len()));
            }
        }
        if let Some(winner_count) = winner_count {
            if !WINNERS_BOUNDS.contains(&(winner_count as i32)) {
                return Err(error::poll_winners_limit_exceeded(WINNERS_BOUNDS, winner_count as i32));
            }
        }
        if let Some(otime) = close_after_time {
            if let Some(time) = otime {
                if time < Utc::now() + Duration::from_secs(60) {
                    return Err(error::poll_duration_invalid(1, &time))
                }
            }
        }
        if let Some(ovotes) = close_after_votes {
            if let Some(votes) = ovotes {
                if !VOTES_BOUNDS.contains(&(votes as i64)) {
                    return Err(error::poll_votes_limit_exceeded(VOTES_BOUNDS, votes as i64));
                }
            }
        }

        Ok(UpdatePollSettings {
            title,
            winner_count,
            write_ins_allowed,
            close_after_time,
            close_after_votes,
        })
    }
}


#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct UnvalidatedUpdatePollSettings {
    pub title: Option<String>,
    pub winner_count: Option<u8>,
    pub write_ins_allowed: Option<bool>,

    // absent fields deserialized as None, explicit null values deserialized as Some(None)
    #[serde(deserialize_with = "deserialize_nested_time")]
    pub close_after_time: Option<Option<DateTime<Utc>>>,
    #[serde(deserialize_with = "deserialize_nested_u32")]
    pub close_after_votes: Option<Option<u32>>,
}

impl Default for UnvalidatedUpdatePollSettings {
    fn default() -> Self {
        Self {
            title: None,
            winner_count: None,
            write_ins_allowed: None,
            close_after_time: None,
            close_after_votes: None,
        }
    }
}

fn deserialize_nested_time<'de, D>(deserializer: D) -> Result<Option<Option<DateTime<Utc>>>, D::Error>
where D: serde::Deserializer<'de> {
    let normal: Option<Option<DateTime<Utc>>> = serde::Deserialize::deserialize(deserializer)?;
    match normal {
        None => Ok(Some(None)),
        Some(_) => Ok(normal),
    }
}

fn deserialize_nested_u32<'de, D>(deserializer: D) -> Result<Option<Option<u32>>, D::Error>
where D: serde::Deserializer<'de> {
    let normal: Option<Option<u32>> = serde::Deserialize::deserialize(deserializer)?;
    match normal {
        None => Ok(Some(None)),
        Some(_) => Ok(normal),
    }
}
