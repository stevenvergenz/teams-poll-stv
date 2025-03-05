use std::convert::Into;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use rand::{self, RngCore};
use serde::Serialize;
use uuid::Uuid;

use crate::voting;
use crate::error::{ContextError, ContextId};
use super::schema;

#[derive(Associations, Identifiable, Queryable, Selectable, Serialize)]
#[diesel(table_name = schema::polls)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = owner_id))]
pub struct Poll {
    pub id: Uuid,
    pub title: String,

    pub winner_count: i32,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<NaiveDateTime>,
    pub close_after_votes: Option<i32>,

    pub owner_id: Uuid,
    pub created_at: NaiveDateTime,
    pub closed_at: Option<NaiveDateTime>,
    pub rng_seed: Vec<u8>,
}

impl Poll {
    pub fn into(self, owner: User, options: Vec<PollOption>) -> voting::Poll {
        let Self {
            id,
            title,
            winner_count,
            write_ins_allowed,
            close_after_time,
            close_after_votes,
            owner_id: _,
            created_at,
            closed_at,
            rng_seed,
        } = self;

        let settings = voting::CreatePollSettings {
            id: Some(id),
            title,
            options: vec![],
            winner_count: winner_count as u8,
            write_ins_allowed,
            close_after_time: close_after_time.map(|t| t.and_utc()),
            close_after_votes: close_after_votes.map(|v| v as u32),
        };

        let mut poll = voting::Poll::new(
            settings,
            options.into_iter().map(|o| o.into()).collect(),
            owner.into(),
            rng_seed,
        );
        poll.close_after_time = close_after_time.map(|t| t.and_utc());
        poll.created_at = created_at.and_utc();
        poll.closed_at = closed_at.map(|t| t.and_utc());

        poll
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::polls)]
pub struct CreatePollSettings {
    pub id: Option<Uuid>,
    pub title: String,

    pub winner_count: i32,
    pub write_ins_allowed: bool,
    pub close_after_time: Option<NaiveDateTime>,
    pub close_after_votes: Option<i32>,

    pub owner_id: Uuid,

    pub rng_seed: Vec<u8>,
}

impl CreatePollSettings {
    pub fn from(owner_id: &Uuid, voting::CreatePollSettings {
        id: _,
        title,
        options,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: voting::CreatePollSettings) -> (Self, Vec<String>) {
        let mut poll_settings = Self {
            id: None, // discard any ID provided as input, force random ID from DB
            title,
            winner_count: winner_count as i32,
            write_ins_allowed,
            close_after_time: close_after_time.map(|t| t.naive_utc()),
            close_after_votes: close_after_votes.map(|v| v as i32),
            owner_id: owner_id.clone(),
            rng_seed: vec![0; 32],
        };
        rand::thread_rng().fill_bytes(&mut poll_settings.rng_seed);

        (poll_settings, options)
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = schema::polls)]
pub struct UpdatePollSettings {
    pub title: Option<String>,
    pub winner_count: Option<i32>,
    pub write_ins_allowed: Option<bool>,
    pub close_after_time: Option<Option<NaiveDateTime>>,
    pub close_after_votes: Option<Option<i32>>,
}

impl From<voting::UpdatePollSettings> for UpdatePollSettings {
    fn from(voting::UpdatePollSettings {
        title,
        winner_count,
        write_ins_allowed,
        close_after_time,
        close_after_votes,
    }: voting::UpdatePollSettings) -> Self {
        Self {
            title,
            winner_count: winner_count.map(|x| x as i32),
            write_ins_allowed,
            close_after_time: close_after_time.map(|odt| {
                odt.map(|dt| {
                    dt.naive_utc()
                })
            }),
            close_after_votes: close_after_votes.map(|ox| {
                ox.map(|x| x as i32)
            }),
        }
    }
}

#[derive(Associations, Identifiable, Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::polloptions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Poll))]
pub struct PollOption {
    pub poll_id: Uuid,
    pub id: i32,
    pub description: String,
}

impl Into<voting::PollOption> for PollOption {
    fn into(self) -> voting::PollOption {
        voting::PollOption {
            id: voting::WeakId(self.id as u32),
            description: self.description,
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Insertable)]
#[diesel(table_name = schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
}

impl Into<voting::User> for User {
    fn into(self) -> voting::User {
        voting::User {
            id: voting::Id(self.id),
            display_name: self.display_name,
        }
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable)]
#[diesel(table_name = schema::ballots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Poll, foreign_key = poll_id))]
pub struct Ballot {
    pub id: i32,
    pub poll_id: Uuid,
    pub user_id: Uuid,
    pub created_at: NaiveDateTime,
}

impl Ballot {
    pub fn try_into(
        db_ballot: Self,
        db_votes: Vec<Vote>,
        db_voter: User,
        poll: voting::Poll,
    ) -> Result<voting::Ballot, ContextError> {
        let mut ballot = voting::UnvalidatedCreateBallot::new();
        for i in 0..db_votes.len() {
            let ov = db_votes.iter().find(|v| v.preference == i as i32);
            if let Some(v) = ov {
                ballot.ranked_preferences.push(voting::WeakId(v.option as u32));
            }
            else {
                return Err(ContextError::ballot_incomplete_selection(i, ContextId::I32(db_ballot.id)));
            }
        }

        Ok(voting::Ballot::new(db_voter.into(), ballot.validate(poll)?))
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::ballots)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CreateBallot {
    poll_id: Uuid,
    user_id: Uuid,
}

impl CreateBallot {
    pub fn new(poll_id: Uuid, user_id: Uuid) -> Self {
        Self { poll_id, user_id }
    }
}

#[derive(Associations, Queryable, Selectable, Insertable, Serialize)]
#[diesel(table_name = schema::votes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Ballot, foreign_key = ballot_id))]
pub struct Vote {
    pub ballot_id: i32,
    pub preference: i32,
    pub option: i32,
}
