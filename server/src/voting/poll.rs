use chrono::{DateTime, Utc};
use serde::{self, Deserialize, Serialize};
use uuid::Uuid;

use super::id::{Id, WeakId};
use super::user::User;

pub fn create_winner_count_default() -> u8 { 1 }
pub fn create_write_ins_allowed_default() -> bool { false }
pub fn create_close_after_time_default() -> Option<DateTime<Utc>> { None }
pub fn create_close_after_votes_default() -> Option<u32> { None }

#[derive(Debug, Deserialize)]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,
    pub options: Vec<String>,

    #[serde(default = "create_winner_count_default")]
    pub winner_count: u8,
    #[serde(default = "create_write_ins_allowed_default")]
    pub write_ins_allowed: bool,
    #[serde(default = "create_close_after_time_default")]
    pub close_after_time: Option<DateTime<Utc>>,
    #[serde(default = "create_close_after_votes_default")]
    pub close_after_votes: Option<u32>,
}

fn update_title_default() -> Option<String> { None }
fn update_winner_count_default() -> Option<u8> { None }
fn update_write_ins_allowed_default() -> Option<bool> { None }
fn update_close_after_time_default() -> Option<Option<DateTime<Utc>>> { None }
fn update_close_after_votes_default() -> Option<Option<u32>> { None }
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


#[derive(Debug, Deserialize)]
pub struct UpdatePollSettings {
    #[serde(default = "update_title_default")]
    pub title: Option<String>,
    #[serde(default = "update_winner_count_default")]
    pub winner_count: Option<u8>,
    #[serde(default = "update_write_ins_allowed_default")]
    pub write_ins_allowed: Option<bool>,
    #[serde(default = "update_close_after_time_default", deserialize_with = "deserialize_nested_time")]
    pub close_after_time: Option<Option<DateTime<Utc>>>,
    #[serde(default = "update_close_after_votes_default", deserialize_with = "deserialize_nested_u32")]
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
