use std::time::Duration;

use chrono::{DateTime, OutOfRangeError, TimeDelta, Utc};
use serde::{self, Deserialize, Serialize};
use uuid::Uuid;

use super::id::{Id, RelativeId, WeakId};
use super::user::User;

#[derive(Debug, Deserialize)]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,
    pub options: Vec<String>,

    #[serde(default = "default_winner_count")]
    pub winner_count: u8,
    #[serde(default = "default_write_ins_allowed")]
    pub write_ins_allowed: bool,
    #[serde(default = "default_close_after_time")]
    pub close_after_time: Option<Duration>,
    #[serde(default = "default_close_after_votes")]
    pub close_after_votes: Option<u32>,
}
const fn default_winner_count() -> u8 { 1 }
const fn default_write_ins_allowed() -> bool { false }
const fn default_close_after_time() -> Option<Duration> { None }
const fn default_close_after_votes() -> Option<u32> { None }

#[derive(Serialize)]
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
}

#[derive(Serialize)]
pub struct PollOption {
    pub id: RelativeId,
    pub description: String,
}

impl Poll {
    pub fn new(owner: User, CreatePollSettings {
        id,
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes: close_after_num_votes
    }: CreatePollSettings) -> Result<Poll, OutOfRangeError> {
        let mut poll = Poll {
            id: match id {
                Some(uuid) => Id(uuid),
                None => Id::new(),
            },
            title,
            option_ids: (0..(options.len() as u32)).map(|i| WeakId(i)).collect(),
            options: None,
            winner_count,
            write_ins_allowed,
            close_after_time: match close_after_time {
                None => None,
                Some(duration) => {
                    Utc::now().checked_add_signed(
                        TimeDelta::from_std(duration)?,
                    )
                },
            },
            close_after_votes: close_after_num_votes,

            owner_id: owner.id.clone(),
            owner: Some(owner),
            created_at: Utc::now(),
            closed_at: None,
        };

        let mut full_options: Vec<PollOption> = vec![];
        for (opt_id, text) in poll.option_ids.iter().zip(options.into_iter()) {
            full_options.push(PollOption {
                id: RelativeId(poll.id.clone(), opt_id.clone()),
                description: text,
            });
        }

        poll.options = Some(full_options);

        Ok(poll)
    }
}
