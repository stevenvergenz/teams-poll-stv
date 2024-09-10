use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Serialize};
use uuid::Uuid;

use super::id::{Id, WeakId};
use super::user::User;

mod defaults {
    pub mod title {
        pub fn update() -> Option<String> { None }
    }
    pub mod winner_count {
        pub fn create() -> u8 { 1 }
        pub fn update() -> Option<u8> { None }
    }
    pub mod write_ins_allowed {
        pub fn create() -> bool { false }
        pub fn update() -> Option<bool> { None }
    }
    pub mod close_after_time {
        use chrono::{DateTime, Utc};
        pub fn create() -> Option<DateTime<Utc>> { None }
        pub fn update() -> Option<Option<DateTime<Utc>>> { None }
    }
    pub mod close_after_votes {
        pub fn create() -> Option<u32> { None }
        pub fn update() -> Option<Option<u32>> { None }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,
    pub options: Vec<String>,

    #[serde(default = "defaults::winner_count::create")]
    pub winner_count: u8,
    #[serde(default = "defaults::write_ins_allowed::create")]
    pub write_ins_allowed: bool,
    #[serde(default = "defaults::close_after_time::create")]
    pub close_after_time: Option<DateTime<Utc>>,
    #[serde(default = "defaults::close_after_votes::create")]
    pub close_after_votes: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePollSettings {
    #[serde(default = "defaults::title::update")]
    pub title: Option<String>,
    #[serde(default = "defaults::winner_count::update")]
    pub winner_count: Option<u8>,
    #[serde(default = "defaults::write_ins_allowed::update")]
    pub write_ins_allowed: Option<bool>,
    #[serde(default = "defaults::close_after_time::update")]
    pub close_after_time: Option<Option<DateTime<Utc>>>,
    #[serde(default = "defaults::close_after_votes::update")]
    pub close_after_votes: Option<Option<u32>>,
}

#[derive(Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct PollOption {
    pub id: WeakId,
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
    }: CreatePollSettings) -> Poll {
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
            close_after_time,
            close_after_votes: close_after_num_votes,

            owner_id: owner.id.clone(),
            owner: Some(owner),
            created_at: Utc::now(),
            closed_at: None,
        };

        let mut full_options: Vec<PollOption> = vec![];
        for (opt_id, text) in poll.option_ids.iter().zip(options.into_iter()) {
            full_options.push(PollOption {
                id: opt_id.clone(),
                description: text,
            });
        }

        poll.options = Some(full_options);

        poll
    }
}
