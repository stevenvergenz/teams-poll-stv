use std::convert::TryInto;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::voting;
use crate::error;
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

impl TryInto<voting::Poll> for (Poll, Vec<PollOption>, User) {
    type Error = error::ValidationError;
    fn try_into(self) -> Result<voting::Poll, Self::Error> {
        let (Poll {
            id,
            title,
            winner_count,
            write_ins_allowed,
            close_after_time,
            close_after_votes,
            owner_id: _,
            created_at,
            closed_at,
        }, options, owner) = self;

        // re-validate timeless settings
        let settings = voting::UnvalidatedCreatePollSettings {
            title, winner_count, write_ins_allowed, close_after_votes,
            ..voting::UnvalidatedCreatePollSettings::from(voting::CreatePollSettings::default())
        };
        let mut settings = match voting::CreatePollSettings::try_from(settings) {
            Err(err) => {
                return Err(err.with_context("poll", error::ContextId::Uuid(id)))
            },
            Ok(p) => p,
        };
        settings.id = Some(id);

        // straight-up copy unvalidated, generated, or timely elements
        let mut poll = voting::Poll::new(
            settings,
            options.into_iter().map(|o| o.into()).collect(),
            owner.into(),
        );
        poll.close_after_time = close_after_time.map(|t| t.and_utc());
        poll.created_at = created_at.and_utc();
        poll.closed_at = closed_at.map(|t| t.and_utc());

        Ok(poll)
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

impl TryInto<voting::Ballot> for (Ballot, Vec<Vote>, User, voting::Poll) {
    type Error = error::ValidationError;
    fn try_into(self) -> Result<voting::Ballot, error::ValidationError> {
        let (db_ballot, db_votes, db_voter, poll) = self;

        let mut ballot = voting::UnvalidatedCreateBallot::new();
        for i in 0..db_votes.len() {
            let ov = db_votes.iter().find(|v| v.preference == i as i32);
            if let Some(v) = ov {
                ballot.ranked_preferences.push(voting::WeakId(v.option as u32));
            }
            else {
                return Err(error::ballot_incomplete_selection(i)
                    .with_context("ballot", error::ContextId::I32(db_ballot.id)));
            }
        }

        let ballot = match voting::CreateBallot::try_from((ballot, poll)) {
            Err(err) => {
                return Err(err.with_context("ballot", error::ContextId::I32(db_ballot.id)))
            },
            Ok(b) => b,
        };

        Ok(voting::Ballot::new(db_voter.into(), ballot))
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

#[derive(Associations, Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::votes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(belongs_to(Ballot, foreign_key = ballot_id))]
pub struct Vote {
    pub ballot_id: i32,
    pub preference: i32,
    pub option: i32,
}
