use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::voting;
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
}

impl Poll {
    pub fn into_voting(self) -> voting::Poll {
        voting::Poll {
            id: voting::Id(self.id),
            title: self.title,
            option_ids: vec![],
            options: None,
            winner_count: self.winner_count as u8,
            write_ins_allowed: self.write_ins_allowed,
            close_after_time: self.close_after_time.map(|t| t.and_utc()),
            close_after_votes: self.close_after_votes.map(|v| v as u32),
            owner_id: voting::Id(self.owner_id),
            owner: None,
            created_at: self.created_at.and_utc(),
            closed_at: self.closed_at.map(|t| t.and_utc()),
        }
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
        let poll_settings = Self {
            id: None, // discard any ID provided as input, force random ID from DB
            title,
            winner_count: winner_count as i32,
            write_ins_allowed,
            close_after_time: close_after_time.map(|t| t.naive_utc()),
            close_after_votes: close_after_votes.map(|v| v as i32),
            owner_id: owner_id.clone(),
        };

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

impl UpdatePollSettings {
    pub fn from(voting::UpdatePollSettings {
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

impl PollOption {
    pub fn into_voting(self) -> voting::PollOption {
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

impl User {
    pub fn into_voting(self) -> voting::User {
        voting::User {
            id: voting::Id(self.id),
            display_name: self.display_name,
        }
    }
}

#[derive(Associations, Queryable, Selectable, Identifiable, Insertable)]
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
    pub fn into_voting(self, votes: Vec<Vote>) -> voting::Ballot {
        todo!()
    }
}

#[derive(Associations, Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::votes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Ballot, foreign_key = ballot_id))]
pub struct Vote {
    pub ballot_id: i32,
    pub option: i32,
    pub preference: i32,
}
